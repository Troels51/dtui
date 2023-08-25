pub mod dbus_handler;
pub mod messages;
pub mod stateful_list;
pub mod stateful_tree;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dbus_handler::{DbusActor, DbusActorHandle};
use itertools::Itertools;
use messages::{AppMessage, DbusMessage};
use stateful_list::StatefulList;
use stateful_tree::StatefulTree;
use std::{
    collections::HashMap,
    error::Error,
    io, path,
    str::FromStr,
    time::{Duration, Instant},
};
use tokio::{
    select,
    sync::mpsc::{self, Receiver, Sender},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use tui_tree_widget::{Tree, TreeItem};
use zbus::{
    fdo::{DBusProxy, Properties},
    names::{OwnedBusName, OwnedInterfaceName},
    xml::Node,
    zvariant::{ObjectPath, OwnedObjectPath, OwnedValue},
    Connection,
};
#[derive(PartialEq)]
enum WorkingArea {
    Services,
    Objects,
}

struct App<'a> {
    dbus_rx: Receiver<AppMessage>,
    dbus_handle: DbusActorHandle,
    services: StatefulList<OwnedBusName>,
    objects: StatefulTree<'a>,

    working_area: WorkingArea,
}

impl<'a> App<'a> {
    fn new(dbus_rx: Receiver<AppMessage>, dbus_handle: DbusActorHandle) -> App<'a> {
        App {
            dbus_rx: dbus_rx,
            dbus_handle: dbus_handle,
            services: StatefulList::with_items(vec![]),
            objects: StatefulTree::new(),
            working_area: WorkingArea::Services,
        }
    }

    fn on_tick(&self) {}
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(250);
    let connection = Connection::system().await?;

    let (dbus_handler_sender, app_receiver) = mpsc::channel::<AppMessage>(16);
    let dbus_handler = DbusActorHandle::new(dbus_handler_sender, connection);
    let app = App::new(app_receiver, dbus_handler);
    let res = run_app(&mut terminal, app, tick_rate).await;
    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

async fn run_app<B: Backend>(
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
            Error => (),
        };
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Enter => {
                        if let Some(selected_index) = app.services.state.selected() {
                            let item = app.services.items[selected_index].clone();
                            app.dbus_handle.request_objects_from(item).await;
                        }
                    }
                    KeyCode::Left => match app.working_area {
                        WorkingArea::Services => app.services.unselect(),
                        WorkingArea::Objects => app.working_area = WorkingArea::Services,
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
                        WorkingArea::Services => app.working_area = WorkingArea::Objects,
                        WorkingArea::Objects => app.objects.right(),
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
fn working_area_border(app: &App, working_area: WorkingArea) -> Color {
    if app.working_area == working_area {
        Color::LightBlue
    } else {
        Color::White
    }
}
fn ui<B: Backend>(frame: &mut Frame<B>, app: &mut App) {
    // Create two chunks with equal horizontal screen space
    let full = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),
            Constraint::Max(2)
        ])
        .split(frame.size());
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(25),
                Constraint::Percentage(75),
            ]
            .as_ref(),
        )
        .split(full[0]);
    let items: Vec<ListItem> = app
        .services
        .items
        .iter()
        .map(|i| {
            let lines = vec![Spans::from(i.as_str())];
            ListItem::new(lines).style(Style::default())
        })
        .collect();

    // Create a List from all list items and highlight the currently selected one
    let items = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Services")
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(working_area_border(app, WorkingArea::Services))),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    // We can now render the item list
    frame.render_stateful_widget(items, chunks[0], &mut app.services.state);

    let objects_view = Tree::new(app.objects.items.clone())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(working_area_border(app, WorkingArea::Objects)))
                .title("Objects"),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");
    frame.render_stateful_widget(objects_view, chunks[1], &mut app.objects.state);
    let bottom_text = vec![
        Spans::from(Span::raw("Navigation: ← ↓ ↑ →, Query Service: Enter, Quit: q")),
    ];
    let helper_paragraph = Paragraph::new(bottom_text).alignment(Alignment::Center);
    frame.render_widget(helper_paragraph, full[1]);
}
