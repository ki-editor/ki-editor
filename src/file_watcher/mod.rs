#[cfg(test)]
mod test;

use notify::{Event, RecursiveMode, Result, Watcher};

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
            Ok(event) => {
                log::info!("watch_file_changes event: {event:?}");
                if let notify::EventKind::Modify(modify_kind) = event.kind { if let notify::event::ModifyKind::Data(_) = modify_kind {
                    for path in event.paths {
                        let _ = app_message_sender.send(AppMessage::FileWatcherEvent(
                            FileWatcherEvent::ContentModified(path.try_into()?),
                        ));
                    }
                } }
            }
            Err(error) => {
                log::error!("watch_file_changes error: {error:?}")
            }
        }
    }
    Ok(())
}
