use std::collections::HashMap;

use zbus::names::OwnedBusName;
use zbus_xml::Node;

pub enum DbusMessage {
    GetObjects(OwnedBusName),
    ServiceRequest(),
}
pub enum AppMessage {
    Objects(HashMap<String, Node<'static>>),
    Services(Vec<OwnedBusName>),
}
