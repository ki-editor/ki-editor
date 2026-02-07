use std::sync::Arc;

use grep_regex::RegexMatcher;
use grep_searcher::{sinks, SearcherBuilder};

use fancy_regex::Regex;

use crate::{
    app::Dispatches, buffer::Buffer, context::LocalSearchConfig, list::Match,
    quickfix_list::Location, selection_mode::regex::get_regex, thread::SendResult,
};
use shared::canonicalized_path::CanonicalizedPath;

use super::WalkBuilderConfig;

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub struct RegexConfig {
    pub escaped: bool,
    pub case_sensitive: bool,
    pub match_whole_word: bool,
}

impl RegexConfig {
    pub fn to_regex(self, pattern: &str) -> Result<Regex, anyhow::Error> {
        get_regex(pattern, self)
    }

    pub fn literal() -> RegexConfig {
        RegexConfig {
            case_sensitive: false,
            escaped: true,
            match_whole_word: false,
        }
    }

    pub fn strict() -> RegexConfig {
        RegexConfig {
            escaped: true,
            match_whole_word: true,
            case_sensitive: true,
        }
    }

    pub fn regex() -> RegexConfig {
        RegexConfig {
            escaped: false,
            match_whole_word: false,
            case_sensitive: false,
        }
    }

    pub fn match_whole_word() -> RegexConfig {
        RegexConfig {
            escaped: true,
            match_whole_word: true,
            case_sensitive: false,
        }
    }

    pub fn case_sensitive() -> RegexConfig {
        RegexConfig {
            escaped: true,
            match_whole_word: false,
            case_sensitive: true,
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
pub fn replace(
    walk_builder_config: WalkBuilderConfig,
    local_search_config: LocalSearchConfig,
) -> anyhow::Result<(Dispatches, Vec<CanonicalizedPath>)> {
    let (dispatches, paths): (Vec<_>, Vec<_>) = walk_builder_config
        .run(Box::new(move |path, sender| {
            let path = path.try_into()?;
            let mut buffer = Buffer::from_path(&path, local_search_config.require_tree_sitter())?;
            let (modified, _, _, _) =
                buffer.replace(local_search_config.clone(), Default::default(), 0)?;
            if modified {
                let (dispatches, _) = buffer.save_without_formatting(false)?;
                sender
                    .send((dispatches, path))
                    .map_err(|err| log::info!("Error = {err:?}"))
                    .unwrap_or_default();
            }
            Ok(())
        }))?
        .into_iter()
        .unzip();
    let dispatches = dispatches
        .into_iter()
        .reduce(Dispatches::chain)
        .unwrap_or_default();
    Ok((dispatches, paths))
}

pub fn run(
    pattern: &str,
    walk_builder_config: WalkBuilderConfig,
    grep_config: RegexConfig,
    send_match: Arc<dyn Fn(Match) -> SendResult + Send + Sync>,
) -> anyhow::Result<()> {
    let pattern = get_regex(pattern, grep_config)?.as_str().to_string();
    let matcher = RegexMatcher::new_line_matcher(&pattern)?;
    let regex = Regex::new(&pattern)?;

    walk_builder_config.run_async(
        false,
        Arc::new(move |path, buffer| {
            let mut searcher = SearcherBuilder::new().build();
            let _ = searcher.search_path(
                &matcher,
                path.clone(),
                sinks::UTF8(|line_number, line| {
                    if let Ok(locations) = to_locations(
                        &buffer,
                        path.clone(),
                        line_number as usize,
                        line,
                        regex.clone(),
                    ) {
                        for location in locations {
                            let m = Match {
                                location,
                                line: line.to_string(),
                            };
                            if send_match(m).is_receiver_disconnected() {
                                // Stop search
                                return Ok(false);
                            }
                        }
                    }
                    Ok(true)
                }),
            );
        }),
    )
}

fn to_locations(
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
            let start = buffer.byte_to_char(range.start + start_byte)?;
            let end = buffer.byte_to_char(range.end + start_byte)?;
            Ok(Location {
                range: (start..end).into(),
                path: path.clone(),
            })
        })
        .collect();

    Ok(locations)
}
