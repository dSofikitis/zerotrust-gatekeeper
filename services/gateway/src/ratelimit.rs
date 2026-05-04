//! Per-tenant fixed-window rate limiter.
//!
//! v0.1 ships an [`InMemoryRateLimiter`] keyed by
//! `<tenant>:<method>:<path-segment>`. The Redis-backed equivalent
//! slots in behind the same shape in v0.2 — both algorithms expose
//! [`check`](InMemoryRateLimiter::check), which atomically
//! increments the counter and either returns the remaining budget
//! or refuses with `Retry-After`-suitable seconds.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;

/// Static per-key budget. Defaults are tuned for the demo: 100
/// requests per 60s window, which is enough to walk through a curl
/// loop without tripping by accident, and small enough that
/// `examples/curl/hit-rate-limit.sh` can blow past it in a few
/// seconds.
#[derive(Debug, Clone, Copy)]
pub struct Limit {
    pub max: u32,
    pub window: Duration,
}

impl Default for Limit {
    fn default() -> Self {
        Self {
            max: 100,
            window: Duration::from_secs(60),
        }
    }
}

#[derive(Debug, Clone)]
struct Bucket {
    count: u32,
    expires_at: Instant,
}

/// In-memory fixed-window counter. Single-process — fine for a v0.1
/// demo, the production path swaps in Redis with the same `check`
/// signature so the middleware doesn't change.
pub struct InMemoryRateLimiter {
    buckets: Arc<Mutex<HashMap<String, Bucket>>>,
    limit: Limit,
}

impl InMemoryRateLimiter {
    pub fn new(limit: Limit) -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
            limit,
        }
    }

    /// Returns `Ok(remaining)` if the request is within budget; the
    /// caller stamps that on the response. Returns `Err(retry_after_secs)`
    /// when the budget is exhausted; the caller emits a 429 with a
    /// `Retry-After` header.
    pub async fn check(&self, key: &str) -> Result<u32, u64> {
        self.check_at(key, Instant::now()).await
    }

    async fn check_at(&self, key: &str, now: Instant) -> Result<u32, u64> {
        let mut buckets = self.buckets.lock().await;
        let entry = buckets.entry(key.to_string()).or_insert(Bucket {
            count: 0,
            expires_at: now + self.limit.window,
        });
        if now >= entry.expires_at {
            entry.count = 0;
            entry.expires_at = now + self.limit.window;
        }
        if entry.count >= self.limit.max {
            let retry = entry
                .expires_at
                .saturating_duration_since(now)
                .as_secs()
                .max(1);
            return Err(retry);
        }
        entry.count += 1;
        Ok(self.limit.max - entry.count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limiter(max: u32, window_secs: u64) -> InMemoryRateLimiter {
        InMemoryRateLimiter::new(Limit {
            max,
            window: Duration::from_secs(window_secs),
        })
    }

    #[tokio::test]
    async fn allows_under_budget() {
        let l = limiter(3, 60);
        assert_eq!(l.check("k").await, Ok(2));
        assert_eq!(l.check("k").await, Ok(1));
        assert_eq!(l.check("k").await, Ok(0));
    }

    #[tokio::test]
    async fn rejects_at_budget() {
        let l = limiter(2, 60);
        let _ = l.check("k").await;
        let _ = l.check("k").await;
        let result = l.check("k").await;
        assert!(matches!(result, Err(retry) if retry >= 1));
    }

    #[tokio::test]
    async fn budget_isolated_per_key() {
        let l = limiter(1, 60);
        assert_eq!(l.check("a").await, Ok(0));
        // "a" is full but "b" isn't.
        assert_eq!(l.check("b").await, Ok(0));
        assert!(l.check("a").await.is_err());
    }

    #[tokio::test]
    async fn budget_resets_after_window() {
        let l = limiter(1, 60);
        let t0 = Instant::now();
        assert_eq!(l.check_at("k", t0).await, Ok(0));
        assert!(l.check_at("k", t0).await.is_err());
        // After the window, the bucket resets.
        let t1 = t0 + Duration::from_secs(61);
        assert_eq!(l.check_at("k", t1).await, Ok(0));
    }
}
