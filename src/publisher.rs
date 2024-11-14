use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use tokio::time::Duration;
pub trait Publisher {
    fn publish(self: &Arc<Self>);
    fn subscribe(self: &Arc<Self>) -> broadcast::Receiver<bool>;
}
#[derive(Clone)]
pub struct TimerPublisher {
    is_running: Arc<AtomicBool>, // Simplified sync primitive
    interval: f32,
    publisher: broadcast::Sender<bool>,
}

impl TimerPublisher {
    pub fn new(interval: f32) -> Arc<Self> {
        let (publisher, _) = broadcast::channel(16);

        Arc::new(Self {
            is_running: Arc::new(AtomicBool::new(false)),
            interval,
            publisher,
        })
    }

    pub fn start(self: &Arc<Self>) {
        if self.is_running.load(SeqCst) {
            return;
        }
        self.is_running.store(true, SeqCst);
        let interval = tokio::time::Duration::from_secs_f32(self.interval);
        let mut interval_timer = tokio::time::interval(interval);
        interval_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let publisher = self.publisher.clone();
        let is_running = self.is_running.clone();
        tokio::spawn(async move {
            while is_running.load(SeqCst) {
                interval_timer.tick().await;
                if let Err(_) = publisher.send(true) {
                    is_running.store(false, SeqCst);
                    break;
                }
            }
        });
    }

    pub async fn stop(&self) {
        self.is_running.store(false, SeqCst);
    }
}

impl Publisher for TimerPublisher {
    fn publish(self: &Arc<Self>) {
        let _ = self.publisher.send(true);
    }

    fn subscribe(self: &Arc<Self>) -> broadcast::Receiver<bool> {
        if !self.is_running.load(SeqCst) {
            self.start();
        }
        return self.publisher.subscribe();
    }
}
pub struct Sink {
    receiver: tokio::sync::Mutex<broadcast::Receiver<bool>>,

    perform: fn(),
}
impl Sink {
    pub fn new<P>(publisher: &Arc<P>, perform: fn()) -> Self
    where
        P: Publisher,
    {
        let sink = Self {
            receiver: Mutex::new(publisher.subscribe()),
            perform: perform,
        };
        return sink;
    }
    // A function that uses a loop to check the receiver for values, and perform the perform action when a value is received
    // The loop will yield control back to the main thread after each iteration
    pub async fn run(&self) {
        loop {
            let mut receiver = self.receiver.lock().await;
            match receiver.recv().await {
                Ok(true) => (self.perform)(),
                Ok(false) => break,
                Err(e) => {
                    println!("Error: {:?}", e);
                    break;
                }
            }
        }
    }
}
