pub mod stateful_list;
pub mod stateful_tree;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use stateful_list::StatefulList;
use stateful_tree::StatefulTree;
use tui_tree_widget::{Tree, TreeItem};
use std::{
    collections::HashMap,
    error::Error,
    io,
    str::FromStr,
    time::{Duration, Instant}, path,
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style, Color},
    text::Spans,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use zbus::{
    fdo::DBusProxy,
    names::{OwnedBusName, OwnedInterfaceName},
    xml::Node,
    zvariant::{ObjectPath, OwnedObjectPath, OwnedValue},
    Connection,
};

enum WorkingArea {
    Services,
    Objects,
}

struct App<'a> {
    connection: Connection,
    services: StatefulList<OwnedBusName>,
    interfaces: Option<Node>,
    objects: StatefulTree<'a>,
    // objects: Option<
    //     HashMap<
    //         OwnedObjectPath,
    //         HashMap<OwnedInterfaceName, HashMap<std::string::String, OwnedValue>>,
    //     >,
    // >,
    working_area : WorkingArea, 
}

impl<'a> App<'a> {
    fn new(connection: Connection) -> App<'a> {
        App {
            connection: connection,
            services: StatefulList::with_items(vec![]),
            interfaces: None,
            objects: StatefulTree::new(),
            working_area: WorkingArea::Services,
        }
    }

    fn on_tick(&self) {}

    async fn get_interfaces_as_tree(&self, 
                                    busname: &OwnedBusName,
                                    path: &ObjectPath<'_>) -> Result<StatefulTree<'a>, zbus::Error> {
        let interfaces = self.get_interfaces(busname, path).await?;
        Ok(StatefulTree::new())
    }

    async fn get_services(&self) -> Result<Vec<OwnedBusName>, zbus::fdo::Error> {
        let dbusproxy = DBusProxy::new(&self.connection).await?;
        dbusproxy.list_names().await
    }

    //If this takes a path as well, it can call itself recursively and fill up its nodes
    async fn get_interfaces(
        &self,
        busname: &OwnedBusName,
        path: &ObjectPath<'_>,
    ) -> Result<Node, zbus::Error> {
        let introspectable_proxy = zbus::fdo::IntrospectableProxy::builder(&self.connection)
            .destination(busname)?
            .path(path)?
            .build()
            .await?;

        let introspect_xml = introspectable_proxy.introspect().await?;
        Node::from_str(&introspect_xml)
    }

    async fn get_objects(
        &self,
        busname: &OwnedBusName,
    ) -> Result<
        HashMap<
            OwnedObjectPath,
            HashMap<OwnedInterfaceName, HashMap<std::string::String, OwnedValue>>,
        >,
        zbus::fdo::Error,
    > {
        let object_manager = zbus::fdo::ObjectManagerProxy::builder(&self.connection)
            .destination(busname)?
            .path("/")?
            .build()
            .await?;
        object_manager.get_managed_objects().await
    }
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
    let connection = Connection::session().await?;
    let app = App::new(connection);
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
    app.services = StatefulList::with_items(app.get_services().await?);
    loop {
        terminal.draw(|frame| ui(frame, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Enter => {
                        if let Some(selected_index) = app.services.state.selected() {
                            let timeout_duration = Duration::from_secs(1);
                            let item = app.services.items[selected_index].clone();
                            if let Ok(timeout) =
                                tokio::time::timeout(timeout_duration, app.get_objects(&item)).await
                            {
                                //app.objects = Some(timeout.unwrap_or_default());
                            }
                            if let Ok(timeout) = tokio::time::timeout(
                                timeout_duration,
                                app.get_interfaces(
                                    &item,
                                    &ObjectPath::try_from("/").unwrap_or_default(),
                                ),
                            )
                            .await
                            {
                                app.interfaces = timeout.ok();
                            }
                        }
                    }
                    KeyCode::Left => {
                        match app.working_area {
                            WorkingArea::Services => app.services.unselect(),
                            WorkingArea::Objects =>  app.working_area = WorkingArea::Services,
                        }
                    }
                    KeyCode::Down => {
                      match app.working_area {
                        WorkingArea::Services => app.services.next(),
                        WorkingArea::Objects => app.objects.down(),  
                        }
                    }
                    KeyCode::Up => {
                        match app.working_area {
                            WorkingArea::Services => app.services.previous(),
                            WorkingArea::Objects => app.objects.up(),
                        }
                    }
                    KeyCode::Right => {
                        match app.working_area {
                            WorkingArea::Services => app.
                            working_area = WorkingArea::Objects,
                            WorkingArea::Objects => app.objects.right(),
                        }
                    }
                    _ => ()
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = Instant::now();
        }
    }
}

fn ui<B: Backend>(frame: &mut Frame<B>, app: &mut App) {
    // Create two chunks with equal horizontal screen space
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(frame.size());

    // Iterate through all elements in the `items` app and append some debug text to it.
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
        .block(Block::default().borders(Borders::ALL).title("Services"))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    // We can now render the item list
    frame.render_stateful_widget(items, chunks[0], &mut app.services.state);

    let objects: String = app
        .interfaces
        .as_ref()
        .map_or("Nothing".to_string(), |node| {
            node.nodes()
                .into_iter()
                .map(|node| {
                    let mut description = String::new();
                    if let Some(name) = node.name() {
                        description.push('/');
                        description.push_str(name);
                        description.push('\n');
                    }
                    description.push('\t');
                    node.interfaces()
                        .into_iter()
                        .fold(description, |acc, interface| {
                            acc + interface.methods()[0].name() + "\n"
                        })
                })
                .collect::<Vec<String>>()
                .join("\n")
        });
    // let objects_view = Paragraph::new(objects)
    //     .style(Style::default())
    //     .block(Block::default().borders(Borders::ALL).title("Objects"))
    //     .alignment(tui::layout::Alignment::Left)
    //     .wrap(tui::widgets::Wrap { trim: true });
    let objects_view =  Tree::new(app.objects.items.clone())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!("Tree Widget {:?}", app.objects.state)),
                )
                .highlight_style(
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");   
    frame.render_stateful_widget(objects_view, chunks[1], &mut app.objects.state);
    //frame.render_widget(objects_view, chunks[1]);
}
