#[cfg(test)]
mod test;

use notify::{event::ModifyKind, Event, EventKind, RecursiveMode, Result, Watcher};

use shared::absolute_path::AbsolutePath;
use std::{
    path::PathBuf,
    sync::{
        mpsc::{self, Sender},
        Arc,
    },
    time::Duration,
};

use crate::{
    app::AppMessage,
    thread::{debounce, Callback},
};

#[derive(Debug)]
pub enum FileWatcherInput {
    SyncOpenedPaths(Vec<AbsolutePath>),
    FileWatcherEvent(FileWatcherEvent),
    SyncFileExplorerExpandedFolders(Vec<AbsolutePath>),
}

#[derive(Default)]
pub struct FileWatcherState {
    opened_paths: Vec<AbsolutePath>,
    expanded_folders: Vec<AbsolutePath>,
}
impl FileWatcherState {
    fn contains_path_buf(&self, path_buf: &PathBuf) -> bool {
        self.opened_paths
            .iter()
            .any(|opened_path| opened_path.to_path_buf() == path_buf)
            || self
                .expanded_folders
                .iter()
                .any(|folder| path_buf.parent() == Some(folder.to_path_buf()))
    }

    fn should_send(&self, file_watcher_event: &FileWatcherEvent) -> bool {
        match file_watcher_event {
            FileWatcherEvent::ContentModified(path)
                if self.contains_path_buf(path.to_path_buf()) =>
            {
                true
            }
            FileWatcherEvent::PathCreated => true,
            FileWatcherEvent::PathRemoved(path)
            | FileWatcherEvent::PathRenamed { source: path, .. }
                if self.contains_path_buf(path) =>
            {
                true
            }
            _ => false,
        }
    }
}

pub fn watch_file_changes(
    path: &AbsolutePath,
    app_message_sender: Sender<AppMessage>,
) -> anyhow::Result<Sender<FileWatcherInput>> {
    let (file_watcher_input_sender, file_watcher_input_receiver) =
        mpsc::channel::<FileWatcherInput>();

    std::thread::spawn({
        let path = path.clone();
        let file_watcher_input_sender = file_watcher_input_sender.clone();
        move || {
            let (notify_sender, notify_receiver) = mpsc::channel::<Result<Event>>();
            let mut watcher = match notify::recommended_watcher(notify_sender) {
                Ok(watcher) => watcher,
                Err(error) => {
                    log::error!("[notify::recommended_watcher] error: {error:?}");
                    return;
                }
            };
            if let Err(error) = watcher.watch(path.to_path_buf(), RecursiveMode::Recursive) {
                log::error!("[watcher::watch] error: {error:?}");
                return;
            }

            let debounced_handler = debounce(
                Callback::new({
                    let file_watcher_input_sender = file_watcher_input_sender.clone();
                    Arc::new({
                        move |event: FileWatcherEvent| {
                            let _ = file_watcher_input_sender
                                .send(FileWatcherInput::FileWatcherEvent(event));
                        }
                    })
                }),
                Duration::from_secs(1),
            );

            std::thread::spawn(move || {
                let mut event_handler = EventHandler::new();
                for result in notify_receiver {
                    match result {
                        Ok(event) => event_handler.handle_event(event, &debounced_handler),
                        Err(error) => {
                            log::error!("watch_file_changes error: {error:?}");
                        }
                    }
                }
            });

            let mut state = FileWatcherState::default();
            for event in file_watcher_input_receiver {
                match event {
                    FileWatcherInput::SyncOpenedPaths(paths) => state.opened_paths = paths,
                    FileWatcherInput::SyncFileExplorerExpandedFolders(paths) => {
                        state.expanded_folders = paths;
                    }
                    FileWatcherInput::FileWatcherEvent(file_watcher_event) => {
                        // Only send events for path that are opened
                        if state.should_send(&file_watcher_event) {
                            let _ = app_message_sender
                                .send(AppMessage::FileWatcherEvent(file_watcher_event));
                        }
                    }
                }
            }
        }
    });

    Ok(file_watcher_input_sender)
}

/// This struct is created so that we can store the partial state of the path renamed event.
struct EventHandler {
    /// This is needed because a file rename will result in two events,
    /// the first being the Source name, and the second being the Destination name
    rename_source: Option<PathBuf>,
}

impl EventHandler {
    fn handle_event(&mut self, event: notify::Event, callback: &Callback<FileWatcherEvent>) {
        let Some(path) = event.paths.first() else {
            return;
        };
        let path = path.to_path_buf();
        match event.kind {
            EventKind::Modify(ModifyKind::Data(_)) => {
                if let Ok(path) = AbsolutePath::try_from(path) {
                    if path.is_file() {
                        callback.call(FileWatcherEvent::ContentModified(path));
                    }
                }
            }
            EventKind::Modify(ModifyKind::Name(_)) => {
                if let Some(source) = self.rename_source.take() {
                    if let Ok(destination) = path.try_into() {
                        callback.call(FileWatcherEvent::PathRenamed {
                            source,
                            destination,
                        });
                    }
                } else {
                    self.rename_source = Some(path);
                }
            }
            EventKind::Create(_) => callback.call(FileWatcherEvent::PathCreated),
            EventKind::Remove(_) => callback.call(FileWatcherEvent::PathRemoved(path)),
            _ => (),
        }
    }

    fn new() -> Self {
        Self {
            rename_source: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileWatcherEvent {
    ContentModified(AbsolutePath),
    /// We don't attached the Path so that all PathCreated event can be grouped together
    /// by the debouncer, since we don't really care what files are added.
    PathCreated,
    PathRemoved(PathBuf),
    PathRenamed {
        source: PathBuf,
        destination: AbsolutePath,
    },
}
