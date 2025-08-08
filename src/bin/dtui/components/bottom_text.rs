use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use tui_tree_widget::Tree;

use super::Component;
use crate::{
    action::Action, config::Config, other::active_area_border_color, stateful_tree::StatefulTree,
};

#[derive(Default)]
pub struct BottomText {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
}

impl BottomText {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Component for BottomText {
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
            Action::Tick => {
                // add any logic here that should run on every tick
            }
            Action::Render => {
                // add any logic here that should run on every render
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        // TODO: Can this change based on the focus?
        let bottom_text =
            Span::raw("Change focus: Tab | Navigation: ← ↓ ↑ → | Get Service: Enter | Quit: q");
        let helper_paragraph = Paragraph::new(bottom_text).alignment(Alignment::Center);
        frame.render_widget(helper_paragraph, area);
        Ok(())
    }
    
    fn active(&mut self, _active:bool) {
        () // Bottom text cannot be the active component
    }
}
