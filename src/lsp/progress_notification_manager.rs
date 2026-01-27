use std::{
    collections::HashMap,
    sync::{mpsc, Arc},
    time::Duration,
};

use itertools::Itertools;
use lsp_types::WorkDoneProgress;

use crate::thread::Callback;

/// This should start a thread loop which aggregates
/// progress notifications from different tokens (tasks)
/// into a single line of concise and renderable text.
pub struct ProgressNotificationManager {
    sender: mpsc::Sender<ProgressNotificationManagerDispatch>,
}

#[derive(Clone)]
struct Progress {
    title: String,
    percentage: Option<u32>,
    message: Option<String>,
}

pub enum ProgressNotificationManagerDispatch {
    WorkDoneProgress {
        token: String,
        progress: WorkDoneProgress,
    },
    Tick {
        count: usize,
    },
}

#[derive(Default)]
struct ProgressNotificationState {
    progresses: HashMap<String, Progress>,
}

impl ProgressNotificationManager {
    pub fn new(lsp_command: String, send_progress: Callback<String>) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel::<ProgressNotificationManagerDispatch>();

        let interval = crate::thread::set_interval(
            Callback::new({
                let sender = sender.clone();
                Arc::new(move |count| {
                    let _ = sender.send(ProgressNotificationManagerDispatch::Tick { count });
                })
            }),
            Duration::from_millis(160),
        );

        std::thread::spawn(move || {
            let mut state = ProgressNotificationState::default();
            let chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            while let Ok(value) = receiver.recv() {
                match value {
                    ProgressNotificationManagerDispatch::WorkDoneProgress { token, progress } => {
                        match progress {
                            WorkDoneProgress::Begin(work_done_progress_begin) => {
                                let title = work_done_progress_begin.title.clone();
                                state.progresses.insert(
                                    token.clone(),
                                    Progress {
                                        title,
                                        percentage: work_done_progress_begin.percentage,
                                        message: work_done_progress_begin.message,
                                    },
                                );
                                interval.resume();
                            }
                            WorkDoneProgress::End(_) => {
                                state.progresses.remove(&token);
                            }
                            WorkDoneProgress::Report(work_done_progress_report) => {
                                if let Some(progress) = state.progresses.get_mut(&token) {
                                    progress.percentage = work_done_progress_report.percentage;
                                    progress.message = work_done_progress_report.message
                                }

                                if state.progresses.is_empty() {
                                    // Clear LSP progress
                                    send_progress.call("".to_string());
                                }
                            }
                        };
                    }
                    ProgressNotificationManagerDispatch::Tick { count } => {
                        if !state.progresses.is_empty() {
                            let message = state
                                .progresses
                                .values()
                                .map(|progress| {
                                    format!(
                                        "{}: {} {}%",
                                        progress.title,
                                        progress
                                            .message
                                            .as_ref()
                                            .map(|m| m.to_string())
                                            .unwrap_or_default(),
                                        progress
                                            .percentage
                                            .map(|p| p.to_string())
                                            .unwrap_or("?".to_string())
                                    )
                                })
                                .collect_vec()
                                .join("/");
                            let char = chars[count % chars.len()];
                            let message = format!("{char} {lsp_command}: {message}");
                            send_progress.call(message)
                        } else {
                            interval.pause();
                            send_progress.call(String::new())
                        }
                    }
                }
            }
            interval.cancel()
        });
        Self { sender }
    }

    pub(crate) fn update_progress(&self, token: String, progress: WorkDoneProgress) {
        let _ = self
            .sender
            .send(ProgressNotificationManagerDispatch::WorkDoneProgress { token, progress });
    }
}
