use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode};
use ratatui::{backend::Backend, Terminal};
use tokio::sync::mpsc::Receiver;
use tracing::Level;
use zbus::names::OwnedBusName;

use crate::{
    dbus_handler::DbusActorHandle,
    messages::AppMessage,
    stateful_list::StatefulList,
    stateful_tree::{MethodDescription, StatefulTree},
    ui::ui,
};

pub struct PopUp {
    pub method_description: MethodDescription,
    pub inputs: Vec<tui_textarea::TextArea<'static>>,
    pub selected: usize,
}
impl PopUp {
    fn new(method_description: MethodDescription) -> Self {
        Self {
            method_description: method_description,
            inputs: Vec::new(), // This gets filled on UI. Maybe there is a better way of doing this
            selected: 0,
        }
    }
}

impl PartialEq for PopUp {
    fn eq(&self, other: &Self) -> bool {
        self.method_description == other.method_description
    }
}

#[derive(PartialEq)]
pub enum WorkingArea {
    Services,
    Objects,
    PopUp(PopUp),
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
                        WorkingArea::PopUp(_) => (),
                    },
                    KeyCode::Enter => match app.working_area {
                        WorkingArea::Services => {
                            if let Some(selected_index) = app.services.state.selected() {
                                let item = app.services.items[selected_index].clone();
                                app.dbus_handle.request_objects_from(item).await;
                            }
                        }
                        WorkingArea::Objects => {
                            if let Some(last) = app.objects.state.selected().last() {
                                match last {
                                    crate::stateful_tree::DbusIdentifier::Method(m) => {
                                        app.working_area =
                                            WorkingArea::PopUp(PopUp::new(m.clone()));
                                    }
                                    crate::stateful_tree::DbusIdentifier::Property(p) => {
                                        // Get the property
                                    }
                                    crate::stateful_tree::DbusIdentifier::Signal(s) => {
                                        // Call the signal
                                    }
                                    _ => (),
                                }
                            }
                        }
                        WorkingArea::PopUp(ref _method) => {}
                    },
                    KeyCode::Left => match app.working_area {
                        WorkingArea::Services => app.services.unselect(),
                        WorkingArea::Objects => app.objects.left(),
                        WorkingArea::PopUp(ref mut popup) => {
                            popup.inputs[0].input(key);
                        }
                    },
                    KeyCode::Down => match app.working_area {
                        WorkingArea::Services => app.services.next(),
                        WorkingArea::Objects => app.objects.down(),
                        WorkingArea::PopUp(ref mut popup) => {
                            popup.selected = std::cmp::min(popup.selected + 1, popup.inputs.len());
                        }
                    },
                    KeyCode::Up => match app.working_area {
                        WorkingArea::Services => app.services.previous(),
                        WorkingArea::Objects => app.objects.up(),
                        WorkingArea::PopUp(ref mut popup) => {
                            popup.selected = popup.selected.saturating_sub(1);
                        }
                    },
                    KeyCode::Right => match app.working_area {
                        WorkingArea::Services => {}
                        WorkingArea::Objects => app.objects.right(),
                        WorkingArea::PopUp(ref mut popup) => {
                            popup.inputs[0].input(key);
                        }
                    },
                    KeyCode::Tab => match app.working_area {
                        WorkingArea::Services => app.working_area = WorkingArea::Objects,
                        WorkingArea::Objects => app.working_area = WorkingArea::Services,
                        WorkingArea::PopUp(ref _method) => {}
                    },
                    KeyCode::Esc => {
                        app.working_area = WorkingArea::Objects;
                    }
                    _ => match app.working_area {
                        WorkingArea::PopUp(ref mut popup) => {
                            popup.inputs[popup.selected].input(key);
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
