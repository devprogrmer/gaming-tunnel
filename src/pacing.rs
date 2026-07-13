use tokio::time::{Duration, Instant};

/// Token-bucket pacer. rate_mbps == 0 disables pacing (burst).
pub struct Pacer {
    rate_bytes_per_sec: f64,
    tokens: f64,
    capacity: f64,
    last: Instant,
}

impl Pacer {
    pub fn new(rate_mbps: u64) -> Self {
        let rate = (rate_mbps as f64) * 1_000_000.0 / 8.0;
        Self { rate_bytes_per_sec: rate, tokens: rate, capacity: rate.max(65535.0), last: Instant::now() }
    }

    pub async fn wait_for(&mut self, bytes: usize) {
        if self.rate_bytes_per_sec <= 0.0 { return; }
        loop {
            let now = Instant::now();
            self.tokens = (self.tokens + now.duration_since(self.last).as_secs_f64()
                * self.rate_bytes_per_sec).min(self.capacity);
            self.last = now;
            if self.tokens >= bytes as f64 { self.tokens -= bytes as f64; return; }
            let deficit = bytes as f64 - self.tokens;
            tokio::time::sleep(Duration::from_secs_f64(deficit / self.rate_bytes_per_sec)).await;
        }
    }
}
