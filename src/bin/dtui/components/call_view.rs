use std::{iter::repeat_n, str::FromStr};

use chumsky::Parser;
use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use tracing::info;
use tui_textarea::CursorMove;
use zbus_names::OwnedMemberName;
use zbus_xml::Method;

use super::Component;
use crate::{
    action::{Action, EditorMode, Invocation, InvokableDbusMember},
    app::Focus,
    config::Config,
    dbus_handler::DbusActorHandle,
    messages::InvocationResponse,
    other::active_area_border_color,
    parser::get_parser,
    stateful_tree::OwnedProperty,
};

pub struct MethodArgVisual {
    pub text_area: tui_textarea::TextArea<'static>,
    pub parser:
        Box<dyn Parser<char, zbus::zvariant::Value<'static>, Error = chumsky::error::Simple<char>>>,
    pub is_input: bool, // Is this Arg an input or output
}
// Encapsulates the information about the ongoing call
struct OngoingCallInfo {
    invocation: Invocation,
    method_arg_vis: Vec<MethodArgVisual>,
    selected: usize,
}

impl OngoingCallInfo {
    fn new(call: Invocation, area: Size) -> Option<OngoingCallInfo> {
        let mut call_info = OngoingCallInfo {
            invocation: call,
            method_arg_vis: vec![],
            selected: 0,
        };
        match &call_info.invocation.invocation_description {
            InvokableDbusMember::Method { method } => {
                // First time init of text areas
                let args = method.args();

                for arg in method.args().iter() {
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
                    call_info.method_arg_vis.push(MethodArgVisual {
                        text_area,
                        parser: Box::new(parser),
                        is_input: input,
                    });
                }
                Some(call_info)
            }
            InvokableDbusMember::Property { property } => {
                let mut text_area = tui_textarea::TextArea::default();

                text_area.set_cursor_line_style(Style::default());
                text_area.set_cursor_style(Style::default());
                text_area.set_block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(
                            "name: {} | {}",
                            property.name(),
                            "input".to_string()
                        ))
                        .title_bottom(format!("type: {}", property.ty().to_string())),
                );
                let parser = Box::new(get_parser(
                    zbus::zvariant::Signature::from_str(property.ty().to_string().as_str())
                        .expect("The type description for the method we got was not good"),
                ));
                call_info.method_arg_vis.push(MethodArgVisual {
                    text_area,
                    parser,
                    is_input: true,
                });

                Some(call_info)
            }
            InvokableDbusMember::Signal { name } => todo!(),
        }
    }

    async fn call(&self, mut values: Vec<zbus::zvariant::OwnedValue>, actor: &DbusActorHandle) {
        match &self.invocation.invocation_description {
            InvokableDbusMember::Method { method } => {
                actor
                    .call_method(
                        self.invocation.service.clone(),
                        self.invocation.object.clone(),
                        self.invocation.interface.clone(),
                        method.name().clone(),
                        values,
                    )
                    .await
            }
            InvokableDbusMember::Property { property } => actor.set_property(
                self.invocation.service.clone(),
                self.invocation.object.clone(),
                self.invocation.interface.clone(),
                property.name().clone(),
                values.pop().expect("Properties can only have one value when setting")).await,
            InvokableDbusMember::Signal { name } => todo!(),
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
    editor_mode: EditorMode,
    area: Size,
}

impl CallView {
    pub fn new() -> Self {
        Self::default()
    }

    fn draw_inner(frame: &mut Frame<'_>, area: Rect, ongoing: &mut OngoingCallInfo, active: bool) {
        let nr_args = ongoing.invocation.nr_args();
        let single_line_layout =
            Layout::vertical(repeat_n(Constraint::Length(3), nr_args).chain([Constraint::Min(1)]));
        let segments = single_line_layout.split(area);
        for (i, input) in ongoing.method_arg_vis.iter_mut().enumerate() {
            let emphasis = if i == ongoing.selected && active{
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
        if let Action::Focus(focus) = action {
            self.active = focus == Focus::Call;
        }
        if self.active {
            match action {
                Action::Down => {
                    if let Some(ongoing) = &mut self.ongoing {
                        let input_count = ongoing.invocation.input_count();

                        ongoing.selected =
                            std::cmp::min(input_count.saturating_sub(1), ongoing.selected + 1);
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
                            ongoing.call(values, self.dbus_actor_handle.as_ref().expect("Cannot call method without a dbus actor handle, init should be called first")).await;
                        } else {
                            // Alert user that call cannot be made if arguments cannot be parsed
                        }
                    }
                }
                Action::EditorMode(mode) => {
                    self.editor_mode = mode;
                }
                _ => (),
            }
        }
        // Handle irregardless of active
        if let Action::StartDbusInvocation(invocation) = action {
            self.ongoing = OngoingCallInfo::new(invocation, self.area);
        }
        Ok(None)
    }

    fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> Result<Option<Action>> {
        if self.editor_mode == EditorMode::Insert {
            // Ignore certain keys as they will just confuse
            match key.code {
                crossterm::event::KeyCode::Enter => return Ok(None),
                crossterm::event::KeyCode::PageUp => return Ok(None),
                crossterm::event::KeyCode::PageDown => return Ok(None),
                crossterm::event::KeyCode::Tab => return Ok(None),
                crossterm::event::KeyCode::BackTab => return Ok(None),
                crossterm::event::KeyCode::Null => return Ok(None),
                crossterm::event::KeyCode::Esc => return Ok(None),
                _ => ()
            }
            if let Some(ongoing) = &mut self.ongoing {
                ongoing.method_arg_vis[ongoing.selected]
                    .text_area
                    .input(key);
            }
        }

        Ok(None)
    }
    fn update_from_dbus(
        &mut self,
        dbus_action: crate::messages::AppMessage,
    ) -> Result<Option<Action>> {
        if let crate::messages::AppMessage::InvocationResponse(InvocationResponse {
            method_name,
            message,
            ..
        }) = dbus_action
        {
            if let Ok(value) = message.body().deserialize::<zbus::zvariant::Structure>()
                && let Some(ref mut ongoing) = self.ongoing
            {
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
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!("Call - Mode: {}", self.editor_mode))
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(active_area_border_color(self.active)));
        let inner = block.inner(area);
        if let Some(ref mut ongoing) = self.ongoing {
            CallView::draw_inner(frame, inner, ongoing, self.active);
        }
        frame.render_widget(block, area);

        Ok(())
    }
    fn init(&mut self, area: Size) -> Result<()> {
        self.area = area;
        Ok(())
    }
}
