use std::time::{SystemTime, UNIX_EPOCH};

pub fn epoch() -> u64 {
    let start = SystemTime::now();
    let duration = start.duration_since(UNIX_EPOCH).unwrap();
    return duration.as_millis() as u64;
}