mod buffer;
mod canonicalized_path;
pub mod char_index_range;
mod clipboard;
mod components;
mod context;
mod edit;
pub mod frontend;
mod grid;
mod integration_test;
pub mod language;
mod layout;
pub mod list;
mod lsp;
mod position;
pub mod process_command;
mod quickfix_list;
mod rectangle;
mod screen;
mod selection;
pub mod selection_mode;
pub mod soft_wrap;
pub mod syntax_highlight;
mod terminal;
pub mod themes;
mod utils;

use std::sync::{Arc, Mutex};

use canonicalized_path::CanonicalizedPath;

use frontend::crossterm::Crossterm;
use log::LevelFilter;

use screen::Screen;

fn main() {
    run().unwrap_or_else(|error| {
        log::error!("main run {:?}", error);
    });
}

fn run() -> anyhow::Result<()> {
    simple_logging::log_to_file("my_log.txt", LevelFilter::Info)?;
    let args = std::env::args().collect::<Vec<_>>();
    // run_integrated_terminal(24, 80).unwrap();
    // return;

    let path = args.get(1).and_then(|arg| {
        CanonicalizedPath::try_from(arg)
            .map(Some)
            .unwrap_or_else(|err| {
                println!("Invalid path: {}", err);
                None
            })
    });
    let mut screen = Screen::new(
        Arc::new(Mutex::new(Crossterm::new())),
        CanonicalizedPath::try_from(".")?,
    )?;

    let (sender, receiver) = std::sync::mpsc::channel();
    let join_handle = std::thread::spawn(move || loop {
        crossterm::event::read()
            .map_err(|error| anyhow::anyhow!("{:?}", error))
            .and_then(|event| {
                sender
                    .send(event.into())
                    .map_err(|error| anyhow::anyhow!("{:?}", error))
            })
            .unwrap_or_else(|error| {
                log::info!("Error running crossterm::event::read: {:?}", error);
            });
    });
    screen
        .run(path, receiver)
        .map_err(|error| anyhow::anyhow!("screen.run {:?}", error))?;

    join_handle.join().unwrap();

    println!("Goodbye!");

    Ok(())
}
