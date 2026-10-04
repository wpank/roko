//! Showcase mode (S11): the passphrase-gated public showcase that `roko serve` runs on Fly.
//!
//! A passphrase login mints a session with the narrow [`SHOWCASE_SCOPE`]; its record lives in
//! [`crate::state::LocalAccess`], with the passphrase generation that minted it. [`auth`] checks
//! the passphrase and the CSRF header and origin of every session request; [`lockout`] counts
//! failed logins and blocks bursts.

pub mod auth;
pub mod lockout;

use chrono::{DateTime, TimeDelta, Utc};

/// The scope a passphrase session carries (S11 §4.2): it reaches the showcase routes only.
pub const SHOWCASE_SCOPE: &str = "showcase";

/// `now` plus `secs`, or `None` when the sum leaves chrono's range (never, in effect).
pub(crate) fn after(now: DateTime<Utc>, secs: u64) -> Option<DateTime<Utc>> {
    let delta = TimeDelta::try_seconds(i64::try_from(secs).ok()?)?;
    now.checked_add_signed(delta)
}
