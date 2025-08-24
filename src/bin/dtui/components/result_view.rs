use color_eyre::Result;
use itertools::Itertools;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{
    action::Action, config::Config, messages::InvocationResponse, other::active_area_border_color,
};

#[derive(Default)]
pub struct ResultsView {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    active: bool,
    table_state: TableState,
    results: Vec<InvocationResponse>,
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
                Action::Up => {}
                Action::Down => {}
                Action::DownTree => {}
                Action::UpTree => {}
                Action::InvokeDbus => {}
                _ => {}
            }
        } else {
            // Handle action when not in focus
            
        }

        Ok(None)
    }
    fn update_from_dbus(
        &mut self,
        dbus_action: crate::messages::AppMessage,
    ) -> Result<Option<Action>> {
        if let crate::messages::AppMessage::InvocationResponse(response) = dbus_action {
            self.results.push(response);
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

        let rows = self.results.iter().map(
            |InvocationResponse {
                 service,
                 object_path,
                 interface,
                 method_name,
                 message,
             }| {
                let message_string = if let Ok(message) =
                    message.body().deserialize::<zbus::zvariant::Structure>()
                {
                    message
                        .fields()
                        .iter()
                        .map(|field| field.to_string())
                        .join(",")
                } else {
                    "".to_string()
                };

                Row::new(vec![
                    service.to_string(),
                    object_path.to_string(),
                    interface.to_string(),
                    method_name.to_string(),
                    message_string,
                ])
            },
        );
        let widths = [
            Constraint::Min(5),
            Constraint::Min(5),
            Constraint::Min(5),
            Constraint::Min(5),
            Constraint::Min(10),
        ];

        let table = Table::new(rows, widths).flex(layout::Flex::Start).header(Row::new(vec![
            "service", "object path", "interface", "method", "result"
        ]));
        frame.render_stateful_widget(table, inner, &mut self.table_state);

        Ok(())
    }
}
