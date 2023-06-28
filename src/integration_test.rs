#[cfg(test)]
mod integration_test {
    use std::time::Duration;

    use crossterm::event::Event;

    use crate::{
        canonicalized_path::CanonicalizedPath, key_event_parser::parse_key_events, screen::Screen,
    };

    // #[test]
    fn lsp_completion() {
        let mut screen = Screen::new().unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::sleep(Duration::from_secs(5));
        for event in parse_key_events("c-q").unwrap() {
            sender.send(Event::Key(event)).unwrap();
        }
        screen.run(None, receiver).unwrap();
    }
}
