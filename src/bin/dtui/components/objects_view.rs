use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use tracing::info;
use tui_tree_widget::Tree;
use zbus::zvariant::{dbus, OwnedObjectPath};
use zbus_names::{OwnedBusName, OwnedInterfaceName, OwnedMemberName, OwnedPropertyName};

use super::Component;
use crate::{
    action::{Action, Invocation}, app::Focus, config::Config, dbus_handler::DbusActorHandle, other::active_area_border_color, stateful_tree::{self, OwnedMethod, StatefulTree}
};

#[derive(Default)]
pub struct ObjectsView {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    objects: StatefulTree,
    originating_service: Option<OwnedBusName>,
    dbus_actor_handle: Option<DbusActorHandle>,
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
    fn register_dbus_actor_handle(&mut self, dbus_actor_handle: DbusActorHandle) -> Result<()> {
        self.dbus_actor_handle = Some(dbus_actor_handle);
        Ok(())
    }
    async fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Focus(focus) => {
                self.active = focus == Focus::Objects;
            }
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
                    info!(
                        "Invoking dbus method or property {:?}",
                        self.objects.state.selected()
                    );
                    if let Some(originating_service) = &self.originating_service {
                        let selected = self.objects.state.selected();
                        let invokable = extract_invokable(originating_service.clone(), selected);
                        if let Some(invokable) = invokable {
                            match &invokable.invocation_description {
                                crate::action::InvokableDbusMember::Method { method } => {
                                    return Ok(Some(Action::StartDbusInvocation(invokable)));
                                },
                                crate::action::InvokableDbusMember::Property { property } => {
                                    let dbus_actor = self.dbus_actor_handle.clone().expect("Component needs dbus handle");
                                    dbus_actor.get_property(invokable.service, invokable.object, invokable.interface, property.as_str().to_string()).await;
                                    return Ok(None)
                                },
                                crate::action::InvokableDbusMember::Signal { name } => {
                                    return Ok(None)
                                },
                            }
                        }
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
            crate::messages::AppMessage::InvocationResponse{..} => {}
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

fn extract_invokable(
    current_service: OwnedBusName,
    selected: &[crate::stateful_tree::DbusIdentifier],
) -> Option<Invocation> {
    let mut selected_iter = selected.iter();
    if let Some(stateful_tree::DbusIdentifier::Object(path)) = selected_iter.next()
        && let Some(stateful_tree::DbusIdentifier::Interface(interface)) = selected_iter.next()
        && let Some(stateful_tree::DbusIdentifier::Member(member_type)) = selected_iter.next()
    {
        let path = OwnedObjectPath::try_from(path.clone()).unwrap();
        let interface = OwnedInterfaceName::try_from(interface.clone()).unwrap();
        if let Some(invokable) = match member_type {
            stateful_tree::MemberTypes::Methods => {
                if let Some(stateful_tree::DbusIdentifier::Method(method)) = selected_iter.next() {
                    Some(crate::action::InvokableDbusMember::Method {
                        method: method.clone(),
                    })
                } else {
                    None
                }
            }
            stateful_tree::MemberTypes::Properties => {
                if let Some(stateful_tree::DbusIdentifier::Property(property)) =
                    selected_iter.next()
                {
                    Some(crate::action::InvokableDbusMember::Property {
                        property: OwnedPropertyName::try_from(property.clone()).unwrap(),
                    })
                } else {
                    None
                }
            }
            stateful_tree::MemberTypes::Signals => {
                if let Some(stateful_tree::DbusIdentifier::Signal(signal)) = selected_iter.next() {
                    Some(crate::action::InvokableDbusMember::Signal {
                        name: OwnedMemberName::try_from(signal.clone()).unwrap(),
                    })
                } else {
                    None
                }
            }
        } {
            Some(Invocation {
                service: current_service,
                object: path,
                interface: interface,
                invocation_description: invokable,
            })
        } else {
            None
        }
    } else {
        None
    }
}
