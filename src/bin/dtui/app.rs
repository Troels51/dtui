use std::time::{Duration, Instant};

use chumsky::Parser;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{backend::Backend, Terminal};
use tokio::sync::mpsc::Receiver;
use tracing::Level;
use tui_textarea::CursorMove;
use zbus::{
    names::{OwnedBusName, OwnedInterfaceName, OwnedMemberName},
    zvariant::OwnedObjectPath,
};

use crate::{
    dbus_handler::DbusActorHandle,
    messages::AppMessage,
    stateful_list::StatefulList,
    stateful_tree::{MethodDescription, StatefulTree},
    ui::ui,
};

pub struct MethodArgVisual {
    pub text_area: tui_textarea::TextArea<'static>,
    pub parser:
        Box<dyn Parser<char, zbus::zvariant::Value<'static>, Error = chumsky::error::Simple<char>>>,
    pub is_input: bool, // Is this Arg an input or output
}
pub struct MethodCallPopUp {
    pub service: OwnedBusName,
    pub object: OwnedObjectPath,
    pub interface: OwnedInterfaceName,
    pub method_description: MethodDescription,
    pub method_arg_vis: Vec<MethodArgVisual>,
    pub selected: usize,
    pub called: bool,
}
impl MethodCallPopUp {
    fn new(
        service: OwnedBusName,
        object: OwnedObjectPath,
        interface: OwnedInterfaceName,
        method_description: MethodDescription,
    ) -> Self {
        Self {
            service,
            object,
            interface,
            method_description,
            method_arg_vis: Vec::new(), // This gets filled on UI. Maybe there is a better way of doing this
            selected: 0,
            called: false,
        }
    }
}

impl PartialEq for MethodCallPopUp {
    fn eq(&self, other: &Self) -> bool {
        self.method_description == other.method_description
    }
}

#[derive(PartialEq)]
pub enum WorkingArea {
    Services,
    Objects,
    MethodCallPopUp(MethodCallPopUp),
}
// TODO: maybe we should use Components instead, Objects/Services/PopUp would be a componenet, and they would have their own input/render functions
pub struct App {
    dbus_rx: Receiver<AppMessage>,
    dbus_handle: DbusActorHandle,
    pub services: StatefulList<OwnedBusName>,
    pub objects: StatefulTree,
    pub working_area: WorkingArea,
}

impl App {
    pub fn new(dbus_rx: Receiver<AppMessage>, dbus_handle: DbusActorHandle) -> App {
        App {
            dbus_rx,
            dbus_handle,
            services: StatefulList::with_items(vec![]),
            objects: StatefulTree::new(),
            working_area: WorkingArea::Services,
        }
    }

    pub fn on_tick(&self) {}
}

pub async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> Result<(), zbus::Error> {
    let mut last_tick = Instant::now();
    app.dbus_handle.request_services().await;

    loop {
        terminal.draw(|frame| ui::<B>(frame, &mut app))?;

        match app.dbus_rx.try_recv() {
            Ok(message) => match message {
                AppMessage::Objects((_service_name, root_node)) => {
                    app.objects = StatefulTree::from_nodes(root_node);
                }
                AppMessage::Services(names) => {
                    app.services = StatefulList::with_items(names);
                }
                AppMessage::MethodCallResponse(_method, message) => {
                    if let WorkingArea::MethodCallPopUp(ref mut popup) = app.working_area {
                        popup.called = true;
                        if let Ok(value) = message.body().deserialize::<zbus::zvariant::Structure>()
                        {
                            for (index, output_field) in popup
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
            },
            _error => (),
        };
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => match app.working_area {
                        WorkingArea::Services => return Ok(()),
                        WorkingArea::Objects => return Ok(()),
                        WorkingArea::MethodCallPopUp(_) => (),
                    },
                    KeyCode::Enter => {
                        match app.working_area {
                            WorkingArea::Services => {
                                if let Some(selected_index) = app.services.state.selected() {
                                    let item = app.services.items[selected_index].clone();
                                    app.dbus_handle.request_objects_from(item).await;
                                }
                            }
                            WorkingArea::Objects => {
                                if let Some(full_description) =
                                    extract_description(app.objects.state.selected())
                                {
                                    app.working_area =
                                        WorkingArea::MethodCallPopUp(MethodCallPopUp::new(
                                            app.services.items
                                                [app.services.state.selected().unwrap()]
                                            .clone(),
                                            full_description.0,
                                            full_description.1,
                                            full_description.2,
                                        ));
                                }
                            }
                            WorkingArea::MethodCallPopUp(ref popup) =>
                            // Call method
                            {
                                let parses = popup
                                    .method_arg_vis
                                    .iter()
                                    .filter(|input| input.is_input)
                                    .map(|input| {
                                        input.parser.parse(input.text_area.lines()[0].clone())
                                    });
                                if parses.clone().all(
                                    |result: Result<
                                        zbus::zvariant::Value<'static>,
                                        Vec<chumsky::error::Simple<char>>,
                                    >| Result::is_ok(&result),
                                ) {
                                    let values: Vec<zbus::zvariant::OwnedValue> = parses
                                        .map(|value| {
                                            // We know that they are all Ok, so unwrap is fine here
                                            zbus::zvariant::OwnedValue::try_from(value.unwrap())
                                                .unwrap()
                                        })
                                        .collect();
                                    app.dbus_handle
                                        .call_method(
                                            popup.service.clone(),
                                            popup.object.clone(),
                                            popup.interface.clone(),
                                            OwnedMemberName::from(
                                                popup.method_description.0.name(),
                                            ),
                                            values,
                                        )
                                        .await;
                                } else {
                                    // Alert user that call cannot be made if arguments cannot be parsed
                                }
                            }
                        }
                    }
                    KeyCode::Left => match app.working_area {
                        WorkingArea::Services => app.services.unselect(),
                        WorkingArea::Objects => app.objects.left(),
                        WorkingArea::MethodCallPopUp(ref mut popup) => {
                            popup.method_arg_vis[0].text_area.input(key);
                        }
                    },
                    KeyCode::Down => match app.working_area {
                        WorkingArea::Services => app.services.next(),
                        WorkingArea::Objects => app.objects.down(),
                        WorkingArea::MethodCallPopUp(ref mut popup) => {
                            popup.selected =
                                std::cmp::min(popup.selected + 1, popup.method_arg_vis.len());
                        }
                    },
                    KeyCode::Up => match app.working_area {
                        WorkingArea::Services => app.services.previous(),
                        WorkingArea::Objects => app.objects.up(),
                        WorkingArea::MethodCallPopUp(ref mut popup) => {
                            popup.selected = popup.selected.saturating_sub(1);
                        }
                    },
                    KeyCode::Right => match app.working_area {
                        WorkingArea::Services => {}
                        WorkingArea::Objects => app.objects.right(),
                        WorkingArea::MethodCallPopUp(ref mut popup) => {
                            popup.method_arg_vis[0].text_area.input(key);
                        }
                    },
                    KeyCode::Tab => match app.working_area {
                        WorkingArea::Services => app.working_area = WorkingArea::Objects,
                        WorkingArea::Objects => app.working_area = WorkingArea::Services,
                        WorkingArea::MethodCallPopUp(ref _method) => {}
                    },
                    KeyCode::Esc => {
                        app.working_area = WorkingArea::Objects;
                    }
                    _ => match app.working_area {
                        WorkingArea::MethodCallPopUp(ref mut popup) => {
                            popup.method_arg_vis[popup.selected].text_area.input(key);
                        }
                        _ => (),
                    },
                }
                tracing::event!(
                    Level::DEBUG,
                    "{}",
                    format!("state = {:?}", app.objects.state)
                );
            }
        }
        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }
    }
}

/// Takes a stateful_tree::DbusIdentifier, which is an identifier for where a node is in the UI tree
/// and if the selection is a method, it will extract the path, interface name and method description
/// Otherwise it returns None
fn extract_description(
    selected: &[crate::stateful_tree::DbusIdentifier],
) -> Option<(OwnedObjectPath, OwnedInterfaceName, MethodDescription)> {
    let object_path = selected
        .iter()
        .filter_map(|identifier| match identifier {
            crate::stateful_tree::DbusIdentifier::Object(o) => Some(o),
            _ => None,
        })
        .next();
    let interface_name = selected
        .iter()
        .filter_map(|identifier| match identifier {
            crate::stateful_tree::DbusIdentifier::Interface(i) => Some(i),
            _ => None,
        })
        .next();
    let member_name = selected
        .iter()
        .filter_map(|identifier| match identifier {
            crate::stateful_tree::DbusIdentifier::Method(m) => Some(m),
            _ => None,
        })
        .next();
    if object_path.is_some() && interface_name.is_some() && member_name.is_some() {
        Some((
            OwnedObjectPath::try_from(object_path.unwrap().clone()).unwrap(),
            OwnedInterfaceName::try_from(interface_name.unwrap().clone()).unwrap(),
            member_name.unwrap().clone(),
        ))
    } else {
        None
    }
}
