mod edit;
mod engine;
mod screen;
mod selection;
mod window;

use std::path::Path;

use log::LevelFilter;

use engine::Buffer;
use screen::Screen;

fn main() {
    simple_logging::log_to_file("my_log.txt", LevelFilter::Info).unwrap();
    let args = std::env::args().collect::<Vec<_>>();
    let filename = Path::new(args.get(1).unwrap());
    let content = std::fs::read_to_string(&filename).unwrap();
    let language = match filename.extension().unwrap().to_str().unwrap() {
        "js" | "ts" | "tsx" | "jsx" => tree_sitter_javascript::language(),
        "rs" => tree_sitter_rust::language(),
        _ => panic!("Unsupported file extension"),
    };

    let mut screen = Screen::new();
    screen.run(Buffer::new(language, &content)).unwrap();
}
