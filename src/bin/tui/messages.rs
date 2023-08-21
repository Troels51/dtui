use zbus::{names::OwnedBusName, xml::Node};

pub enum DbusMessage {
    GetObjects(OwnedBusName),
    ServiceRequest(),
}
pub enum AppMessage {
    Objects(Node),
    Services(Vec<OwnedBusName>),
}
