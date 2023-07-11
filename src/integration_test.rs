#[cfg(test)]
mod integration_test {
    use std::{
        sync::{
            mpsc::{channel, Sender},
            Arc, Mutex,
        },
        thread::sleep,
        time::Duration,
    };

    use event::{event::Event, KeyEvent};
    use key_event_macro::keys;

    use crate::{
        canonicalized_path::CanonicalizedPath, frontend::mock::MockFrontend, screen::Screen,
    };

    struct TestRunner {
        key_event_sender: Sender<Event>,
    }

    impl TestRunner {
        fn new(key_event_sender: Sender<Event>) -> Self {
            Self { key_event_sender }
        }

        fn send_key(&self, key: KeyEvent) -> anyhow::Result<()> {
            self.key_event_sender
                .send(Event::Key(key))
                .map_err(|error| anyhow::anyhow!("{:?}", error))
        }

        fn send_keys(&self, keys: &[KeyEvent]) -> anyhow::Result<()> {
            for key in keys.iter() {
                self.send_key(key.clone())?;
            }
            Ok(())
        }
    }

    #[test]
    fn lsp_completion() -> anyhow::Result<()> {
        let (sender, receiver) = channel();
        let frontend = Arc::new(Mutex::new(MockFrontend::new()));

        let test_runner = TestRunner::new(sender);

        let cloned_frontend = frontend.clone();
        std::thread::spawn(move || -> anyhow::Result<()> {
            let mut screen = Screen::new(
                cloned_frontend.clone(),
                CanonicalizedPath::try_from("./tests/mock_repos/rust1")?,
            )?;
            screen.run(
                Some("./tests/mock_repos/rust1/src/main.rs".try_into()?),
                receiver,
            )?;
            Ok(())
        });

        sleep(Duration::from_secs(5));

        test_runner.send_keys(keys!("enter s t d : : o p"))?;

        sleep(Duration::from_secs(2));

        insta::assert_snapshot!(frontend.lock().unwrap().content());

        Ok(())
    }
}
