//! Login lockout for showcase mode (S11 §4.3, D22): failed passphrase logins are counted per
//! client IP and in total, and a burst blocks further tries before any Argon2 work.
//!
//! Per IP, `per_ip_max_failures` failures within `per_ip_window_secs` block that IP for
//! `per_ip_block_secs`, doubling with each block up to `block_backoff_max_secs`. In total,
//! `global_max_failures` failures within `global_window_secs` disable passphrase login for
//! `global_block_secs`. The counters live in memory, so a restart resets them; an attack keeps
//! the Machine awake anyway. `POST /api/showcase/admin/login-unlock` clears them.

use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, MutexGuard, PoisonError};

use chrono::{DateTime, TimeDelta, Utc};
use roko_core::config::showcase::ShowcaseLoginConfig;

/// Which block refused a login.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockScope {
    /// The client's address made too many failed logins.
    Ip,
    /// Every client together did.
    Global,
}

impl LockScope {
    /// The word the 429 body carries: `ip` or `global`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ip => "ip",
            Self::Global => "global",
        }
    }
}

/// A block in force: its scope, and the seconds until it lifts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Locked {
    pub scope: LockScope,
    pub retry_after_s: u64,
}

#[derive(Debug, Default)]
struct IpCounter {
    failures: VecDeque<DateTime<Utc>>,
    blocked_until: Option<DateTime<Utc>>,
    blocks: u32,
}

#[derive(Debug, Default)]
struct Counters {
    per_ip: HashMap<String, IpCounter>,
    global: VecDeque<DateTime<Utc>>,
    global_blocked_until: Option<DateTime<Utc>>,
}

/// The failure counters of passphrase logins.
#[derive(Debug, Default)]
pub struct LoginLockout {
    counters: Mutex<Counters>,
}

impl LoginLockout {
    /// The block that refuses a login from `ip` at `now`, if one is in force.
    pub fn check(&self, ip: &str, now: DateTime<Utc>) -> Option<Locked> {
        let counters = self.counters();
        let ip_until = counters
            .per_ip
            .get(ip)
            .and_then(|counter| counter.blocked_until);
        remaining(LockScope::Global, counters.global_blocked_until, now)
            .or_else(|| remaining(LockScope::Ip, ip_until, now))
    }

    /// Count a failed login from `ip` at `now`, starting a block when it completes a burst.
    pub fn record_failure(&self, ip: &str, config: &ShowcaseLoginConfig, now: DateTime<Utc>) {
        let mut guard = self.counters();
        let counters = &mut *guard;
        let counter = counters.per_ip.entry(ip.to_string()).or_default();
        push_within(&mut counter.failures, now, config.per_ip_window_secs);
        if counter.failures.len() >= config.per_ip_max_failures.max(1) as usize {
            counter.failures.clear();
            let factor = 2_u64.saturating_pow(counter.blocks);
            let secs = config
                .per_ip_block_secs
                .saturating_mul(factor)
                .min(config.block_backoff_max_secs);
            counter.blocks = counter.blocks.saturating_add(1);
            counter.blocked_until = Some(until(now, secs));
        }
        push_within(&mut counters.global, now, config.global_window_secs);
        if counters.global.len() >= config.global_max_failures.max(1) as usize {
            counters.global.clear();
            counters.global_blocked_until = Some(until(now, config.global_block_secs));
        }
    }

    /// Clear every counter and block: the admin's unlock.
    pub fn unlock(&self) {
        *self.counters() = Counters::default();
    }

    fn counters(&self) -> MutexGuard<'_, Counters> {
        self.counters.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// `now` plus `secs`, or the end of time when that leaves chrono's range.
fn until(now: DateTime<Utc>, secs: u64) -> DateTime<Utc> {
    super::after(now, secs).unwrap_or(DateTime::<Utc>::MAX_UTC)
}

/// The block of `scope` in force at `now` when `until` lies ahead, rounded up to whole seconds.
fn remaining(scope: LockScope, until: Option<DateTime<Utc>>, now: DateTime<Utc>) -> Option<Locked> {
    let left = until? - now;
    let millis = u64::try_from(left.num_milliseconds())
        .ok()
        .filter(|millis| *millis > 0)?;
    Some(Locked {
        scope,
        retry_after_s: millis.div_ceil(1000),
    })
}

/// Add `now` to `times`, dropping the entries older than `window_secs`.
fn push_within(times: &mut VecDeque<DateTime<Utc>>, now: DateTime<Utc>, window_secs: u64) {
    let window = i64::try_from(window_secs)
        .ok()
        .and_then(TimeDelta::try_seconds)
        .unwrap_or(TimeDelta::MAX);
    while times.front().is_some_and(|at| now - *at >= window) {
        times.pop_front();
    }
    times.push_back(now);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each block from one address doubles the one before, up to the cap.
    #[test]
    fn login_lockout_backoff_doubles_up_to_the_cap() {
        let lockout = LoginLockout::default();
        let config = ShowcaseLoginConfig {
            per_ip_max_failures: 2,
            per_ip_block_secs: 100,
            block_backoff_max_secs: 350,
            global_max_failures: 1_000,
            ..ShowcaseLoginConfig::default()
        };
        let mut now = Utc::now();
        for expected in [100, 200, 350, 350] {
            lockout.record_failure("203.0.113.7", &config, now);
            assert_eq!(lockout.check("203.0.113.7", now), None);
            lockout.record_failure("203.0.113.7", &config, now);
            let locked = lockout.check("203.0.113.7", now).expect("blocked");
            let block = Locked {
                scope: LockScope::Ip,
                retry_after_s: expected,
            };
            assert_eq!(locked, block);
            assert_eq!(lockout.check("198.51.100.1", now), None);
            now += TimeDelta::seconds(i64::try_from(expected).expect("seconds"));
            assert_eq!(lockout.check("203.0.113.7", now), None);
        }
    }

    /// Failures outside the window do not add up to a block.
    #[test]
    fn login_lockout_window_forgets_old_failures() {
        let lockout = LoginLockout::default();
        let config = ShowcaseLoginConfig {
            per_ip_max_failures: 2,
            per_ip_window_secs: 60,
            ..ShowcaseLoginConfig::default()
        };
        let now = Utc::now();
        let later = now + TimeDelta::seconds(61);
        lockout.record_failure("203.0.113.7", &config, now);
        lockout.record_failure("203.0.113.7", &config, later);
        assert_eq!(lockout.check("203.0.113.7", later), None);
    }
}
