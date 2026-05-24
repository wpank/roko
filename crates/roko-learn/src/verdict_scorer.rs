//! STATUS: NOT WIRED -- called internally by floating code but no runtime entrypoint.
//!
//! VerdictAwareScorer — weights gate verdict signals by recency and severity (GATE-05).
//!
//! This scorer implements the `ScoreFn` trait from `roko-core` and specifically
//! targets engrams of `Kind::GateVerdict`. It assigns higher scores to:
//!
//! 1. **Recent** verdicts (exponential time decay)
//! 2. **Severe** failures (compile errors > lint warnings)
//! 3. **Relevant** verdicts (matching the current task context)
//!
//! Non-verdict engrams receive a neutral score so the scorer composes cleanly
//! with other scorers in a `SumScorer` or `MulScorer` chain.

use roko_core::traits::Score as ScoreFn;
use roko_core::{Context, Kind, Score, Signal};

/// Weights for verdict scoring dimensions.
#[derive(Debug, Clone)]
pub struct VerdictScorerConfig {
    /// Half-life for recency decay in milliseconds (default: 10 minutes).
    pub recency_half_life_ms: f64,
    /// Weight given to recency dimension [0, 1] (default: 0.35).
    pub recency_weight: f32,
    /// Weight given to severity dimension [0, 1] (default: 0.40).
    pub severity_weight: f32,
    /// Weight given to relevance dimension [0, 1] (default: 0.25).
    pub relevance_weight: f32,
}

impl Default for VerdictScorerConfig {
    fn default() -> Self {
        Self {
            recency_half_life_ms: 600_000.0, // 10 minutes
            recency_weight: 0.35,
            severity_weight: 0.40,
            relevance_weight: 0.25,
        }
    }
}

/// ScoreFn that weights `Kind::GateVerdict` engrams by recency, severity, and relevance.
///
/// Designed to be composed with other scorers. Non-verdict engrams receive
/// `Score::ZERO` so they don't interfere in aggregate pipelines.
pub struct VerdictAwareScorer {
    config: VerdictScorerConfig,
}

impl VerdictAwareScorer {
    /// Create a scorer with default weights.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: VerdictScorerConfig::default(),
        }
    }

    /// Create a scorer with custom configuration.
    #[must_use]
    pub fn with_config(config: VerdictScorerConfig) -> Self {
        Self { config }
    }

    /// Compute recency factor using exponential decay.
    ///
    /// Returns 1.0 for current signals, decaying toward 0.0 with the
    /// configured half-life.
    fn recency_factor(&self, signal_ts_ms: i64, now_ms: i64) -> f32 {
        let age_ms = (now_ms - signal_ts_ms).max(0) as f64;
        let decay = (-age_ms * (2.0_f64.ln()) / self.config.recency_half_life_ms).exp();
        decay as f32
    }

    /// Compute severity factor from verdict metadata.
    ///
    /// Severity ordering: compile error (1.0) > test failure (0.8) >
    /// lint warning (0.5) > pass (0.1).
    fn severity_factor(&self, signal: &Signal) -> f32 {
        let gate_name = signal.tag("gate").unwrap_or("");

        let passed = signal
            .tag("verdict_passed")
            .map(|v| v == "true")
            .unwrap_or(true);

        if passed {
            return 0.1; // Passing verdicts have low severity signal.
        }

        // Severity by gate type (higher = more severe).
        match gate_name {
            g if g.contains("compile") => 1.0,
            g if g.contains("test") => 0.8,
            g if g.contains("lint") || g.contains("clippy") => 0.5,
            g if g.contains("diff") => 0.3,
            _ => 0.6, // Unknown gate — moderate severity.
        }
    }

    /// Compute relevance factor by matching tags against current context.
    ///
    /// Checks for matching `task_type`, `crate`, and `task_category` tags
    /// between the verdict engram and the current context.
    fn relevance_factor(&self, signal: &Signal, ctx: &Context) -> f32 {
        let mut relevance: f32 = 0.0;
        let mut checks: f32 = 0.0;

        // Match task_type.
        if let Some(ctx_task_type) = ctx.attr("roko.task_type") {
            checks += 1.0;
            if signal.tag("task_type") == Some(ctx_task_type) {
                relevance += 1.0;
            }
        }

        // Match crate.
        if let Some(ctx_crate) = ctx.attr("roko.crate") {
            checks += 1.0;
            if signal.tag("crate") == Some(ctx_crate) {
                relevance += 1.0;
            }
        }

        // Match task_category.
        if let Some(ctx_category) = ctx.attr("roko.task_category") {
            checks += 1.0;
            if signal.tag("task_category") == Some(ctx_category) {
                relevance += 1.0;
            }
        }

        if checks == 0.0 {
            return 0.5; // No context to match — neutral relevance.
        }
        relevance / checks
    }
}

impl Default for VerdictAwareScorer {
    fn default() -> Self {
        Self::new()
    }
}

impl ScoreFn for VerdictAwareScorer {
    fn score(&self, signal: &Signal, ctx: &Context) -> Score {
        // Only score GateVerdict engrams.
        if signal.kind != Kind::GateVerdict {
            return Score::ZERO;
        }

        let now_ms = ctx.now_ms;
        let recency = self.recency_factor(signal.created_at_ms, now_ms);
        let severity = self.severity_factor(signal);
        let relevance = self.relevance_factor(signal, ctx);

        // Weighted combination.
        let salience = recency * self.config.recency_weight
            + severity * self.config.severity_weight
            + relevance * self.config.relevance_weight;

        Score {
            confidence: recency,
            novelty: if severity > 0.5 { severity * 0.5 } else { 0.0 },
            utility: salience,
            reputation: 1.0, // Verdicts come from our own gates — trusted.
            precision: relevance,
            salience,
            coherence: if severity > 0.5 { 0.8 } else { 0.5 },
        }
    }

    fn name(&self) -> &'static str {
        "verdict_aware_scorer"
    }
}

// ─── Verdict history for CascadeRouter integration ───────────────────────────

/// A recorded verdict observation for verdict-aware routing.
#[derive(Debug, Clone)]
pub struct VerdictRecord {
    /// Which model produced the code that triggered this verdict.
    pub model_slug: String,
    /// Task type (e.g., "implement", "fix", "refactor").
    pub task_type: String,
    /// Target crate.
    pub target_crate: String,
    /// The gate that rendered this verdict.
    pub gate: String,
    /// Whether the verdict passed.
    pub passed: bool,
    /// Unix milliseconds when this verdict was recorded.
    pub timestamp_ms: i64,
}

/// Tracks recent verdicts and computes routing penalties for models with
/// compile failure streaks.
///
/// The router queries this to adjust bandit rewards: a model with a streak
/// of >2 consecutive compile failures on a `(task_type, crate)` pair receives
/// a 0.5x reward penalty, encouraging the router to try a different model.
#[derive(Debug, Clone, Default)]
pub struct VerdictHistory {
    /// Rolling window of recent verdict records.
    records: Vec<VerdictRecord>,
    /// Maximum history size.
    max_records: usize,
}

impl VerdictHistory {
    /// Create a new history with default capacity.
    #[must_use]
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            max_records: 500,
        }
    }

    /// Create a history with custom capacity.
    #[must_use]
    pub fn with_capacity(max_records: usize) -> Self {
        Self {
            records: Vec::new(),
            max_records: max_records.max(10),
        }
    }

    /// Record a verdict observation.
    pub fn record(&mut self, record: VerdictRecord) {
        if self.records.len() >= self.max_records {
            self.records.remove(0);
        }
        self.records.push(record);
    }

    /// Count consecutive compile failures for a model on a `(task_type, crate)` pair.
    ///
    /// Scans backward from the most recent record, counting consecutive compile
    /// failures. Stops at the first non-compile or passing verdict.
    #[must_use]
    pub fn compile_failure_streak(
        &self,
        model_slug: &str,
        task_type: &str,
        target_crate: &str,
    ) -> usize {
        let mut streak = 0;
        for record in self.records.iter().rev() {
            if record.model_slug != model_slug
                || record.task_type != task_type
                || record.target_crate != target_crate
            {
                continue;
            }
            if !record.passed && record.gate.contains("compile") {
                streak += 1;
            } else {
                break;
            }
        }
        streak
    }

    /// Compute a reward penalty multiplier for a model based on verdict history.
    ///
    /// Returns 1.0 (no penalty) if the model has no concerning streaks.
    /// Returns 0.5 if the model has >2 consecutive compile failures.
    /// Returns 0.25 if the model has >5 consecutive compile failures.
    #[must_use]
    pub fn reward_penalty(&self, model_slug: &str, task_type: &str, target_crate: &str) -> f64 {
        let streak = self.compile_failure_streak(model_slug, task_type, target_crate);
        match streak {
            0..=2 => 1.0,
            3..=5 => 0.5,
            _ => 0.25,
        }
    }

    /// Return the most recent verdicts for a `(task_type, crate)` pair.
    #[must_use]
    pub fn recent_verdicts(
        &self,
        task_type: &str,
        target_crate: &str,
        limit: usize,
    ) -> Vec<&VerdictRecord> {
        self.records
            .iter()
            .rev()
            .filter(|r| r.task_type == task_type && r.target_crate == target_crate)
            .take(limit)
            .collect()
    }

    /// Return the overall failure rate for a model across all recent observations.
    #[must_use]
    pub fn model_failure_rate(&self, model_slug: &str) -> f64 {
        let model_records: Vec<&VerdictRecord> = self
            .records
            .iter()
            .filter(|r| r.model_slug == model_slug)
            .collect();

        if model_records.is_empty() {
            return 0.0;
        }

        let failures = model_records.iter().filter(|r| !r.passed).count();
        failures as f64 / model_records.len() as f64
    }

    /// Total number of recorded verdicts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the history is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::{Body, Context, Kind, Signal};

    fn verdict_engram(gate: &str, passed: bool, age_ms: i64) -> Signal {
        let now = chrono::Utc::now().timestamp_millis();
        let mut e = Signal::builder(Kind::GateVerdict)
            .body(Body::empty())
            .build();
        e.created_at_ms = now - age_ms;
        e.tags.insert("gate".to_string(), gate.to_string());
        e.tags
            .insert("verdict_passed".to_string(), passed.to_string());
        e
    }

    fn non_verdict_engram() -> Signal {
        Signal::builder(Kind::Task).body(Body::empty()).build()
    }

    fn ctx_at_now() -> Context {
        Context::at(chrono::Utc::now().timestamp_millis())
    }

    // ─── VerdictAwareScorer tests ────────────────────────────────────

    #[test]
    fn non_verdict_engrams_get_zero_score() {
        let scorer = VerdictAwareScorer::new();
        let ctx = ctx_at_now();
        let score = scorer.score(&non_verdict_engram(), &ctx);
        assert_eq!(score.utility, 0.0);
        assert_eq!(score.salience, 0.0);
    }

    #[test]
    fn recent_verdict_scores_higher_than_old() {
        let scorer = VerdictAwareScorer::new();
        let ctx = ctx_at_now();

        let recent = verdict_engram("compile", false, 1_000); // 1 second ago
        let old = verdict_engram("compile", false, 3_600_000); // 1 hour ago

        let recent_score = scorer.score(&recent, &ctx);
        let old_score = scorer.score(&old, &ctx);

        assert!(
            recent_score.salience > old_score.salience,
            "recent={} > old={}",
            recent_score.salience,
            old_score.salience,
        );
    }

    #[test]
    fn compile_failure_has_higher_severity_than_lint() {
        let scorer = VerdictAwareScorer::new();
        let ctx = ctx_at_now();

        let compile_fail = verdict_engram("compile", false, 100);
        let lint_fail = verdict_engram("clippy_lint", false, 100);

        let compile_score = scorer.score(&compile_fail, &ctx);
        let lint_score = scorer.score(&lint_fail, &ctx);

        assert!(
            compile_score.salience > lint_score.salience,
            "compile={} > lint={}",
            compile_score.salience,
            lint_score.salience,
        );
    }

    #[test]
    fn passing_verdict_has_low_severity() {
        let scorer = VerdictAwareScorer::new();
        let ctx = ctx_at_now();

        let pass = verdict_engram("compile", true, 100);
        let fail = verdict_engram("compile", false, 100);

        let pass_score = scorer.score(&pass, &ctx);
        let fail_score = scorer.score(&fail, &ctx);

        assert!(
            fail_score.salience > pass_score.salience,
            "fail={} > pass={}",
            fail_score.salience,
            pass_score.salience,
        );
    }

    #[test]
    fn scorer_name_is_correct() {
        let scorer = VerdictAwareScorer::new();
        assert_eq!(scorer.name(), "verdict_aware_scorer");
    }

    // ─── VerdictHistory tests ────────────────────────────────────────

    #[test]
    fn empty_history_returns_no_penalty() {
        let history = VerdictHistory::new();
        assert_eq!(
            history.reward_penalty("model-a", "implement", "roko-core"),
            1.0
        );
        assert!(history.is_empty());
    }

    #[test]
    fn compile_failure_streak_counted_correctly() {
        let mut history = VerdictHistory::new();
        let now = chrono::Utc::now().timestamp_millis();

        // 3 consecutive compile failures.
        for i in 0..3 {
            history.record(VerdictRecord {
                model_slug: "model-a".to_string(),
                task_type: "implement".to_string(),
                target_crate: "roko-core".to_string(),
                gate: "compile".to_string(),
                passed: false,
                timestamp_ms: now + i * 1000,
            });
        }

        assert_eq!(
            history.compile_failure_streak("model-a", "implement", "roko-core"),
            3
        );
        assert_eq!(
            history.reward_penalty("model-a", "implement", "roko-core"),
            0.5
        );
    }

    #[test]
    fn passing_verdict_breaks_streak() {
        let mut history = VerdictHistory::new();
        let now = chrono::Utc::now().timestamp_millis();

        // 2 failures, then a pass, then 1 failure.
        history.record(VerdictRecord {
            model_slug: "model-a".to_string(),
            task_type: "fix".to_string(),
            target_crate: "roko-gate".to_string(),
            gate: "compile".to_string(),
            passed: false,
            timestamp_ms: now,
        });
        history.record(VerdictRecord {
            model_slug: "model-a".to_string(),
            task_type: "fix".to_string(),
            target_crate: "roko-gate".to_string(),
            gate: "compile".to_string(),
            passed: false,
            timestamp_ms: now + 1000,
        });
        history.record(VerdictRecord {
            model_slug: "model-a".to_string(),
            task_type: "fix".to_string(),
            target_crate: "roko-gate".to_string(),
            gate: "compile".to_string(),
            passed: true,
            timestamp_ms: now + 2000,
        });
        history.record(VerdictRecord {
            model_slug: "model-a".to_string(),
            task_type: "fix".to_string(),
            target_crate: "roko-gate".to_string(),
            gate: "compile".to_string(),
            passed: false,
            timestamp_ms: now + 3000,
        });

        // Only 1 failure after the pass.
        assert_eq!(
            history.compile_failure_streak("model-a", "fix", "roko-gate"),
            1
        );
        assert_eq!(history.reward_penalty("model-a", "fix", "roko-gate"), 1.0);
    }

    #[test]
    fn different_model_not_counted_in_streak() {
        let mut history = VerdictHistory::new();
        let now = chrono::Utc::now().timestamp_millis();

        for i in 0..4 {
            history.record(VerdictRecord {
                model_slug: "model-b".to_string(),
                task_type: "implement".to_string(),
                target_crate: "roko-core".to_string(),
                gate: "compile".to_string(),
                passed: false,
                timestamp_ms: now + i * 1000,
            });
        }

        // model-a has no failures.
        assert_eq!(
            history.compile_failure_streak("model-a", "implement", "roko-core"),
            0
        );
        // model-b has 4.
        assert_eq!(
            history.compile_failure_streak("model-b", "implement", "roko-core"),
            4
        );
    }

    #[test]
    fn model_failure_rate_computed_correctly() {
        let mut history = VerdictHistory::new();
        let now = chrono::Utc::now().timestamp_millis();

        for i in 0..10 {
            history.record(VerdictRecord {
                model_slug: "model-c".to_string(),
                task_type: "implement".to_string(),
                target_crate: "roko-core".to_string(),
                gate: "compile".to_string(),
                passed: i % 2 == 0, // 50% pass rate
                timestamp_ms: now + i * 1000,
            });
        }

        let rate = history.model_failure_rate("model-c");
        assert!((rate - 0.5).abs() < 0.01);
    }

    #[test]
    fn high_streak_gets_severe_penalty() {
        let mut history = VerdictHistory::new();
        let now = chrono::Utc::now().timestamp_millis();

        for i in 0..8 {
            history.record(VerdictRecord {
                model_slug: "model-d".to_string(),
                task_type: "implement".to_string(),
                target_crate: "roko-gate".to_string(),
                gate: "compile".to_string(),
                passed: false,
                timestamp_ms: now + i * 1000,
            });
        }

        assert_eq!(
            history.reward_penalty("model-d", "implement", "roko-gate"),
            0.25
        );
    }

    #[test]
    fn recent_verdicts_returns_limited_results() {
        let mut history = VerdictHistory::new();
        let now = chrono::Utc::now().timestamp_millis();

        for i in 0..20 {
            history.record(VerdictRecord {
                model_slug: "model-e".to_string(),
                task_type: "fix".to_string(),
                target_crate: "roko-learn".to_string(),
                gate: "test".to_string(),
                passed: true,
                timestamp_ms: now + i * 1000,
            });
        }

        let recent = history.recent_verdicts("fix", "roko-learn", 5);
        assert_eq!(recent.len(), 5);
    }

    #[test]
    fn history_respects_capacity() {
        let mut history = VerdictHistory::with_capacity(10);
        let now = chrono::Utc::now().timestamp_millis();

        for i in 0..20 {
            history.record(VerdictRecord {
                model_slug: "model-f".to_string(),
                task_type: "implement".to_string(),
                target_crate: "roko-core".to_string(),
                gate: "compile".to_string(),
                passed: true,
                timestamp_ms: now + i * 1000,
            });
        }

        assert_eq!(history.len(), 10);
    }

    // ─── Additional VerdictAwareScorer tests ─────────────────────────

    #[test]
    fn all_gates_passed_yields_low_severity_scores() {
        let scorer = VerdictAwareScorer::new();
        let ctx = ctx_at_now();

        let gates = ["compile", "test", "clippy_lint", "diff_check"];
        for gate in &gates {
            let signal = verdict_engram(gate, true, 100);
            let score = scorer.score(&signal, &ctx);
            // Passing verdicts always have severity 0.1, so severity contribution
            // is 0.1 * 0.40 = 0.04. Maximum possible salience for a pass is
            // 1.0*0.35 + 0.1*0.40 + 1.0*0.25 = 0.64. With neutral relevance
            // (0.5) and recent signal: 0.35+0.04+0.125 = 0.515.
            assert!(
                score.salience < 0.65,
                "gate={gate} salience={} should be < 0.65 for a pass",
                score.salience,
            );
            // Novelty should be zero for passing verdicts (severity 0.1 <= 0.5).
            assert_eq!(
                score.novelty, 0.0,
                "gate={gate} novelty={} should be 0 for a pass",
                score.novelty,
            );
            // Coherence should be 0.5 for low-severity signals.
            assert_eq!(
                score.coherence, 0.5,
                "gate={gate} coherence={} should be 0.5 for a pass",
                score.coherence,
            );
        }
    }

    #[test]
    fn some_gates_failed_mixed_scoring() {
        let scorer = VerdictAwareScorer::new();
        let ctx = ctx_at_now();

        let compile_pass = verdict_engram("compile", true, 100);
        let test_fail = verdict_engram("test", false, 100);
        let lint_pass = verdict_engram("clippy_lint", true, 100);
        let diff_fail = verdict_engram("diff_check", false, 100);

        let pass_score = scorer.score(&compile_pass, &ctx);
        let test_score = scorer.score(&test_fail, &ctx);
        let lint_score = scorer.score(&lint_pass, &ctx);
        let diff_score = scorer.score(&diff_fail, &ctx);

        // Failed gates should score higher than passed gates (same age).
        assert!(
            test_score.salience > pass_score.salience,
            "test_fail={} > compile_pass={}",
            test_score.salience,
            pass_score.salience,
        );
        assert!(
            diff_score.salience > lint_score.salience,
            "diff_fail={} > lint_pass={}",
            diff_score.salience,
            lint_score.salience,
        );

        // Test failure (0.8 severity) should score higher than diff failure (0.3 severity).
        assert!(
            test_score.salience > diff_score.salience,
            "test_fail={} > diff_fail={}",
            test_score.salience,
            diff_score.salience,
        );
    }

    #[test]
    fn score_computation_exact_values() {
        // Use a fixed time so we can compute exact values.
        let now_ms: i64 = 1_000_000_000;
        let signal_age_ms: i64 = 0; // Created at now_ms, age = 0.
        let ctx = Context::at(now_ms);

        let mut signal = Signal::builder(Kind::GateVerdict)
            .body(Body::empty())
            .build();
        signal.created_at_ms = now_ms - signal_age_ms;
        signal
            .tags
            .insert("gate".to_string(), "compile".to_string());
        signal
            .tags
            .insert("verdict_passed".to_string(), "false".to_string());

        let config = VerdictScorerConfig::default();
        let scorer = VerdictAwareScorer::with_config(config);
        let score = scorer.score(&signal, &ctx);

        // recency: age=0 → decay = exp(0) = 1.0
        let expected_recency: f32 = 1.0;
        // severity: compile fail → 1.0
        let _expected_severity: f32 = 1.0;
        // relevance: no context attrs → 0.5
        let expected_relevance: f32 = 0.5;

        // salience = recency * 0.35 + severity * 0.40 + relevance * 0.25
        //          = 1.0 * 0.35 + 1.0 * 0.40 + 0.5 * 0.25
        //          = 0.35 + 0.40 + 0.125 = 0.875
        let expected_salience: f32 = 0.875;

        assert!(
            (score.salience - expected_salience).abs() < 1e-5,
            "salience: got={}, expected={}",
            score.salience,
            expected_salience,
        );
        assert!(
            (score.confidence - expected_recency).abs() < 1e-5,
            "confidence: got={}, expected={}",
            score.confidence,
            expected_recency,
        );
        assert!(
            (score.utility - expected_salience).abs() < 1e-5,
            "utility: got={}, expected={}",
            score.utility,
            expected_salience,
        );
        assert_eq!(
            score.reputation, 1.0,
            "verdicts are trusted, reputation=1.0"
        );
        assert!(
            (score.precision - expected_relevance).abs() < 1e-5,
            "precision: got={}, expected={}",
            score.precision,
            expected_relevance,
        );
        // severity > 0.5 → novelty = severity * 0.5 = 0.5
        assert!(
            (score.novelty - 0.5).abs() < 1e-5,
            "novelty: got={}, expected=0.5",
            score.novelty,
        );
        // severity > 0.5 → coherence = 0.8
        assert_eq!(score.coherence, 0.8);
    }

    #[test]
    fn score_computation_with_decay() {
        // Verify that age exactly equal to half-life produces 0.5 recency.
        let half_life_ms: i64 = 600_000; // default 10 minutes
        let now_ms: i64 = 2_000_000_000;
        let ctx = Context::at(now_ms);

        let mut signal = Signal::builder(Kind::GateVerdict)
            .body(Body::empty())
            .build();
        signal.created_at_ms = now_ms - half_life_ms; // exactly one half-life old
        signal.tags.insert("gate".to_string(), "test".to_string());
        signal
            .tags
            .insert("verdict_passed".to_string(), "false".to_string());

        let scorer = VerdictAwareScorer::new();
        let score = scorer.score(&signal, &ctx);

        // recency should be ~0.5 at exactly one half-life
        assert!(
            (score.confidence - 0.5).abs() < 1e-4,
            "recency at half-life: got={}, expected~0.5",
            score.confidence,
        );
    }

    #[test]
    fn score_computation_passing_verdict_exact() {
        let now_ms: i64 = 1_000_000_000;
        let ctx = Context::at(now_ms);

        let mut signal = Signal::builder(Kind::GateVerdict)
            .body(Body::empty())
            .build();
        signal.created_at_ms = now_ms; // age = 0
        signal.tags.insert("gate".to_string(), "test".to_string());
        signal
            .tags
            .insert("verdict_passed".to_string(), "true".to_string());

        let scorer = VerdictAwareScorer::new();
        let score = scorer.score(&signal, &ctx);

        // recency=1.0, severity=0.1 (pass), relevance=0.5 (no context)
        // salience = 1.0*0.35 + 0.1*0.40 + 0.5*0.25 = 0.35 + 0.04 + 0.125 = 0.515
        let expected_salience: f32 = 0.515;
        assert!(
            (score.salience - expected_salience).abs() < 1e-5,
            "salience: got={}, expected={}",
            score.salience,
            expected_salience,
        );
        // severity 0.1 <= 0.5 → novelty = 0.0
        assert_eq!(score.novelty, 0.0);
        // severity 0.1 <= 0.5 → coherence = 0.5
        assert_eq!(score.coherence, 0.5);
    }

    #[test]
    fn all_gates_failed_high_severity_scores() {
        let scorer = VerdictAwareScorer::new();
        let ctx = ctx_at_now();

        let gates_and_severities = [
            ("compile", 1.0_f32),
            ("test", 0.8),
            ("clippy_lint", 0.5),
            ("diff_check", 0.3),
            ("unknown_gate", 0.6),
        ];

        for (gate, expected_severity) in &gates_and_severities {
            let signal = verdict_engram(gate, false, 100);
            let score = scorer.score(&signal, &ctx);

            // All failed verdicts should produce non-trivial scores.
            assert!(
                score.salience > 0.0,
                "gate={gate} should have non-zero salience, got={}",
                score.salience,
            );

            // Verify severity ordering is reflected: higher severity → higher novelty.
            if *expected_severity > 0.5 {
                assert!(
                    score.novelty > 0.0,
                    "gate={gate} severity={expected_severity} should yield positive novelty",
                );
                assert_eq!(
                    score.coherence, 0.8,
                    "gate={gate} severity > 0.5 should yield coherence=0.8",
                );
            } else {
                assert_eq!(
                    score.novelty, 0.0,
                    "gate={gate} severity={expected_severity} should yield zero novelty",
                );
                assert_eq!(
                    score.coherence, 0.5,
                    "gate={gate} severity <= 0.5 should yield coherence=0.5",
                );
            }
        }
    }

    #[test]
    fn custom_config_weights_change_scoring() {
        let now_ms: i64 = 1_000_000_000;
        let ctx = Context::at(now_ms);

        let mut signal = Signal::builder(Kind::GateVerdict)
            .body(Body::empty())
            .build();
        signal.created_at_ms = now_ms; // age = 0
        signal
            .tags
            .insert("gate".to_string(), "compile".to_string());
        signal
            .tags
            .insert("verdict_passed".to_string(), "false".to_string());

        // Severity-heavy config: 0.0 recency, 1.0 severity, 0.0 relevance.
        let severity_config = VerdictScorerConfig {
            recency_half_life_ms: 600_000.0,
            recency_weight: 0.0,
            severity_weight: 1.0,
            relevance_weight: 0.0,
        };
        let severity_scorer = VerdictAwareScorer::with_config(severity_config);
        let severity_score = severity_scorer.score(&signal, &ctx);

        // salience should be purely severity: 1.0 * 1.0 = 1.0
        assert!(
            (severity_score.salience - 1.0).abs() < 1e-5,
            "severity-only salience: got={}, expected=1.0",
            severity_score.salience,
        );

        // Recency-heavy config: 1.0 recency, 0.0 severity, 0.0 relevance.
        let recency_config = VerdictScorerConfig {
            recency_half_life_ms: 600_000.0,
            recency_weight: 1.0,
            severity_weight: 0.0,
            relevance_weight: 0.0,
        };
        let recency_scorer = VerdictAwareScorer::with_config(recency_config);
        let recency_score = recency_scorer.score(&signal, &ctx);

        // salience should be purely recency: 1.0 * 1.0 = 1.0 (age = 0)
        assert!(
            (recency_score.salience - 1.0).abs() < 1e-5,
            "recency-only salience: got={}, expected=1.0",
            recency_score.salience,
        );
    }

    #[test]
    fn custom_half_life_changes_decay() {
        let now_ms: i64 = 1_000_000_000;
        let age_ms: i64 = 60_000; // 1 minute old
        let ctx = Context::at(now_ms);

        let mut signal = Signal::builder(Kind::GateVerdict)
            .body(Body::empty())
            .build();
        signal.created_at_ms = now_ms - age_ms;
        signal
            .tags
            .insert("gate".to_string(), "compile".to_string());
        signal
            .tags
            .insert("verdict_passed".to_string(), "false".to_string());

        // Short half-life: 30 seconds → signal is ~2 half-lives old → recency ~0.25.
        let short_config = VerdictScorerConfig {
            recency_half_life_ms: 30_000.0,
            recency_weight: 1.0,
            severity_weight: 0.0,
            relevance_weight: 0.0,
        };
        let short_scorer = VerdictAwareScorer::with_config(short_config);
        let short_score = short_scorer.score(&signal, &ctx);

        // Long half-life: 10 minutes → signal is 0.1 half-lives old → recency ~0.93.
        let long_config = VerdictScorerConfig {
            recency_half_life_ms: 600_000.0,
            recency_weight: 1.0,
            severity_weight: 0.0,
            relevance_weight: 0.0,
        };
        let long_scorer = VerdictAwareScorer::with_config(long_config);
        let long_score = long_scorer.score(&signal, &ctx);

        assert!(
            long_score.salience > short_score.salience,
            "longer half-life should yield higher recency for same age: long={} > short={}",
            long_score.salience,
            short_score.salience,
        );

        // Verify short half-life recency is ~0.25 (2 half-lives → 0.5^2).
        assert!(
            (short_score.confidence - 0.25).abs() < 0.02,
            "short half-life recency: got={}, expected~0.25",
            short_score.confidence,
        );
    }

    #[test]
    fn relevance_factor_with_matching_context() {
        let now_ms: i64 = 1_000_000_000;
        let ctx = Context::at(now_ms)
            .with_attr("roko.task_type", "implement")
            .with_attr("roko.crate", "roko-core")
            .with_attr("roko.task_category", "feature");

        let mut signal = Signal::builder(Kind::GateVerdict)
            .body(Body::empty())
            .build();
        signal.created_at_ms = now_ms;
        signal
            .tags
            .insert("gate".to_string(), "compile".to_string());
        signal
            .tags
            .insert("verdict_passed".to_string(), "false".to_string());
        signal
            .tags
            .insert("task_type".to_string(), "implement".to_string());
        signal
            .tags
            .insert("crate".to_string(), "roko-core".to_string());
        signal
            .tags
            .insert("task_category".to_string(), "feature".to_string());

        // Use relevance-only config to isolate the relevance dimension.
        let config = VerdictScorerConfig {
            recency_half_life_ms: 600_000.0,
            recency_weight: 0.0,
            severity_weight: 0.0,
            relevance_weight: 1.0,
        };
        let scorer = VerdictAwareScorer::with_config(config);
        let score = scorer.score(&signal, &ctx);

        // All 3 context attrs match → relevance = 3/3 = 1.0.
        assert!(
            (score.salience - 1.0).abs() < 1e-5,
            "full relevance salience: got={}, expected=1.0",
            score.salience,
        );
        assert!(
            (score.precision - 1.0).abs() < 1e-5,
            "full relevance precision: got={}, expected=1.0",
            score.precision,
        );
    }

    #[test]
    fn relevance_factor_with_partial_context_match() {
        let now_ms: i64 = 1_000_000_000;
        let ctx = Context::at(now_ms)
            .with_attr("roko.task_type", "implement")
            .with_attr("roko.crate", "roko-core")
            .with_attr("roko.task_category", "feature");

        let mut signal = Signal::builder(Kind::GateVerdict)
            .body(Body::empty())
            .build();
        signal.created_at_ms = now_ms;
        signal
            .tags
            .insert("gate".to_string(), "compile".to_string());
        signal
            .tags
            .insert("verdict_passed".to_string(), "false".to_string());
        // Only task_type matches, crate and category do not.
        signal
            .tags
            .insert("task_type".to_string(), "implement".to_string());
        signal
            .tags
            .insert("crate".to_string(), "roko-gate".to_string());
        signal
            .tags
            .insert("task_category".to_string(), "bugfix".to_string());

        let config = VerdictScorerConfig {
            recency_half_life_ms: 600_000.0,
            recency_weight: 0.0,
            severity_weight: 0.0,
            relevance_weight: 1.0,
        };
        let scorer = VerdictAwareScorer::with_config(config);
        let score = scorer.score(&signal, &ctx);

        // 1 of 3 attrs match → relevance = 1/3 ≈ 0.333.
        let expected_relevance = 1.0_f32 / 3.0;
        assert!(
            (score.salience - expected_relevance).abs() < 1e-5,
            "partial relevance salience: got={}, expected={}",
            score.salience,
            expected_relevance,
        );
    }

    #[test]
    fn relevance_factor_with_no_context_attrs() {
        let now_ms: i64 = 1_000_000_000;
        // Context with no roko.* attrs.
        let ctx = Context::at(now_ms);

        let mut signal = Signal::builder(Kind::GateVerdict)
            .body(Body::empty())
            .build();
        signal.created_at_ms = now_ms;
        signal
            .tags
            .insert("gate".to_string(), "compile".to_string());
        signal
            .tags
            .insert("verdict_passed".to_string(), "false".to_string());

        let config = VerdictScorerConfig {
            recency_half_life_ms: 600_000.0,
            recency_weight: 0.0,
            severity_weight: 0.0,
            relevance_weight: 1.0,
        };
        let scorer = VerdictAwareScorer::with_config(config);
        let score = scorer.score(&signal, &ctx);

        // No context attrs → neutral relevance = 0.5.
        assert!(
            (score.salience - 0.5).abs() < 1e-5,
            "neutral relevance salience: got={}, expected=0.5",
            score.salience,
        );
    }

    #[test]
    fn severity_factor_all_gate_types() {
        let scorer = VerdictAwareScorer::new();
        // Test severity for every recognized gate type by isolating severity.
        let config = VerdictScorerConfig {
            recency_half_life_ms: 600_000.0,
            recency_weight: 0.0,
            severity_weight: 1.0,
            relevance_weight: 0.0,
        };
        let severity_scorer = VerdictAwareScorer::with_config(config);

        let now_ms: i64 = 1_000_000_000;
        let ctx = Context::at(now_ms);

        let cases: Vec<(&str, bool, f32)> = vec![
            ("compile", false, 1.0),
            ("test", false, 0.8),
            ("lint", false, 0.5),
            ("clippy", false, 0.5),
            ("diff", false, 0.3),
            ("something_unknown", false, 0.6),
            ("compile", true, 0.1),
            ("test", true, 0.1),
        ];

        // Suppress the unused-variable warning on `scorer`.
        let _ = scorer;

        for (gate, passed, expected_severity) in &cases {
            let mut signal = Signal::builder(Kind::GateVerdict)
                .body(Body::empty())
                .build();
            signal.created_at_ms = now_ms;
            signal.tags.insert("gate".to_string(), gate.to_string());
            signal
                .tags
                .insert("verdict_passed".to_string(), passed.to_string());

            let score = severity_scorer.score(&signal, &ctx);
            assert!(
                (score.salience - expected_severity).abs() < 1e-5,
                "gate={gate} passed={passed}: severity salience got={}, expected={}",
                score.salience,
                expected_severity,
            );
        }
    }

    #[test]
    fn very_old_signal_has_near_zero_recency() {
        let scorer = VerdictAwareScorer::new();
        let ctx = ctx_at_now();

        // Signal from ~24 hours ago. With 10-min half-life, that is
        // 144 half-lives → recency ≈ 0.
        let signal = verdict_engram("compile", false, 86_400_000);
        let score = scorer.score(&signal, &ctx);

        assert!(
            score.confidence < 1e-10,
            "very old signal recency should be near zero, got={}",
            score.confidence,
        );
    }

    #[test]
    fn future_signal_gets_recency_one() {
        // A signal with a timestamp in the future (clock skew) should
        // be clamped: age = max(0, now - ts) = 0 → recency = 1.0.
        let now_ms: i64 = 1_000_000_000;
        let ctx = Context::at(now_ms);

        let mut signal = Signal::builder(Kind::GateVerdict)
            .body(Body::empty())
            .build();
        signal.created_at_ms = now_ms + 10_000; // 10 seconds in the future
        signal
            .tags
            .insert("gate".to_string(), "compile".to_string());
        signal
            .tags
            .insert("verdict_passed".to_string(), "false".to_string());

        let scorer = VerdictAwareScorer::new();
        let score = scorer.score(&signal, &ctx);

        assert!(
            (score.confidence - 1.0).abs() < 1e-5,
            "future signal should have recency=1.0, got={}",
            score.confidence,
        );
    }

    #[test]
    fn default_config_weights_sum_to_one() {
        let config = VerdictScorerConfig::default();
        let sum = config.recency_weight + config.severity_weight + config.relevance_weight;
        assert!(
            (sum - 1.0).abs() < 1e-5,
            "default weights should sum to 1.0, got={}",
            sum,
        );
    }

    #[test]
    fn default_scorer_equals_new() {
        let default_scorer = VerdictAwareScorer::default();
        let new_scorer = VerdictAwareScorer::new();
        // Both should produce identical results on the same signal.
        let now_ms: i64 = 1_000_000_000;
        let ctx = Context::at(now_ms);

        let mut signal = Signal::builder(Kind::GateVerdict)
            .body(Body::empty())
            .build();
        signal.created_at_ms = now_ms;
        signal
            .tags
            .insert("gate".to_string(), "compile".to_string());
        signal
            .tags
            .insert("verdict_passed".to_string(), "false".to_string());

        let score1 = default_scorer.score(&signal, &ctx);
        let score2 = new_scorer.score(&signal, &ctx);
        assert_eq!(
            score1, score2,
            "default() and new() should produce identical scores"
        );
    }

    #[test]
    fn missing_verdict_passed_tag_defaults_to_pass() {
        // When verdict_passed tag is absent, severity_factor defaults to
        // passed=true → severity=0.1.
        let now_ms: i64 = 1_000_000_000;
        let ctx = Context::at(now_ms);

        let mut signal = Signal::builder(Kind::GateVerdict)
            .body(Body::empty())
            .build();
        signal.created_at_ms = now_ms;
        signal
            .tags
            .insert("gate".to_string(), "compile".to_string());
        // No verdict_passed tag.

        let config = VerdictScorerConfig {
            recency_half_life_ms: 600_000.0,
            recency_weight: 0.0,
            severity_weight: 1.0,
            relevance_weight: 0.0,
        };
        let scorer = VerdictAwareScorer::with_config(config);
        let score = scorer.score(&signal, &ctx);

        // Should default to passed=true → severity=0.1.
        assert!(
            (score.salience - 0.1).abs() < 1e-5,
            "missing verdict_passed should default to pass (severity=0.1), got={}",
            score.salience,
        );
    }

    #[test]
    fn missing_gate_tag_uses_default_severity() {
        // When gate tag is absent and verdict is failed, should use
        // default severity (0.6 for unknown gate).
        let now_ms: i64 = 1_000_000_000;
        let ctx = Context::at(now_ms);

        let mut signal = Signal::builder(Kind::GateVerdict)
            .body(Body::empty())
            .build();
        signal.created_at_ms = now_ms;
        // No gate tag.
        signal
            .tags
            .insert("verdict_passed".to_string(), "false".to_string());

        let config = VerdictScorerConfig {
            recency_half_life_ms: 600_000.0,
            recency_weight: 0.0,
            severity_weight: 1.0,
            relevance_weight: 0.0,
        };
        let scorer = VerdictAwareScorer::with_config(config);
        let score = scorer.score(&signal, &ctx);

        // Empty gate → doesn't match compile/test/lint/diff → falls to default 0.6.
        assert!(
            (score.salience - 0.6).abs() < 1e-5,
            "missing gate tag should use default severity 0.6, got={}",
            score.salience,
        );
    }

    // ─── Additional VerdictHistory tests ─────────────────────────────

    #[test]
    fn empty_history_has_zero_failure_rate() {
        let history = VerdictHistory::new();
        assert_eq!(history.model_failure_rate("any-model"), 0.0);
    }

    #[test]
    fn empty_history_returns_empty_recent_verdicts() {
        let history = VerdictHistory::new();
        let recent = history.recent_verdicts("implement", "roko-core", 10);
        assert!(recent.is_empty());
    }

    #[test]
    fn all_failed_history_gives_full_failure_rate() {
        let mut history = VerdictHistory::new();
        let now = chrono::Utc::now().timestamp_millis();

        for i in 0..5 {
            history.record(VerdictRecord {
                model_slug: "model-x".to_string(),
                task_type: "implement".to_string(),
                target_crate: "roko-core".to_string(),
                gate: "compile".to_string(),
                passed: false,
                timestamp_ms: now + i * 1000,
            });
        }

        assert!((history.model_failure_rate("model-x") - 1.0).abs() < 1e-10);
    }

    #[test]
    fn non_compile_failures_dont_count_in_streak() {
        let mut history = VerdictHistory::new();
        let now = chrono::Utc::now().timestamp_millis();

        // 5 test failures (not compile) should not create a compile streak.
        for i in 0..5 {
            history.record(VerdictRecord {
                model_slug: "model-y".to_string(),
                task_type: "implement".to_string(),
                target_crate: "roko-core".to_string(),
                gate: "test".to_string(),
                passed: false,
                timestamp_ms: now + i * 1000,
            });
        }

        assert_eq!(
            history.compile_failure_streak("model-y", "implement", "roko-core"),
            0,
            "test failures should not count as compile failures",
        );
        // But penalty should still be 1.0 (no compile streak).
        assert_eq!(
            history.reward_penalty("model-y", "implement", "roko-core"),
            1.0,
        );
    }

    #[test]
    fn with_capacity_minimum_is_ten() {
        let history = VerdictHistory::with_capacity(3);
        // Internal max_records should be clamped to at least 10.
        let now = chrono::Utc::now().timestamp_millis();
        let mut h = history;
        for i in 0..15 {
            h.record(VerdictRecord {
                model_slug: "m".to_string(),
                task_type: "t".to_string(),
                target_crate: "c".to_string(),
                gate: "compile".to_string(),
                passed: true,
                timestamp_ms: now + i * 1000,
            });
        }
        // Capacity clamped to 10, so only 10 records should survive.
        assert_eq!(h.len(), 10);
    }

    #[test]
    fn streak_skips_interleaved_different_context() {
        let mut history = VerdictHistory::new();
        let now = chrono::Utc::now().timestamp_millis();

        // model-a compile fail on roko-core
        history.record(VerdictRecord {
            model_slug: "model-a".to_string(),
            task_type: "implement".to_string(),
            target_crate: "roko-core".to_string(),
            gate: "compile".to_string(),
            passed: false,
            timestamp_ms: now,
        });
        // model-a compile fail on roko-gate (different crate — should be skipped)
        history.record(VerdictRecord {
            model_slug: "model-a".to_string(),
            task_type: "implement".to_string(),
            target_crate: "roko-gate".to_string(),
            gate: "compile".to_string(),
            passed: false,
            timestamp_ms: now + 1000,
        });
        // model-a compile fail on roko-core again
        history.record(VerdictRecord {
            model_slug: "model-a".to_string(),
            task_type: "implement".to_string(),
            target_crate: "roko-core".to_string(),
            gate: "compile".to_string(),
            passed: false,
            timestamp_ms: now + 2000,
        });

        // The interleaved roko-gate record is for a different context,
        // so it's skipped. The streak should be 2 (both roko-core failures
        // are consecutive when filtering to matching context).
        assert_eq!(
            history.compile_failure_streak("model-a", "implement", "roko-core"),
            2,
        );
    }

    #[test]
    fn recent_verdicts_filters_by_context() {
        let mut history = VerdictHistory::new();
        let now = chrono::Utc::now().timestamp_millis();

        for i in 0..5 {
            history.record(VerdictRecord {
                model_slug: "m".to_string(),
                task_type: "implement".to_string(),
                target_crate: "roko-core".to_string(),
                gate: "compile".to_string(),
                passed: true,
                timestamp_ms: now + i * 1000,
            });
        }
        for i in 0..3 {
            history.record(VerdictRecord {
                model_slug: "m".to_string(),
                task_type: "fix".to_string(),
                target_crate: "roko-gate".to_string(),
                gate: "test".to_string(),
                passed: false,
                timestamp_ms: now + (5 + i) * 1000,
            });
        }

        let core_verdicts = history.recent_verdicts("implement", "roko-core", 100);
        assert_eq!(core_verdicts.len(), 5);

        let gate_verdicts = history.recent_verdicts("fix", "roko-gate", 100);
        assert_eq!(gate_verdicts.len(), 3);

        let no_verdicts = history.recent_verdicts("refactor", "roko-cli", 100);
        assert!(no_verdicts.is_empty());
    }

    #[test]
    fn reward_penalty_boundary_values() {
        let mut history = VerdictHistory::new();
        let now = chrono::Utc::now().timestamp_millis();

        // Exactly 2 failures → no penalty (boundary).
        for i in 0..2 {
            history.record(VerdictRecord {
                model_slug: "m".to_string(),
                task_type: "t".to_string(),
                target_crate: "c".to_string(),
                gate: "compile".to_string(),
                passed: false,
                timestamp_ms: now + i * 1000,
            });
        }
        assert_eq!(
            history.reward_penalty("m", "t", "c"),
            1.0,
            "streak=2 → no penalty"
        );

        // Add 1 more → streak=3 → 0.5 penalty.
        history.record(VerdictRecord {
            model_slug: "m".to_string(),
            task_type: "t".to_string(),
            target_crate: "c".to_string(),
            gate: "compile".to_string(),
            passed: false,
            timestamp_ms: now + 2000,
        });
        assert_eq!(
            history.reward_penalty("m", "t", "c"),
            0.5,
            "streak=3 → 0.5 penalty"
        );

        // Add 2 more → streak=5 → still 0.5.
        for i in 3..5 {
            history.record(VerdictRecord {
                model_slug: "m".to_string(),
                task_type: "t".to_string(),
                target_crate: "c".to_string(),
                gate: "compile".to_string(),
                passed: false,
                timestamp_ms: now + i * 1000,
            });
        }
        assert_eq!(
            history.reward_penalty("m", "t", "c"),
            0.5,
            "streak=5 → 0.5 penalty"
        );

        // Add 1 more → streak=6 → 0.25 penalty.
        history.record(VerdictRecord {
            model_slug: "m".to_string(),
            task_type: "t".to_string(),
            target_crate: "c".to_string(),
            gate: "compile".to_string(),
            passed: false,
            timestamp_ms: now + 5000,
        });
        assert_eq!(
            history.reward_penalty("m", "t", "c"),
            0.25,
            "streak=6 → 0.25 penalty"
        );
    }
}
