use std::collections::HashMap;

use zbus::{
    names::{OwnedBusName, OwnedInterfaceName, OwnedMemberName},
    zvariant::{OwnedObjectPath, OwnedValue},
    Message,
};
use zbus_xml::Node;

pub enum DbusMessage {
    GetObjects(OwnedBusName),
    ServiceRequest(),
    MethodCallRequest(
        OwnedBusName,
        OwnedObjectPath,
        OwnedInterfaceName,
        OwnedMemberName,
        Vec<OwnedValue>,
    ),
}
pub enum AppMessage {
    Objects((OwnedBusName, HashMap<String, Node<'static>>)), // Service name + Map of (Object names, node)
    Services(Vec<OwnedBusName>),
    MethodCallResponse(OwnedMemberName, Message),
}
