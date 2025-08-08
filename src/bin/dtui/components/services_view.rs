use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use tracing::info;
use zbus_names::OwnedBusName;

use super::Component;
use crate::{action::{self, Action}, app::Focus, config::Config, dbus_handler::DbusActorHandle, other::active_area_border_color, stateful_list::StatefulList, stateful_tree::StatefulTree};

#[derive(Default)]
pub struct ServicesView {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    services: StatefulList<OwnedBusName>,
    active: bool,
    dbus_actor_handle: Option<DbusActorHandle>,
}

impl ServicesView {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Component for ServicesView {
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
        if self.active {
            match action {
                Action::Down => {
                    self.services.next();
                }
                Action::Up => {
                    self.services.previous();
                }
                Action::GetService => {
                    if let Some(selected_index) = self.services.state.selected() {
                        let item = self.services.items[selected_index].clone();
                        match &self.dbus_actor_handle {
                            Some(handle) => handle.request_objects_from(item).await,
                            None => (),
                        }
                    }
                }
                _ => {}
            }
        }
        else {
            // Handle actions when not focused
            match action {
                _ => ()
            }
        }

        Ok(None)
    }

    fn update_from_dbus(&mut self, dbus_action: crate::messages::AppMessage) -> Result<Option<Action>> {
        
        match dbus_action {
            crate::messages::AppMessage::Objects(_) => {
            },
            crate::messages::AppMessage::Services(owned_bus_names) => {
                info!("services view got services");

                self.services = StatefulList::with_items(owned_bus_names);
            },
            crate::messages::AppMessage::MethodCallResponse(owned_member_name, message) => {

            },
        }
        Ok(None)
    }


    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let items: Vec<ListItem> = self
            .services
            .items
            .iter()
            .map(|i| {
                let lines = Span::from(i.as_str());
                ListItem::new(lines).style(Style::default())
            })
            .collect();
        let items = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Services")
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(active_area_border_color(self.active))),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");
        frame.render_stateful_widget(items, area, &mut self.services.state);

        Ok(())
    }
    
    fn active(&mut self, active: bool) {
        self.active = active
    }
}
