mod auto_key_map;
mod buffer;
mod edit;
mod engine;
mod grid;
mod rectangle;
mod screen;
mod selection;
mod terminal;
mod utils;

use std::path::Path;

use buffer::Buffer;
use log::LevelFilter;

use screen::Screen;
use terminal::run_integrated_terminal;

fn main() {
    simple_logging::log_to_file("my_log.txt", LevelFilter::Info).unwrap();
    let args = std::env::args().collect::<Vec<_>>();

    // run_integrated_terminal(24, 80).unwrap();
    // return;

    let default_path = "./src/main.rs".to_string();

    let path = Path::new(args.get(1).unwrap_or(&default_path));
    let mut screen = Screen::new();
    screen.run(Buffer::from_path(path)).unwrap();
}
