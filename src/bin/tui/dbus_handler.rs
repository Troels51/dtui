use std::str::FromStr;

use tokio::sync::mpsc::{self, Receiver, Sender};
use zbus::{Connection, names::OwnedBusName, xml::Node, zvariant::ObjectPath};

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
            app_sender: app_sender,
            app_receiver: app_receiver,
            connection: connection,
        }
    }
    async fn get_objects(&self, service_name: OwnedBusName) -> Result<Node, zbus::Error> {
        let path_name = "/".to_string();
        let path = ObjectPath::try_from(path_name)?;
        let introspectable_proxy = zbus::fdo::IntrospectableProxy::builder(&self.connection)
            .destination(service_name)?
            .path(path.clone())?
            .build()
            .await?;
        let introspect_xml = introspectable_proxy.introspect().await?;
        let introspect = Node::from_str(&introspect_xml)?;
        Ok(introspect)
    }
    pub async fn handle_message(&mut self, msg: DbusMessage) {
        match msg {
            DbusMessage::GetObjects(path) => {
                if let Ok(objects) = self.get_objects(path).await {
                    self.app_sender
                        .send(AppMessage::Objects(objects))
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
