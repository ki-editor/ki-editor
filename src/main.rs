mod buffer;
mod canonicalized_path;
mod clipboard;
mod components;
mod context;
mod edit;
mod grid;
mod key_event_parser;
pub mod language;
mod layout;
mod lsp;
mod position;
pub mod process_command;
mod quickfix_list;
mod rectangle;
mod screen;
mod selection;
mod terminal;
mod utils;

use canonicalized_path::CanonicalizedPath;

use log::LevelFilter;

use screen::Screen;

fn main() {
    run().unwrap_or_else(|error| {
        log::error!("{:?}", error);
    });
}

fn run() -> anyhow::Result<()> {
    simple_logging::log_to_file("my_log.txt", LevelFilter::Info)?;
    let args = std::env::args().collect::<Vec<_>>();
    // run_integrated_terminal(24, 80).unwrap();
    // return;

    let default_path = "./src/main.rs".to_string();

    let path = CanonicalizedPath::try_from(args.get(1).unwrap_or(&default_path)).unwrap();
    let mut screen = Screen::new()?;
    screen.run(&path).unwrap();
    Ok(())
}
