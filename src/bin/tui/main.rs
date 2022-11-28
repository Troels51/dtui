use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use zbus::{Connection, fdo::DBusProxy, xml::Node, names::OwnedBusName};
use std::{
    error::Error,
    io,
    time::{Duration, Instant}, str::FromStr,
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Corner, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};

struct StatefulList<T> {
    state: ListState,
    items: Vec<T>,
}

impl<T> StatefulList<T> {
    fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn unselect(&mut self) {
        self.state.select(None);
    }
}

/// This struct holds the current state of the app. In particular, it has the `items` field which is a wrapper
/// around `ListState`. Keeping track of the items state let us render the associated widget with its state
/// and have access to features such as natural scrolling.
///
/// Check the event handling at the bottom to see how to change the state on incoming events.
/// Check the drawing logic for items on how to specify the highlighting style for selected items.
struct App {
    services: StatefulList<OwnedBusName>,
    interfaces: Option<Node>,
}

impl App {
    fn new(busnames: Vec<OwnedBusName>) -> App {
        App {
            services: StatefulList::with_items(busnames),
            interfaces: None
        }
    }

    /// Rotate through the event list.
    /// This only exists to simulate some kind of "progress"
    fn on_tick(&mut self) {
    }
}

async fn get_services() -> Result<Vec<OwnedBusName>, zbus::fdo::Error> {
    let connection = Connection::system().await?;

    let dbusproxy = DBusProxy::new(&connection).await?;
    dbusproxy.list_names().await
}

async fn get_interfaces(busname: &OwnedBusName) -> Result<Node, zbus::Error> {
    let connection = Connection::system().await?;
    let introspectableProxy = zbus::fdo::IntrospectableProxy::builder(&connection)
                                .destination(busname)?
                                .build().await?;

    let introspect_xml = introspectableProxy.introspect().await?;
    dbg!(&introspect_xml);
    Node::from_str(&introspect_xml)
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
    let services = get_services().await?;
    let app = App::new(services);
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
    mut app: App,
    tick_rate: Duration,
) -> Result<(), zbus::Error> {
    let mut last_tick = Instant::now();
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
                            app.interfaces = Some(get_interfaces(&app.services.items[selected_index]).await?)
                        }
                    },
                    KeyCode::Left => app.services.unselect(),
                    KeyCode::Down => app.services.next(),
                    KeyCode::Up => app.services.previous(),
                    _ => {}
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
        .block(Block::default().borders(Borders::ALL).title("List"))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    // We can now render the item list
    frame.render_stateful_widget(items, chunks[0], &mut app.services.state);

    // Let's do the same for the events.
    // The event list doesn't have any state and only displays the current state of the list.
    
    let a : String = app.interfaces.as_ref().map_or("default".to_string(), |node| {
        node.interfaces().into_iter()
            .map(|interface|
            {
                interface.name()
            })
            .collect::<Vec<&str>>().join("\n")
    });


    let paragraph = Paragraph::new(a)
        .block(Block::default().borders(Borders::ALL).title("Interfaces"));

    frame.render_widget(paragraph, chunks[1]);
}
