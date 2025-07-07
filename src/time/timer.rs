use super::Instant;

use std::time::Duration;

#[derive(Debug)]
pub struct Timer {
    duration: std::time::Duration,
    start: Instant,
    ticked: bool,
}

impl Timer {
    pub fn new(duration: std::time::Duration) -> Self {
        Self {
            duration,
            start: Instant::now(),
            ticked: false,
        }
    }

    pub fn ticked(&mut self) -> bool {
        if Instant::now().duration_since(self.start) > self.duration {
            self.ticked = true;
            return true;
        }
        false
    }
    pub fn just_ticked(&mut self) -> bool {
        if Instant::now().duration_since(self.start) > self.duration && !self.ticked {
            self.ticked = true;
            return true;
        }
        false
    }

    pub fn reset(&mut self) {
        self.start = Instant::now();
        self.ticked = false;
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            duration: Duration::ZERO,
            start: Instant::now(),
            ticked: false,
        }
    }
}
