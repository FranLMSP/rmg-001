use std::time::Instant;

pub struct Frames {
    count: usize,
    timer: Instant,
    time_start: u128,
}

impl Frames {
    pub fn new() -> Self {
        Self {
            count: 0,
            timer: Instant::now(),
            time_start: 0,
        }
    }

    pub fn reset_count(&mut self) {
        self.count = 0;
    }

    pub fn reset_timer(&mut self) {
        self.time_start = self.timer.elapsed().as_millis();
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
}
