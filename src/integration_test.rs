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
    use my_proc_macros::keys;

    use crate::{
        canonicalized_path::CanonicalizedPath,
        frontend::mock::MockFrontend,
        screen::{Screen, ScreenMessage},
    };

    struct TestRunner {
        key_event_sender: Sender<ScreenMessage>,
        temp_dir: CanonicalizedPath,
        frontend: Arc<Mutex<MockFrontend>>,
    }

    impl Drop for TestRunner {
        fn drop(&mut self) {
            self.temp_dir.remove_dir_all().unwrap();
        }
    }

    impl TestRunner {
        fn new() -> anyhow::Result<Self> {
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
            std::fs::create_dir_all(path.clone())?;

            let options = fs_extra::dir::CopyOptions::new();
            fs_extra::dir::copy(MOCK_REPO_PATH, path.clone(), &options)?;

            let temp_dir = CanonicalizedPath::try_from(path)?;
            let path = temp_dir.join("rust1")?;

            // Initialize the repo as a Git repo, so that we can test Git related features
            Self::git_init(path.clone())?;

            let cloned_frontend = frontend.clone();
            let (sender, receiver) = channel();
            let key_event_sender = sender.clone();
            std::thread::spawn(move || -> anyhow::Result<()> {
                let screen = Screen::from_channel(cloned_frontend, path.clone(), sender, receiver)?;
                screen.run(Some(path.join("src/main.rs")?))?;
                Ok(())
            });
            Ok(Self {
                key_event_sender,
                temp_dir,
                frontend,
            })
        }

        fn dump_log_file(&self) -> anyhow::Result<()> {
            let log_file = self.temp_dir.join("my_log.txt")?;
            let log_file = std::fs::read_to_string(log_file)?;
            println!("{}", log_file);
            Ok(())
        }

        fn git_init(path: CanonicalizedPath) -> anyhow::Result<()> {
            use git2::{Repository, RepositoryInitOptions};

            let repo = Repository::init_opts(path, RepositoryInitOptions::new().mkdir(false))?;
            let mut index = repo.index()?;
            index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
            let tree_oid = index.write_tree()?;
            let tree = repo.find_tree(tree_oid)?;
            let sig = repo.signature()?;
            repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])?;
            Ok(())
        }

        fn send_key(&self, key: KeyEvent) -> anyhow::Result<()> {
            self.key_event_sender
                .send(ScreenMessage::Event(Event::Key(key)))
                .map_err(|error| anyhow::anyhow!("{:?}", error))
        }

        fn send_keys(&self, keys: &[KeyEvent]) -> anyhow::Result<()> {
            for key in keys.iter() {
                self.send_key(key.clone())?;
            }
            Ok(())
        }

        fn content(&self) -> String {
            sleep(1);
            self.frontend.lock().unwrap().content()
        }
    }

    fn sleep(seconds: u64) {
        std::thread::sleep(Duration::from_secs(seconds));
    }

    #[test]
    fn lsp_completion() -> anyhow::Result<()> {
        let test_runner = TestRunner::new()?;
        sleep(3);
        test_runner.send_keys(keys!("enter u s e space s t d : : o p t"))?;

        sleep(1);
        test_runner
            .dump_log_file()
            .unwrap_or_else(|error| println!("Failed to dump log file: {:?}", error));

        insta::assert_snapshot!(test_runner.content());

        Ok(())
    }

    #[test]
    fn saving_should_not_crash() -> anyhow::Result<()> {
        let test_runner = TestRunner::new()?;
        sleep(1);

        // Go to the last line
        test_runner.send_keys(keys!("l f"))?;

        // Insert blank spaces at the end
        test_runner.send_keys(keys!("i space space space"))?;

        // Save the file
        test_runner.send_keys(keys!("ctrl+s"))?;

        // Insert a b c
        test_runner.send_keys(keys!("i a b c"))?;

        // Expect 'a b c' to be inserted at the end
        // Because the cursor is clamped to the end of the file, as it was out of bound after the
        // file is formatted
        // This will only work if the previous saving didn't crash
        insta::assert_snapshot!(test_runner.content());

        Ok(())
    }

    #[test]
    fn search() -> anyhow::Result<()> {
        let test_runner = TestRunner::new()?;

        // Go to foo.rs
        test_runner.send_keys(keys!("g f f o o enter"))?;

        insta::assert_snapshot!(test_runner.content());

        // Go to the original file
        test_runner.send_keys(keys!("alt+left"))?;

        // Search for "main"
        test_runner.send_keys(keys!("ctrl+f l m a i n enter"))?;

        // Insert "_hello"
        test_runner.send_keys(keys!("i _ h e l l o"))?;

        // Expect the main function to be named "main_hello" in the original file
        insta::assert_snapshot!(test_runner.content());

        Ok(())
    }
}
