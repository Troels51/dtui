use chumsky::chain::Chain;
use color_eyre::Result;
use itertools::Itertools;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use tui_widget_list::ListBuilder;

use super::Component;
use crate::{
    action::Action, config::Config, messages::AppMessage, other::active_area_border_color,
};

#[derive(Default)]
pub struct ResultsView {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    active: bool,
    list_state: tui_widget_list::ListState,
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

        let builder = ListBuilder::new(|context| {
            let widget = self.results.get(context.index).unwrap();
            (widget, (widget.characters() / context.cross_axis_size) + 1) // We want the number of times the amount of characters can fit into the cross_axis
        });

        let list = tui_widget_list::ListView::new(builder, self.results.len());
        list.render(inner, frame.buffer_mut(), &mut self.list_state);

        Ok(())
    }
}

fn dbus_result_to_string(message: &zbus::Message) -> String {
    
    if let Ok(message) = message.body().deserialize::<zbus::zvariant::Structure>() {
            message
                .fields()
                .iter()
                .map(|field| field.to_string())
                .join(",")
        } else {
            "".to_string()
        }
}

const RESULT_STYLE: Style = Style::new();
const ERROR_STYLE: Style = Style::new().fg(Color::Red);

impl Widget for &AppMessage {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        match self {
            AppMessage::Objects(object) => {
                Paragraph::new(format!("Service: {}", &object.0)).style(RESULT_STYLE)
            }
            AppMessage::Services(owned_bus_names) => {
                Paragraph::new("All services read".to_string()).style(RESULT_STYLE)
            }
            AppMessage::InvocationResponse(invocation_response) => Paragraph::new(dbus_result_to_string(&invocation_response.message).to_string())
            .style(RESULT_STYLE),
            AppMessage::Error(dbus_error) => {
                Paragraph::new(format!("Error! {}", dbus_error.message)).style(ERROR_STYLE)
            }
        }
        .add_modifier(Modifier::BOLD)
        .wrap(Wrap { trim: false })
        .render(area, buf);
    }
}
