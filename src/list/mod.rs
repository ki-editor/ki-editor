use std::path::PathBuf;

use crossbeam::channel::Sender;
use globset::Glob;
use ignore::{WalkBuilder, WalkState};
use itertools::Itertools;
use shared::canonicalized_path::CanonicalizedPath;

use crate::{buffer::Buffer, quickfix_list::Location, selection_mode::ByteRange};

pub(crate) mod ast_grep;

pub(crate) mod grep;
pub(crate) mod naming_convention_agnostic;

pub(crate) struct WalkBuilderConfig {
    pub(crate) root: PathBuf,
    pub(crate) include: Option<Glob>,
    pub(crate) exclude: Option<Glob>,
}

type SearchFn = dyn Fn(&Buffer) -> anyhow::Result<Vec<ByteRange>> + Send + Sync;
impl WalkBuilderConfig {
    pub(crate) fn run_with_search(
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
                            log::error!("sender.send {error:?}");
                        });

                    Ok(())
                })
                .collect::<Vec<_>>();
            Ok(())
        }))
    }
    pub(crate) fn run<T: Send>(
        self,
        f: Box<dyn Fn(PathBuf, Sender<T>) -> anyhow::Result<()> + Send + Sync>,
    ) -> anyhow::Result<Vec<T>> {
        let WalkBuilderConfig {
            root,
            include,
            exclude,
        } = self;
        let (sender, receiver) = crossbeam::channel::unbounded::<T>();
        let build_matcher = |glob: Option<&Glob>| -> anyhow::Result<_> {
            let pattern = if let Some(glob) = glob {
                Some(Glob::new(&root.join(glob.glob()).to_string_lossy())?.compile_matcher())
            } else {
                None
            };
            Ok(Box::new(move |path: &str| {
                pattern.as_ref().map(|pattern| pattern.is_match(path))
            }))
        };
        let include_match = build_matcher(include.as_ref())?;
        let exclude_match = build_matcher(exclude.as_ref())?;
        WalkBuilder::new(root)
            .filter_entry(move |entry| {
                let path = entry.path().display().to_string();

                entry
                    .file_type()
                    .map(|file_type| !file_type.is_file())
                    .unwrap_or(false)
                    || (include_match(&path).unwrap_or(true)
                        && !exclude_match(&path).unwrap_or(false))
            })
            .hidden(false)
            .build_parallel()
            .run(|| {
                Box::new(|path| {
                    if let Ok(path) = path {
                        if path
                            .file_type()
                            .is_some_and(|file_type| file_type.is_file())
                        {
                            let path = path.path().into();
                            if let Err(error) = f(path, sender.clone()) {
                                log::error!("sender.send {error:?}")
                            }
                        } else if path.path().ends_with(".git") {
                            return WalkState::Skip;
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

    pub(crate) fn stream(self, sender: std::sync::mpsc::Sender<PathBuf>) -> anyhow::Result<()> {
        let WalkBuilderConfig {
            root,
            include,
            exclude,
        } = self;
        let build_matcher = |glob: Option<&Glob>| -> anyhow::Result<_> {
            let pattern = if let Some(glob) = glob {
                Some(Glob::new(&root.join(glob.glob()).to_string_lossy())?.compile_matcher())
            } else {
                None
            };
            Ok(Box::new(move |path: &str| {
                pattern.as_ref().map(|pattern| pattern.is_match(path))
            }))
        };
        let include_match = build_matcher(include.as_ref())?;
        let exclude_match = build_matcher(exclude.as_ref())?;
        WalkBuilder::new(root)
            .filter_entry(move |entry| {
                let path = entry.path().display().to_string();

                entry
                    .file_type()
                    .map(|file_type| !file_type.is_file())
                    .unwrap_or(false)
                    || (include_match(&path).unwrap_or(true)
                        && !exclude_match(&path).unwrap_or(false))
            })
            .hidden(false)
            .build_parallel()
            .run(|| {
                Box::new(|path| {
                    if let Ok(path) = path {
                        if path
                            .file_type()
                            .is_some_and(|file_type| file_type.is_file())
                        {
                            let path = path.path().into();
                            match sender.send(path) {
                                Ok(_) => {}
                                Err(err) => {
                                    log::error!(
                                        "WalkBuilderConfig: Failed to stream because of: {err:?}"
                                    );
                                    return WalkState::Quit;
                                }
                            }
                        } else if path.path().ends_with(".git") {
                            return WalkState::Skip;
                        }
                    }
                    WalkState::Continue
                })
            });
        Ok(())
    }

    pub(crate) fn new(root: PathBuf) -> Self {
        Self {
            root,
            include: None,
            exclude: None,
        }
    }

    /// This method returns `PathBuf` instead of `CanonicalizedPath`
    /// because constructing `CanonicalizedPath` is expensive.
    /// For reference: read https://blobfolio.com/2021/faster-path-canonicalization-rust/
    pub(crate) fn non_git_ignored_files(root: CanonicalizedPath) -> anyhow::Result<Vec<PathBuf>> {
        WalkBuilderConfig::new(root.to_path_buf().clone())
            .run(Box::new(|path, sender| Ok(sender.send(path)?)))
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
            exclude: Some(Glob::new("src/*.rs")?),
        };
        let paths = config.run(Box::new(|path, sender| {
            sender.send(path).unwrap();
            Ok(())
        }))?;
        assert_eq!(
            paths.into_iter().sorted().collect_vec(),
            [
                PathBuf::from("./tests/mock_repos/rust1/.gitignore"),
                PathBuf::from("./tests/mock_repos/rust1/Cargo.lock"),
                PathBuf::from("./tests/mock_repos/rust1/Cargo.toml"),
                PathBuf::from("./tests/mock_repos/rust1/src/hello.ts")
            ]
        );
        Ok(())
    }

    #[test]
    fn test_include() -> anyhow::Result<()> {
        let config = WalkBuilderConfig {
            root: "./tests/mock_repos/rust1".into(),
            include: Some(Glob::new("src/*.rs")?),
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
