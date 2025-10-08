use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{
    action::{Action, DbusInvocationAction, EditorMode},
    app::Focus,
    config::Config,
};

#[derive(Default)]
pub struct HelpView {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    help_text: String,
}

impl HelpView {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Component for HelpView {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        Ok(())
    }

    async fn update(&mut self, action: Action) -> Result<Option<Action>> {

        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        frame.render_widget(Clear, area);
        let bottom_text = Span::raw("test");
        let popup_block = Block::bordered().title("Help").border_type(BorderType::Rounded);
        let helper_paragraph = Paragraph::new(bottom_text).alignment(Alignment::Center);
        frame.render_widget(popup_block, area);
        Ok(())
    }
}
