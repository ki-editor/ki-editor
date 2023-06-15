mod auto_key_map;
mod buffer;
mod components;
mod edit;
mod grid;
mod lsp;
mod position;
mod quickfix_list;
mod rectangle;
mod screen;
mod selection;
mod terminal;
mod utils;

use std::path::Path;

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

    let path = Path::new(args.get(1).unwrap_or(&default_path));

    let mut screen = Screen::new()?;
    screen.run(&path.to_path_buf()).unwrap();
    Ok(())
}
