use std::sync::Arc;
use std::time::Duration;

pub(crate) fn start_thread(
    callback: Arc<dyn Fn() + Send + Sync>,
    duration: Duration,
) -> Arc<dyn Fn() + Send + Sync> {
    let (sender, receiver) = std::sync::mpsc::channel::<()>();
    use debounce::EventDebouncer;

    std::thread::spawn(move || {
        let debounce = EventDebouncer::new(duration, move |_| callback());

        while let Ok(()) = receiver.recv() {
            debounce.put(())
        }
    });

    Arc::new(move || {
        let _ = sender.send(());
    })
}
