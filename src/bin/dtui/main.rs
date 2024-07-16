pub mod app;
pub mod dbus_handler;
pub mod messages;
pub mod stateful_list;
pub mod stateful_tree;
pub mod ui;

use app::{run_app, App};
use clap::{command, ArgGroup, Parser, ValueEnum};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dbus_handler::DbusActorHandle;

use messages::AppMessage;

use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::{error::Error, io, time::Duration};
use tokio::sync::mpsc::{self};
use tracing::{level_filters::LevelFilter};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{layer::SubscriberExt};

use zbus::{Connection, ConnectionBuilder};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum BusType {
    System,
    Session,
}
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[clap(group(ArgGroup::new("bus_or_address").args(&["bus", "address"])))]
struct Args {
    //Which bus to connect to
    #[clap(default_value_t = BusType::System)]
    #[arg(value_enum)]
    bus: BusType,

    //Address of potentially remote connection
    #[clap(long)]
    address: Option<String>,

    #[clap(default_value_t = LevelFilter::OFF)]
    debug_level: LevelFilter,
}

// This function is mainly used to make error handling nicer, so that we can cleanup the terminal nicely
async fn run<B>(terminal: &mut Terminal<B>, args: Args) -> Result<(), zbus::Error>
where
    B: Backend,
{
    // create app and run it
    let tick_rate = Duration::from_millis(250);
    let mut connection = match args.bus {
        BusType::System => Connection::system().await?,
        BusType::Session => Connection::session().await?,
    };
    if let Some(address) = args.address {
        connection = ConnectionBuilder::address(address.as_str())?
            .build()
            .await?;
    }
    let (dbus_handler_sender, app_receiver) = mpsc::channel::<AppMessage>(16);
    let dbus_handler = DbusActorHandle::new(dbus_handler_sender, connection);
    // setup terminal
    let app = App::new(app_receiver, dbus_handler);
    run_app(terminal, app, tick_rate).await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    // Initialize tracing to journald
    let registry = tracing_subscriber::registry();
    match tracing_journald::layer() {
        Ok(layer) => registry.with(layer.with_filter(args.debug_level)).init(),
        Err(_) => (),
    }
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run(&mut terminal, args).await;

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{}", err);
    }
    Ok(())
}
