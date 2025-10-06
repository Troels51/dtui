use std::collections::HashMap;

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tui_tree_widget::{TreeItem, TreeState};
use zbus_names::{OwnedMemberName, OwnedPropertyName};
use zbus_xml::{Annotation, Arg, ArgDirection, Method, Node, Property};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemberTypes {
    Methods,
    Properties,
    Signals,
}
// This enum encodes where we are in the GUI tree
// > Objects
//  > Interfaces
//   > Member (Aka one of Method/Property/Signal)
//    > Methods/Properties/Signals (The actual list of the methods)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DbusIdentifier {
    Object(String),          // ObjectPath
    Interface(String),       // InterfaceName
    Member(MemberTypes),     // Can be Method, Properties, Signals
    Method(OwnedMethod),     // zbus_name::MemberName
    Property(OwnedProperty), // zbus_name::PropertyName
    Signal(String),          // zbus_name::MemberName
}

// OwnedMethod and OwnedProperty is zbus_xml::Method/Property but owned
//TODO: Consider moving getting this or something similar into zbus_xml
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OwnedMethod {
    pub(crate) name: OwnedMemberName,
    pub(crate) args: Vec<Arg>,
    pub(crate) annotations: Vec<Annotation>,
}

impl From<Method<'_>> for OwnedMethod {
    fn from(value: Method<'_>) -> Self {
        OwnedMethod {
            name: value.name().to_owned().into(),
            args: value.args().to_owned(),
            annotations: value.annotations().to_owned(),
        }
    }
}

impl OwnedMethod {
    pub fn name(&self) -> &OwnedMemberName {
        &self.name
    }
    pub fn args(&self) -> &Vec<Arg> {
        &self.args
    }
    pub fn annotations(&self) -> &Vec<Annotation> {
        &self.annotations
    }
}

// Rely on PartialEq
impl Eq for OwnedMethod {}

impl std::hash::Hash for OwnedMethod {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name().hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OwnedProperty {
    pub(crate) name: OwnedPropertyName,
    pub(crate) ty: zbus_xml::Signature,
    pub(crate) access: zbus_xml::PropertyAccess,
    pub(crate) annotations: Vec<Annotation>,
}

impl From<Property<'_>> for OwnedProperty {
    fn from(value: Property<'_>) -> Self {
        Self {
            name: value.name().to_owned().into(),
            ty: value.ty().to_owned(),
            access: value.access().to_owned(),
            annotations: value.annotations().to_owned(),
        }
    }
}

impl OwnedProperty {
    pub fn name(&self) -> &OwnedPropertyName {
        &self.name
    }
    pub fn ty(&self) -> &zbus_xml::Signature {
        &self.ty
    }
    pub fn access(&self) -> &zbus_xml::PropertyAccess {
        &self.access
    }
    pub fn annotations(&self) -> &Vec<Annotation> {
        &self.annotations
    }
}

// Rely on PartialEq
impl Eq for OwnedProperty {}

impl std::hash::Hash for OwnedProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name().hash(state);
    }
}

impl Default for DbusIdentifier {
    fn default() -> Self {
        DbusIdentifier::Object("/".to_string())
    }
}

#[derive(Debug)]
pub struct StatefulTree {
    pub state: TreeState<DbusIdentifier>,
    pub items: Vec<TreeItem<'static, DbusIdentifier>>,
}

impl Default for StatefulTree {
    fn default() -> Self {
        Self::new()
    }
}

impl StatefulTree {
    pub fn new() -> Self {
        Self {
            state: TreeState::default(),
            items: Vec::new(),
        }
    }

    pub fn from_nodes(nodes: HashMap<String, Node<'static>>) -> Self {
        let items = nodes
            .iter()
            .sorted_by(|a, b| a.0.cmp(b.0))
            .map(|(object_name, node)| -> TreeItem<DbusIdentifier> {
                let children = node_to_treeitems(node);
                TreeItem::new(
                    DbusIdentifier::Object(object_name.clone()),
                    object_name.clone(),
                    children,
                )
                .unwrap()
            })
            .collect();
        Self {
            state: TreeState::default(),
            items,
        }
    }

    pub fn down(&mut self) {
        self.state.key_down();
    }

    pub fn up(&mut self) {
        self.state.key_up();
    }

    pub fn left(&mut self) {
        self.state.key_left();
    }

    pub fn right(&mut self) {
        self.state.key_right();
    }

    pub fn toggle(&mut self) {
        self.state.toggle_selected();
    }
}

fn node_to_treeitems(node: &zbus_xml::Node<'static>) -> Vec<TreeItem<'static, DbusIdentifier>> {
    let children: Vec<TreeItem<DbusIdentifier>> = node
        .interfaces()
        .iter()
        .map(|interface| {
            let methods: Vec<TreeItem<DbusIdentifier>> = interface
                .methods()
                .iter()
                .cloned()
                .map(|method| {
                    let inputs: Vec<String> = method
                        .args()
                        .iter()
                        .filter(|arg| arg.direction().is_some_and(|s| s == ArgDirection::In))
                        .map(|arg| {
                            format!(
                                "{}: {}",
                                arg.name().unwrap_or_default(),
                                arg.ty().to_string()
                            )
                        })
                        .collect();
                    let outputs: Vec<String> = method
                        .args()
                        .iter()
                        .filter(|arg| arg.direction().is_some_and(|s| s == ArgDirection::Out))
                        .map(|arg| {
                            format!(
                                "{}: {}",
                                arg.name().unwrap_or_default(),
                                arg.ty().to_string()
                            )
                        })
                        .collect();
                    let return_arrow = if outputs.is_empty() { "" } else { "=>" }; // If we dont return anything, the arrow shouldnt be there
                    let leaf_string: String = format!(
                        "{}({}) {} {}",
                        method.name(),
                        inputs.join(", "),
                        return_arrow,
                        outputs.join(", ")
                    );
                    TreeItem::new_leaf(DbusIdentifier::Method(method.into()), leaf_string)
                })
                .collect();
            let properties: Vec<TreeItem<DbusIdentifier>> = interface
                .properties()
                .iter()
                .map(|property| {
                    TreeItem::new_leaf(
                        DbusIdentifier::Property(property.clone().into()),
                        format!("{}: {}", property.name(), property.ty().to_string()),
                    )
                })
                .collect();
            let signals: Vec<TreeItem<DbusIdentifier>> = interface
                .signals()
                .iter()
                .map(|signal| {
                    // Signals can only have input parameters
                    let inputs: Vec<String> = signal
                        .args()
                        .iter()
                        .filter(|arg| arg.direction().is_some_and(|s| s == ArgDirection::In))
                        .map(|arg| {
                            format!(
                                "{}: {}",
                                arg.name().unwrap_or_default(),
                                arg.ty().to_string()
                            )
                        })
                        .collect();
                    let leaf_string: String = format!("{}({})", signal.name(), inputs.join(", "));
                    TreeItem::new_leaf(
                        DbusIdentifier::Signal(signal.name().to_string()),
                        leaf_string,
                    )
                })
                .collect();
            // let annotations: Vec<TreeItem> = interface
            //     .annotations()
            //     .iter()
            //     .map(|annotation| {
            //         TreeItem::new_leaf(annotation.name().to_string())
            //     })
            //     .collect();
            let methods_tree = TreeItem::new(
                DbusIdentifier::Member(MemberTypes::Methods),
                "Methods",
                methods,
            )
            .expect("Methods should have different ids");
            let properties_tree = TreeItem::new(
                DbusIdentifier::Member(MemberTypes::Properties),
                "Properties",
                properties,
            )
            .expect("Properties should have different ids");
            let signals_tree = TreeItem::new(
                DbusIdentifier::Member(MemberTypes::Signals),
                "Signals",
                signals,
            )
            .expect("Signals should have different ids");
            // let annotations_tree =
            //     TreeItem::new("Annotations", annotations);
            // TODO: Annotations are used differently, so i dont want to waste space with it
            TreeItem::new(
                DbusIdentifier::Interface(interface.name().to_string()),
                interface.name().to_string(),
                vec![
                    methods_tree,
                    properties_tree,
                    signals_tree,
                    // annotations_tree,
                ],
            )
            .unwrap()
        })
        .collect();

    children
}
