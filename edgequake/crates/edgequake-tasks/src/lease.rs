//! Task lease TTL helpers (SPEC-057 P1 / SPEC-156 heartbeat).

use chrono::{DateTime, Utc};
use std::time::Duration;

/// Default lease TTL (seconds).
pub const DEFAULT_TASK_LEASE_TTL_SECS: u64 = 120;

/// Minimum allowed lease TTL.
pub const MIN_TASK_LEASE_TTL_SECS: u64 = 30;

/// Floor for heartbeat interval so we never spin the poller.
pub const MIN_HEARTBEAT_INTERVAL_SECS: u64 = 5;

/// Resolve lease TTL from `EDGEQUAKE_TASK_LEASE_TTL_SECS` (default 120, min 30).
pub fn task_lease_ttl_from_env() -> Duration {
    let secs = std::env::var("EDGEQUAKE_TASK_LEASE_TTL_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_TASK_LEASE_TTL_SECS)
        .max(MIN_TASK_LEASE_TTL_SECS);
    Duration::from_secs(secs)
}

/// Heartbeat interval derived from lease TTL (SPEC-156).
///
/// WHY `ttl/3`: at least three refresh attempts before expiry. A fixed 60s
/// heartbeat with TTL=30s allowed another worker to reclaim mid-flight.
pub fn heartbeat_interval_for_lease_ttl(ttl: Duration) -> Duration {
    let secs = (ttl.as_secs() / 3).max(MIN_HEARTBEAT_INTERVAL_SECS);
    Duration::from_secs(secs)
}

/// Compute lease expiry timestamp from `now` + `ttl`.
pub fn lease_expires_at(now: DateTime<Utc>, ttl: Duration) -> DateTime<Utc> {
    now + chrono::Duration::from_std(ttl).unwrap_or(chrono::Duration::seconds(
        DEFAULT_TASK_LEASE_TTL_SECS as i64,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ttl_is_at_least_min() {
        // Do not mutate env in parallel tests — just validate constants.
        const {
            assert!(DEFAULT_TASK_LEASE_TTL_SECS >= MIN_TASK_LEASE_TTL_SECS);
        }
        assert_eq!(DEFAULT_TASK_LEASE_TTL_SECS, 120);
    }

    #[test]
    fn heartbeat_interval_is_ttl_over_three_with_floor() {
        assert_eq!(
            heartbeat_interval_for_lease_ttl(Duration::from_secs(30)),
            Duration::from_secs(10)
        );
        assert_eq!(
            heartbeat_interval_for_lease_ttl(Duration::from_secs(120)),
            Duration::from_secs(40)
        );
        assert_eq!(
            heartbeat_interval_for_lease_ttl(Duration::from_secs(600)),
            Duration::from_secs(200)
        );
        // Tiny TTL still floors at 5s (MIN_TASK_LEASE is 30, but pure fn accepts any).
        assert_eq!(
            heartbeat_interval_for_lease_ttl(Duration::from_secs(9)),
            Duration::from_secs(5)
        );
    }

    /// SPEC-156: worker must derive heartbeat from lease TTL (not a fixed 60s).
    #[test]
    fn contract_worker_uses_lease_derived_heartbeat() {
        let worker = include_str!("worker.rs");
        assert!(
            worker.contains("heartbeat_interval_for_lease_ttl(heartbeat_ttl)"),
            "worker lease heartbeat must call heartbeat_interval_for_lease_ttl"
        );
        assert!(
            !worker.contains("interval(tokio::time::Duration::from_secs(60))"),
            "fixed 60s heartbeat must not return (TTL can be 30s)"
        );
        assert!(
            worker.contains("Lease refresh failed — will retry next heartbeat"),
            "lease refresh errors must warn (not silent debug)"
        );
    }
}
