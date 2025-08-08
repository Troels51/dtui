use std::{collections::HashMap, fmt::Display};

use serde::{Deserialize, Serialize};
use strum::Display;
use zbus::{
    names::{OwnedBusName, OwnedInterfaceName, OwnedMemberName},
    zvariant::{OwnedObjectPath, OwnedValue},
    Message,
};
use zbus_xml::Node;

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
}
/// Message from the Dbus Actor to the App.
/// TODO: Needs better name, or it needs to be refactored into Action
#[derive(Debug, Clone)]
pub enum AppMessage {
    Objects((OwnedBusName, HashMap<String, Node<'static>>)), // Service name + Map of (Object names, node)
    Services(Vec<OwnedBusName>),
    MethodCallResponse(OwnedMemberName, zbus::Message),
}
