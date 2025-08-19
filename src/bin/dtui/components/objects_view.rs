use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use tracing::info;
use tui_tree_widget::Tree;
use zbus::zvariant::OwnedObjectPath;
use zbus_names::{OwnedBusName, OwnedInterfaceName};

use super::Component;
use crate::{
    action::{Action, MethodCall}, app::Focus, config::Config, other::active_area_border_color, stateful_tree::{OwnedMethod, StatefulTree}
};

#[derive(Default)]
pub struct ObjectsView {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    objects: StatefulTree,
    originating_service: Option<OwnedBusName>,
    active: bool,
}

impl ObjectsView {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Component for ObjectsView {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        Ok(())
    }

    async fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Focus(focus) => {
                self.active = focus == Focus::Objects;
            },
            _ => (),
        }
        if self.active {
            match action {
                Action::Up => {
                    self.objects.up();
                }
                Action::Down => {
                    self.objects.down();
                }
                Action::DownTree => {
                    self.objects.right();
                }
                Action::UpTree => {
                    self.objects.left();
                }
                Action::InvokeDbus => {
                    info!("Invoking dbus method or property {:?}", self.objects.state.selected());
                    if let Some(full_description) = extract_description(self.objects.state.selected())
                        {
                            return Ok(Some(Action::StartDbusMethodCall(MethodCall{
                                service: self.originating_service.clone().expect("Must have an originating service if an object is being selected"),
                                object: full_description.0,
                                interface: full_description.1,
                                method_description: full_description.2,
                            })));
                        }
                }
                _ => {}
            }
        }

        Ok(None)
    }
    fn update_from_dbus(
        &mut self,
        dbus_action: crate::messages::AppMessage,
    ) -> Result<Option<Action>> {
        match dbus_action {
            crate::messages::AppMessage::Objects((service_name, objects)) => {
                info!("Got objects from service: {}", service_name);
                self.originating_service = Some(service_name);
                self.objects = StatefulTree::from_nodes(objects);
            }
            crate::messages::AppMessage::Services(owned_bus_names) => (),
            crate::messages::AppMessage::MethodCallResponse(owned_member_name, message) => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let objects_view = Tree::new(&self.objects.items)
            .unwrap()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(active_area_border_color(self.active)))
                    .title("Objects"),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(">> ");
        frame.render_stateful_widget(objects_view, area, &mut self.objects.state);
        Ok(())
    }

}



/// Takes a stateful_tree::DbusIdentifier, which is an identifier for where a node is in the UI tree
/// and if the selection is a method, it will extract the path, interface name and method description
/// Otherwise it returns None
fn extract_description(
    selected: &[crate::stateful_tree::DbusIdentifier],
) -> Option<(OwnedObjectPath, OwnedInterfaceName, OwnedMethod)> {
    let object_path = selected
        .iter()
        .filter_map(|identifier| match identifier {
            crate::stateful_tree::DbusIdentifier::Object(o) => Some(o),
            _ => None,
        })
        .next();
    let interface_name = selected
        .iter()
        .filter_map(|identifier| match identifier {
            crate::stateful_tree::DbusIdentifier::Interface(i) => Some(i),
            _ => None,
        })
        .next();
    let member_name = selected
        .iter()
        .filter_map(|identifier| match identifier {
            crate::stateful_tree::DbusIdentifier::Method(m) => Some(m),
            _ => None,
        })
        .next();
    if object_path.is_some() && interface_name.is_some() && member_name.is_some() {
        Some((
            OwnedObjectPath::try_from(object_path.unwrap().clone()).unwrap(),
            OwnedInterfaceName::try_from(interface_name.unwrap().clone()).unwrap(),
            member_name.unwrap().clone(),
        ))
    } else {
        None
    }
}
