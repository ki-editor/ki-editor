use grep_regex::RegexMatcher;
use grep_searcher::{sinks, SearcherBuilder};

use regex::Regex;

use crate::{
    buffer::Buffer, context::LocalSearchConfig, quickfix_list::Location,
    selection_mode::regex::get_regex,
};
use shared::canonicalized_path::CanonicalizedPath;
use std::path::PathBuf;

use super::WalkBuilderConfig;

#[derive(Debug)]
pub struct Match {
    pub path: PathBuf,
    pub line_number: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub struct RegexConfig {
    pub escaped: bool,
    pub case_sensitive: bool,
    pub match_whole_word: bool,
}
impl RegexConfig {
    pub(crate) fn to_regex(self, pattern: &str) -> Result<Regex, anyhow::Error> {
        get_regex(pattern, self)
    }
}

impl Default for RegexConfig {
    fn default() -> Self {
        Self {
            escaped: true,
            case_sensitive: false,
            match_whole_word: false,
        }
    }
}

/// Returns list of affected files
pub fn replace(
    walk_builder_config: WalkBuilderConfig,
    local_search_config: LocalSearchConfig,
) -> anyhow::Result<Vec<CanonicalizedPath>> {
    Ok(walk_builder_config
        .run(Box::new(move |path, sender| {
            let path = path.try_into()?;
            let mut buffer = Buffer::from_path(&path)?;
            let (modified, _) = buffer.replace(local_search_config.clone(), Default::default())?;
            if modified {
                buffer.save_without_formatting()?;
                sender
                    .send(path)
                    .map_err(|err| log::info!("Error = {:?}", err))
                    .unwrap_or_default();
            }
            Ok(())
        }))?
        .into_iter()
        .collect())
}

pub fn run(
    pattern: &str,
    walk_builder_config: WalkBuilderConfig,
    grep_config: RegexConfig,
) -> anyhow::Result<Vec<Location>> {
    let pattern = get_regex(pattern, grep_config)?.as_str().to_string();
    let matcher = RegexMatcher::new_line_matcher(&pattern)?;
    let regex = Regex::new(&pattern)?;

    Ok(walk_builder_config
        .run(Box::new(move |path, sender| {
            let path = path.try_into()?;
            let buffer = Buffer::from_path(&path)?;
            let mut searcher = SearcherBuilder::new().build();
            searcher.search_path(
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
            )?;
            Ok(())
        }))?
        .into_iter()
        .flatten()
        .collect())
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
