use std::{iter::repeat_n, str::FromStr};

use chumsky::Parser;
use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use tracing::info;
use tui_textarea::CursorMove;

use super::Component;
use crate::{
    action::{Action, MethodCall}, app::Focus, config::Config, dbus_handler::DbusActorHandle, other::active_area_border_color, parser::get_parser
};

pub struct MethodArgVisual {
    pub text_area: tui_textarea::TextArea<'static>,
    pub parser:
        Box<dyn Parser<char, zbus::zvariant::Value<'static>, Error = chumsky::error::Simple<char>>>,
    pub is_input: bool, // Is this Arg an input or output
}

// Encapsulates the information about the ongoing call
struct OngoingCallInfo {
    service: zbus_names::OwnedBusName,
    object: zbus::zvariant::OwnedObjectPath,
    interface: zbus_names::OwnedInterfaceName,
    method_description: crate::stateful_tree::OwnedMethod,
    method_arg_vis: Vec<MethodArgVisual>,
    selected: usize,
    called: bool,
}

impl OngoingCallInfo {
    fn new(call: MethodCall) -> OngoingCallInfo {
        OngoingCallInfo {
            service: call.service,
            object: call.object,
            interface: call.interface,
            method_description: call.method_description,
            method_arg_vis: vec![],
            selected: 0,
            called: false,
        }
    }
}

#[derive(Default)]
pub struct CallView {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    active: bool,
    ongoing: Option<OngoingCallInfo>,
    dbus_actor_handle: Option<DbusActorHandle>,
}

impl CallView {
    pub fn new() -> Self {
        Self::default()
    }

    fn draw_inner(frame: &mut Frame<'_>, area: Rect, ongoing: &mut OngoingCallInfo) {
        // TODO: Big ass block, lets refactor to smaller functions

        let method = &ongoing.method_description;
        let args = ongoing.method_description.args();
        let single_line_layout = Layout::vertical(
            repeat_n(Constraint::Length(3), args.len()).chain([Constraint::Min(1)]),
        );

        let segments = single_line_layout.split(area);
        //TODO: Move into init
        if ongoing.method_arg_vis.is_empty() {
            // First time init of text areas
            for arg in method.args().iter().take(segments.len()) {
                let mut text_area = tui_textarea::TextArea::default();
                let inout: String = if let Some(direction) = arg.direction() {
                    match direction {
                        zbus_xml::ArgDirection::In => "input".to_string(),
                        zbus_xml::ArgDirection::Out => "output".to_string(),
                    }
                } else {
                    "".to_string()
                };
                text_area.set_cursor_line_style(Style::default());
                text_area.set_cursor_style(Style::default());
                text_area.set_block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!("name: {} | {}", arg.name().unwrap(), inout))
                        .title_bottom(format!("type: {}", arg.ty().to_string())),
                );
                let parser = get_parser(
                    zbus::zvariant::Signature::from_str(arg.ty().to_string().as_str())
                        .expect("The type description for the method we got was not good"),
                );
                let input = match arg.direction().unwrap_or(zbus_xml::ArgDirection::In) {
                    zbus_xml::ArgDirection::In => true,
                    zbus_xml::ArgDirection::Out => false,
                };
                ongoing.method_arg_vis.push(MethodArgVisual {
                    text_area,
                    parser: Box::new(parser),
                    is_input: input,
                });
            }
        }
        for (i, input) in ongoing.method_arg_vis.iter_mut().enumerate() {
            let emphasis = if i == ongoing.selected {
                let method_arg: String = input.text_area.lines()[0].clone();
                let parsed = input.parser.parse(method_arg);
                match parsed {
                    Ok(_) => Style::default().fg(Color::Green),
                    Err(_) => Style::default().fg(Color::Red),
                }
            } else {
                Style::default()
            };
            input.text_area.set_block(
                input
                    .text_area
                    .block()
                    .unwrap()
                    .clone()
                    .border_style(emphasis),
            );
            input.text_area.set_cursor_line_style(emphasis);
            frame.render_widget(&input.text_area, segments[i]);
        }
    }
}

impl Component for CallView {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        Ok(())
    }
    fn register_dbus_actor_handle(&mut self, dbus_actor_handle: DbusActorHandle) -> Result<()> {
        self.dbus_actor_handle = Some(dbus_actor_handle);
        Ok(())
    }
    async fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Focus(focus) => {
                self.active = focus == Focus::Call;
            }
            _ => (),
        }
        if self.active {
            match action {
                Action::Down => {
                    if let Some(ongoing) = &mut self.ongoing {
                        let input_count = ongoing
                            .method_description
                            .args()
                            .iter()
                            .filter(|arg| match arg.direction() {
                                Some(direction) => match direction {
                                    zbus_xml::ArgDirection::In => true,
                                    zbus_xml::ArgDirection::Out => false,
                                },
                                None => false,
                            })
                            .count();
                        ongoing.selected =
                            std::cmp::min(input_count - 1, ongoing.selected + 1);
                    }
                }
                Action::Up => {
                    if let Some(ongoing) = &mut self.ongoing {
                        ongoing.selected = ongoing.selected.saturating_sub(1);
                    }
                }

                Action::CallActiveMethod => {
                    if let Some(ongoing) = &mut self.ongoing {
                        info!("Calling active method");
                        let parses = ongoing
                            .method_arg_vis
                            .iter()
                            .filter(|input| input.is_input)
                            .map(|input| input.parser.parse(input.text_area.lines()[0].clone()));
                        if parses.clone().all(
                            |result: Result<
                                zbus::zvariant::Value<'static>,
                                Vec<chumsky::error::Simple<char>>,
                            >| Result::is_ok(&result),
                        ) {
                            let values: Vec<zbus::zvariant::OwnedValue> = parses
                                .map(|value| {
                                    // We know that they are all Ok, so unwrap is fine here
                                    zbus::zvariant::OwnedValue::try_from(value.unwrap()).unwrap()
                                })
                                .collect();
                            self.dbus_actor_handle.as_ref().expect("Cannot call method without a dbus actor handle, init should be called first")
                                .call_method(
                                    ongoing.service.clone(),
                                    ongoing.object.clone(),
                                    ongoing.interface.clone(),
                                    ongoing.method_description.name().clone(),
                                    values,
                                )
                                .await;
                        } else {
                            // Alert user that call cannot be made if arguments cannot be parsed
                        }
                    }
                }
                _ => (),
            }
        }
        // Handle irregardless of active
        match action {
            Action::StartDbusMethodCall(method_call) => {
                self.ongoing = Some(OngoingCallInfo::new(method_call));
            }
            _ => (),
        }
        Ok(None)
    }

    fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> Result<Option<Action>> {
        // ignore certain keys
        let ignored = [
            crossterm::event::KeyCode::Enter,
            crossterm::event::KeyCode::Home,
            crossterm::event::KeyCode::End,
            crossterm::event::KeyCode::PageUp,
            crossterm::event::KeyCode::PageDown,
            crossterm::event::KeyCode::Tab,
            crossterm::event::KeyCode::BackTab,
        ];
        if ignored.contains(&key.code) {
            return Ok(None);
        }
        if let Some(ongoing) = &mut self.ongoing {
            ongoing.method_arg_vis[ongoing.selected]
                .text_area
                .input(key);
        }
        Ok(None)
    }
    fn update_from_dbus(
        &mut self,
        dbus_action: crate::messages::AppMessage,
    ) -> Result<Option<Action>> {
        match dbus_action {
            crate::messages::AppMessage::MethodCallResponse(owned_member_name, message) => {
                if let Ok(value) = message.body().deserialize::<zbus::zvariant::Structure>() {
                    if let Some(ref mut ongoing) = self.ongoing {
                        for (index, output_field) in ongoing
                            .method_arg_vis
                            .iter_mut()
                            .filter(|field| !field.is_input)
                            .enumerate()
                        {
                            output_field.text_area.move_cursor(CursorMove::Head);
                            output_field.text_area.delete_line_by_end(); // The way to clear a text area
                            output_field
                                .text_area
                                .insert_str(format!("{}", value.fields()[index]));
                        }
                    }
                }
            }
            _ => (),
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Call")
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(active_area_border_color(self.active)));
        let inner = block.inner(area);
        if let Some(ref mut ongoing) = self.ongoing {
            CallView::draw_inner(frame, inner, ongoing);
        }
        frame.render_widget(block, area);

        Ok(())
    }
}
