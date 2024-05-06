use std::path::PathBuf;

use crossbeam::channel::Sender;
use globset::Glob;
use ignore::{WalkBuilder, WalkState};
use shared::canonicalized_path::CanonicalizedPath;

use crate::{buffer::Buffer, quickfix_list::Location, selection_mode::ByteRange};

pub mod ast_grep;

pub mod case_agnostic;
pub mod grep;

pub struct WalkBuilderConfig {
    pub root: PathBuf,
    pub include: Option<Glob>,
    pub exclude: Option<Glob>,
}

type SearchFn = dyn Fn(&Buffer) -> anyhow::Result<Vec<ByteRange>> + Send + Sync;
impl WalkBuilderConfig {
    pub fn run_with_search(
        self,
        enable_tree_sitter: bool,
        f: Box<SearchFn>,
    ) -> anyhow::Result<Vec<Location>> {
        self.run(Box::new(move |path, sender| {
            let path = path.try_into()?;
            let buffer = Buffer::from_path(&path, enable_tree_sitter)?;
            // Tree-sitter should be disabled whenever possible during
            // global search, because it will slow down the operation tremendously
            if !enable_tree_sitter {
                debug_assert!(buffer.tree().is_none())
            }
            let _ = f(&buffer)?
                .into_iter()
                .flat_map(move |node_match| -> anyhow::Result<_> {
                    let range = node_match.range();
                    let range = buffer.byte_to_position(range.start)?
                        ..buffer.byte_to_position(range.end)?;

                    let _ = sender
                        .send(Location {
                            path: path.clone(),
                            range,
                        })
                        .map_err(|error| {
                            log::error!("sender.send {:?}", error);
                        });

                    Ok(())
                })
                .collect::<Vec<_>>();
            Ok(())
        }))
    }
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
                        .map(|pattern| pattern.compile_matcher().is_match(&path))
                        .unwrap_or(true)
                        && exclude
                            .as_ref()
                            .map(|pattern| !pattern.compile_matcher().is_match(&path))
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
                            let path = path.path().into();
                            if let Err(error) = f(path, sender.clone()) {
                                log::error!("sender.send {:?}", error)
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
    use globset::Glob;
    use std::path::PathBuf;

    use itertools::Itertools;

    use super::WalkBuilderConfig;

    #[test]
    fn test_exclude() -> anyhow::Result<()> {
        let config = WalkBuilderConfig {
            root: "./tests/mock_repos/rust1".into(),
            include: None,
            exclude: Some(Glob::new("*.rs")?),
        };
        let paths = config.run(Box::new(|path, sender| {
            sender.send(path).unwrap();
            Ok(())
        }))?;
        assert_eq!(
            paths.into_iter().sorted().collect_vec(),
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
            include: Some(Glob::new("**/src/*.rs")?),
            exclude: None,
        };
        let paths = config.run(Box::new(|path, sender| {
            sender.send(path).unwrap();
            Ok(())
        }))?;
        assert_eq!(
            paths.into_iter().sorted().collect_vec(),
            [
                PathBuf::from("./tests/mock_repos/rust1/src/foo.rs"),
                PathBuf::from("./tests/mock_repos/rust1/src/main.rs")
            ]
        );
        Ok(())
    }
}

#[cfg(test)]
mod test_glob {
    use globset::Glob;
    #[test]
    fn alternatives() -> anyhow::Result<()> {
        let glob = Glob::new("{*.toml,foo/*}")?.compile_matcher();

        assert!(glob.is_match("foo/bar.js"));
        assert!(glob.is_match("foo/bar.rs"));
        assert!(glob.is_match("Cargo.toml"));
        assert!(!glob.is_match("Cargo.lock"));
        Ok(())
    }
}
