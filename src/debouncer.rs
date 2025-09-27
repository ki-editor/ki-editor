use std::sync::mpsc::Sender;
use std::time::Duration;
use std::{path::PathBuf, sync::mpsc};

use itertools::Itertools;
use shared::canonicalized_path::CanonicalizedPath;

use crate::{app::AppMessage, git, list};

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum DebounceMessage {
    NucleoTick,
}

pub(crate) fn start_thread(callback: Sender<AppMessage>) -> Sender<DebounceMessage> {
    let (sender, receiver) = std::sync::mpsc::channel::<DebounceMessage>();
    use debounce::EventDebouncer;
    use std::cell::RefCell;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    std::thread::spawn(move || {
        let debounce = EventDebouncer::new(
            Duration::from_millis(150),
            move |request: DebounceMessage| {
                let app_message = match request {
                    DebounceMessage::NucleoTick => AppMessage::NucleoTickDebounced,
                };
                let _ = callback.send(app_message);
            },
        );

        while let Ok(request) = receiver.recv() {
            debounce.put(request)
        }
    });

    sender
}
