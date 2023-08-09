use grep_regex::RegexMatcher;
use grep_searcher::{sinks, BinaryDetection, Searcher, SearcherBuilder, Sink, SinkMatch};
use ignore::{WalkBuilder, WalkState};

use std::{
    error::Error,
    ops::Range,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct Match {
    pub path: PathBuf,
    pub line_number: u64,
}

pub fn run(pattern: &str, path: PathBuf) -> anyhow::Result<Vec<Match>> {
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
                            sinks::UTF8(|line_number, line| {
                                let _ = sender
                                    .send(Match {
                                        path: path.to_owned(),
                                        line_number,
                                    })
                                    .map_err(|error| {
                                        log::error!("sender.send {:?}", error);
                                    });
                                Ok(true)
                            }),
                            // MySink {
                            //     path: path.to_owned(),
                            //     sender: sender.clone(),
                            // },
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

struct MySink {
    path: PathBuf,
    sender: crossbeam::channel::Sender<Match>,
}

// impl Sink for MySink {
//     type Error = Box<dyn Error>;

//     fn matched(&mut self, _searcher: &Searcher, mat: &SinkMatch) -> Result<bool, Self::Error> {
//         mat.bytes_range_in_buffer();
//         self.sender.send(Match {
//             path: self.path.clone(),
//             byte_range: mat.bytes_range_in_buffer(),
//         })?;
//         Ok(true)
//     }
// }
