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
#[cfg(test)]
mod integration_test;

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

mod divide_viewport;
mod env;
mod format_path_list;
use std::{rc::Rc, sync::Mutex};

use anyhow::Context;
use frontend::crossterm::Crossterm;
use log::LevelFilter;
use shared::canonicalized_path::CanonicalizedPath;

use app::{App, StatusLineComponent};

use crate::app::AppMessage;

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
    let mut app = App::from_channel(
        Rc::new(Mutex::new(Crossterm::new()?)),
        config.working_directory.unwrap_or(".".try_into()?),
        sender,
        receiver,
        [
            StatusLineComponent::Help,
            StatusLineComponent::KeyboardLayout,
            StatusLineComponent::CurrentWorkingDirectory,
            StatusLineComponent::GitBranch,
            StatusLineComponent::ViewAlignment,
            StatusLineComponent::Reveal,
            StatusLineComponent::Mode,
            StatusLineComponent::SelectionMode,
            StatusLineComponent::LocalSearchConfig,
            StatusLineComponent::LastDispatch,
        ]
        .to_vec(),
    )?;
    app.set_syntax_highlight_request_sender(syntax_highlighter_sender);

    let sender = app.sender();

    let crossterm_join_handle = std::thread::spawn(move || loop {
        let message = match crossterm::event::read() {
            Ok(event) => {
                match event {
                    crossterm::event::Event::Key(key_event) => {
                        // Only process key press events, not releases
                        // This is especially important for Windows compatibility
                        if key_event.kind == crossterm::event::KeyEventKind::Press {
                            AppMessage::Event(event.into())
                        } else {
                            // Skip release events by continuing the loop
                            continue;
                        }
                    }
                    // For non-keyboard events, process as before
                    other_event => AppMessage::Event(other_event.into()),
                }
            }
            Err(err) => AppMessage::NotifyError(err),
        };

        let _ = sender
            .send(message)
            .map_err(|err| log::info!("main::run::crossterm {err:#?}"));
    });

    app.run(config.entry_path)
        .map_err(|error| anyhow::anyhow!("screen.run {:?}", error))?;

    crossterm_join_handle.join().unwrap();

    Ok(())
}
