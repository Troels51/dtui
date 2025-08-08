use std::iter;

use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use tracing::info;
use tui_scrollview::{ScrollView, ScrollViewState};
use tui_tree_widget::Tree;

use super::Component;
use crate::{action::Action, config::Config, other::active_area_border_color, stateful_tree::StatefulTree};

#[derive(Default)]
pub struct ResultsView {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    active: bool,
    scroll_view_state: ScrollViewState,
}

impl ResultsView {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Component for ResultsView {
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
                },
                Action::Down => {
                },
                Action::DownTree => {
                },
                Action::UpTree => {
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
            crate::messages::AppMessage::MethodCallResponse(owned_member_name, message) => {
            },
            _ => ()
        }
        Ok(None)
    }


    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let block = Block::default()
                .borders(Borders::ALL)
                .title("Results")
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(active_area_border_color(self.active)));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let content =
            iter::repeat("Lorem ipsum dolor sit amet, consectetur adipiscing elit.\n")
                .take(100)
                .collect::<String>();

        let mut scroll_view = ScrollView::new(inner.as_size());
        scroll_view.render_widget(Paragraph::new(content).block(Block::default()), scroll_view.area());
        frame.render_stateful_widget(scroll_view, inner, &mut self.scroll_view_state);

        Ok(())
    }
    
    fn active(&mut self, active: bool) {
        self.active = active
    }
}
