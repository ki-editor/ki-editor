#[cfg(test)]
mod integration_test {
    use std::{
        path::PathBuf,
        sync::{
            mpsc::{channel, Sender},
            Arc, Mutex,
        },
        time::{Duration, UNIX_EPOCH},
    };

    use event::{event::Event, KeyEvent};
    use key_event_macro::keys;

    use crate::{
        canonicalized_path::CanonicalizedPath, frontend::mock::MockFrontend, screen::Screen,
    };

    struct TestRunner {
        key_event_sender: Sender<Event>,
        temp_dir: CanonicalizedPath,
        frontend: Arc<Mutex<MockFrontend>>,
    }

    impl Drop for TestRunner {
        fn drop(&mut self) {
            self.temp_dir.remove_dir_all().unwrap();
        }
    }

    impl TestRunner {
        fn new() -> Self {
            let (key_event_sender, receiver) = channel();
            let frontend = Arc::new(Mutex::new(MockFrontend::new()));

            const MOCK_REPO_PATH: &str = "tests/mock_repos/rust1";

            // Copy the mock repo to a temporary directory using the current date
            // Why don't we use the `tempfile` crate? Because LSP doesn't work inside a the system temporary directory
            let epoch_time = std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards");

            let random_number = rand::random::<u8>();
            let temp_dir = format!("../temp_dir/{}_{}", epoch_time.as_secs(), random_number);

            let path: PathBuf = temp_dir.into();
            std::fs::create_dir_all(path.clone()).unwrap();

            let options = fs_extra::dir::CopyOptions::new();
            fs_extra::dir::copy(MOCK_REPO_PATH, path.clone(), &options).unwrap();

            let temp_dir = CanonicalizedPath::try_from(path).unwrap();
            let path = temp_dir.join("rust1").unwrap();

            let cloned_frontend = frontend.clone();
            std::thread::spawn(move || -> anyhow::Result<()> {
                let mut screen = Screen::new(cloned_frontend, path.clone())?;
                screen.run(Some(path.join("src/main.rs")?), receiver)?;
                Ok(())
            });
            Self {
                key_event_sender,
                temp_dir,
                frontend,
            }
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

        fn content(&self) -> String {
            self.frontend.lock().unwrap().content()
        }
    }

    fn sleep(seconds: u64) {
        std::thread::sleep(Duration::from_secs(seconds));
    }

    #[test]
    fn lsp_completion() -> anyhow::Result<()> {
        let test_runner = TestRunner::new();
        sleep(3);
        test_runner.send_keys(keys!("enter s t d : : o p"))?;

        sleep(1);
        insta::assert_snapshot!(test_runner.content());

        Ok(())
    }

    #[test]
    fn saving_should_not_crash() -> anyhow::Result<()> {
        let test_runner = TestRunner::new();
        sleep(1);

        // Go to the last line
        test_runner.send_keys(keys!("l f"))?;

        // Insert blank spaces at the end
        test_runner.send_keys(keys!("i space space space"))?;

        // Save the file
        test_runner.send_keys(keys!("ctrl+s"))?;

        // Insert a b c
        test_runner.send_keys(keys!("i a b c"))?;

        sleep(1);

        // Expect 'a b c' to be inserted at the end
        // Because the cursor is clamped to the end of the file, as it was out of bound after the
        // file is formatted
        // This will only work if the previous saving didn't crash
        insta::assert_snapshot!(test_runner.content());

        Ok(())
    }
}
