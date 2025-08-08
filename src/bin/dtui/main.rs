mod action;
mod app;
mod components;
mod config;
pub mod dbus_handler;
mod error;
mod logging;
pub mod messages;
mod other;
pub mod parser;
pub mod stateful_list;
pub mod stateful_tree;
mod tui;

use clap::{ArgGroup, Parser, ValueEnum, command};

use std::error::Error;
use tracing::level_filters::LevelFilter;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum BusType {
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
        Ok(mut app) => app.run().await?,
        Err(e) => println!("{}", e),
    }

    Ok(())
}
