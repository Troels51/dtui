use std::collections::HashMap;

use zbus::names::OwnedBusName;
use zbus_xml::Node;

pub enum DbusMessage {
    GetObjects(OwnedBusName),
    ServiceRequest(),
}
pub enum AppMessage {
    Objects((OwnedBusName, HashMap<String, Node<'static>>)), // Service name + Map of (Object names, node)
    Services(Vec<OwnedBusName>),
}
