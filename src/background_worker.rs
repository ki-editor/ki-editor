use std::sync::mpsc;
use std::sync::mpsc::Sender;

use shared::canonicalized_path::CanonicalizedPath;

use crate::{app::AppMessage, git, list};

#[derive(Debug, Clone)]
pub(crate) enum BackgroundTask {
    ListFile(ListFileKind),
}

#[derive(Debug, Clone)]
pub(crate) enum ListFileKind {
    NonGitIgnoredFiles {
        working_directory: CanonicalizedPath,
    },
    GitStatusFiles {
        diff_mode: git::DiffMode,
        working_directory: CanonicalizedPath,
    },
}
impl ListFileKind {
    pub(crate) fn display(&self) -> String {
        match self {
            ListFileKind::NonGitIgnoredFiles { .. } => "Not Git Ignored".to_string(),
            ListFileKind::GitStatusFiles { diff_mode, .. } => {
                format!("Git Status ({})", diff_mode.display())
            }
        }
    }
}

pub(crate) fn start_thread(app_message_sender: Sender<AppMessage>) -> Sender<BackgroundTask> {
    let (sender, receiver) = mpsc::channel::<BackgroundTask>();

    std::thread::spawn(move || {
        while let Ok(task) = receiver.recv() {
            match task {
                BackgroundTask::ListFile(list_file_kind) => {
                    match list_file_kind {
                        ListFileKind::NonGitIgnoredFiles { working_directory } => {
                            let (path_sender, path_receiver) = std::sync::mpsc::channel();
                            let app_message_sender = app_message_sender.clone();
                            std::thread::spawn(move || {
                                while let Ok(path) = path_receiver.recv() {
                                    if app_message_sender
                                        .send(AppMessage::ListFileEntry(path))
                                        .is_err()
                                    {
                                        break; // Callback receiver dropped
                                    }
                                }
                            });
                            list::WalkBuilderConfig::new(working_directory.to_path_buf().clone())
                                .stream(path_sender.clone());
                        }
                        ListFileKind::GitStatusFiles {
                            diff_mode,
                            working_directory,
                        } => (),
                    };
                }
            }
        }
    });

    sender
}
