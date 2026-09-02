//! Provider-change tracking and attempt-scoped attribution (T009).
//!
//! When a provider fallback, rotation, or rate-limit switch occurs during
//! a run, a [`ProviderChangeEvent`] captures the transition. Each distinct
//! provider attempt gets a scoped [`AttemptAttribution`] that ties token
//! usage and tool results back to the attempt they originated from.
//!
//! This replaces implicit "last provider wins" accounting with explicit
//! per-attempt cost and result attribution.

use serde::{Deserialize, Serialize};

/// Reason a provider change occurred during a run.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderChangeReason {
    /// The previous provider returned a rate-limit error.
    RateLimited,
    /// The previous provider returned a transient error and exhausted retries.
    RetriesExhausted,
    /// The previous provider timed out (TTFT or total request).
    Timeout,
    /// The previous provider returned a non-retryable error.
    ProviderError,
    /// Cascade routing selected a different provider for this turn.
    CascadeRouting,
    /// Manual override or configuration change.
    ManualOverride,
    /// The model/provider was rotated for load balancing.
    LoadBalancing,
    /// Provider health check marked the previous provider unhealthy.
    HealthCheck,
}

impl ProviderChangeReason {
    /// Stable string tag for logs and metrics.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::RateLimited => "rate_limited",
            Self::RetriesExhausted => "retries_exhausted",
            Self::Timeout => "timeout",
            Self::ProviderError => "provider_error",
            Self::CascadeRouting => "cascade_routing",
            Self::ManualOverride => "manual_override",
            Self::LoadBalancing => "load_balancing",
            Self::HealthCheck => "health_check",
        }
    }
}

/// Records a provider transition during a run.
///
/// Emitted when the active provider changes (fallback, rotation, etc.).
/// Downstream consumers use this for cost attribution, telemetry, and
/// understanding why a particular provider was used for part of a run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderChangeEvent {
    /// Provider name before the change (`None` for the initial assignment).
    pub from_provider: Option<String>,
    /// Provider name after the change.
    pub to_provider: String,
    /// Model slug before the change (`None` for the initial assignment).
    pub from_model: Option<String>,
    /// Model slug after the change.
    pub to_model: String,
    /// Why the change happened.
    pub reason: ProviderChangeReason,
    /// The attempt ID for the new provider session.
    pub attempt_id: String,
    /// Unix-millisecond timestamp when the change occurred.
    pub timestamp_ms: i64,
}

/// Per-attempt attribution for usage and results.
///
/// When a provider changes mid-run, each attempt captures its own
/// slice of the work. This lets cost accounting and error attribution
/// be precise rather than lumping everything under the final provider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttemptAttribution {
    /// Unique attempt identifier (typically `"{run_id}:{attempt_number}"`).
    pub attempt_id: String,
    /// Provider active during this attempt.
    pub provider: String,
    /// Model active during this attempt.
    pub model: String,
    /// Input tokens consumed during this attempt.
    pub input_tokens: u64,
    /// Output tokens generated during this attempt.
    pub output_tokens: u64,
    /// Estimated cost in USD for this attempt.
    pub cost_usd: Option<f64>,
    /// Number of tool calls dispatched during this attempt.
    pub tool_calls: u32,
    /// Number of turns completed during this attempt.
    pub turns: u32,
    /// Whether this attempt ended in error.
    pub errored: bool,
    /// Unix-millisecond timestamp when this attempt started.
    pub started_ms: i64,
    /// Unix-millisecond timestamp when this attempt ended.
    pub ended_ms: Option<i64>,
}

impl AttemptAttribution {
    /// Create a new attribution record for an attempt starting now.
    #[must_use]
    pub fn new(
        attempt_id: impl Into<String>,
        provider: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            attempt_id: attempt_id.into(),
            provider: provider.into(),
            model: model.into(),
            input_tokens: 0,
            output_tokens: 0,
            cost_usd: None,
            tool_calls: 0,
            turns: 0,
            errored: false,
            started_ms: now,
            ended_ms: None,
        }
    }

    /// Record token usage for this attempt.
    pub fn record_usage(&mut self, input: u64, output: u64) {
        self.input_tokens = self.input_tokens.saturating_add(input);
        self.output_tokens = self.output_tokens.saturating_add(output);
    }

    /// Record a completed tool call.
    pub fn record_tool_call(&mut self) {
        self.tool_calls = self.tool_calls.saturating_add(1);
    }

    /// Record a completed turn.
    pub fn record_turn(&mut self) {
        self.turns = self.turns.saturating_add(1);
    }

    /// Mark this attempt as ended (optionally with error).
    pub fn finish(&mut self, errored: bool) {
        self.errored = errored;
        self.ended_ms = Some(chrono::Utc::now().timestamp_millis());
    }

    /// Wall-clock duration of this attempt in milliseconds, if finished.
    #[must_use]
    pub fn duration_ms(&self) -> Option<i64> {
        self.ended_ms.map(|end| end - self.started_ms)
    }
}

/// Tracks provider changes and attempt attributions across a run.
///
/// Each provider transition creates a new attempt; all usage and tool
/// calls during that attempt are attributed to it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderChangeTracker {
    /// History of provider changes in chronological order.
    pub changes: Vec<ProviderChangeEvent>,
    /// Per-attempt attribution records. The last entry is the current attempt.
    pub attempts: Vec<AttemptAttribution>,
}

impl ProviderChangeTracker {
    /// Create a new tracker, starting with the initial provider.
    #[must_use]
    pub fn new(provider: impl Into<String>, model: impl Into<String>, run_id: &str) -> Self {
        let attempt_id = format!("{run_id}:0");
        let attribution = AttemptAttribution::new(attempt_id, provider, model);
        Self {
            changes: Vec::new(),
            attempts: vec![attribution],
        }
    }

    /// Record a provider change and start a new attempt.
    pub fn record_change(
        &mut self,
        to_provider: impl Into<String>,
        to_model: impl Into<String>,
        reason: ProviderChangeReason,
        run_id: &str,
    ) {
        let to_provider = to_provider.into();
        let to_model = to_model.into();
        let attempt_number = self.attempts.len();
        let attempt_id = format!("{run_id}:{attempt_number}");

        // Close the current attempt.
        if let Some(current) = self.attempts.last_mut() {
            current.finish(matches!(
                reason,
                ProviderChangeReason::ProviderError
                    | ProviderChangeReason::RetriesExhausted
                    | ProviderChangeReason::Timeout
            ));
        }

        let (from_provider, from_model) = self
            .attempts
            .last()
            .map(|a| (Some(a.provider.clone()), Some(a.model.clone())))
            .unwrap_or((None, None));

        let now = chrono::Utc::now().timestamp_millis();
        self.changes.push(ProviderChangeEvent {
            from_provider,
            to_provider: to_provider.clone(),
            from_model,
            to_model: to_model.clone(),
            reason,
            attempt_id: attempt_id.clone(),
            timestamp_ms: now,
        });

        self.attempts
            .push(AttemptAttribution::new(attempt_id, to_provider, to_model));
    }

    /// Get the current (most recent) attempt attribution.
    #[must_use]
    pub fn current_attempt(&self) -> Option<&AttemptAttribution> {
        self.attempts.last()
    }

    /// Get a mutable reference to the current attempt.
    pub fn current_attempt_mut(&mut self) -> Option<&mut AttemptAttribution> {
        self.attempts.last_mut()
    }

    /// Number of provider changes that have occurred.
    #[must_use]
    pub fn change_count(&self) -> usize {
        self.changes.len()
    }

    /// Total token usage across all attempts.
    #[must_use]
    pub fn total_usage(&self) -> (u64, u64) {
        let input: u64 = self.attempts.iter().map(|a| a.input_tokens).sum();
        let output: u64 = self.attempts.iter().map(|a| a.output_tokens).sum();
        (input, output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracker_starts_with_initial_attempt() {
        let tracker = ProviderChangeTracker::new("anthropic", "claude-opus-4-6", "run-1");
        assert_eq!(tracker.change_count(), 0);
        assert_eq!(tracker.attempts.len(), 1);

        let attempt = tracker.current_attempt().unwrap();
        assert_eq!(attempt.provider, "anthropic");
        assert_eq!(attempt.model, "claude-opus-4-6");
        assert_eq!(attempt.attempt_id, "run-1:0");
    }

    #[test]
    fn record_change_creates_new_attempt() {
        let mut tracker = ProviderChangeTracker::new("anthropic", "claude-opus-4-6", "run-1");

        tracker.record_change(
            "openai",
            "gpt-5",
            ProviderChangeReason::RateLimited,
            "run-1",
        );

        assert_eq!(tracker.change_count(), 1);
        assert_eq!(tracker.attempts.len(), 2);

        let current = tracker.current_attempt().unwrap();
        assert_eq!(current.provider, "openai");
        assert_eq!(current.model, "gpt-5");
        assert_eq!(current.attempt_id, "run-1:1");

        // The first attempt should be finished (but not errored, since rate-limiting
        // is not an error on the attempt itself).
        let first = &tracker.attempts[0];
        assert!(first.ended_ms.is_some());
    }

    #[test]
    fn attempt_attribution_records_usage() {
        let mut attr = AttemptAttribution::new("run-1:0", "anthropic", "claude-opus-4-6");
        attr.record_usage(100, 50);
        attr.record_usage(200, 100);
        attr.record_tool_call();
        attr.record_tool_call();
        attr.record_turn();

        assert_eq!(attr.input_tokens, 300);
        assert_eq!(attr.output_tokens, 150);
        assert_eq!(attr.tool_calls, 2);
        assert_eq!(attr.turns, 1);
    }

    #[test]
    fn total_usage_sums_across_attempts() {
        let mut tracker = ProviderChangeTracker::new("anthropic", "claude-opus-4-6", "run-1");
        tracker.current_attempt_mut().unwrap().record_usage(100, 50);

        tracker.record_change("openai", "gpt-5", ProviderChangeReason::Timeout, "run-1");
        tracker
            .current_attempt_mut()
            .unwrap()
            .record_usage(200, 100);

        let (input, output) = tracker.total_usage();
        assert_eq!(input, 300);
        assert_eq!(output, 150);
    }

    #[test]
    fn provider_change_event_serde_roundtrip() {
        let event = ProviderChangeEvent {
            from_provider: Some("anthropic".into()),
            to_provider: "openai".into(),
            from_model: Some("claude-opus-4-6".into()),
            to_model: "gpt-5".into(),
            reason: ProviderChangeReason::RateLimited,
            attempt_id: "run-1:1".into(),
            timestamp_ms: 1_700_000_000_000,
        };
        let json = serde_json::to_string(&event).unwrap();
        let decoded: ProviderChangeEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, event);
    }

    #[test]
    fn attempt_attribution_serde_roundtrip() {
        let mut attr = AttemptAttribution::new("run-1:0", "anthropic", "claude-opus-4-6");
        attr.record_usage(500, 200);
        attr.record_tool_call();
        attr.finish(false);

        let json = serde_json::to_string(&attr).unwrap();
        let decoded: AttemptAttribution = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, attr);
    }

    #[test]
    fn provider_change_reason_as_str() {
        assert_eq!(ProviderChangeReason::RateLimited.as_str(), "rate_limited");
        assert_eq!(ProviderChangeReason::Timeout.as_str(), "timeout");
        assert_eq!(
            ProviderChangeReason::CascadeRouting.as_str(),
            "cascade_routing"
        );
        assert_eq!(
            ProviderChangeReason::ManualOverride.as_str(),
            "manual_override"
        );
    }

    #[test]
    fn error_attempts_are_marked_errored() {
        let mut tracker = ProviderChangeTracker::new("anthropic", "claude-opus-4-6", "run-1");
        tracker.record_change(
            "openai",
            "gpt-5",
            ProviderChangeReason::ProviderError,
            "run-1",
        );

        // The first attempt was closed with errored=true because the reason was ProviderError.
        assert!(tracker.attempts[0].errored);
    }

    #[test]
    fn change_event_captures_from_provider() {
        let mut tracker = ProviderChangeTracker::new("anthropic", "claude-opus-4-6", "run-1");
        tracker.record_change(
            "openai",
            "gpt-5",
            ProviderChangeReason::RateLimited,
            "run-1",
        );

        let change = &tracker.changes[0];
        assert_eq!(change.from_provider.as_deref(), Some("anthropic"));
        assert_eq!(change.from_model.as_deref(), Some("claude-opus-4-6"));
        assert_eq!(change.to_provider, "openai");
        assert_eq!(change.to_model, "gpt-5");
    }

    #[test]
    fn attempt_duration_computed_on_finish() {
        let mut attr = AttemptAttribution::new("run-1:0", "anthropic", "claude-opus-4-6");
        assert!(attr.duration_ms().is_none());

        attr.finish(false);
        // Duration should be non-negative (could be 0 if the clock didn't advance).
        assert!(attr.duration_ms().unwrap() >= 0);
    }
}
