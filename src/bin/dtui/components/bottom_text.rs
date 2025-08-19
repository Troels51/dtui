use color_eyre::Result;
use itertools::Itertools;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{
    action::Action, app::Focus, config::Config
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
        // TODO: Generate keybinds from config
        let generic_keybinds = self.config.keybindings.get(&Focus::All).expect("We need to have a generic keybind");
        // This does not generate a correct description because an action can have multiple keyevents
        let generic_description = generic_keybinds.iter().map(|(key, action)| {
            format!("{}: {}", action, key.iter().map(|event| event.code).join("+"))
        }).join(" | ");
        match action {
            Action::Focus(focus) => {
                let specific_keybinds = self.config.keybindings.get(&focus);
                self.help_text = match focus {
                    crate::app::Focus::Services => {
                        generic_description
                        //format!("Change focus: {} | Navigation: ← ↓ ↑ → | Get Service: Enter | Quit: Esc", "Tab")
                    },
                    crate::app::Focus::Objects => {
                        format!("Change focus: {} | Navigation: ← ↓ ↑ → | Start Call Method: Enter | Quit: Esc", "Tab")
                    },
                    crate::app::Focus::Call => {
                        format!("Change focus: {} | Navigation: ← ↓ ↑ → | Call Method: Enter| Quit: Esc", "Tab")
                    },
                    crate::app::Focus::All => {
                        format!("")
                    },
                };
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        // TODO: Can this change based on the focus?
        let bottom_text =
            Span::raw(self.help_text.clone());
        let helper_paragraph = Paragraph::new(bottom_text).alignment(Alignment::Center);
        frame.render_widget(helper_paragraph, area);
        Ok(())
    }

}
