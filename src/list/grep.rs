use std::{sync::Arc, time::Instant};

use grep_regex::RegexMatcher;
use grep_searcher::{sinks, SearcherBuilder};

use fancy_regex::Regex;
use itertools::Itertools;

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

    // Create a thread to buffer non-first matches
    // This is ensure the first entry sent to the main UI loop
    // is also the first entry that in the final sorted list
    enum Message {
        MatchReceived { path_index: usize, match_: Match },
        FileFinishedSearching { index: usize },
    };
    let (sender, receiver) = std::sync::mpsc::channel::<Message>();

    let started_at = Instant::now();
    std::thread::spawn(move || {
        // Store the indices of files finished searching
        let mut indices = vec![];
        let mut buffered_matches = vec![];
        let mut first_entry_sent = false;
        let mut first_entry_sent_at = None;

        while let Ok(message) = receiver.recv() {
            match message {
                Message::MatchReceived { path_index, match_ } => {
                    if first_entry_sent {
                        // If the first entry is already sent, we no longer need
                        // to worry about subsequent items being sent in random order

                        match send_match(match_) {
                            SendResult::Succeeed => continue,
                            SendResult::ReceiverDisconnected => {
                                // Break the loop is the receiver of matches is killed
                                return;
                            }
                        }
                    } else {
                        buffered_matches.push((path_index, match_))
                    }
                }
                Message::FileFinishedSearching { index } => {
                    if first_entry_sent {
                        // Ignore the index if the first entry is already sent
                    } else {
                        // If all previous files have finished searching
                        // send the first entry over.

                        // For example: if `index` is 3 and `indices` is [0, 1, 2]
                        // then we can send the results over.
                        if indices.iter().take_while(|i| *i < &index).count() == index {
                            // Send the buffered entries over in an ordered manner
                            for (_, match_) in buffered_matches.drain(..).into_iter().sorted_by_key(
                                |(path_index, m)| (*path_index, m.location.range.start),
                            ) {
                                match send_match(match_) {
                                    SendResult::Succeeed => continue,
                                    SendResult::ReceiverDisconnected => {
                                        // Break the loop is the receiver of matches is killed
                                        return;
                                    }
                                }
                            }

                            first_entry_sent = true;
                            first_entry_sent_at = Some(Instant::now());
                        } else {
                            indices.push(index)
                        }
                    }
                }
            }
        }

        let finished_at = Instant::now();

        if let Some(first_entry_sent_at) = first_entry_sent_at {
            println!(
                "First entry painted after {:?}, ahead of finished time by {:?}",
                first_entry_sent_at - started_at,
                finished_at - first_entry_sent_at
            );
        }
    });

    walk_builder_config.run_async(
        false,
        Arc::new(move |path_index, path, buffer| {
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
                            let match_ = Match {
                                location,
                                line: line.to_string(),
                            };

                            if sender
                                .send(Message::MatchReceived { match_, path_index })
                                .is_err()
                            {
                                // Stop search if receiving thread is killed
                                return Ok(false);
                            }
                        }
                    }
                    Ok(true)
                }),
            );
            let _ = sender.send(Message::FileFinishedSearching { index: path_index });
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
