use std::collections::HashMap;

use zbus::{names::OwnedBusName, xml::Node};

pub enum DbusMessage {
    GetObjects(OwnedBusName),
    ServiceRequest(),
}
pub enum AppMessage {
    Objects(HashMap<String, Node>),
    Services(Vec<OwnedBusName>),
}
