use std::path::PathBuf;

use crossbeam::channel::Sender;
use ignore::{WalkBuilder, WalkParallel, WalkState};
use shared::canonicalized_path::CanonicalizedPath;

pub mod ast_grep;

pub mod grep;

pub struct WalkBuilderConfig {
    pub root: PathBuf,
    pub include: Option<glob::Pattern>,
    pub exclude: Option<glob::Pattern>,
}

impl WalkBuilderConfig {
    pub fn run<T: Send>(
        self,
        f: Box<dyn Fn(PathBuf, Sender<T>) -> anyhow::Result<()> + Send + Sync>,
    ) -> anyhow::Result<Vec<T>> {
        let WalkBuilderConfig {
            root,
            include,
            exclude,
        } = self;
        let (sender, receiver) = crossbeam::channel::unbounded::<T>();
        WalkBuilder::new(root)
            .filter_entry(move |entry| {
                let path = CanonicalizedPath::try_from(entry.path())
                    .map(|path| path.display_absolute())
                    .unwrap_or_else(|_| entry.path().display().to_string());

                entry
                    .file_type()
                    .map(|file_type| !file_type.is_file())
                    .unwrap_or(false)
                    || (include
                        .as_ref()
                        .map(|pattern| pattern.matches(&path))
                        .unwrap_or(true)
                        && exclude
                            .as_ref()
                            .map(|pattern| !pattern.matches(&path))
                            .unwrap_or(true))
            })
            .build_parallel()
            .run(|| {
                Box::new(|path| {
                    if let Ok(path) = path {
                        if path
                            .file_type()
                            .map_or(false, |file_type| file_type.is_file())
                        {
                            let path = path.path();
                            if let Ok(path) = path.try_into() {
                                if let Err(error) = f(path, sender.clone()) {
                                    log::error!("sender.send {:?}", error)
                                }
                            }
                        }
                    }
                    WalkState::Continue
                })
            });
        {
            // This line is necessary to prevent deadlock
            // See https://stackoverflow.com/a/71413508/6587634
            drop(sender);
        }

        Ok(receiver.into_iter().collect::<Vec<_>>())
    }
}

#[cfg(test)]
mod test_walk_builder_config {
    use std::path::PathBuf;

    use super::WalkBuilderConfig;

    #[test]
    fn test_exclude() -> anyhow::Result<()> {
        let config = WalkBuilderConfig {
            root: "./tests/mock_repos/rust1".into(),
            include: None,
            exclude: Some(glob::Pattern::new("*.rs")?),
        };
        let paths = config.run(Box::new(|path, sender| {
            sender.send(path).unwrap();
            Ok(())
        }))?;
        assert_eq!(
            paths,
            [
                PathBuf::from("./tests/mock_repos/rust1/Cargo.lock"),
                PathBuf::from("./tests/mock_repos/rust1/Cargo.toml")
            ]
        );
        Ok(())
    }

    #[test]
    fn test_include() -> anyhow::Result<()> {
        let config = WalkBuilderConfig {
            root: "./tests/mock_repos/rust1".into(),
            include: Some(glob::Pattern::new("**/src/*.rs")?),
            exclude: None,
        };
        let paths = config.run(Box::new(|path, sender| {
            sender.send(path).unwrap();
            Ok(())
        }))?;
        assert_eq!(
            paths,
            [
                PathBuf::from("./tests/mock_repos/rust1/src/main.rs"),
                PathBuf::from("./tests/mock_repos/rust1/src/foo.rs")
            ]
        );
        Ok(())
    }
}
