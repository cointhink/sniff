use std::time;

pub struct Timer {
    rx_byte_count: u128,
    rx_msg_count: u128,
    now: time::Instant,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            rx_byte_count: 0,
            rx_msg_count: 0,
            now: time::Instant::now(),
        }
    }

    pub fn next_msg(self: &mut Self, msg_len: usize) {
        self.rx_byte_count += msg_len as u128;
        self.rx_msg_count += 1;
    }

    pub fn report(self: &Self) -> (f64, f64) {
        let duration_ms10 = self.now.elapsed().as_millis() + 1;
        let kb_sec = (self.rx_msg_count * 1000).div_ceil(duration_ms10) as f64 / 10.0;
        let msg_sec = (self.rx_byte_count).div_ceil(duration_ms10) as f64 / 10.0;
        (kb_sec, msg_sec)
    }

    pub fn reset_after_seconds(self: &mut Self, seconds: usize) {
        let duration = self.now.elapsed();
        if duration.as_secs() > seconds as u64 {
            self.now = std::time::Instant::now();
            self.rx_byte_count = 0;
        }
    }
}
