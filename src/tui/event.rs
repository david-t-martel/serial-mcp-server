//! Event handling for the TUI.

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// Terminal events.
#[derive(Debug, Clone)]
pub enum Event {
    /// Terminal tick for UI refresh
    Tick,
    /// Keyboard input
    Key(KeyEvent),
    /// Mouse input
    Mouse(MouseEvent),
    /// Terminal resize
    Resize(u16, u16),
    /// Serial data received
    SerialRx(Vec<u8>),
    /// Serial port connected
    PortConnected(String),
    /// Serial port disconnected
    PortDisconnected(String),
    /// Error occurred
    Error(String),
}

/// Event handler that polls for terminal events.
pub struct EventHandler {
    /// Event sender
    sender: mpsc::Sender<Event>,
    /// Event receiver
    receiver: mpsc::Receiver<Event>,
    /// Event handler thread
    #[allow(dead_code)]
    handler: thread::JoinHandle<()>,
}

impl EventHandler {
    /// Create a new event handler with the specified tick rate.
    pub fn new(tick_rate: Duration) -> Self {
        let (sender, receiver) = mpsc::channel();
        let handler_sender = sender.clone();

        let handler = thread::spawn(move || {
            let mut last_tick = Instant::now();
            loop {
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or(Duration::ZERO);

                if event::poll(timeout).unwrap_or(false) {
                    match event::read() {
                        Ok(CrosstermEvent::Key(key)) => {
                            if handler_sender.send(Event::Key(key)).is_err() {
                                break;
                            }
                        }
                        Ok(CrosstermEvent::Mouse(mouse)) => {
                            if handler_sender.send(Event::Mouse(mouse)).is_err() {
                                break;
                            }
                        }
                        Ok(CrosstermEvent::Resize(width, height)) => {
                            if handler_sender.send(Event::Resize(width, height)).is_err() {
                                break;
                            }
                        }
                        Ok(_) => {}
                        Err(e) => {
                            let _ = handler_sender.send(Event::Error(e.to_string()));
                        }
                    }
                }

                if last_tick.elapsed() >= tick_rate {
                    if handler_sender.send(Event::Tick).is_err() {
                        break;
                    }
                    last_tick = Instant::now();
                }
            }
        });

        Self {
            sender,
            receiver,
            handler,
        }
    }

    /// Get the next event, blocking until one is available.
    pub fn next(&self) -> Result<Event, mpsc::RecvError> {
        self.receiver.recv()
    }

    /// Try to get the next event without blocking.
    pub fn try_next(&self) -> Option<Event> {
        self.receiver.try_recv().ok()
    }

    /// Get a sender for pushing custom events.
    pub fn sender(&self) -> mpsc::Sender<Event> {
        self.sender.clone()
    }
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new(Duration::from_millis(33)) // ~30 FPS
    }
}
