#[cfg(test)]
mod test;

use notify::{event::ModifyKind, Event, EventKind, RecursiveMode, Result, Watcher};

use shared::canonicalized_path::CanonicalizedPath;
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

pub(crate) fn watch_file_changes(
    path: &CanonicalizedPath,
    app_message_sender: Sender<AppMessage>,
) -> anyhow::Result<()> {
    let (sender, receiver) = mpsc::channel::<Result<Event>>();

    let mut watcher = notify::recommended_watcher(sender)?;
    let debounced_handler = debounce(
        Callback::new(Arc::new(move |event: FileWatcherEvent| {
            let _ = app_message_sender.send(AppMessage::FileWatcherEvent(event));
        })),
        Duration::from_secs(1),
    );

    watcher.watch(path.to_path_buf(), RecursiveMode::Recursive)?;

    for result in receiver {
        match result {
            Ok(event) => handle_event(event, &debounced_handler),
            Err(error) => {
                log::error!("watch_file_changes error: {error:?}")
            }
        }
    }
    Ok(())
}

fn handle_event(event: notify::Event, callback: &Callback<FileWatcherEvent>) {
    for path in event.paths.clone() {
        match event.kind {
            EventKind::Modify(ModifyKind::Data(_)) => {
                if let Ok(path) = CanonicalizedPath::try_from(path) {
                    if path.is_file() {
                        callback.call(FileWatcherEvent::ContentModified(path))
                    }
                }
            }
            EventKind::Modify(ModifyKind::Name(_)) => {
                callback.call(FileWatcherEvent::PathRenamed(path))
            }
            EventKind::Create(_) => callback.call(FileWatcherEvent::PathCreated),
            EventKind::Remove(_) => callback.call(FileWatcherEvent::PathRemoved(path)),
            _ => (),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FileWatcherEvent {
    ContentModified(CanonicalizedPath),
    /// We don't attached the Path so that all PathCreated event can be grouped together
    /// by the debouncer, since we don't really care what files are added.
    PathCreated,
    PathRemoved(PathBuf),
    PathRenamed(PathBuf),
}
