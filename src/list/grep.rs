use grep_regex::RegexMatcher;
use grep_searcher::{sinks, SearcherBuilder};
use ignore::{WalkBuilder, WalkState};
use regex::Regex;

use std::path::PathBuf;

use crate::{
    buffer::Buffer, canonicalized_path::CanonicalizedPath, quickfix_list::Location,
    selection_mode::regex::get_regex,
};

#[derive(Debug)]
pub struct Match {
    pub path: PathBuf,
    pub line_number: u64,
}

pub fn run(
    pattern: &str,
    path: PathBuf,
    escape: bool,
    ignore_case: bool,
) -> anyhow::Result<Vec<Location>> {
    let pattern = get_regex(pattern, escape, ignore_case)?
        .as_str()
        .to_string();
    let matcher = RegexMatcher::new_line_matcher(&pattern)?;
    let regex = Regex::new(&pattern)?;
    let searcher = SearcherBuilder::new().build();

    let (sender, receiver) = crossbeam::channel::unbounded();

    let start_time = std::time::Instant::now();
    WalkBuilder::new(path).build_parallel().run(move || {
        let mut searcher = searcher.clone();
        let sender = sender.clone();
        let matcher = matcher.clone();
        let regex = regex.clone();

        Box::new(move |path| {
            if let Ok(path) = path {
                if path
                    .file_type()
                    .map_or(false, |file_type| file_type.is_file())
                {
                    let path = path.path();
                    if let Ok(path) = path.try_into() {
                        if let Ok(buffer) = Buffer::from_path(&path) {
                            let _ = searcher
                                .search_path(
                                    &matcher,
                                    path.clone(),
                                    sinks::UTF8(|line_number, line| {
                                        if let Ok(location) = to_location(
                                            &buffer,
                                            path.clone(),
                                            line_number as usize,
                                            line,
                                            regex.clone(),
                                        ) {
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
                }
            }
            WalkState::Continue
        })
    });

    let time_taken = start_time.elapsed();
    log::info!("time_taken to search: {:?}", time_taken);
    Ok(receiver.into_iter().flatten().collect::<Vec<_>>())
}

fn to_location(
    buffer: &Buffer,
    path: CanonicalizedPath,
    line_number: usize,
    line: &str,
    regex: Regex,
) -> anyhow::Result<Vec<Location>> {
    let start_byte = buffer.line_to_byte(line_number.saturating_sub(1))?;
    let locations = regex
        .find_iter(line)
        .flat_map(|match_| -> anyhow::Result<Location> {
            let range = match_.range();
            let start = buffer.byte_to_position(range.start + start_byte)?;
            let end = buffer.byte_to_position(range.end + start_byte)?;
            Ok(Location {
                range: start..end,
                path: path.clone(),
            })
        })
        .collect();

    Ok(locations)
}
