mod buffer;
mod git;

pub mod char_index_range;
mod cli;
mod clipboard;
pub mod command;
mod components;
mod context;
mod edit;
pub mod frontend;
mod grid;
mod integration_test;

mod layout;
pub mod list;
mod lsp;
mod position;

mod app;
mod quickfix_list;
mod rectangle;
mod selection;
pub mod selection_mode;
pub mod selection_range;
pub mod soft_wrap;
pub mod syntax_highlight;
mod terminal;
mod test_app;
pub mod themes;
mod utils;

use std::sync::{Arc, Mutex};

use frontend::crossterm::Crossterm;
use log::LevelFilter;
use shared::canonicalized_path::CanonicalizedPath;

use app::App;

use crate::app::AppMessage;

fn main() {
    cli::cli().unwrap();
}

pub fn run(path: Option<CanonicalizedPath>) -> anyhow::Result<()> {
    simple_logging::log_to_file("my_log.txt", LevelFilter::Info)?;
    let _args = std::env::args().collect::<Vec<_>>();
    // run_integrated_terminal(24, 80).unwrap();
    // return;

    let (sender, receiver) = std::sync::mpsc::channel();
    let app = App::from_channel(
        Arc::new(Mutex::new(Crossterm::new())),
        CanonicalizedPath::try_from(".")?,
        sender,
        receiver,
    )?;

    let sender = app.sender();

    let crossterm_join_handle = std::thread::spawn(move || loop {
        if let Err(_) = crossterm::event::read()
            .map_err(|error| anyhow::anyhow!("{:?}", error))
            .and_then(|event| Ok(sender.send(AppMessage::Event(event.into()))?))
        {
            break;
        }
    });

    app.run(path)
        .map_err(|error| anyhow::anyhow!("screen.run {:?}", error))?;

    crossterm_join_handle.join().unwrap();

    println!("Goodbye!");

    Ok(())
}
