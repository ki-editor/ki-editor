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
        self.0(event);
    }

    pub fn no_op() -> Callback<T> {
        Callback::new(Arc::new(|_| {}))
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
            debounce.put(event);
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

#[derive(Debug)]
pub enum BatchResult<T> {
    Items(Vec<T>),
    LimitReached,
}

pub fn batch<T: std::fmt::Debug + Send + Sync + 'static>(
    callback: Arc<dyn Fn(BatchResult<T>) -> SendResult + Send + Sync>,
    on_finish: Callback<()>,
    interval: Duration,
    limit: usize,
) -> Arc<dyn Fn(T) -> SendResult + Send + Sync> {
    let (sender, receiver) = std::sync::mpsc::channel::<T>();

    std::thread::spawn(move || {
        let mut count = 0;
        let mut batch = vec![];
        let mut last_sent = Instant::now();
        while let Ok(item) = receiver.recv() {
            // Send the first item immediatly to allow instant UI feedback
            if count == 0 {
                callback(BatchResult::Items(std::iter::once(item).collect()));
            } else {
                batch.push(item);
            };
            count += 1;
            if Instant::now() - last_sent > interval {
                callback(BatchResult::Items(std::mem::take(&mut batch)));
                last_sent = Instant::now();
            }
            if count > limit {
                callback(BatchResult::LimitReached);
                break;
            }
        }

        // Sending the remaining items
        callback(BatchResult::Items(std::mem::take(&mut batch)));

        on_finish.call(());
    });

    Arc::new(move |item| SendResult::from(sender.send(item)))
}

#[derive(Clone)]
pub struct Interval {
    callback: Callback<IntervalEvent>,
}

impl Interval {
    pub fn cancel(&self) {
        self.callback.call(IntervalEvent::Cancel);
    }

    pub(crate) fn resume(&self) {
        self.callback.call(IntervalEvent::Resume);
    }

    pub(crate) fn pause(&self) {
        self.callback.call(IntervalEvent::Pause);
    }
}

#[derive(Clone)]
enum IntervalEvent {
    Resume,
    Pause,
    Cancel,
}

pub fn set_interval(callback: Callback<usize>, duration: Duration) -> Interval {
    let (sender, receiver) = std::sync::mpsc::channel::<IntervalEvent>();

    let mut iteration_count = 0;

    std::thread::spawn(move || loop {
        callback.call(iteration_count);
        iteration_count += 1;
        std::thread::sleep(duration);

        if let Ok(event) = receiver.try_recv() {
            match event {
                IntervalEvent::Resume => continue,
                IntervalEvent::Pause => {
                    while let Ok(event) = receiver.recv() {
                        if matches!(event, IntervalEvent::Resume) {
                            break;
                        }
                    }
                }
                IntervalEvent::Cancel => break,
            }
        }
    });

    Interval {
        callback: Callback::new(Arc::new(move |event| {
            let _ = sender.send(event);
        })),
    }
}
