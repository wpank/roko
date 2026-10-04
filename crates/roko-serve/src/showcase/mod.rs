//! Showcase mode (S11): the passphrase-gated public showcase that `roko serve` runs on Fly.
//!
//! A passphrase login mints a session with the narrow [`SHOWCASE_SCOPE`]; its record lives in
//! [`crate::state::LocalAccess`], with the passphrase generation that minted it. [`auth`] checks
//! the passphrase and the CSRF header and origin of every session request.

pub mod auth;

/// The scope a passphrase session carries (S11 §4.2): it reaches the showcase routes only.
pub const SHOWCASE_SCOPE: &str = "showcase";
