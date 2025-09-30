use std::sync::mpsc::Sender;
use std::time::Duration;

use crate::app::AppMessage;

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum DebounceMessage {
    NucleoTick,
}

pub(crate) fn start_thread(callback: Sender<AppMessage>) -> Sender<DebounceMessage> {
    let (sender, receiver) = std::sync::mpsc::channel::<DebounceMessage>();
    use debounce::EventDebouncer;

    std::thread::spawn(move || {
        let debounce = EventDebouncer::new(
            Duration::from_millis(1000 / 30), // 30 FPS
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
