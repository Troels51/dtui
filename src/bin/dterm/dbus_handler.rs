use std::{collections::HashMap, str::FromStr};

use async_recursion::async_recursion;
use tokio::sync::mpsc::{self, Receiver, Sender};
use zbus::{names::OwnedBusName, xml::Node, zvariant::ObjectPath, Connection};

use crate::messages::{AppMessage, DbusMessage};

pub struct DbusActor {
    app_sender: Sender<AppMessage>,
    app_receiver: Receiver<DbusMessage>,
    connection: Connection,
}
impl DbusActor {
    pub fn new(
        app_sender: Sender<AppMessage>,
        app_receiver: Receiver<DbusMessage>,
        connection: Connection,
    ) -> Self {
        Self {
            app_sender,
            app_receiver,
            connection,
        }
    }
    async fn get_node<'a>(
        &self,
        service_name: &OwnedBusName,
        path: &ObjectPath<'a>,
    ) -> Result<Node, zbus::Error> {
        let introspectable_proxy = zbus::fdo::IntrospectableProxy::builder(&self.connection)
            .destination(service_name)?
            .path(path.clone())?
            .build()
            .await?;
        let introspect_xml = introspectable_proxy.introspect().await?;
        let introspect = Node::from_str(&introspect_xml)?;
        Ok(introspect)
    }
    #[async_recursion]
    async fn get_sub_nodes(
        &self,
        service_name: &OwnedBusName,
        path: &ObjectPath<'async_recursion>,
    ) -> Result<HashMap<String, Node>, zbus::Error> {
        let mut result = HashMap::new();
        let node = self.get_node(service_name, path).await?;

        for sub_node in node.nodes() {
            if let Some(name) = sub_node.name() {
                let path_name = if path.as_str().ends_with('/') {
                    path.as_str().to_string() + name
                } else {
                    path.as_str().to_string() + "/" + name
                };
                let sub_path = ObjectPath::try_from(path_name)?;
                result.extend(self.get_sub_nodes(service_name, &sub_path).await?)
            }
        }
        result.insert(path.to_string(), node);
        Ok(result)
    }

    pub async fn handle_message(&mut self, msg: DbusMessage) {
        match msg {
            DbusMessage::GetObjects(service_name) => {
                let path_name = "/".to_string();
                let path = ObjectPath::try_from(path_name).expect("/ is always a valid path");
                if let Ok(nodes) = self.get_sub_nodes(&service_name, &path).await {
                    self.app_sender
                        .send(AppMessage::Objects(nodes))
                        .await
                        .expect("channel dead");
                }
            }
            DbusMessage::ServiceRequest() => {
                let proxy = zbus::fdo::DBusProxy::new(&self.connection)
                    .await
                    .expect("Could not create DbusProxy");
                if let Ok(names) = proxy.list_names().await {
                    let _ = self.app_sender.send(AppMessage::Services(names)).await;
                }
            }
        }
    }
}

async fn run_actor(mut actor: DbusActor) {
    while let Some(msg) = actor.app_receiver.recv().await {
        actor.handle_message(msg).await
    }
}

#[derive(Clone)]
pub struct DbusActorHandle {
    sender: mpsc::Sender<DbusMessage>,
}

impl DbusActorHandle {
    pub fn new(app_sender: Sender<AppMessage>, connection: Connection) -> Self {
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
}
