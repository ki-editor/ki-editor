use std::sync::mpsc::Sender;
use std::{path::PathBuf, sync::mpsc};

use itertools::Itertools;
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
    GetGitStatusFiles {
        diff_mode: git::DiffMode,
        working_directory: CanonicalizedPath,
    },
}
impl ListFileKind {
    pub(crate) fn display(&self) -> String {
        match self {
            ListFileKind::NonGitIgnoredFiles { .. } => "Not Git Ignored".to_string(),
            ListFileKind::GetGitStatusFiles { diff_mode, .. } => {
                format!("Git Status ({})", diff_mode.display())
            }
        }
    }
}

#[derive(Debug)]
pub(crate) enum BackgroundTaskResult {
    Error {
        task: BackgroundTask,
        error: anyhow::Error,
    },
    Ok(BackgroundTaskBody),
}

#[derive(Debug)]
pub(crate) enum BackgroundTaskBody {
    ListFiles {
        paths: Vec<std::path::PathBuf>,
        kind: ListFileKind,
    },
}

pub(crate) fn start_thread_old(callback: Sender<AppMessage>) -> Sender<BackgroundTask> {
    let (sender, receiver) = mpsc::channel::<BackgroundTask>();
    std::thread::spawn(move || {
        while let Ok(task) = receiver.recv() {
            let result = handle_background_task(&task);
            match result {
                Ok(body) => {
                    let _ = callback
                        .send(AppMessage::BackgroundTaskResult(BackgroundTaskResult::Ok(
                            body,
                        )))
                        .map(|err| log::error!("background_worker::start_thread::Ok: {err:?}"));
                }
                Err(error) => {
                    let _ = callback
                        .send(AppMessage::BackgroundTaskResult(
                            BackgroundTaskResult::Error { task, error },
                        ))
                        .map(|err| log::error!("background_worker::start_thread::Err: {err:?}"));
                }
            };
        }
    });

    sender
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
                                let mut paths: Vec<PathBuf> = Default::default();
                                while let Ok(path) = path_receiver.recv() {
                                    if paths.len() < 100 {
                                        paths.push(path)
                                    } else {
                                        let paths = std::mem::take(&mut paths);
                                        if app_message_sender
                                            .send(AppMessage::ListFileEntries(paths))
                                            .is_err()
                                        {
                                            break; // Callback receiver dropped
                                        }
                                    }
                                }
                                // Send any remaining paths
                                if !paths.is_empty() {
                                    let _ =
                                        app_message_sender.send(AppMessage::ListFileEntries(paths));
                                }
                            });
                            list::WalkBuilderConfig::new(working_directory.to_path_buf().clone())
                                .stream(path_sender.clone());
                        }
                        ListFileKind::GetGitStatusFiles {
                            diff_mode,
                            working_directory,
                        } => {}
                    };
                }
            }
        }
    });

    sender
}

fn handle_background_task(task: &BackgroundTask) -> anyhow::Result<BackgroundTaskBody> {
    match task {
        BackgroundTask::ListFile(kind) => {
            let paths = match kind {
                ListFileKind::NonGitIgnoredFiles { working_directory } => {
                    list::WalkBuilderConfig::non_git_ignored_files(working_directory.clone())?
                }
                ListFileKind::GetGitStatusFiles {
                    diff_mode,
                    working_directory,
                } => git::GitRepo::try_from(working_directory)?
                    .diff_entries(*diff_mode)?
                    .into_iter()
                    .map(|entry| entry.new_path().into_path_buf())
                    .collect_vec(),
            };
            Ok(BackgroundTaskBody::ListFiles {
                paths,
                kind: kind.clone(),
            })
        }
    }
}
