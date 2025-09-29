use color_eyre::Result;
use itertools::Itertools;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{
    action::Action,
    config::Config,
    messages::{AppMessage, DbusError, InvocationResponse},
    other::active_area_border_color,
};

#[derive(Default)]
pub struct ResultsView {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    active: bool,
    list_state: ListState,
    results: Vec<AppMessage>,
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
        self.results.push(dbus_action);
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

        let list = List::new(&self.results);
        frame.render_stateful_widget(list, inner, &mut self.list_state);

        Ok(())
    }
}

fn dbus_result_to_string(message: &zbus::Message) -> String {
    let message_string =
        if let Ok(message) = message.body().deserialize::<zbus::zvariant::Structure>() {
            message
                .fields()
                .iter()
                .map(|field| field.to_string())
                .join(",")
        } else {
            "".to_string()
        };
    message_string
}

const RESULT_STYLE: Style = Style::new();
const ERROR_STYLE: Style = Style::new().fg(Color::Red);


impl From<&AppMessage> for ListItem<'_> {
    fn from(value: &AppMessage) -> Self {
        // https://github.com/ratatui/ratatui/issues/128
        // https://crates.io/crates/tui-widget-list
        match value {
            AppMessage::Objects(object) => ListItem::new(Text::styled(
                format!("Service: {}", &object.0),
                RESULT_STYLE,
            )),
            AppMessage::Services(owned_bus_names) => ListItem::new(Text::styled(
                format!("All services read"),
                RESULT_STYLE,
            )),
            AppMessage::InvocationResponse(invocation_response) => ListItem::new(Text::styled(
                format!("{}", dbus_result_to_string(&invocation_response.message)),
                RESULT_STYLE,
            )),
            AppMessage::Error(dbus_error) => ListItem::new(Text::styled(
                format!("Error! {}", dbus_error.message),
                ERROR_STYLE,
            ).alignment(Alignment::Left)),
        }.add_modifier(Modifier::BOLD)
    }
}
