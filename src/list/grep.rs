use grep_regex::RegexMatcher;
use grep_searcher::{sinks, SearcherBuilder};
use ignore::{WalkBuilder, WalkState};

use std::path::{Path, PathBuf};

use crate::{buffer::Buffer, quickfix_list::Location};

#[derive(Debug)]
pub struct Match {
    pub path: PathBuf,
    pub line_number: u64,
}

pub fn run(pattern: &str, path: PathBuf) -> anyhow::Result<Vec<Location>> {
    let matcher = RegexMatcher::new_line_matcher(pattern)?;
    let searcher = SearcherBuilder::new().build();

    let (sender, receiver) = crossbeam::channel::unbounded();

    WalkBuilder::new(path).build_parallel().run(move || {
        let mut searcher = searcher.clone();
        let sender = sender.clone();
        let matcher = matcher.clone();
        Box::new(move |path| {
            if let Ok(path) = path {
                if path
                    .file_type()
                    .map_or(false, |file_type| file_type.is_file())
                {
                    let path = path.path();
                    let _ = searcher
                        .search_path(
                            &matcher,
                            path,
                            sinks::UTF8(|line_number, _| {
                                if let Ok(location) =
                                    line_path_to_location(path, line_number as usize)
                                {
                                    let _ = sender.send(location).map_err(|error| {
                                        log::error!("sender.send {:?}", error);
                                    });
                                }
                                Ok(true)
                            }),
                        )
                        .map_err(|error| {
                            log::error!("searcher.search_path {:?}", error);
                        });
                }
            }
            WalkState::Continue
        })
    });

    Ok(receiver.into_iter().collect::<Vec<_>>())
}

fn line_path_to_location(path: &Path, line_number: usize) -> anyhow::Result<Location> {
    let buffer = Buffer::from_path(&path.try_into()?)?;
    let location = Location {
        path: path.try_into()?,
        range: buffer.line_to_position_range(line_number.saturating_sub(1))?,
    };

    Ok(location)
}
