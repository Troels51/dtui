use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode};
use itertools::Itertools;
use ratatui::{backend::Backend, Terminal};
use tokio::sync::mpsc::Receiver;
use tui_tree_widget::TreeItem;
use zbus::names::OwnedBusName;
use zbus_xml::ArgDirection;

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
    mut app: App<'static>,
    tick_rate: Duration,
) -> Result<(), zbus::Error> {
    let mut last_tick = Instant::now();
    app.dbus_handle.request_services().await;

    loop {
        terminal.draw(|frame| ui::<B>(frame, &mut app))?;

        match app.dbus_rx.try_recv() {
            Ok(message) => match message {
                AppMessage::Objects(nodes) => {
                    app.objects = StatefulTree::with_items(
                        nodes
                            .iter()
                            .sorted_by(|a, b| a.0.cmp(b.0))
                            .enumerate()
                            .map(|(id, (object_name, node))| {
                                let children: Vec<TreeItem<usize>> = node
                                    .interfaces()
                                    .iter()
                                    .enumerate()
                                    .map(|(id, interface)| {
                                        let methods: Vec<TreeItem<usize>> = interface
                                            .methods()
                                            .iter()
                                            .enumerate()
                                            .map(|(id, method)| {
                                                let inputs: Vec<String> = method
                                                    .args()
                                                    .iter()
                                                    .filter(|arg| {
                                                        arg.direction()
                                                            .is_some_and(|s| s == ArgDirection::In)
                                                    })
                                                    .map(|arg| {
                                                        format!(
                                                            "{}: {}",
                                                            arg.name().unwrap_or_default(),
                                                            arg.ty()
                                                        )
                                                    })
                                                    .collect();
                                                let outputs: Vec<String> = method
                                                    .args()
                                                    .iter()
                                                    .filter(|arg| {
                                                        arg.direction()
                                                            .is_some_and(|s| s == ArgDirection::Out)
                                                    })
                                                    .map(|arg| {
                                                        format!(
                                                            "{}: {}",
                                                            arg.name().unwrap_or_default(),
                                                            arg.ty()
                                                        )
                                                    })
                                                    .collect();
                                                let return_arrow =
                                                    if outputs.is_empty() { "" } else { "=>" }; // If we dont return anything, the arrow shouldnt be there
                                                let leaf_string: String = format!(
                                                    "{}({}) {} {}",
                                                    method.name(),
                                                    inputs.join(", "),
                                                    return_arrow,
                                                    outputs.join(", ")
                                                );
                                                TreeItem::new_leaf(id, leaf_string)
                                            })
                                            .collect();
                                        let properties: Vec<TreeItem<usize>> = interface
                                            .properties()
                                            .iter()
                                            .enumerate()
                                            .map(|(id, property)| {
                                                TreeItem::new_leaf(
                                                    id,
                                                    format!(
                                                        "{}: {}",
                                                        property.name(),
                                                        property.ty()
                                                    ),
                                                )
                                            })
                                            .collect();
                                        let signals: Vec<TreeItem<usize>> = interface
                                            .signals()
                                            .iter()
                                            .enumerate()
                                            .map(|(id, signal)| {
                                                // Signals can only have input parameters
                                                let inputs: Vec<String> = signal
                                                    .args()
                                                    .iter()
                                                    .filter(|arg| {
                                                        arg.direction()
                                                            .is_some_and(|s| s == ArgDirection::In)
                                                    })
                                                    .map(|arg| {
                                                        format!(
                                                            "{}: {}",
                                                            arg.name().unwrap_or_default(),
                                                            arg.ty()
                                                        )
                                                    })
                                                    .collect();
                                                let leaf_string: String = format!(
                                                    "{}({})",
                                                    signal.name(),
                                                    inputs.join(", ")
                                                );
                                                TreeItem::new_leaf(id, leaf_string)
                                            })
                                            .collect();
                                        // let annotations: Vec<TreeItem> = interface
                                        //     .annotations()
                                        //     .iter()
                                        //     .map(|annotation| {
                                        //         TreeItem::new_leaf(annotation.name().to_string())
                                        //     })
                                        //     .collect();
                                        let methods_tree = TreeItem::new(0, "Methods", methods)
                                            .expect("Methods should have different ids");
                                        let properties_tree =
                                            TreeItem::new(1, "Properties", properties)
                                                .expect("Properties should have different ids");
                                        let signals_tree = TreeItem::new(2, "Signals", signals)
                                            .expect("Signals should have different ids");
                                        // let annotations_tree =
                                        //     TreeItem::new("Annotations", annotations);
                                        // TODO: Annotations are used differently, so i dont want to waste space with it
                                        TreeItem::new(
                                            id,
                                            interface.name().to_string(),
                                            vec![
                                                methods_tree,
                                                properties_tree,
                                                signals_tree,
                                                // annotations_tree,
                                            ],
                                        )
                                        .unwrap()
                                    })
                                    .collect();
                                TreeItem::new(id, object_name.clone(), children).unwrap()
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
            }
        }
        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }
    }
}
