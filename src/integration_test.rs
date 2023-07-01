#[cfg(test)]
mod integration_test {
    use std::time::Duration;

    
    

    use crate::screen::Screen;

    // #[test]
    fn lsp_completion() {
        let mut screen = Screen::new().unwrap();
        let (_sender, receiver) = std::sync::mpsc::channel();
        std::thread::sleep(Duration::from_secs(5));
        // for event in parse_key_events("c-q").unwrap() {
        //     sender.send(event).unwrap();
        // }
        screen.run(None, receiver).unwrap();
    }
}
