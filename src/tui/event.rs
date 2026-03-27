use crossterm::event::{Event as CrosstermEvent, EventStream, KeyEvent};
use futures::StreamExt;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::ChildStdout;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

#[derive(Debug)]
pub enum Event {
    Key(KeyEvent),
    LogLine(String),
    Tick,
    Resize,
    AdbDisconnected,
}

pub struct EventLoop {
    rx: mpsc::UnboundedReceiver<Event>,
}

impl EventLoop {
    pub fn new(logcat_stdout: ChildStdout) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        // Keyboard events
        let tx_key = tx.clone();
        tokio::spawn(async move {
            let mut reader = EventStream::new();
            loop {
                match reader.next().await {
                    Some(Ok(CrosstermEvent::Key(key))) => {
                        if tx_key.send(Event::Key(key)).is_err() {
                            break;
                        }
                    }
                    Some(Ok(CrosstermEvent::Resize(_, _))) => {
                        if tx_key.send(Event::Resize).is_err() {
                            break;
                        }
                    }
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                    None => break,
                }
            }
        });

        // Logcat line events
        let tx_log = tx.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(logcat_stdout);
            let mut lines = reader.lines();
            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        if tx_log.send(Event::LogLine(line)).is_err() {
                            break;
                        }
                    }
                    Ok(None) => {
                        let _ = tx_log.send(Event::AdbDisconnected);
                        break;
                    }
                    Err(_) => {
                        let _ = tx_log.send(Event::AdbDisconnected);
                        break;
                    }
                }
            }
        });

        // Tick events
        let tx_tick = tx;
        tokio::spawn(async move {
            let mut tick = interval(Duration::from_millis(60));
            loop {
                tick.tick().await;
                if tx_tick.send(Event::Tick).is_err() {
                    break;
                }
            }
        });

        EventLoop { rx }
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }

    /// Drain up to `max` pending events without blocking
    pub fn drain(&mut self, max: usize) -> Vec<Event> {
        let mut events = Vec::new();
        for _ in 0..max {
            match self.rx.try_recv() {
                Ok(event) => events.push(event),
                Err(_) => break,
            }
        }
        events
    }
}
