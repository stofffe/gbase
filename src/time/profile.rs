use super::Instant;

pub struct ProfileTimer {
    name: &'static str,
    start: Instant,
}

impl ProfileTimer {
    pub fn new(name: &'static str) -> Self {
        let start = Instant::now();
        Self { name, start }
    }

    pub fn print(self) {
        let time = self.start.elapsed().as_millis();
        log::info!("[PROFILE] {} ms: {}", time, self.name);
    }
}
