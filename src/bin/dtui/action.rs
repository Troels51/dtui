use serde::{Deserialize, Serialize};
use strum::Display;
use zbus_names::{BusName, OwnedBusName};

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
    UpTree, // Go up an object tree
    GetService,
    InvokeDbus, // Call method, get property, set property, listen to signal

    
}
