use std::collections::HashMap;

use zbus::{
    names::{OwnedBusName, OwnedInterfaceName, OwnedMemberName},
    zvariant::{OwnedObjectPath, OwnedValue},
};
use zbus_xml::Node;

use crate::action::Invocation;

#[derive(Debug)]
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
    Invoke(Invocation)
}
/// Message from the Dbus Actor to the App.
/// TODO: Needs better name, or it needs to be refactored into Action
#[derive(Debug, Clone)]
pub enum AppMessage {
    Objects((OwnedBusName, HashMap<String, Node<'static>>)), // Service name + Map of (Object names, node)
    Services(Vec<OwnedBusName>),
    MethodCallResponse(OwnedMemberName, zbus::Message),
}
