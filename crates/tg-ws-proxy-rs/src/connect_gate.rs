//! Per-destination TCP fallback singleflight and exponential failure backoff.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
#[derive(Default)]
pub struct Backoff {
    entries: Mutex<HashMap<String, Entry>>,
}
#[derive(Default)]
struct Entry {
    failures: u32,
    until: Option<Instant>,
    connecting: bool,
}
pub struct Attempt {
    state: Arc<Backoff>,
    key: String,
    success: bool,
}
pub fn delay(failures: u32) -> Duration {
    Duration::from_secs(
        30u64
            .saturating_mul(1u64 << failures.saturating_sub(1).min(7))
            .min(3600),
    )
}
impl Backoff {
    pub fn enter(self: &Arc<Self>, key: &str) -> Option<Attempt> {
        let mut entries = self.entries.lock().unwrap();
        let entry = entries.entry(key.to_string()).or_default();
        if entry.connecting || entry.until.is_some_and(|until| Instant::now() < until) {
            return None;
        }
        entry.connecting = true;
        Some(Attempt {
            state: self.clone(),
            key: key.to_string(),
            success: false,
        })
    }
}
impl Attempt {
    pub fn success(&mut self) {
        self.success = true;
    }
}
impl Drop for Attempt {
    fn drop(&mut self) {
        let mut entries = self.state.entries.lock().unwrap();
        if self.success {
            entries.remove(&self.key);
        } else if let Some(entry) = entries.get_mut(&self.key) {
            entry.connecting = false;
            entry.failures = entry.failures.saturating_add(1);
            entry.until = Some(Instant::now() + delay(entry.failures));
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn limits_parallel_attempts_and_resets_after_success() {
        let state = Arc::new(Backoff::default());
        let mut attempt = state.enter("dc:443").unwrap();
        assert!(state.enter("dc:443").is_none());
        attempt.success();
        drop(attempt);
        drop(state.enter("dc:443").unwrap());
        assert!(state.enter("dc:443").is_none());
        assert_eq!(delay(1), Duration::from_secs(30));
        assert_eq!(delay(2), Duration::from_secs(60));
        assert_eq!(delay(100), Duration::from_secs(3600));
    }
}
