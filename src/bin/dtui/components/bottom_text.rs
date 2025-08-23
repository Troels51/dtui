use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{action::Action, app::Focus, config::Config};

#[derive(Default)]
pub struct BottomText {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    help_text: String,
}

impl BottomText {
    pub fn new() -> Self {
        Self::default()
    }

    fn get_action_key(&self, focus: Focus, action: Action) -> String {
        let next_focus_key = self
            .config
            .keybindings
            .get_key_from_action(focus, action)
            .and_then(|key_events| key_events.first()) // This takes the first key possible, so won't show all possibilities
            .map_or("none".to_string(), |key_event| key_event.code.to_string());
        next_focus_key
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
            Action::Focus(focus) => {
                let next_focus_key = self.get_action_key(Focus::All, Action::NextFocus);
                self.help_text = match focus {
                    crate::app::Focus::Services => {
                        format!(
                            "Change focus: {} | Navigation: ← ↓ ↑ → | Get Service: {} | Quit: Esc",
                            next_focus_key,
                            self.get_action_key(focus, Action::GetService)
                        )
                    }
                    crate::app::Focus::Objects => {
                        format!(
                            "Change focus: {} | Navigation: ← ↓ ↑ → | Invoke Dbus: {} | Quit: Esc",
                            next_focus_key,
                            self.get_action_key(focus, Action::InvokeDbus)
                        )
                    }
                    crate::app::Focus::Call => {
                        format!(
                            "Change focus: {} | Navigation: ← ↓ ↑ → | Call Method: {} | Quit: Esc",
                            next_focus_key,
                            self.get_action_key(focus, Action::CallActiveMethod)
                        )
                    }
                    crate::app::Focus::All => {
                        format!("")
                    }
                };
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let bottom_text = Span::raw(self.help_text.clone());
        let helper_paragraph = Paragraph::new(bottom_text).alignment(Alignment::Center);
        frame.render_widget(helper_paragraph, area);
        Ok(())
    }
}
