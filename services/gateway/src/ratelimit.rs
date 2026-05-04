//! Per-tenant fixed-window rate limiter.
//!
//! Two backends behind a single [`RateLimiter`] dispatch enum:
//!
//! - [`InMemoryRateLimiter`] — single-process counter keyed by
//!   `<tenant>:<method>:<path-segment>`. Fine for the default deploy
//!   and dev runs.
//! - [`RedisRateLimiter`] — atomic `INCR` + `EXPIRE` against a
//!   shared Redis. Use when the gateway runs as more than one
//!   replica so the counter is consistent across them.
//!
//! Both expose `check(key)`, which either returns the remaining
//! budget or refuses with `Retry-After`-suitable seconds. The Redis
//! backend fails *open* on connection errors — a flaky cache must
//! not take the gateway down with it.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use redis::aio::ConnectionManager;
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

/// Backend-agnostic rate-limit handle. The middleware holds an
/// `Arc<RateLimiter>` and never sees which backend is wired.
pub enum RateLimiter {
    InMemory(InMemoryRateLimiter),
    Redis(RedisRateLimiter),
}

impl RateLimiter {
    /// Returns `Ok(remaining)` on success, `Err(retry_after_secs)` when
    /// the key is at budget. See backend-specific docs for failure
    /// semantics.
    pub async fn check(&self, key: &str) -> Result<u32, u64> {
        match self {
            Self::InMemory(l) => l.check(key).await,
            Self::Redis(l) => l.check(key).await,
        }
    }
}

#[derive(Debug, Clone)]
struct Bucket {
    count: u32,
    expires_at: Instant,
}

/// In-memory fixed-window counter.
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

/// Redis-backed fixed-window counter. Each `check` runs an atomic
/// `INCR` + `TTL` pipeline; the first request of a new window also
/// sets `EXPIRE window_secs`. Connection failures *fail open* (the
/// request is allowed and a warning is logged) so a transient Redis
/// outage doesn't take the data plane down.
pub struct RedisRateLimiter {
    conn: ConnectionManager,
    limit: Limit,
}

impl RedisRateLimiter {
    /// Connect to Redis and resolve the connection manager. The
    /// returned limiter is `Send + Sync` and cheap to clone the
    /// underlying connection on each call.
    pub async fn connect(url: &str, limit: Limit) -> anyhow::Result<Self> {
        let client = redis::Client::open(url)?;
        let conn = ConnectionManager::new(client).await?;
        Ok(Self { conn, limit })
    }

    pub async fn check(&self, key: &str) -> Result<u32, u64> {
        let mut conn = self.conn.clone();
        let prefixed = format!("zt:rl:{key}");
        let window_secs = self.limit.window.as_secs() as i64;

        let pipe_result: redis::RedisResult<(u64, i64)> = redis::pipe()
            .atomic()
            .incr(&prefixed, 1u64)
            .ttl(&prefixed)
            .query_async(&mut conn)
            .await;

        let (count, ttl) = match pipe_result {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "redis rate limiter unavailable; allowing request (fail-open)"
                );
                return Ok(self.limit.max);
            }
        };

        if ttl < 0 {
            // First request in this window — set the expiry.
            let _: redis::RedisResult<()> = redis::cmd("EXPIRE")
                .arg(&prefixed)
                .arg(window_secs)
                .query_async(&mut conn)
                .await;
        }

        if count > self.limit.max as u64 {
            let retry = if ttl > 0 { ttl as u64 } else { window_secs as u64 };
            return Err(retry.max(1));
        }
        Ok(self.limit.max - count as u32)
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

    #[tokio::test]
    async fn dispatch_through_enum_routes_to_backend() {
        let rl = RateLimiter::InMemory(limiter(2, 60));
        assert_eq!(rl.check("k").await, Ok(1));
        assert_eq!(rl.check("k").await, Ok(0));
        assert!(rl.check("k").await.is_err());
    }

    /// Smoke check that a Redis URL pointing at a closed port surfaces
    /// the connect error rather than panicking.
    #[tokio::test]
    async fn redis_connect_error_surfaces() {
        let result = RedisRateLimiter::connect(
            "redis://127.0.0.1:1/", // port 1 = closed
            Limit::default(),
        )
        .await;
        assert!(result.is_err());
    }
}
