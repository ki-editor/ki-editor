use std::sync::mpsc::SendError;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub(crate) struct Callback<T>(Arc<dyn Fn(T) + Send + Sync>);

impl<T> Callback<T> {
    pub(crate) fn new(callback: Arc<dyn Fn(T) + Send + Sync>) -> Self {
        Self(callback)
    }

    pub(crate) fn call(&self, event: T) {
        self.0(event)
    }
}

pub(crate) fn debounce<T: PartialEq + Eq + Send + Sync + 'static>(
    callback: Callback<T>,
    duration: Duration,
) -> Callback<T> {
    let (sender, receiver) = std::sync::mpsc::channel::<T>();
    use debounce::EventDebouncer;

    std::thread::spawn(move || {
        let debounce = EventDebouncer::new(duration, move |x| callback.call(x));

        while let Ok(event) = receiver.recv() {
            debounce.put(event)
        }
    });

    Callback::new(Arc::new(move |event| {
        let _ = sender.send(event);
    }))
}

pub(crate) enum SendResult {
    Succeeed,
    ReceiverDisconnected,
}
impl SendResult {
    pub(crate) fn is_receiver_disconnected(&self) -> bool {
        matches!(self, SendResult::ReceiverDisconnected)
    }
}

impl<T> From<Result<(), SendError<T>>> for SendResult {
    fn from(value: Result<(), SendError<T>>) -> Self {
        match value {
            Ok(_) => SendResult::Succeeed,
            Err(_) => SendResult::ReceiverDisconnected,
        }
    }
}

pub(crate) fn batch<T: Send + Sync + 'static>(
    callback: Arc<dyn Fn(Vec<T>) -> SendResult + Send + Sync>,
    interval: Duration,
) -> Arc<dyn Fn(T) -> SendResult + Send + Sync> {
    let (sender, receiver) = std::sync::mpsc::channel::<T>();

    std::thread::spawn(move || {
        let mut batch = vec![];
        let mut last_sent = Instant::now();
        while let Ok(item) = receiver.recv() {
            batch.push(item);
            if Instant::now() - last_sent > interval {
                callback(std::mem::take(&mut batch));
                last_sent = Instant::now();
            }
        }
        callback(std::mem::take(&mut batch))
    });

    Arc::new(move |item| SendResult::from(sender.send(item)))
}
