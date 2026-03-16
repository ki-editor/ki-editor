mod buffer;
mod git;

mod alternator;
mod app;
pub mod char_index_range;
mod cli;
mod clipboard;
mod components;
pub mod config;
mod context;
mod divide_viewport;
mod edit;
mod embed;
mod env;
pub mod file_watcher;
mod format_path_list;
pub mod frontend;
#[cfg(test)]
mod generate_recipes;
mod grid;
pub mod history;
mod integration_event;
#[cfg(test)]
mod integration_test;
mod keymap_override;
mod layout;
pub mod list;
mod lsp;
mod non_empty_extensions;
pub mod persistence;
mod position;
mod quickfix_list;
#[cfg(test)]
mod recipes;
mod rectangle;
mod render_flex_layout;
mod screen;
pub mod scripting;
mod search;
mod selection;
pub mod selection_mode;
pub mod selection_range;
pub mod soft_wrap;
pub mod style;
pub mod surround;
pub mod syntax_highlight;
#[cfg(test)]
mod test_app;
#[cfg(test)]
mod test_lsp;
#[cfg(test)]
mod test_search;
pub mod themes;
mod thread;
pub mod transformation;
pub mod ui_tree;
mod utils;
use std::{rc::Rc, sync::Mutex};

use anyhow::Context;
use frontend::crossterm::Crossterm;
use log::LevelFilter;
use shared::absolute_path::AbsolutePath;

use app::App;

use crate::{app::AppMessage, config::AppConfig, persistence::Persistence};

pub fn main() {
    cli::cli().unwrap();
}

#[derive(Default)]
pub struct RunConfig {
    pub entry_path: Option<AbsolutePath>,
    pub working_directory: Option<AbsolutePath>,
}

pub fn run(config: RunConfig) -> anyhow::Result<()> {
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
        AppConfig::singleton().status_lines(),
        None, // No integration event sender
        true,
        true,
        false,
        Some(Persistence::load_or_default(
            grammar::cache_dir().join("data.json"),
        )),
    )?;

    let sender = app.sender();

    std::thread::spawn(move || loop {
        let message = match crossterm::event::read() {
            Ok(event) => AppMessage::Event(event.into()),
            Err(err) => AppMessage::NotifyError(err),
        };

        let _ = sender
            .send(message)
            .map_err(|err| log::info!("main::run::crossterm {err:#?}"));
    });

    app.run(config.entry_path)
        .map_err(|error| anyhow::anyhow!("screen.run {:?}", error))?;

    Ok(())
}
