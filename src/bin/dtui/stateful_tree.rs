use std::collections::HashMap;

use itertools::Itertools;
use tui_tree_widget::{TreeItem, TreeState};
use zbus_xml::{ArgDirection, Method, Node};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DbusIdentifier {
    Object(String),            // ObjectPath
    Interface(String),         // InterfaceName
    Member(MemberTypes),       // Can be Method, Properties, Signals
    Method(MethodDescription), // zbus_name::MemberName
    Property(String),          // zbus_name::PropertyName
    Signal(String),            // zbus_name::MemberName
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodDescription(pub Method<'static>);

// Rely on PartialEq
impl Eq for MethodDescription {}

impl std::hash::Hash for MethodDescription {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.name().hash(state);
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
                        .map(|arg| format!("{}: {}", arg.name().unwrap_or_default(), arg.ty()))
                        .collect();
                    let outputs: Vec<String> = method
                        .args()
                        .iter()
                        .filter(|arg| arg.direction().is_some_and(|s| s == ArgDirection::Out))
                        .map(|arg| format!("{}: {}", arg.name().unwrap_or_default(), arg.ty()))
                        .collect();
                    let return_arrow = if outputs.is_empty() { "" } else { "=>" }; // If we dont return anything, the arrow shouldnt be there
                    let leaf_string: String = format!(
                        "{}({}) {} {}",
                        method.name(),
                        inputs.join(", "),
                        return_arrow,
                        outputs.join(", ")
                    );
                    TreeItem::new_leaf(
                        DbusIdentifier::Method(MethodDescription(method)),
                        leaf_string,
                    )
                })
                .collect();
            let properties: Vec<TreeItem<DbusIdentifier>> = interface
                .properties()
                .iter()
                .map(|property| {
                    TreeItem::new_leaf(
                        DbusIdentifier::Property(property.name().to_string()),
                        format!("{}: {}", property.name(), property.ty()),
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
                        .map(|arg| format!("{}: {}", arg.name().unwrap_or_default(), arg.ty()))
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
