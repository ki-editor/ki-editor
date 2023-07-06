mod buffer;
mod canonicalized_path;
mod clipboard;
mod components;
mod context;
mod edit;
pub mod frontend;
mod grid;
mod integration_test;
pub mod language;
mod layout;
mod lsp;
mod position;
pub mod process_command;
mod quickfix_list;
mod rectangle;
mod screen;
mod selection;
pub mod syntax_highlight;
mod terminal;
pub mod themes;
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

    let path = args.get(1).and_then(|arg| {
        CanonicalizedPath::try_from(arg)
            .map(Some)
            .unwrap_or_else(|err| {
                println!("Invalid path: {}", err);
                None
            })
    });
    let mut screen = Screen::new()?;

    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || loop {
        crossterm::event::read()
            .map_err(|error| anyhow::anyhow!("{:?}", error))
            .and_then(|event| {
                sender
                    .send(event)
                    .map_err(|error| anyhow::anyhow!("{:?}", error))
            })
            .unwrap_or_else(|error| {
                log::info!("Error running crossterm::event::read: {:?}", error);
            });
    });
    screen.run(path, receiver).unwrap();

    Ok(())
}
