use enigo::{Button, Coordinate, Direction, Enigo, Mouse};
use rdev::{listen, Event, EventType};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClickEvent {
    pub x: i32,
    pub y: i32,
    pub delay_ms: u64,
    pub button: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClickSequence {
    pub name: String,
    pub events: Vec<ClickEvent>,
}

pub struct ClickRecorder {
    events: Arc<Mutex<Vec<ClickEvent>>>,
    last: Arc<Mutex<Option<Instant>>>,
    running: Arc<Mutex<bool>>,
}

impl ClickRecorder {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(vec![])),
            last: Arc::new(Mutex::new(None)),
            running: Arc::new(Mutex::new(false)),
        }
    }
    pub fn start(&self) {
        *self.running.lock().unwrap() = true;
        *self.last.lock().unwrap() = Some(Instant::now());
        let events = Arc::clone(&self.events);
        let last = Arc::clone(&self.last);
        let running = Arc::clone(&self.running);
        std::thread::spawn(move || {
            let cb = move |e: Event| {
                if !*running.lock().unwrap() { return; }
                if let EventType::ButtonPress(_) = e.event_type {
                    let now = Instant::now();
                    let delay = last.lock().unwrap().map(|l| now.duration_since(l).as_millis() as u64).unwrap_or(0);
                    *last.lock().unwrap() = Some(now);
                    events.lock().unwrap().push(ClickEvent { x: 0, y: 0, delay_ms: delay, button: "left".into() });
                }
            };
            let _ = listen(cb);
        });
    }
    pub fn stop(&self) -> ClickSequence {
        *self.running.lock().unwrap() = false;
        let ev = self.events.lock().unwrap().clone();
        self.events.lock().unwrap().clear();
        ClickSequence { name: format!("seq_{}", chrono::Utc::now().timestamp()), events: ev }
    }
}

pub fn play(seq: &ClickSequence) {
    let mut enigo = Enigo::new(&enigo::Settings::default()).unwrap();
    for e in &seq.events {
        std::thread::sleep(Duration::from_millis(e.delay_ms));
        let _ = enigo.move_mouse(e.x, e.y, Coordinate::Abs);
        let _ = enigo.button(Button::Left, Direction::Click);
    }
}
