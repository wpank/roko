//! Reusable agent-session policy and resume metadata.
//!
//! These types keep warm-agent reuse explicit. A warmed instance may only be
//! selected for a new task when the caller supplies a matching request; old
//! prompts or context fingerprints are never implicitly carried forward.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use roko_core::ContentHash;
use serde::{Deserialize, Serialize};

/// Scope in which a warmed agent/session may be reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReuseScope {
    /// Reuse is disabled.
    Disabled,
    /// Reuse is valid only for the exact task id.
    Task,
    /// Reuse is valid within the same plan id.
    Plan,
    /// Reuse is valid for a caller-defined session id.
    Session,
}

/// Opt-in policy attached to a warm agent entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarmReusePolicy {
    /// Stable policy id for audit/debugging.
    pub policy_id: String,
    /// The maximum scope where reuse is allowed.
    pub scope: ReuseScope,
    /// Optional maximum idle age before this warm entry is stale.
    pub max_idle_ms: Option<u64>,
    /// Plan id this warmed session is bound to.
    pub plan_id: Option<String>,
    /// Task id this warmed session is bound to.
    pub task_id: Option<String>,
    /// Caller-defined session id this warmed session is bound to.
    pub session_id: Option<String>,
    /// Prompt-policy fingerprint used when the session was warmed.
    pub prompt_policy_fingerprint: Option<String>,
    /// Context fingerprint used when the session was warmed.
    pub context_fingerprint: Option<String>,
    /// Whether context from the warmed session may carry into the next task.
    pub allow_context_carryover: bool,
}

impl WarmReusePolicy {
    /// A policy for stateless warm entries that carry no context.
    #[must_use]
    pub fn stateless(policy_id: impl Into<String>) -> Self {
        Self {
            policy_id: policy_id.into(),
            scope: ReuseScope::Session,
            max_idle_ms: None,
            plan_id: None,
            task_id: None,
            session_id: None,
            prompt_policy_fingerprint: None,
            context_fingerprint: None,
            allow_context_carryover: false,
        }
    }

    /// A disabled policy. This is useful for preserving legacy warm-pool API
    /// behavior while making production reuse call a checked path.
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            policy_id: "disabled".to_string(),
            scope: ReuseScope::Disabled,
            max_idle_ms: Some(0),
            plan_id: None,
            task_id: None,
            session_id: None,
            prompt_policy_fingerprint: None,
            context_fingerprint: None,
            allow_context_carryover: false,
        }
    }

    /// Bind this policy to a plan.
    #[must_use]
    pub fn for_plan(mut self, plan_id: impl Into<String>) -> Self {
        self.plan_id = Some(plan_id.into());
        self
    }

    /// Bind this policy to a task.
    #[must_use]
    pub fn for_task(mut self, task_id: impl Into<String>) -> Self {
        self.task_id = Some(task_id.into());
        self
    }

    /// Bind this policy to a session.
    #[must_use]
    pub fn for_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Attach prompt and context fingerprints.
    #[must_use]
    pub fn with_fingerprints(
        mut self,
        prompt_policy_fingerprint: Option<String>,
        context_fingerprint: Option<String>,
    ) -> Self {
        self.prompt_policy_fingerprint = prompt_policy_fingerprint;
        self.context_fingerprint = context_fingerprint;
        self
    }

    /// Set the maximum idle age for this policy.
    #[must_use]
    pub fn with_max_idle(mut self, max_idle: Duration) -> Self {
        self.max_idle_ms = Some(u64::try_from(max_idle.as_millis()).unwrap_or(u64::MAX));
        self
    }

    /// Allow explicit context carryover.
    #[must_use]
    pub fn allow_context_carryover(mut self, allow: bool) -> Self {
        self.allow_context_carryover = allow;
        self
    }

    /// Validate whether this policy can satisfy `request` at `now`.
    #[must_use]
    pub fn allows(&self, request: &WarmReuseRequest, warmed_at: Instant, now: Instant) -> bool {
        if self.scope == ReuseScope::Disabled {
            return false;
        }
        if let Some(max_idle_ms) = self.max_idle_ms {
            let age_ms =
                u64::try_from(now.duration_since(warmed_at).as_millis()).unwrap_or(u64::MAX);
            if age_ms > max_idle_ms {
                return false;
            }
        }
        if self.prompt_policy_fingerprint != request.prompt_policy_fingerprint {
            return false;
        }
        if self.context_fingerprint != request.context_fingerprint {
            return false;
        }
        if request.context_fingerprint.is_some() && !self.allow_context_carryover {
            return false;
        }

        match self.scope {
            ReuseScope::Disabled => false,
            ReuseScope::Task => {
                self.plan_id.as_deref() == request.plan_id.as_deref()
                    && self.task_id.as_deref() == request.task_id.as_deref()
            }
            ReuseScope::Plan => self.plan_id.as_deref() == request.plan_id.as_deref(),
            ReuseScope::Session => self.session_id.as_deref() == request.session_id.as_deref(),
        }
    }
}

/// Request supplied by the scheduler when selecting a warm entry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WarmReuseRequest {
    /// Plan id for scope checks.
    pub plan_id: Option<String>,
    /// Task id for scope checks.
    pub task_id: Option<String>,
    /// Caller-defined session id for scope checks.
    pub session_id: Option<String>,
    /// Required prompt-policy fingerprint.
    pub prompt_policy_fingerprint: Option<String>,
    /// Required context fingerprint.
    pub context_fingerprint: Option<String>,
}

impl WarmReuseRequest {
    /// Build a request scoped to one task.
    #[must_use]
    pub fn task(plan_id: impl Into<String>, task_id: impl Into<String>) -> Self {
        Self {
            plan_id: Some(plan_id.into()),
            task_id: Some(task_id.into()),
            ..Default::default()
        }
    }

    /// Build a request scoped to one session.
    #[must_use]
    pub fn session(session_id: impl Into<String>) -> Self {
        Self {
            session_id: Some(session_id.into()),
            ..Default::default()
        }
    }

    /// Attach required fingerprints.
    #[must_use]
    pub fn with_fingerprints(
        mut self,
        prompt_policy_fingerprint: Option<String>,
        context_fingerprint: Option<String>,
    ) -> Self {
        self.prompt_policy_fingerprint = prompt_policy_fingerprint;
        self.context_fingerprint = context_fingerprint;
        self
    }
}

/// Persistable metadata for one agent invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentInvocationSession {
    /// Unique invocation id.
    pub invocation_id: String,
    /// Optional provider/native session id.
    pub provider_session_id: Option<String>,
    /// Backend id, for example `claude_cli` or `openai_compat`.
    pub backend_id: String,
    /// Model slug/key used for the invocation.
    pub model: String,
    /// Role label used for the invocation.
    pub role: String,
    /// Plan id, if this invocation belongs to a plan.
    pub plan_id: Option<String>,
    /// Task id, if this invocation belongs to a task.
    pub task_id: Option<String>,
    /// Prompt fingerprint for resume/reuse validation.
    pub prompt_fingerprint: String,
    /// Context fingerprint for resume/reuse validation.
    pub context_fingerprint: Option<String>,
    /// Warm reuse policy used for the invocation.
    pub reuse_policy: WarmReusePolicy,
    /// Working directory for the invocation.
    pub working_dir: Option<PathBuf>,
    /// Unix milliseconds when the invocation started.
    pub started_at_ms: u64,
    /// Unix milliseconds when the invocation ended.
    pub ended_at_ms: Option<u64>,
    /// Timeout configured for the invocation.
    pub timeout_ms: Option<u64>,
    /// Final state, if known.
    pub state: InvocationState,
}

/// Durable invocation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvocationState {
    /// Invocation is running or was interrupted before a terminal update.
    InProgress,
    /// Invocation completed successfully.
    Succeeded,
    /// Invocation failed.
    Failed,
    /// Invocation timed out and should not be treated as success.
    TimedOut,
    /// Invocation was cancelled.
    Cancelled,
}

impl InvocationState {
    /// Whether the state is terminal.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::TimedOut | Self::Cancelled
        )
    }

    /// Whether resume can be attempted from this state.
    #[must_use]
    pub const fn is_resumable(self) -> bool {
        matches!(self, Self::InProgress | Self::TimedOut | Self::Cancelled)
    }
}

/// Validate that persisted invocation metadata matches a requested resume.
///
/// This fails closed on model/backend/prompt/context mismatches.
pub fn validate_resume_request(
    persisted: &AgentInvocationSession,
    requested: &AgentInvocationSession,
) -> Result<(), ResumeValidationError> {
    if !persisted.state.is_resumable() {
        return Err(ResumeValidationError::TerminalState(persisted.state));
    }
    if persisted.backend_id != requested.backend_id {
        return Err(ResumeValidationError::BackendMismatch);
    }
    if persisted.model != requested.model {
        return Err(ResumeValidationError::ModelMismatch);
    }
    if persisted.role != requested.role {
        return Err(ResumeValidationError::RoleMismatch);
    }
    if persisted.plan_id != requested.plan_id || persisted.task_id != requested.task_id {
        return Err(ResumeValidationError::ScopeMismatch);
    }
    if persisted.prompt_fingerprint != requested.prompt_fingerprint {
        return Err(ResumeValidationError::PromptMismatch);
    }
    if persisted.context_fingerprint != requested.context_fingerprint {
        return Err(ResumeValidationError::ContextMismatch);
    }
    Ok(())
}

/// Resume validation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ResumeValidationError {
    /// Persisted state is terminal and should not be resumed.
    #[error("invocation already reached terminal state {0:?}")]
    TerminalState(InvocationState),
    /// Backend changed.
    #[error("backend mismatch")]
    BackendMismatch,
    /// Model changed.
    #[error("model mismatch")]
    ModelMismatch,
    /// Role changed.
    #[error("role mismatch")]
    RoleMismatch,
    /// Plan/task scope changed.
    #[error("scope mismatch")]
    ScopeMismatch,
    /// Prompt fingerprint changed.
    #[error("prompt fingerprint mismatch")]
    PromptMismatch,
    /// Context fingerprint changed.
    #[error("context fingerprint mismatch")]
    ContextMismatch,
}

/// Stable BLAKE3 fingerprint for prompt/context policy material.
#[must_use]
pub fn fingerprint_text(text: &str) -> String {
    ContentHash::of(text.as_bytes()).to_hex()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    // ── WarmReusePolicy::stateless() constructor ────────────────────────

    #[test]
    fn stateless_policy_has_session_scope() {
        let p = WarmReusePolicy::stateless("test-policy");
        assert_eq!(p.scope, ReuseScope::Session);
    }

    #[test]
    fn stateless_policy_stores_policy_id() {
        let p = WarmReusePolicy::stateless("my-id");
        assert_eq!(p.policy_id, "my-id");
    }

    #[test]
    fn stateless_policy_has_no_bindings() {
        let p = WarmReusePolicy::stateless("clean");
        assert!(p.plan_id.is_none());
        assert!(p.task_id.is_none());
        assert!(p.session_id.is_none());
        assert!(p.max_idle_ms.is_none());
        assert!(p.prompt_policy_fingerprint.is_none());
        assert!(p.context_fingerprint.is_none());
        assert!(!p.allow_context_carryover);
    }

    // ── WarmReusePolicy::for_plan() builder chain ───────────────────────

    #[test]
    fn for_plan_binds_plan_id() {
        let p = WarmReusePolicy::stateless("p1").for_plan("plan-42");
        assert_eq!(p.plan_id.as_deref(), Some("plan-42"));
    }

    #[test]
    fn for_plan_preserves_other_fields() {
        let p = WarmReusePolicy::stateless("p2")
            .for_plan("plan-7")
            .for_task("task-3");
        assert_eq!(p.policy_id, "p2");
        assert_eq!(p.plan_id.as_deref(), Some("plan-7"));
        assert_eq!(p.task_id.as_deref(), Some("task-3"));
        assert_eq!(p.scope, ReuseScope::Session);
    }

    #[test]
    fn for_plan_chain_replaces_plan_id() {
        let p = WarmReusePolicy::stateless("p3")
            .for_plan("old-plan")
            .for_plan("new-plan");
        assert_eq!(p.plan_id.as_deref(), Some("new-plan"));
    }

    #[test]
    fn for_session_binds_session_id() {
        let p = WarmReusePolicy::stateless("s1").for_session("sess-abc");
        assert_eq!(p.session_id.as_deref(), Some("sess-abc"));
    }

    #[test]
    fn full_builder_chain() {
        let p = WarmReusePolicy::stateless("full")
            .for_plan("p1")
            .for_task("t1")
            .for_session("s1")
            .with_max_idle(Duration::from_secs(60))
            .allow_context_carryover(true)
            .with_fingerprints(Some("fp-prompt".into()), Some("fp-ctx".into()));
        assert_eq!(p.plan_id.as_deref(), Some("p1"));
        assert_eq!(p.task_id.as_deref(), Some("t1"));
        assert_eq!(p.session_id.as_deref(), Some("s1"));
        assert_eq!(p.max_idle_ms, Some(60_000));
        assert!(p.allow_context_carryover);
        assert_eq!(p.prompt_policy_fingerprint.as_deref(), Some("fp-prompt"));
        assert_eq!(p.context_fingerprint.as_deref(), Some("fp-ctx"));
    }

    // ── WarmReusePolicy::allows() reuse permission logic ────────────────

    fn now_pair() -> (Instant, Instant) {
        let t = Instant::now();
        (t, t)
    }

    #[test]
    fn disabled_policy_always_denies() {
        let p = WarmReusePolicy::disabled();
        let req = WarmReuseRequest::default();
        let (warmed, now) = now_pair();
        assert!(!p.allows(&req, warmed, now));
    }

    #[test]
    fn session_scope_matches_same_session() {
        let p = WarmReusePolicy::stateless("s").for_session("sess-1");
        let req = WarmReuseRequest::session("sess-1");
        let (warmed, now) = now_pair();
        assert!(p.allows(&req, warmed, now));
    }

    #[test]
    fn session_scope_rejects_different_session() {
        let p = WarmReusePolicy::stateless("s").for_session("sess-1");
        let req = WarmReuseRequest::session("sess-2");
        let (warmed, now) = now_pair();
        assert!(!p.allows(&req, warmed, now));
    }

    #[test]
    fn plan_scope_matches_same_plan() {
        let mut p = WarmReusePolicy::stateless("pl");
        p.scope = ReuseScope::Plan;
        p.plan_id = Some("plan-42".into());
        let req = WarmReuseRequest {
            plan_id: Some("plan-42".into()),
            ..Default::default()
        };
        let (warmed, now) = now_pair();
        assert!(p.allows(&req, warmed, now));
    }

    #[test]
    fn plan_scope_rejects_different_plan() {
        let mut p = WarmReusePolicy::stateless("pl");
        p.scope = ReuseScope::Plan;
        p.plan_id = Some("plan-42".into());
        let req = WarmReuseRequest {
            plan_id: Some("plan-99".into()),
            ..Default::default()
        };
        let (warmed, now) = now_pair();
        assert!(!p.allows(&req, warmed, now));
    }

    #[test]
    fn task_scope_requires_both_plan_and_task_match() {
        let mut p = WarmReusePolicy::stateless("tk");
        p.scope = ReuseScope::Task;
        p.plan_id = Some("p1".into());
        p.task_id = Some("t1".into());

        // Both match -> allowed
        let req_ok = WarmReuseRequest::task("p1", "t1");
        let (warmed, now) = now_pair();
        assert!(p.allows(&req_ok, warmed, now));

        // Task mismatch -> denied
        let req_bad_task = WarmReuseRequest::task("p1", "t2");
        assert!(!p.allows(&req_bad_task, warmed, now));

        // Plan mismatch -> denied
        let req_bad_plan = WarmReuseRequest::task("p2", "t1");
        assert!(!p.allows(&req_bad_plan, warmed, now));
    }

    #[test]
    fn expired_idle_denies_reuse() {
        let p = WarmReusePolicy::stateless("exp")
            .for_session("s")
            .with_max_idle(Duration::from_millis(100));
        let req = WarmReuseRequest::session("s");
        let warmed = Instant::now();
        let now = warmed + Duration::from_millis(200);
        assert!(!p.allows(&req, warmed, now));
    }

    #[test]
    fn within_idle_allows_reuse() {
        let p = WarmReusePolicy::stateless("fresh")
            .for_session("s")
            .with_max_idle(Duration::from_secs(60));
        let req = WarmReuseRequest::session("s");
        let warmed = Instant::now();
        let now = warmed + Duration::from_millis(10);
        assert!(p.allows(&req, warmed, now));
    }

    #[test]
    fn prompt_fingerprint_mismatch_denies() {
        let p = WarmReusePolicy::stateless("fp")
            .for_session("s")
            .with_fingerprints(Some("abc".into()), None);
        let req = WarmReuseRequest::session("s").with_fingerprints(Some("xyz".into()), None);
        let (warmed, now) = now_pair();
        assert!(!p.allows(&req, warmed, now));
    }

    #[test]
    fn context_fingerprint_requires_carryover_flag() {
        // Policy has context fingerprint but carryover disabled
        let p = WarmReusePolicy::stateless("ctx")
            .for_session("s")
            .with_fingerprints(None, Some("ctx-fp".into()))
            .allow_context_carryover(false);
        let req = WarmReuseRequest::session("s").with_fingerprints(None, Some("ctx-fp".into()));
        let (warmed, now) = now_pair();
        assert!(!p.allows(&req, warmed, now));

        // Same but with carryover enabled -> allowed
        let p2 = WarmReusePolicy::stateless("ctx")
            .for_session("s")
            .with_fingerprints(None, Some("ctx-fp".into()))
            .allow_context_carryover(true);
        assert!(p2.allows(&req, warmed, now));
    }

    #[test]
    fn no_fingerprints_match_none_to_none() {
        let p = WarmReusePolicy::stateless("none").for_session("s");
        let req = WarmReuseRequest::session("s");
        let (warmed, now) = now_pair();
        assert!(p.allows(&req, warmed, now));
    }
}
