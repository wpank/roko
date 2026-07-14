//! Centralized timeout configuration.
//!
//! All operational timeouts live here so they can be tuned from `roko.toml`
//! without touching code.  Each field has a sensible default; the struct
//! derives `Default` via explicit `const fn` helpers for serde compatibility.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Centralized timeout configuration for all roko subsystems.
///
/// Serialized under `[timeouts]` in `roko.toml`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Hard cap for the whole run. `None` preserves legacy plan-timeout fallback.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hard_run_secs: Option<u64>,

    /// Default timeout for a task that has no authored override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_attempt_secs: Option<u64>,

    /// Default timeout for an owned gate effect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_effect_secs: Option<u64>,

    /// Maximum silence for an active agent attempt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_silence_secs: Option<u64>,

    /// Maximum interval without a durable scheduler milestone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduler_no_progress_secs: Option<u64>,

    /// Agent dispatch wall-clock timeout (seconds).
    #[serde(default = "default_agent_dispatch_secs")]
    pub agent_dispatch_secs: u64,

    /// Gate: compile step timeout (seconds).
    #[serde(default = "default_gate_compile_secs")]
    pub gate_compile_secs: u64,

    /// Gate: test step timeout (seconds).
    #[serde(default = "default_gate_test_secs")]
    pub gate_test_secs: u64,

    /// Gate: clippy step timeout (seconds).
    #[serde(default = "default_gate_clippy_secs")]
    pub gate_clippy_secs: u64,

    /// Single LLM call timeout (seconds).
    #[serde(default = "default_llm_call_secs")]
    pub llm_call_secs: u64,

    /// HTTP request timeout (seconds).
    #[serde(default = "default_http_request_secs")]
    pub http_request_secs: u64,

    /// Workspace lock acquisition timeout (seconds).
    #[serde(default = "default_workspace_lock_secs")]
    pub workspace_lock_secs: u64,

    /// Health check timeout (seconds).
    #[serde(default = "default_health_check_secs")]
    pub health_check_secs: u64,

    /// Total plan execution timeout (seconds).
    #[serde(default = "default_plan_total_secs")]
    pub plan_total_secs: u64,
}

// ── Default helpers (const fn for serde) ─────────────────────────────────

const fn default_agent_dispatch_secs() -> u64 {
    600
}
const fn default_gate_compile_secs() -> u64 {
    600
}
const fn default_gate_test_secs() -> u64 {
    900
}
const fn default_gate_clippy_secs() -> u64 {
    300
}
const fn default_llm_call_secs() -> u64 {
    120
}
const fn default_http_request_secs() -> u64 {
    30
}
const fn default_workspace_lock_secs() -> u64 {
    5
}
const fn default_health_check_secs() -> u64 {
    3
}
const fn default_plan_total_secs() -> u64 {
    3_600
}

// ── Duration accessors ───────────────────────────────────────────────────

impl TimeoutConfig {
    pub fn hard_run(&self) -> Duration {
        Duration::from_secs(self.hard_run_secs.unwrap_or(3_600).max(1))
    }

    pub fn task_attempt(&self) -> Duration {
        Duration::from_secs(self.task_attempt_secs.unwrap_or(600).max(1))
    }

    pub fn gate_effect(&self) -> Duration {
        Duration::from_secs(self.gate_effect_secs.unwrap_or(900).max(1))
    }

    pub fn agent_silence(&self) -> Duration {
        Duration::from_secs(self.agent_silence_secs.unwrap_or(180).max(1))
    }

    pub fn scheduler_no_progress(&self) -> Duration {
        Duration::from_secs(self.scheduler_no_progress_secs.unwrap_or(600).max(1))
    }

    /// Agent dispatch as [`Duration`].
    pub fn agent_dispatch(&self) -> Duration {
        Duration::from_secs(self.agent_dispatch_secs)
    }

    /// Gate compile as [`Duration`].
    pub fn gate_compile(&self) -> Duration {
        Duration::from_secs(self.gate_compile_secs)
    }

    /// Gate test as [`Duration`].
    pub fn gate_test(&self) -> Duration {
        Duration::from_secs(self.gate_test_secs)
    }

    /// Gate clippy as [`Duration`].
    pub fn gate_clippy(&self) -> Duration {
        Duration::from_secs(self.gate_clippy_secs)
    }

    /// LLM call as [`Duration`].
    pub fn llm_call(&self) -> Duration {
        Duration::from_secs(self.llm_call_secs)
    }

    /// HTTP request as [`Duration`].
    pub fn http_request(&self) -> Duration {
        Duration::from_secs(self.http_request_secs)
    }

    /// Workspace lock as [`Duration`].
    pub fn workspace_lock(&self) -> Duration {
        Duration::from_secs(self.workspace_lock_secs)
    }

    /// Health check as [`Duration`].
    pub fn health_check(&self) -> Duration {
        Duration::from_secs(self.health_check_secs)
    }

    /// Plan total as [`Duration`].
    pub fn plan_total(&self) -> Duration {
        Duration::from_secs(self.plan_total_secs)
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            hard_run_secs: None,
            task_attempt_secs: None,
            gate_effect_secs: None,
            agent_silence_secs: None,
            scheduler_no_progress_secs: None,
            agent_dispatch_secs: default_agent_dispatch_secs(),
            gate_compile_secs: default_gate_compile_secs(),
            gate_test_secs: default_gate_test_secs(),
            gate_clippy_secs: default_gate_clippy_secs(),
            llm_call_secs: default_llm_call_secs(),
            http_request_secs: default_http_request_secs(),
            workspace_lock_secs: default_workspace_lock_secs(),
            health_check_secs: default_health_check_secs(),
            plan_total_secs: default_plan_total_secs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sensible() {
        let cfg = TimeoutConfig::default();
        assert_eq!(cfg.agent_dispatch_secs, 600);
        assert_eq!(cfg.gate_compile_secs, 600);
        assert_eq!(cfg.gate_test_secs, 900);
        assert_eq!(cfg.gate_clippy_secs, 300);
        assert_eq!(cfg.llm_call_secs, 120);
        assert_eq!(cfg.http_request_secs, 30);
        assert_eq!(cfg.workspace_lock_secs, 5);
        assert_eq!(cfg.health_check_secs, 3);
        assert_eq!(cfg.plan_total_secs, 3_600);
    }

    #[test]
    fn duration_accessors_match_secs() {
        let cfg = TimeoutConfig::default();
        assert_eq!(cfg.hard_run(), Duration::from_secs(3_600));
        assert_eq!(cfg.task_attempt(), Duration::from_secs(600));
        assert_eq!(cfg.gate_effect(), Duration::from_secs(900));
        assert_eq!(cfg.agent_silence(), Duration::from_secs(180));
        assert_eq!(cfg.scheduler_no_progress(), Duration::from_secs(600));
        assert_eq!(cfg.agent_dispatch(), Duration::from_secs(600));
        assert_eq!(cfg.gate_compile(), Duration::from_secs(600));
        assert_eq!(cfg.gate_test(), Duration::from_secs(900));
        assert_eq!(cfg.gate_clippy(), Duration::from_secs(300));
        assert_eq!(cfg.llm_call(), Duration::from_secs(120));
        assert_eq!(cfg.http_request(), Duration::from_secs(30));
        assert_eq!(cfg.workspace_lock(), Duration::from_secs(5));
        assert_eq!(cfg.health_check(), Duration::from_secs(3));
        assert_eq!(cfg.plan_total(), Duration::from_secs(3_600));
    }

    #[test]
    fn serde_roundtrip() {
        let cfg = TimeoutConfig {
            hard_run_secs: Some(3_600),
            task_attempt_secs: Some(600),
            gate_effect_secs: Some(900),
            agent_silence_secs: Some(180),
            scheduler_no_progress_secs: Some(600),
            agent_dispatch_secs: 900,
            gate_compile_secs: 180,
            gate_test_secs: 600,
            gate_clippy_secs: 90,
            llm_call_secs: 240,
            http_request_secs: 60,
            workspace_lock_secs: 10,
            health_check_secs: 5,
            plan_total_secs: 7_200,
        };
        let toml_str = toml::to_string_pretty(&cfg).expect("serialize");
        let parsed: TimeoutConfig = toml::from_str(&toml_str).expect("deserialize");
        assert_eq!(cfg, parsed);
    }

    #[test]
    fn partial_toml_fills_defaults() {
        let toml_str = "agent_dispatch_secs = 1200\n";
        let parsed: TimeoutConfig = toml::from_str(toml_str).expect("deserialize partial");
        assert_eq!(parsed.agent_dispatch_secs, 1200);
        // Other fields should be defaults.
        assert_eq!(parsed.gate_compile_secs, 600);
        assert_eq!(parsed.plan_total_secs, 3_600);
    }

    #[test]
    fn empty_toml_yields_default() {
        let parsed: TimeoutConfig = toml::from_str("").expect("deserialize empty");
        assert_eq!(parsed, TimeoutConfig::default());
    }

    #[test]
    fn gate_timeouts_are_ordered() {
        let cfg = TimeoutConfig::default();
        // Clippy should be fastest, then compile, then test.
        assert!(cfg.gate_clippy_secs < cfg.gate_compile_secs);
        assert!(cfg.gate_compile_secs < cfg.gate_test_secs);
    }

    #[test]
    fn plan_total_exceeds_individual_timeouts() {
        let cfg = TimeoutConfig::default();
        assert!(cfg.plan_total_secs > cfg.agent_dispatch_secs);
        assert!(cfg.plan_total_secs > cfg.gate_test_secs);
    }
}
