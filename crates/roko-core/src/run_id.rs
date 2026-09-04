//! ULID-based plan-scoped run identifier.
//!
//! A `RunId` uniquely identifies a single execution run within a plan.
//! It is generated once at the start of each plan execution and threaded
//! through all downstream components (graph engine, cells, receipts).

use std::fmt;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// A unique, sortable run identifier.
///
/// Encodes a millisecond timestamp and a random component to ensure
/// uniqueness across concurrent runs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RunId(String);

impl RunId {
    /// Generate a new `RunId` with the current timestamp.
    #[must_use]
    pub fn new() -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let random: u64 = rand_u64();
        Self(format!("run-{ts:013x}-{random:08x}"))
    }

    /// Create a `RunId` from a raw string (for deserialization / restore).
    #[must_use]
    pub fn from_raw(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Return the string representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for RunId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for RunId {
    type Err = RunIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(RunIdParseError::Empty);
        }
        Ok(Self(s.to_string()))
    }
}

impl AsRef<str> for RunId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Error type for `RunId` parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunIdParseError {
    /// The input string was empty.
    Empty,
}

impl fmt::Display for RunIdParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "run ID cannot be empty"),
        }
    }
}

impl std::error::Error for RunIdParseError {}

/// Simple random u64 using timestamp + counter fallback.
fn rand_u64() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let c = COUNTER.fetch_add(1, Ordering::Relaxed);
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    ts.wrapping_mul(6364136223846793005).wrapping_add(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_id_uniqueness() {
        let a = RunId::new();
        let b = RunId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn run_id_display_and_parse() {
        let id = RunId::new();
        let s = id.to_string();
        let parsed: RunId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn run_id_from_raw() {
        let id = RunId::from_raw("custom-run-id");
        assert_eq!(id.as_str(), "custom-run-id");
    }

    #[test]
    fn empty_parse_fails() {
        let result: Result<RunId, _> = "".parse();
        assert!(result.is_err());
    }
}
