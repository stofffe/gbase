use super::Instant;

pub struct Timer {
    duration: std::time::Duration,
    start: Instant,
}

impl Timer {
    pub fn new(duration: std::time::Duration) -> Self {
        Self {
            duration,
            start: Instant::now(),
        }
    }

    pub fn ticked(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.start) > self.duration {
            self.start = now;
            return true;
        }
        false
    }
}
