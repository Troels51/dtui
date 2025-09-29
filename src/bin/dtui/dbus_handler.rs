use std::{collections::HashMap, error::Error, io::BufReader};

use async_recursion::async_recursion;
use tokio::sync::mpsc::{self, Receiver, UnboundedSender};
use zbus::{
    Connection,
    names::{OwnedBusName, OwnedInterfaceName, OwnedMemberName},
    zvariant::{ObjectPath, OwnedValue, Str, StructureBuilder},
};
use zbus_xml::Node;

use crate::messages::{AppMessage, DbusError, DbusMessage, InvocationResponse};

pub struct DbusActor {
    app_sender: UnboundedSender<AppMessage>,
    app_receiver: Receiver<DbusMessage>,
    connection: Connection,
}
impl DbusActor {
    pub fn new(
        app_sender: UnboundedSender<AppMessage>,
        app_receiver: Receiver<DbusMessage>,
        connection: Connection,
    ) -> Self {
        Self {
            app_sender,
            app_receiver,
            connection,
        }
    }
    async fn get_node(
        &self,
        service_name: &OwnedBusName,
        path: &ObjectPath<'_>,
    ) -> Result<Node<'static>, Box<dyn Error + Sync + Send>> {
        let introspectable_proxy = zbus::fdo::IntrospectableProxy::builder(&self.connection)
            .destination(service_name)?
            .path(path.clone())?
            .build()
            .await?;
        let introspect_xml: String = introspectable_proxy.introspect().await?;
        let introspect = Node::from_reader(BufReader::new(introspect_xml.as_bytes()))?;

        Ok(introspect)
    }

    #[async_recursion]
    async fn get_sub_nodes(
        &self,
        service_name: &OwnedBusName,
        path: &ObjectPath<'async_recursion>,
    ) -> Result<HashMap<String, Node<'static>>, Box<dyn Error + Send + Sync>> {
        let node = self.get_node(service_name, path).await?;
        let mut result = HashMap::new();

        for sub_node in node.nodes() {
            if let Some(name) = sub_node.name() {
                let path_name = if path.as_str().ends_with('/') {
                    path.as_str().to_string() + name
                } else {
                    path.as_str().to_string() + "/" + name
                };
                let sub_path = ObjectPath::try_from(path_name)?;
                result.extend(self.get_sub_nodes(service_name, &sub_path).await?);
            }
        }
        result.insert(path.to_string(), node);
        Ok(result)
    }

    pub async fn handle_message(&mut self, msg: DbusMessage) {
        tracing::info!("Handle message {:?}", msg);
        match msg {
            DbusMessage::GetObjects(service_name) => {
                let path_name = "/".to_string();
                let path = ObjectPath::try_from(path_name).expect("/ is always a valid path");

                if let Ok(nodes) = self.get_sub_nodes(&service_name, &path).await {
                    self.app_sender
                        .send(AppMessage::Objects((service_name, nodes)))
                        .expect("channel dead");
                }
            }
            DbusMessage::ServiceRequest() => {
                let proxy = zbus::fdo::DBusProxy::new(&self.connection)
                    .await
                    .expect("Could not create DbusProxy");
                if let Ok(names) = proxy.list_names().await {
                    let _ = self.app_sender.send(AppMessage::Services(names));
                }
            }
            DbusMessage::MethodCallRequest(service, object_path, interface, method, values) => {
                let mut body = StructureBuilder::new();
                let is_empty = values.is_empty();
                for v in values {
                    body.push_value(v.into());
                }
                let method_call_response = if !is_empty {
                    self.connection
                        .call_method(
                            Some(service.clone()),
                            object_path.clone(),
                            Some(interface.clone()),
                            method.clone(),
                            &body.build().unwrap(),
                        )
                        .await
                } else {
                    self.connection
                        .call_method(
                            Some(service.clone()),
                            object_path.clone(),
                            Some(interface.clone()),
                            method.clone(),
                            &(),
                        )
                        .await
                };
                match method_call_response {
                    Ok(message) => {
                        let _ = self.app_sender.send(AppMessage::InvocationResponse(
                            InvocationResponse {
                                service,
                                object_path,
                                method_name: method,
                                interface,
                                message,
                            },
                        ));
                    }
                    Err(e) => 
                    {
                        tracing::info!("Method call error {}", e); 
                        let _ = self.app_sender.send(AppMessage::Error(
                            DbusError {
                                message: e.to_string(),
                            },
                        ));
                    }
                };
            }
        }
    }
}

async fn run_actor(mut actor: DbusActor) {
    while let Some(msg) = actor.app_receiver.recv().await {
        actor.handle_message(msg).await
    }
}

#[derive(Clone, Debug)]
pub struct DbusActorHandle {
    sender: mpsc::Sender<DbusMessage>,
}

impl DbusActorHandle {
    pub fn new(app_sender: UnboundedSender<AppMessage>, connection: Connection) -> Self {
        let (sender, receiver) = mpsc::channel(8);
        let actor = DbusActor::new(app_sender, receiver, connection);
        tokio::spawn(run_actor(actor));

        Self { sender }
    }

    pub async fn request_objects_from(&self, object: OwnedBusName) {
        let msg = DbusMessage::GetObjects(object);
        let _ = self.sender.send(msg).await;
    }

    pub async fn request_services(&self) {
        let msg = DbusMessage::ServiceRequest();
        let _ = self.sender.send(msg).await;
    }

    pub async fn call_method(
        &self,
        service: OwnedBusName,
        object: zbus::zvariant::OwnedObjectPath,
        interface: OwnedInterfaceName,
        method: OwnedMemberName,
        values: Vec<OwnedValue>,
    ) {
        let msg = DbusMessage::MethodCallRequest(service, object, interface, method, values);
        let _ = self.sender.send(msg).await;
    }

    pub async fn get_property(
        &self,
        service: OwnedBusName,
        object: zbus::zvariant::OwnedObjectPath,
        interface: OwnedInterfaceName,
        property_name: String,
    ) {
        let property_interface = OwnedInterfaceName::try_from("org.freedesktop.DBus.Properties")
            .expect("org.freedesktop.Dbus.Properties is valid interface name");
        let method = OwnedMemberName::try_from("Get").expect("Get is a valid Method");
        let mut values: Vec<zbus::zvariant::OwnedValue> = Vec::new();
        values.push(
            OwnedValue::try_from(interface.clone()).expect("OwnedInterfaceName is valid value"),
        );
        values.push(OwnedValue::from(Str::from(property_name)));

        let msg =
            DbusMessage::MethodCallRequest(service, object, property_interface, method, values);
        let _ = self.sender.send(msg).await;
    }

    pub async fn set_property(
        &self,
        service: OwnedBusName,
        object: zbus::zvariant::OwnedObjectPath,
        interface: OwnedInterfaceName,
        property_name: zbus_names::OwnedPropertyName,
        value: OwnedValue,
    ) {
        let property_interface = OwnedInterfaceName::try_from("org.freedesktop.DBus.Properties")
            .expect("org.freedesktop.Dbus.Properties is valid interface name");
        let method = OwnedMemberName::try_from("Set").expect("Set is a valid Method");
        let mut values: Vec<zbus::zvariant::OwnedValue> = Vec::new();
        values.push(
            OwnedValue::try_from(interface.clone()).expect("OwnedInterfaceName is valid value"),
        );
        values.push(OwnedValue::from(Str::from(property_name)));
        values.push(value);

        let msg =
            DbusMessage::MethodCallRequest(service, object, property_interface, method, values);
        let _ = self.sender.send(msg).await;
    }
}
