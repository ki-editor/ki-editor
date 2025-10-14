use std::sync::Arc;

use globset::Glob;
use grep_regex::RegexMatcher;
use grep_searcher::{sinks, SearcherBuilder};

use fancy_regex::Regex;

use crate::{
    buffer::Buffer, context::LocalSearchConfig, quickfix_list::Location,
    selection_mode::regex::get_regex, thread::SendResult,
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
        RegexConfig {
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

    pub(crate) fn match_whole_word() -> RegexConfig {
        RegexConfig {
            escaped: true,
            match_whole_word: true,
            case_sensitive: false,
        }
    }

    pub(crate) fn case_sensitive() -> RegexConfig {
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
pub(crate) fn replace(
    walk_builder_config: WalkBuilderConfig,
    local_search_config: LocalSearchConfig,
) -> anyhow::Result<Vec<CanonicalizedPath>> {
    Ok(walk_builder_config
        .run(Box::new(move |path, sender| {
            let path = path.try_into()?;
            let mut buffer = Buffer::from_path(&path, local_search_config.require_tree_sitter())?;
            let (modified, _, _, _) =
                buffer.replace(local_search_config.clone(), Default::default(), 0)?;
            if modified {
                buffer.save_without_formatting(false)?;
                sender
                    .send(path)
                    .map_err(|err| log::info!("Error = {err:?}"))
                    .unwrap_or_default();
            }
            Ok(())
        }))?
        .into_iter()
        .collect())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Match {
    pub(crate) location: Location,
    pub(crate) line: String,
}

pub(crate) fn run_async(
    pattern: &str,
    walk_builder_config: WalkBuilderConfig,
    grep_config: RegexConfig,
    send_matches: Arc<dyn Fn(Vec<Match>) -> SendResult + Send + Sync>,
) -> anyhow::Result<()> {
    let pattern = get_regex(pattern, grep_config)?.as_str().to_string();
    let matcher = RegexMatcher::new_line_matcher(&pattern)?;
    let regex = Regex::new(&pattern)?;

    let send = crate::thread::batch(send_matches, 30);
    let build_matcher = |glob: Option<&Glob>| -> anyhow::Result<_> {
        let pattern = if let Some(glob) = glob {
            Some(
                Glob::new(&walk_builder_config.root.join(glob.glob()).to_string_lossy())?
                    .compile_matcher(),
            )
        } else {
            None
        };
        Ok(Box::new(move |path: &str| {
            pattern.as_ref().map(|pattern| pattern.is_match(path))
        }))
    };
    let include_match = Arc::new(build_matcher(walk_builder_config.include.as_ref())?);
    let exclude_match = Arc::new(build_matcher(walk_builder_config.exclude.as_ref())?);

    std::thread::spawn(|| {
        walk_builder_config.run_async(
            include_match,
            exclude_match,
            Arc::new(move |path| {
                let Ok(path) = path.try_into() else { return };
                // Tree-sitter should be disabled whenever possible during
                // global search, because it will slow down the operation tremendously
                let Ok(buffer) = Buffer::from_path(&path, false) else {
                    return;
                };
                debug_assert!(buffer.tree().is_none());
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
                                    line: line.trim_end_matches(['\n', '\r']).to_string(),
                                };
                                if send(m).is_receiver_disconnected() {
                                    // Stop search
                                    return Ok(false);
                                }
                            }
                        }
                        Ok(true)
                    }),
                );
            }),
        );
    });

    Ok(())
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
