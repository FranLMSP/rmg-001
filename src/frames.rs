use std::{thread, time};
use std::time::Instant;

pub struct Frames {
    count: usize,
    timer: Instant,
    time_start: u128,
    fps: u128,
}

impl Frames {
    pub fn new() -> Self {
        Self {
            count: 0,
            timer: Instant::now(),
            time_start: 0,
            fps: 16600,
        }
    }

    pub fn reset_count(&mut self) {
        self.count = 0;
    }

    pub fn reset_timer(&mut self) {
        self.timer = Instant::now();
    }

    pub fn increment(&mut self) {
        self.count = self.count.saturating_add(1);
    }

    pub fn elapsed_ms(&self) -> u128 {
        self.timer.elapsed().as_millis().saturating_sub(self.time_start)
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn limit(&self) {
        let elapsed = self.timer.elapsed().as_micros();
        if elapsed > self.fps {
            return;
        }
        let wait = (self.fps - elapsed).try_into().unwrap();
        thread::sleep(time::Duration::from_micros(wait));
    }
}
