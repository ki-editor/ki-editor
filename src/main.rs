mod auto_key_map;
mod edit;
mod engine;
mod screen;
mod selection;
mod terminal;
mod window;

use std::{
    io::{Read, Write},
    path::Path,
};

use log::LevelFilter;

use engine::Buffer;
use screen::Screen;
use terminal::run_integrated_terminal;

fn main() {
    simple_logging::log_to_file("my_log.txt", LevelFilter::Info).unwrap();
    let args = std::env::args().collect::<Vec<_>>();

    // run_integrated_terminal(24, 80).unwrap();
    // return;

    let default_path = "./src/main.rs".to_string();
    let filename = Path::new(args.get(1).unwrap_or(&default_path));
    let content = std::fs::read_to_string(&filename).unwrap();
    let language = match filename.extension().unwrap().to_str().unwrap() {
        "js" | "jsx" => tree_sitter_javascript::language(),
        "ts" => tree_sitter_typescript::language_typescript(),
        "tsx" => tree_sitter_typescript::language_tsx(),
        "rs" => tree_sitter_rust::language(),
        "md" => tree_sitter_md::language(),
        _ => panic!("Unsupported file extension"),
    };

    let mut screen = Screen::new();
    screen.run(Buffer::new(language, &content)).unwrap();
}
