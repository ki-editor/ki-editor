use std::sync::Arc;
use std::time::{Duration, Instant};

pub(crate) fn debounce(
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

pub(crate) fn batch<T: Send + Sync + 'static>(
    callback: Arc<dyn Fn(Vec<T>) + Send + Sync>,
    fps: u32,
) -> Arc<dyn Fn(T) + Send + Sync> {
    let (sender, receiver) = std::sync::mpsc::channel::<T>();

    std::thread::spawn(move || {
        let mut batch = vec![];
        let mut last_sent = Instant::now();
        while let Ok(item) = receiver.recv() {
            batch.push(item);
            if Instant::now() - last_sent > (Duration::from_secs(1) / fps) {
                callback(std::mem::take(&mut batch));
                last_sent = Instant::now();
            }
        }
        callback(std::mem::take(&mut batch))
    });

    Arc::new(move |item| {
        let _ = sender.send(item);
    })
}
