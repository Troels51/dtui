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
        self.config
            .keybindings
            .get_key_from_action(focus, action)
            .and_then(|key_events| key_events.first()) // This takes the first key possible, so won't show all possibilities
            .map_or("none".to_string(), |key_event| key_event.code.to_string())
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
        if let Action::Focus(focus) = action {
            let next_focus_key = self.get_action_key(Focus::All, Action::NextFocus);
            let quit_key = self.get_action_key(Focus::All, Action::Quit);
            let normal_mode =
                self.get_action_key(Focus::Call, Action::EditorMode(EditorMode::Normal));
            let insert_mode =
                self.get_action_key(Focus::Call, Action::EditorMode(EditorMode::Insert));
            let help_key = self.get_action_key(Focus::All, Action::Help);

            self.help_text = match focus {
                crate::app::Focus::Services => {
                    format!(
                        "Change focus: {} | Navigation: ← ↓ ↑ → | Get Service: {} | Quit: {} | Help: {}",
                        next_focus_key,
                        self.get_action_key(focus, Action::GetService),
                        quit_key,
                        help_key
                    )
                }
                crate::app::Focus::Objects => {
                    format!(
                        "Change focus: {} | Navigation: ← ↓ ↑ → | Call method: {} | Get property {} | Set property {} | Quit: {} | Help: {}",
                        next_focus_key,
                        self.get_action_key(
                            focus,
                            Action::InvokeDbus(DbusInvocationAction::CallMethod)
                        ),
                        self.get_action_key(
                            focus,
                            Action::InvokeDbus(DbusInvocationAction::GetProperty)
                        ),
                        self.get_action_key(
                            focus,
                            Action::InvokeDbus(DbusInvocationAction::SetProperty)
                        ),
                        quit_key,
                        help_key
                    )
                }
                crate::app::Focus::Call => {
                    format!(
                        "Change focus: {} | Navigation: ← ↓ ↑ → | Call Method: {} | Quit: {}, InsertMode: {}, NormalMode: {} | Help: {}",
                        next_focus_key,
                        self.get_action_key(focus, Action::CallActiveMethod),
                        quit_key,
                        insert_mode,
                        normal_mode,
                        help_key
                    )
                }
                crate::app::Focus::All => String::new(),
                crate::app::Focus::Help => String::new(),
            };
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
