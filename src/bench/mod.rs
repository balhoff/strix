use std::time::Instant;

pub struct StageTimer {
    started_at: Instant,
}

impl StageTimer {
    pub fn start() -> Self {
        Self {
            started_at: Instant::now(),
        }
    }

    pub fn elapsed_ms(&self) -> u128 {
        self.started_at.elapsed().as_millis()
    }
}

pub fn peak_rss_bytes() -> Option<u64> {
    None
}
