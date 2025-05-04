use grep_regex::RegexMatcher;
use grep_searcher::{sinks, SearcherBuilder};

use fancy_regex::Regex;

use crate::{
    buffer::Buffer, context::LocalSearchConfig, quickfix_list::Location,
    selection_mode::regex::get_regex,
};
use shared::canonicalized_path::CanonicalizedPath;

use super::WalkBuilderConfig;

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub(crate) struct RegexConfig {
    pub(crate) escaped: bool,
    pub(crate) case_sensitive: bool,
    pub(crate) match_whole_word: bool,
}
impl RegexConfig {
    pub(crate) fn to_regex(self, pattern: &str) -> Result<Regex, anyhow::Error> {
        get_regex(pattern, self)
    }

    pub(crate) fn literal() -> RegexConfig {
        Self {
            case_sensitive: false,
            escaped: true,
            match_whole_word: false,
        }
    }

    pub(crate) fn strict() -> RegexConfig {
        RegexConfig {
            escaped: true,
            match_whole_word: true,
            case_sensitive: true,
        }
    }

    pub(crate) fn regex() -> RegexConfig {
        RegexConfig {
            escaped: false,
            match_whole_word: false,
            case_sensitive: false,
        }
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
pub(crate) fn replace(
    walk_builder_config: WalkBuilderConfig,
    local_search_config: LocalSearchConfig,
) -> anyhow::Result<Vec<CanonicalizedPath>> {
    Ok(walk_builder_config
        .run(Box::new(move |path, sender| {
            let path = path.try_into()?;
            let mut buffer = Buffer::from_path(&path, local_search_config.require_tree_sitter())?;
            let (modified, _) =
                buffer.replace(local_search_config.clone(), Default::default(), 0)?;
            if modified {
                buffer.save_without_formatting(false)?;
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

pub(crate) fn run(
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
            let buffer = Buffer::from_path(&path, false)?;
            // Tree-sitter should be disabled whenever possible during
            // global search, because it will slow down the operation tremendously
            debug_assert!(buffer.tree().is_none());
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
            let range = match_?.range();
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
