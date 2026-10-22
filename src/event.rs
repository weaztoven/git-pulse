use std::{
    thread,
    time::{Duration, Instant},
};

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind};
use tokio::sync::mpsc;

use crate::github::models::{FetchResult, Repo, TrendingResults};

pub const TICK_RATE: Duration = Duration::from_millis(250);

pub enum Event {
    Tick,
    Key(KeyEvent),
    Resize,
    RepoUpdated {
        name: String,
        result: Result<FetchResult, String>,
    },
    TrendingUpdated(Result<TrendingResults, String>),
    MyReposUpdated(Result<Vec<Repo>, String>),
}

pub struct EventHandler {
    tx: mpsc::UnboundedSender<Event>,
    rx: mpsc::UnboundedReceiver<Event>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let thread_tx = tx.clone();

        thread::spawn(move || {
            let mut last_tick = Instant::now();
            loop {
                let timeout = tick_rate.saturating_sub(last_tick.elapsed());

                if event::poll(timeout).unwrap_or(false) {
                    let sent = match event::read() {
                        Ok(CrosstermEvent::Key(key)) if key.kind == KeyEventKind::Press => {
                            thread_tx.send(Event::Key(key))
                        }
                        Ok(CrosstermEvent::Resize(_, _)) => thread_tx.send(Event::Resize),
                        _ => Ok(()),
                    };
                    if sent.is_err() {
                        break;
                    }
                }

                if last_tick.elapsed() >= tick_rate {
                    if thread_tx.send(Event::Tick).is_err() {
                        break;
                    }
                    last_tick = Instant::now();
                }
            }
        });

        Self { tx, rx }
    }

    pub fn sender(&self) -> mpsc::UnboundedSender<Event> {
        self.tx.clone()
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}
