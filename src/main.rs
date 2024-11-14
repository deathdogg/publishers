use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

pub mod publisher;
#[tokio::main]
async fn main() {
    let t = publisher::TimerPublisher::new(1.0);
    let s = Arc::new(publisher::Sink::new(&t, || println!("Hello, world!")));
    s.run().await;
    t.start();
}
// Timer that performs a function at a given interval once started.

struct Timer {
    interval: f32,
    perform: fn(),
    thread: Option<JoinHandle<()>>,
    sender: Sender<bool>,
    is_running: bool,
}
impl Timer {
    fn new(interval: f32, sender: Sender<bool>, perform: fn()) -> Self {
        Timer {
            interval: interval,
            sender: sender,
            perform: perform,
            thread: None,
            is_running: false,
        }
    }
    fn start(&mut self, receiver: Receiver<bool>) {
        self.is_running = true;
        let interval = self.interval;
        let p = self.perform;
        self.thread = Some(thread::spawn(move || loop {
            thread::sleep(std::time::Duration::from_secs_f32(interval));
            (p)();
            if let Ok(value) = receiver.try_recv() {
                if value {
                    break;
                }
            }
        }));
    }
    fn stop(&mut self) {
        self.sender.send(true);
        self.is_running = false;
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap_or(());
        }
    }
}
