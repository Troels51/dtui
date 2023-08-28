pub mod app;
pub mod dbus_handler;
pub mod messages;
pub mod stateful_list;
pub mod stateful_tree;
pub mod ui;

use app::{run_app, App};
use clap::{Parser, command, ValueEnum};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use dbus_handler::DbusActorHandle;

use messages::AppMessage;

use std::{error::Error, io, time::Duration};
use tokio::sync::mpsc::{self};
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

use zbus::Connection;


#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum BusType {
    System,
    Session,
}
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    //Which bus to connect to
    #[clap(default_value_t = BusType::Session)]
    #[arg(value_enum)]
    bus: BusType,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    let args = Args::parse();
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(250);
    let connection = match args.bus {
        BusType::System => Connection::system().await?,
        BusType::Session => Connection::session().await?,
    };
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
