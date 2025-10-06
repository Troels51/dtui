use serde::{Deserialize, Serialize};
use strum::Display;
use zbus_names::OwnedBusName;

use crate::{
    app::Focus,
    stateful_tree::{OwnedMethod, OwnedProperty},
};

#[derive(Default, Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize, Copy)]
pub enum EditorMode {
    #[default]
    Normal,
    Insert,
}
#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize, Copy)]
pub enum DbusInvocationAction {
    CallMethod,
    GetProperty,
    SetProperty,
    SubscribeSignal,
}

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,
    Error(String),
    Help,

    NextFocus,
    Focus(Focus),
    EditorMode(EditorMode),
    Up,
    Down,
    DownTree, // Go further into an object tree
    UpTree,   // Go up an object tree
    GetService,
    InvokeDbus(DbusInvocationAction), // Call method, get property, set property, listen to signal. TODO: Rename
    StartDbusInvocation(Invocation),

    // Call view
    CallActiveMethod,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invocation {
    pub(crate) service: OwnedBusName,
    pub(crate) object: zbus::zvariant::OwnedObjectPath,
    pub(crate) interface: zbus_names::OwnedInterfaceName,
    pub(crate) invocation_description: InvokableDbusMember,
}
impl Invocation {
    pub(crate) fn nr_args(&self) -> usize {
        match &self.invocation_description {
            InvokableDbusMember::Method { method } => method.args().len(),
            InvokableDbusMember::Property { property } => 1,
            InvokableDbusMember::Signal { name } => 1,
        }
    }

    pub(crate) fn input_count(&self) -> usize {
        match &self.invocation_description {
            InvokableDbusMember::Method { method } => method
                .args()
                .iter()
                .filter(|arg| match arg.direction() {
                    Some(direction) => match direction {
                        zbus_xml::ArgDirection::In => true,
                        zbus_xml::ArgDirection::Out => false,
                    },
                    None => false,
                })
                .count(),
            InvokableDbusMember::Property { property } => 1,
            InvokableDbusMember::Signal { name } => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvokableDbusMember {
    Method { method: OwnedMethod },
    Property { property: OwnedProperty },
    Signal { name: zbus_names::OwnedMemberName },
}
