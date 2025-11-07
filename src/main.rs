mod buffer;
mod git;

pub(crate) mod char_index_range;
mod cli;
mod clipboard;
mod components;
mod context;
mod edit;
pub(crate) mod frontend;
mod grid;
mod integration_event;
#[cfg(test)]
mod integration_test;
mod search;

mod layout;
pub(crate) mod list;
mod lsp;
mod position;

mod app;
#[cfg(test)]
mod generate_recipes;
pub(crate) mod history;
mod non_empty_extensions;
mod osc52;
pub(crate) mod process_manager;
mod quickfix_list;
#[cfg(test)]
mod recipes;
mod rectangle;
mod screen;
mod selection;
pub(crate) mod selection_mode;
pub(crate) mod selection_range;
pub(crate) mod soft_wrap;
pub(crate) mod style;
pub(crate) mod surround;
pub(crate) mod syntax_highlight;
mod terminal;
#[cfg(test)]
mod test_app;
pub(crate) mod themes;
pub(crate) mod transformation;
pub(crate) mod ui_tree;
mod utils;

mod embed;

mod alternator;
pub(crate) mod custom_config;
mod divide_viewport;
mod env;
pub(crate) mod file_watcher;
mod format_path_list;
pub(crate) mod persistence;
#[cfg(test)]
mod test_lsp;
mod thread;
use std::{rc::Rc, sync::Mutex};

use anyhow::Context;
use frontend::crossterm::Crossterm;
use log::LevelFilter;
use shared::canonicalized_path::CanonicalizedPath;

use app::{App, StatusLineComponent};

use crate::{app::AppMessage, persistence::Persistence};

fn main() {
    cli::cli().unwrap();
}
#[derive(Default)]
pub(crate) struct RunConfig {
    pub(crate) entry_path: Option<CanonicalizedPath>,
    pub(crate) working_directory: Option<CanonicalizedPath>,
}

pub(crate) fn run(config: RunConfig) -> anyhow::Result<()> {
    std::fs::create_dir_all(grammar::cache_dir()).context("Failed to create cache_dir")?;
    simple_logging::log_to_file(grammar::default_log_file(), LevelFilter::Info)?;
    let (sender, receiver) = std::sync::mpsc::channel();
    let syntax_highlighter_sender = syntax_highlight::start_thread(sender.clone());

    let app = App::from_channel(
        Rc::new(Mutex::new(Crossterm::new()?)),
        config.working_directory.unwrap_or(".".try_into()?),
        sender,
        receiver,
        Some(syntax_highlighter_sender),
        [
            StatusLineComponent::Mode,
            StatusLineComponent::SelectionMode,
            StatusLineComponent::LastSearchString,
            StatusLineComponent::Reveal,
            StatusLineComponent::CurrentWorkingDirectory,
            StatusLineComponent::GitBranch,
            StatusLineComponent::KeyboardLayout,
            StatusLineComponent::Help,
            StatusLineComponent::LastDispatch,
        ]
        .to_vec(),
        None, // No integration event sender
        true,
        true,
        false,
        Some(Persistence::load_or_default(
            grammar::cache_dir().join("data.json"),
        )),
    )?;

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

    app.run(config.entry_path)
        .map_err(|error| anyhow::anyhow!("screen.run {:?}", error))?;

    crossterm_join_handle.join().unwrap();

    Ok(())
}
