use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode};
use tokio::sync::mpsc::Receiver;
use tui::{backend::Backend, Terminal};
use tui_tree_widget::TreeItem;
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

pub struct App<'a> {
    dbus_rx: Receiver<AppMessage>,
    dbus_handle: DbusActorHandle,
    pub services: StatefulList<OwnedBusName>,
    pub objects: StatefulTree<'a>,

    pub working_area: WorkingArea,
}

impl<'a> App<'a> {
    pub fn new(dbus_rx: Receiver<AppMessage>, dbus_handle: DbusActorHandle) -> App<'a> {
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
    mut app: App<'_>,
    tick_rate: Duration,
) -> Result<(), zbus::Error> {
    let mut last_tick = Instant::now();
    app.dbus_handle.request_services().await;

    loop {
        terminal.draw(|frame| ui(frame, &mut app))?;

        match app.dbus_rx.try_recv() {
            Ok(message) => match message {
                AppMessage::Objects(nodes) => {
                    app.objects = StatefulTree::with_items(
                        nodes
                            .iter()
                            .map(|(object_name, node)| {
                                let children: Vec<TreeItem> = node
                                    .interfaces()
                                    .iter()
                                    .map(|interface| {
                                        let methods: Vec<TreeItem> = interface
                                            .methods()
                                            .iter()
                                            .map(|method| {
                                                TreeItem::new_leaf(method.name().to_string())
                                            })
                                            .collect();
                                        let properties: Vec<TreeItem> = interface
                                            .properties()
                                            .iter()
                                            .map(|property| {
                                                TreeItem::new_leaf(property.name().to_string())
                                            })
                                            .collect();
                                        let signals: Vec<TreeItem> = interface
                                            .signals()
                                            .iter()
                                            .map(|signal| {
                                                TreeItem::new_leaf(signal.name().to_string())
                                            })
                                            .collect();
                                        let annotations: Vec<TreeItem> = interface
                                            .annotations()
                                            .iter()
                                            .map(|annotation| {
                                                TreeItem::new_leaf(annotation.name().to_string())
                                            })
                                            .collect();
                                        let methods_tree = TreeItem::new("Methods", methods);
                                        let properties_tree =
                                            TreeItem::new("Properties", properties);
                                        let signals_tree = TreeItem::new("Signals", signals);
                                        let annotations_tree =
                                            TreeItem::new("Annotations", annotations);

                                        TreeItem::new(
                                            interface.name().to_string(),
                                            vec![
                                                methods_tree,
                                                properties_tree,
                                                signals_tree,
                                                annotations_tree,
                                            ],
                                        )
                                    })
                                    .collect();
                                TreeItem::new(object_name.clone(), children)
                            })
                            .collect(),
                    );
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
                        },
                        WorkingArea::Objects => {
                            //TOTO
                        }
                    }
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
                        WorkingArea::Services => {},
                        WorkingArea::Objects => app.objects.right(),
                    },
                    KeyCode::Tab => match app.working_area {
                        WorkingArea::Services => app.working_area = WorkingArea::Objects,
                        WorkingArea::Objects => app.working_area = WorkingArea::Services,
                    }
                    _ => (),
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }
    }
}
