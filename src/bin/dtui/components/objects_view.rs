use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use tracing::info;
use tui_tree_widget::Tree;

use super::Component;
use crate::{action::Action, config::Config, other::active_area_border_color, stateful_tree::StatefulTree};

#[derive(Default)]
pub struct ObjectsView {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    objects: StatefulTree,
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
        if self.active {
            match action {
                Action::Up => {
                    self.objects.up();
                },
                Action::Down => {
                    self.objects.down();
                },
                Action::DownTree => {
                    self.objects.right();
                },
                Action::UpTree => {
                    self.objects.left();
                },
                Action::InvokeDbus => {
                },
                _ => {}
            }
        }
        else {
            // Handle action when not in focus
            match action {
                _ => ()
            }
        }

        Ok(None)
    }
    fn update_from_dbus(&mut self, dbus_action: crate::messages::AppMessage) -> Result<Option<Action>> {
        match dbus_action {
            crate::messages::AppMessage::Objects((service_name, objects)) => {
                info!("Got objects from service: {}", service_name);
                self.objects = StatefulTree::from_nodes(objects);

            },
            crate::messages::AppMessage::Services(owned_bus_names) => (),
            crate::messages::AppMessage::MethodCallResponse(owned_member_name, message) => {

            },
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
    
    fn active(&mut self, active: bool) {
        self.active = active
    }
}
