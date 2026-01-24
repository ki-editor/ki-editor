use std::sync::mpsc::SendError;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct Callback<T>(Arc<dyn Fn(T) + Send + Sync>);

impl<T> PartialEq for Callback<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Callback<T> {
    pub fn new(callback: Arc<dyn Fn(T) + Send + Sync>) -> Self {
        Self(callback)
    }

    pub fn call(&self, event: T) {
        self.0(event)
    }
}

pub fn debounce<T: PartialEq + Eq + Send + Sync + 'static>(
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

pub enum SendResult {
    Succeeed,
    ReceiverDisconnected,
}
impl SendResult {
    pub fn is_receiver_disconnected(&self) -> bool {
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

pub fn batch<T: Send + Sync + 'static>(
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

#[derive(Clone)]
pub struct Interval {
    callback: Callback<()>,
}

impl Interval {
    pub fn cancel(&self) {
        self.callback.call(())
    }
}

pub fn set_interval(callback: Callback<usize>, duration: Duration) -> Interval {
    let (sender, receiver) = std::sync::mpsc::channel::<()>();

    let mut iteration_count = 0;

    std::thread::spawn(move || loop {
        callback.call(iteration_count);
        iteration_count += 1;
        std::thread::sleep(duration);
        if receiver.try_recv().is_ok() {
            break;
        }
    });

    Interval {
        callback: Callback::new(Arc::new(move |event| {
            let _ = sender.send(event);
        })),
    }
}

pub fn set_timeout(callback: Callback<()>, duration: Duration) {
    std::thread::spawn(move || {
        std::thread::sleep(duration);
        callback.call(())
    });
}
