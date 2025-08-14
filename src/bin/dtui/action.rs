use serde::{Deserialize, Serialize};
use strum::Display;
use zbus_names::OwnedBusName;

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
    Up,
    Down,
    DownTree, // Go further into an object tree
    UpTree,   // Go up an object tree
    GetService,
    InvokeDbus, // Call method, get property, set property, listen to signal
    StartDbusMethodCall(MethodCall),

    // Call view
    CallActiveMethod,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MethodCall {
    pub(crate) service: OwnedBusName,
    pub(crate) object: zbus::zvariant::OwnedObjectPath,
    pub(crate) interface: zbus_names::OwnedInterfaceName,
    pub(crate) method_description: crate::stateful_tree::OwnedMethod,
}
