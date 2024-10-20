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

    pub fn log(self) {
        drop(self);
    }
}

impl Drop for ProfileTimer {
    fn drop(&mut self) {
        let time = self.start.elapsed().as_secs_f64() * 1000.0;
        log::info!("[PROFILE] {:.5} ms: {}", time, self.name);
    }
}
