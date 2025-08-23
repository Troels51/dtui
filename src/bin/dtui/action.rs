use serde::{Deserialize, Serialize};
use strum::Display;
use zbus_names::OwnedBusName;

use crate::{app::Focus, stateful_tree::OwnedMethod};

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
    Up,
    Down,
    DownTree, // Go further into an object tree
    UpTree,   // Go up an object tree
    GetService,
    InvokeDbus, // Call method, get property, set property, listen to signal
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


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvokableDbusMember {
    Method {
        method: OwnedMethod,
    },
    Property {
        property: zbus_names::OwnedPropertyName,
    },
    Signal {
        name: zbus_names::OwnedMemberName,
    },
}