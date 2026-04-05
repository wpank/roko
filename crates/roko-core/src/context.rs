//! Execution context — the shared environment passed to every trait method.
//!
//! [`Context`] carries time, goal, and budget information that scorers,
//! routers, composers, policies, and gates all need. It's deliberately small
//! and extensible via its `attrs` field.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The shared runtime context for a Roko operation.
///
/// Passed to every trait method so impls can be pure functions of their input
/// plus this context. Context is cheap to clone.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Context {
    /// Current time in Unix milliseconds. Used for decay calculations.
    pub now_ms: i64,

    /// Optional goal or intent driving this operation. Scorers/routers can
    /// use it for goal-alignment scoring.
    pub goal: Option<String>,

    /// Optional session identifier — groups related signals under one run.
    pub session: Option<String>,

    /// Extension attributes. Use for signals not covered by the fields above.
    /// Reverse-DNS keys recommended (`"com.example.feature"`).
    pub attrs: BTreeMap<String, String>,
}

impl Context {
    /// A fresh context with the current wall-clock time.
    #[must_use]
    pub fn now() -> Self {
        Self {
            now_ms: chrono::Utc::now().timestamp_millis(),
            ..Default::default()
        }
    }

    /// A context with a fixed time (useful for tests).
    #[must_use]
    pub fn at(now_ms: i64) -> Self {
        Self {
            now_ms,
            ..Default::default()
        }
    }

    /// Attach a goal to this context.
    #[must_use]
    pub fn with_goal(mut self, goal: impl Into<String>) -> Self {
        self.goal = Some(goal.into());
        self
    }

    /// Attach a session identifier.
    #[must_use]
    pub fn with_session(mut self, session: impl Into<String>) -> Self {
        self.session = Some(session.into());
        self
    }

    /// Set an extension attribute.
    #[must_use]
    pub fn with_attr(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attrs.insert(key.into(), value.into());
        self
    }

    /// Look up an extension attribute.
    #[must_use]
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attrs.get(key).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn at_fixes_time() {
        let c = Context::at(12_345);
        assert_eq!(c.now_ms, 12_345);
    }

    #[test]
    fn with_goal_sets_goal() {
        let c = Context::at(0).with_goal("ship feature X");
        assert_eq!(c.goal.as_deref(), Some("ship feature X"));
    }

    #[test]
    fn attrs_roundtrip() {
        let c = Context::at(0).with_attr("env", "prod");
        assert_eq!(c.attr("env"), Some("prod"));
        assert_eq!(c.attr("missing"), None);
    }

    #[test]
    fn now_gives_recent_time() {
        let c = Context::now();
        // Should be within a minute of wall clock
        let diff = (chrono::Utc::now().timestamp_millis() - c.now_ms).abs();
        assert!(diff < 60_000);
    }
}
