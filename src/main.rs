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
#[cfg(test)]
mod integration_test;

mod layout;
pub mod list;
mod lsp;
mod position;

mod app;
pub mod history;
mod quickfix_list;
mod rectangle;
mod screen;
mod selection;
pub mod selection_mode;
pub mod selection_range;
pub mod soft_wrap;
pub mod style;
pub mod surround;
pub mod syntax_highlight;
mod terminal;
#[cfg(test)]
mod test_app;
pub mod themes;
pub mod transformation;
pub mod tree_sitter_traversal;
pub mod ui_tree;
pub mod undo_tree;
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

pub(crate) fn run(path: Option<CanonicalizedPath>) -> anyhow::Result<()> {
    simple_logging::log_to_file("my_log.txt", LevelFilter::Info)?;
    let _args = std::env::args().collect::<Vec<_>>();
    // run_integrated_terminal(24, 80).unwrap();
    // return;

    let (sender, receiver) = std::sync::mpsc::channel();
    let syntax_highlighter_sender = syntax_highlight::start_thread(sender.clone());
    let mut app = App::from_channel(
        Arc::new(Mutex::new(Crossterm::default())),
        CanonicalizedPath::try_from(".")?,
        sender,
        receiver,
    )?;
    app.set_syntax_highlight_request_sender(syntax_highlighter_sender);

    let sender = app.sender();

    let crossterm_join_handle = std::thread::spawn(move || loop {
        if crossterm::event::read()
            .map_err(|error| anyhow::anyhow!("{:?}", error))
            .and_then(|event| Ok(sender.send(AppMessage::Event(event.into()))?))
            .is_err()
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
