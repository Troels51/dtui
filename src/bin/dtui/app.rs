use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode};
use ratatui::{backend::Backend, Terminal};
use tokio::sync::mpsc::Receiver;
use tracing::Level;
use zbus::names::OwnedBusName;

use crate::{
    dbus_handler::DbusActorHandle, messages::AppMessage, stateful_list::StatefulList,
    stateful_tree::StatefulTree, ui::ui,
};

#[derive(PartialEq)]
pub enum WorkingArea {
    Services,
    Objects,
}

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
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Enter => match app.working_area {
                        WorkingArea::Services => {
                            if let Some(selected_index) = app.services.state.selected() {
                                let item = app.services.items[selected_index].clone();
                                app.dbus_handle.request_objects_from(item).await;
                            }
                        }
                        WorkingArea::Objects => {
                            //TOTO
                        }
                    },
                    KeyCode::Left => match app.working_area {
                        WorkingArea::Services => app.services.unselect(),
                        WorkingArea::Objects => app.objects.left(),
                    },
                    KeyCode::Down => match app.working_area {
                        WorkingArea::Services => app.services.next(),
                        WorkingArea::Objects => app.objects.down(),
                    },
                    KeyCode::Up => match app.working_area {
                        WorkingArea::Services => app.services.previous(),
                        WorkingArea::Objects => app.objects.up(),
                    },
                    KeyCode::Right => match app.working_area {
                        WorkingArea::Services => {}
                        WorkingArea::Objects => app.objects.right(),
                    },
                    KeyCode::Tab => match app.working_area {
                        WorkingArea::Services => app.working_area = WorkingArea::Objects,
                        WorkingArea::Objects => app.working_area = WorkingArea::Services,
                    },
                    _ => (),
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
