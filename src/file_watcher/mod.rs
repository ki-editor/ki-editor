#[cfg(test)]
mod test;

use notify::{event::ModifyKind, Event, EventKind, RecursiveMode, Result, Watcher};

use shared::canonicalized_path::CanonicalizedPath;
use std::sync::mpsc::{self, Sender};

use crate::app::{AppMessage, FileWatcherEvent};

pub(crate) fn watch_file_changes(
    path: &CanonicalizedPath,
    app_message_sender: Sender<AppMessage>,
) -> anyhow::Result<()> {
    let (sender, receiver) = mpsc::channel::<Result<Event>>();

    let mut watcher = notify::recommended_watcher(sender)?;

    watcher.watch(path.to_path_buf(), RecursiveMode::Recursive)?;

    for result in receiver {
        match result {
            Ok(event) => handle_event(event, &app_message_sender),
            Err(error) => {
                log::error!("watch_file_changes error: {error:?}")
            }
        }
    }
    Ok(())
}

fn handle_event(event: notify::Event, app_message_sender: &Sender<AppMessage>) {
    if let EventKind::Modify(ModifyKind::Data(_)) = event.kind {
        for path in event.paths {
            if let Ok(path) = CanonicalizedPath::try_from(path) {
                if path.is_file() {
                    let _ = app_message_sender.send(AppMessage::FileWatcherEvent(
                        FileWatcherEvent::ContentModified(path),
                    ));
                }
            }
        }
    }
}
