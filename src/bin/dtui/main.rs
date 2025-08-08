pub mod app2;
pub mod dbus_handler;
pub mod messages;
pub mod parser;
pub mod stateful_list;
pub mod stateful_tree;
pub mod ui;
mod action;
mod app;
mod components;
mod config;
mod error;
mod logging;
mod tui;
mod other;

use app2::{run_app, App};
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
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::prelude::*;

use zbus::{conn, Connection};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum BusType {
    System,
    Session,
}
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[clap(group(ArgGroup::new("bus_or_address").args(&["bus", "address"])))]
pub struct Args {
    //Which bus to connect to
    #[clap(default_value_t = BusType::System)]
    #[arg(value_enum)]
    pub bus: BusType,

    //Address of potentially remote connection
    #[clap(long)]
    pub address: Option<String>,

    #[clap(default_value_t = LevelFilter::OFF)]
    pub debug_level: LevelFilter,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    logging::init()?;
    error::init()?;
    let app = app::App::new(10.0, 60.0, args).await;
    match app {
        Ok(mut app) => {
            app.run().await?
        },
        Err(e) => println!("{}", e),
    }

    Ok(())
}
