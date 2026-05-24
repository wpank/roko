//! Anomaly detection for runaway agent loops, cost spikes, and quality drift.
//!
//! This module keeps lightweight session-local state so the dispatch pipeline
//! can detect repeated prompt loops, sudden cost spikes, and sustained quality
//! degradation without depending on heavier planner or conductor machinery.

use std::collections::VecDeque;

const PROMPT_LOOP_WINDOW: usize = 20;
const PROMPT_LOOP_THRESHOLD: usize = 5;
const COST_SPIKE_Z_THRESHOLD: f64 = 3.0;
const QUALITY_WINDOW: usize = 50;
const QUALITY_RECENT_WINDOW: usize = 5;
const QUALITY_EARLIER_WINDOW: usize = 10;
const QUALITY_MIN_SAMPLES: usize = QUALITY_RECENT_WINDOW + QUALITY_EARLIER_WINDOW;
const DEFAULT_EWMA_ALPHA: f64 = 0.2;

/// Session-local detector for runaway loops and cost spikes.
#[derive(Debug, Clone)]
pub struct AnomalyDetector {
    prompt_hash_window: VecDeque<u64>,
    cost_ewma: EwmaState,
    quality_history: VecDeque<f64>,
    session_cost_usd: f64,
    session_start_ms: i64,
}

impl AnomalyDetector {
    /// Create a detector for a new session.
    #[must_use]
    pub fn new(session_start_ms: i64) -> Self {
        Self {
            prompt_hash_window: VecDeque::with_capacity(PROMPT_LOOP_WINDOW),
            cost_ewma: EwmaState::default(),
            quality_history: VecDeque::with_capacity(QUALITY_WINDOW),
            session_cost_usd: 0.0,
            session_start_ms,
        }
    }

    /// Return the session start timestamp in Unix milliseconds.
    #[must_use]
    pub const fn session_start_ms(&self) -> i64 {
        self.session_start_ms
    }

    /// Check a prompt hash for repeated-loop behavior.
    ///
    /// Returns [`Anomaly::PromptLoop`] when the same hash appears at least
    /// five times in the last twenty prompts.
    #[must_use]
    pub fn check_prompt(&mut self, prompt_hash: u64) -> Option<Anomaly> {
        self.prompt_hash_window.push_back(prompt_hash);
        if self.prompt_hash_window.len() > PROMPT_LOOP_WINDOW {
            self.prompt_hash_window.pop_front();
        }

        let repeated_count = self
            .prompt_hash_window
            .iter()
            .filter(|&&hash| hash == prompt_hash)
            .count();

        if repeated_count >= PROMPT_LOOP_THRESHOLD {
            Some(Anomaly::PromptLoop { repeated_count })
        } else {
            None
        }
    }

    /// Check a cost observation for a spike relative to the EWMA baseline.
    ///
    /// The new value is compared against the current EWMA state before the
    /// state is updated, which keeps sudden spikes visible instead of folding
    /// them into the baseline immediately.
    #[must_use]
    pub fn check_cost(&mut self, cost_usd: f64) -> Option<Anomaly> {
        let z_score = self.cost_ewma.z_score(cost_usd);
        self.cost_ewma.update(cost_usd);
        self.session_cost_usd += cost_usd;

        if z_score > COST_SPIKE_Z_THRESHOLD {
            Some(Anomaly::CostSpike { z_score })
        } else {
            None
        }
    }

    /// Check a rolling quality score for sustained degradation.
    ///
    /// The detector compares the most recent five scores against the ten
    /// scores immediately before them. If the average drops by more than 0.15
    /// and the recent average is below 0.5, it flags degradation.
    #[must_use]
    pub fn check_quality(&mut self, score: f64) -> Option<Anomaly> {
        self.quality_history.push_back(score);
        if self.quality_history.len() > QUALITY_WINDOW {
            self.quality_history.pop_front();
        }

        if self.quality_history.len() < QUALITY_MIN_SAMPLES {
            return None;
        }

        let recent_avg = average_recent(&self.quality_history, QUALITY_RECENT_WINDOW);
        let earlier_avg = average_earlier(
            &self.quality_history,
            QUALITY_RECENT_WINDOW,
            QUALITY_EARLIER_WINDOW,
        );
        let avg_drop = earlier_avg - recent_avg;

        if recent_avg < earlier_avg - 0.15 && recent_avg < 0.5 {
            Some(Anomaly::QualityDegradation { avg_drop })
        } else {
            None
        }
    }

    /// Check whether the accumulated session cost has exhausted a budget.
    #[must_use]
    pub fn check_budget(&self, limit_usd: f64) -> Option<Anomaly> {
        if self.session_cost_usd >= limit_usd {
            Some(Anomaly::BudgetExhausted {
                used: self.session_cost_usd,
                limit: limit_usd,
            })
        } else {
            None
        }
    }
}

fn average_recent(history: &VecDeque<f64>, count: usize) -> f64 {
    let sum: f64 = history.iter().rev().take(count).copied().sum();
    sum / count as f64
}

fn average_earlier(history: &VecDeque<f64>, recent_count: usize, count: usize) -> f64 {
    let sum: f64 = history
        .iter()
        .rev()
        .skip(recent_count)
        .take(count)
        .copied()
        .sum();
    sum / count as f64
}

/// Exponentially weighted moving average state used for cost spike detection.
#[derive(Debug, Clone)]
pub struct EwmaState {
    /// Current EWMA mean.
    pub mean: f64,
    /// Current EWMA variance estimate.
    pub variance: f64,
    alpha: f64,
}

impl EwmaState {
    /// Create a new EWMA state with a custom smoothing factor.
    #[must_use]
    pub fn new(alpha: f64) -> Self {
        Self {
            mean: 0.0,
            variance: 0.0,
            alpha: sanitize_alpha(alpha),
        }
    }

    /// Update the EWMA state with a new observation.
    pub fn update(&mut self, value: f64) {
        let diff = value - self.mean;
        self.mean += self.alpha * diff;
        self.variance = (1.0 - self.alpha) * (self.variance + self.alpha * diff * diff);
    }

    /// Compute the z-score of a value relative to the current EWMA state.
    #[must_use]
    pub fn z_score(&self, value: f64) -> f64 {
        let stddev = self.variance.sqrt();
        if stddev < 1e-10 {
            0.0
        } else {
            (value - self.mean) / stddev
        }
    }
}

impl Default for EwmaState {
    fn default() -> Self {
        Self::new(DEFAULT_EWMA_ALPHA)
    }
}

fn sanitize_alpha(alpha: f64) -> f64 {
    if alpha.is_finite() {
        alpha.clamp(0.0, 1.0)
    } else {
        DEFAULT_EWMA_ALPHA
    }
}

/// An anomaly detected in the current session.
#[derive(Debug, Clone, PartialEq)]
pub enum Anomaly {
    /// The same prompt hash appeared repeatedly in the sliding window.
    PromptLoop {
        /// Number of repeated occurrences seen in the window.
        repeated_count: usize,
    },
    /// The current cost is more than three standard deviations above baseline.
    CostSpike {
        /// Cost z-score against the EWMA baseline.
        z_score: f64,
    },
    /// Quality dropped materially across the recent rolling window.
    QualityDegradation {
        /// Average drop between the earlier and recent windows.
        avg_drop: f64,
    },
    /// The running session cost has exhausted the configured budget.
    BudgetExhausted {
        /// Total cost used so far.
        used: f64,
        /// Budget limit that was exceeded.
        limit: f64,
    },
}

#[cfg(test)]
mod tests {
    use super::{
        Anomaly, AnomalyDetector, COST_SPIKE_Z_THRESHOLD, DEFAULT_EWMA_ALPHA, EwmaState,
        PROMPT_LOOP_THRESHOLD, PROMPT_LOOP_WINDOW, QUALITY_MIN_SAMPLES, sanitize_alpha,
    };

    // ---------------------------------------------------------------
    // Prompt loop detection
    // ---------------------------------------------------------------

    #[test]
    fn anomaly_detector_prompt_loop_detection_triggers_at_five_identical_hashes() {
        let mut detector = AnomalyDetector::new(1_700_000_000_000);

        for _ in 0..4 {
            assert!(detector.check_prompt(42).is_none());
        }

        let anomaly = detector
            .check_prompt(42)
            .expect("expected prompt loop at five repeats");

        assert_eq!(anomaly, Anomaly::PromptLoop { repeated_count: 5 });
    }

    #[test]
    fn prompt_loop_no_anomaly_with_diverse_hashes() {
        let mut detector = AnomalyDetector::new(0);
        // Feed 20 distinct hashes — none should repeat enough to trigger.
        for i in 0..20u64 {
            assert!(
                detector.check_prompt(i).is_none(),
                "unexpected anomaly at hash {i}"
            );
        }
    }

    #[test]
    fn prompt_loop_does_not_trigger_below_threshold() {
        let mut detector = AnomalyDetector::new(0);
        // Feed (PROMPT_LOOP_THRESHOLD - 1) copies interleaved with different hashes.
        for i in 0..(PROMPT_LOOP_THRESHOLD - 1) {
            assert!(detector.check_prompt(99).is_none());
            // Insert a different hash to keep the count below threshold.
            assert!(detector.check_prompt(i as u64 + 1000).is_none());
        }
    }

    #[test]
    fn prompt_loop_window_eviction_clears_old_hashes() {
        let mut detector = AnomalyDetector::new(0);

        // Insert 4 copies of hash 7 (just below threshold).
        for _ in 0..4 {
            assert!(detector.check_prompt(7).is_none());
        }

        // Now push enough distinct hashes to evict all copies from the window.
        for i in 100..(100 + PROMPT_LOOP_WINDOW as u64) {
            let _ = detector.check_prompt(i);
        }

        // The old copies of 7 should be gone, so 4 more should NOT trigger.
        for _ in 0..4 {
            assert!(
                detector.check_prompt(7).is_none(),
                "old copies were not evicted from the window"
            );
        }
    }

    #[test]
    fn prompt_loop_reports_increasing_repeated_count() {
        let mut detector = AnomalyDetector::new(0);

        // Insert exactly PROMPT_LOOP_THRESHOLD copies to trigger once.
        for _ in 0..(PROMPT_LOOP_THRESHOLD - 1) {
            assert!(detector.check_prompt(42).is_none());
        }
        let a = detector
            .check_prompt(42)
            .expect("should trigger at threshold");
        assert_eq!(
            a,
            Anomaly::PromptLoop {
                repeated_count: PROMPT_LOOP_THRESHOLD,
            }
        );

        // One more should report count + 1.
        let b = detector.check_prompt(42).expect("should still trigger");
        assert_eq!(
            b,
            Anomaly::PromptLoop {
                repeated_count: PROMPT_LOOP_THRESHOLD + 1,
            }
        );
    }

    // ---------------------------------------------------------------
    // Cost spike detection
    // ---------------------------------------------------------------

    #[test]
    fn anomaly_detector_cost_spike_detection_triggers_on_large_z_score() {
        let mut detector = AnomalyDetector::new(1_700_000_000_000);

        for cost in [1.0, 1.2, 0.9, 1.1, 1.05, 0.95, 1.15, 1.0] {
            assert!(detector.check_cost(cost).is_none());
        }

        let anomaly = detector
            .check_cost(10.0)
            .expect("expected cost spike anomaly");

        match anomaly {
            Anomaly::CostSpike { z_score } => {
                assert!(z_score > 3.0, "expected z-score > 3.0, got {z_score}");
            }
            other => panic!("expected cost spike, got {other:?}"),
        }
    }

    #[test]
    fn cost_no_anomaly_with_stable_costs() {
        let mut detector = AnomalyDetector::new(0);
        // All costs are very similar — no spike should be detected.
        for cost in [1.0, 1.01, 0.99, 1.02, 0.98, 1.0, 1.01, 0.99, 1.0, 1.0] {
            assert!(
                detector.check_cost(cost).is_none(),
                "unexpected spike at cost {cost}"
            );
        }
    }

    #[test]
    fn cost_single_observation_never_spikes() {
        let mut detector = AnomalyDetector::new(0);
        // With no baseline, the z-score should be 0 (stddev ~0).
        assert!(detector.check_cost(1000.0).is_none());
    }

    #[test]
    fn cost_accumulates_session_total() {
        let mut detector = AnomalyDetector::new(0);
        detector.check_cost(1.5);
        detector.check_cost(2.5);
        detector.check_cost(1.0);
        // Total should be 5.0.
        assert!(detector.check_budget(5.0).is_some());
        assert!(detector.check_budget(5.01).is_none());
    }

    // ---------------------------------------------------------------
    // Quality degradation detection
    // ---------------------------------------------------------------

    #[test]
    fn anomaly_detector_quality_degradation_triggers_after_sustained_drop() {
        let mut detector = AnomalyDetector::new(1_700_000_000_000);

        for _ in 0..10 {
            assert!(detector.check_quality(0.82).is_none());
        }

        let mut fired = None;
        for score in [0.34, 0.32, 0.31, 0.29, 0.30] {
            fired = detector.check_quality(score);
        }

        match fired.expect("expected quality degradation") {
            Anomaly::QualityDegradation { avg_drop } => {
                assert!(avg_drop > 0.15);
            }
            other => panic!("expected quality degradation, got {other:?}"),
        }
    }

    #[test]
    fn quality_no_anomaly_with_stable_high_scores() {
        let mut detector = AnomalyDetector::new(0);
        // Feed 20 scores all around 0.8 — no degradation expected.
        for _ in 0..20 {
            assert!(detector.check_quality(0.8).is_none());
        }
    }

    #[test]
    fn quality_no_anomaly_with_insufficient_samples() {
        let mut detector = AnomalyDetector::new(0);
        // Feed fewer than QUALITY_MIN_SAMPLES scores. Even terrible scores
        // should not trigger because we need enough history.
        for _ in 0..(QUALITY_MIN_SAMPLES - 1) {
            assert!(
                detector.check_quality(0.1).is_none(),
                "should not trigger with insufficient samples"
            );
        }
    }

    #[test]
    fn quality_no_anomaly_when_recent_above_half() {
        let mut detector = AnomalyDetector::new(0);
        // Earlier scores are high, recent scores drop but stay above 0.5.
        // The condition requires recent_avg < 0.5, so no anomaly.
        for _ in 0..10 {
            detector.check_quality(0.9);
        }
        for _ in 0..5 {
            let result = detector.check_quality(0.55);
            assert!(
                result.is_none(),
                "should not fire when recent avg is above 0.5"
            );
        }
    }

    #[test]
    fn quality_no_anomaly_when_drop_is_small() {
        let mut detector = AnomalyDetector::new(0);
        // Earlier scores around 0.45, recent scores around 0.40.
        // Drop is only 0.05 which is below the 0.15 threshold.
        for _ in 0..10 {
            detector.check_quality(0.45);
        }
        for _ in 0..5 {
            let result = detector.check_quality(0.40);
            assert!(
                result.is_none(),
                "should not fire when drop is less than 0.15"
            );
        }
    }

    #[test]
    fn quality_degradation_reports_correct_drop_magnitude() {
        let mut detector = AnomalyDetector::new(0);
        // Earlier window: 10 scores of 0.80.
        for _ in 0..10 {
            detector.check_quality(0.80);
        }
        // Recent window: 5 scores of 0.30.
        let mut last = None;
        for _ in 0..5 {
            last = detector.check_quality(0.30);
        }
        match last.expect("should detect degradation") {
            Anomaly::QualityDegradation { avg_drop } => {
                // Expected drop = 0.80 - 0.30 = 0.50.
                let expected = 0.50;
                assert!(
                    (avg_drop - expected).abs() < 0.01,
                    "expected avg_drop ~{expected}, got {avg_drop}"
                );
            }
            other => panic!("expected QualityDegradation, got {other:?}"),
        }
    }

    // ---------------------------------------------------------------
    // Budget exhaustion
    // ---------------------------------------------------------------

    #[test]
    fn anomaly_detector_budget_exhaustion_triggers_when_cost_exceeds_limit() {
        let mut detector = AnomalyDetector::new(1_700_000_000_000);

        assert!(detector.check_cost(3.0).is_none());
        assert!(detector.check_budget(5.0).is_none());

        assert!(detector.check_cost(2.5).is_none());

        match detector
            .check_budget(5.0)
            .expect("expected budget exhaustion")
        {
            Anomaly::BudgetExhausted { used, limit } => {
                assert!(used >= limit);
                assert_eq!(limit, 5.0);
            }
            other => panic!("expected budget exhaustion, got {other:?}"),
        }
    }

    #[test]
    fn budget_no_anomaly_when_under_limit() {
        let mut detector = AnomalyDetector::new(0);
        detector.check_cost(1.0);
        detector.check_cost(1.0);
        assert!(detector.check_budget(10.0).is_none());
    }

    #[test]
    fn budget_triggers_at_exact_limit() {
        let mut detector = AnomalyDetector::new(0);
        detector.check_cost(5.0);
        let result = detector.check_budget(5.0);
        assert!(result.is_some(), "should trigger when used == limit");
        match result.unwrap() {
            Anomaly::BudgetExhausted { used, limit } => {
                assert!((used - 5.0).abs() < f64::EPSILON);
                assert!((limit - 5.0).abs() < f64::EPSILON);
            }
            other => panic!("expected BudgetExhausted, got {other:?}"),
        }
    }

    #[test]
    fn budget_zero_limit_triggers_on_any_cost() {
        let mut detector = AnomalyDetector::new(0);
        detector.check_cost(0.001);
        assert!(
            detector.check_budget(0.0).is_some(),
            "any positive cost should exceed a zero budget"
        );
    }

    // ---------------------------------------------------------------
    // EwmaState unit tests
    // ---------------------------------------------------------------

    #[test]
    fn ewma_default_uses_default_alpha() {
        let state = EwmaState::default();
        assert!((state.alpha - DEFAULT_EWMA_ALPHA).abs() < f64::EPSILON);
        assert_eq!(state.mean, 0.0);
        assert_eq!(state.variance, 0.0);
    }

    #[test]
    fn ewma_custom_alpha() {
        let state = EwmaState::new(0.5);
        assert!((state.alpha - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn ewma_alpha_clamped_to_unit_interval() {
        let over = EwmaState::new(2.0);
        assert!(
            (over.alpha - 1.0).abs() < f64::EPSILON,
            "alpha > 1 should clamp to 1.0"
        );

        let under = EwmaState::new(-0.5);
        assert!(
            (under.alpha - 0.0).abs() < f64::EPSILON,
            "alpha < 0 should clamp to 0.0"
        );
    }

    #[test]
    fn ewma_alpha_nan_falls_back_to_default() {
        let nan_state = EwmaState::new(f64::NAN);
        assert!(
            (nan_state.alpha - DEFAULT_EWMA_ALPHA).abs() < f64::EPSILON,
            "NaN alpha should fall back to default"
        );
    }

    #[test]
    fn ewma_alpha_infinity_falls_back_to_default() {
        let inf = EwmaState::new(f64::INFINITY);
        assert!(
            (inf.alpha - DEFAULT_EWMA_ALPHA).abs() < f64::EPSILON,
            "+Inf alpha should fall back to default"
        );
        let neg_inf = EwmaState::new(f64::NEG_INFINITY);
        assert!(
            (neg_inf.alpha - DEFAULT_EWMA_ALPHA).abs() < f64::EPSILON,
            "-Inf alpha should fall back to default"
        );
    }

    #[test]
    fn ewma_z_score_zero_when_no_variance() {
        let state = EwmaState::default();
        // With zero variance (stddev < 1e-10), z_score should return 0.
        assert_eq!(state.z_score(100.0), 0.0);
        assert_eq!(state.z_score(-50.0), 0.0);
    }

    #[test]
    fn ewma_update_moves_mean_toward_value() {
        let mut state = EwmaState::new(0.5);
        state.update(10.0);
        // mean should move from 0 toward 10: 0 + 0.5 * (10 - 0) = 5.0
        assert!((state.mean - 5.0).abs() < f64::EPSILON);
        state.update(10.0);
        // mean should move closer to 10: 5 + 0.5 * (10 - 5) = 7.5
        assert!((state.mean - 7.5).abs() < f64::EPSILON);
    }

    #[test]
    fn ewma_z_score_positive_for_above_mean() {
        let mut state = EwmaState::new(0.3);
        // Build some baseline.
        for v in [5.0, 5.0, 5.0, 5.0, 5.0] {
            state.update(v);
        }
        // A value well above the mean should have a positive z-score,
        // but since all values are the same, variance is very small.
        // Let's add one different value first to build variance.
        state.update(6.0);
        state.update(4.0);
        state.update(5.0);

        let z = state.z_score(10.0);
        assert!(
            z > 0.0,
            "z-score for value above mean should be positive, got {z}"
        );
    }

    #[test]
    fn ewma_z_score_negative_for_below_mean() {
        let mut state = EwmaState::new(0.3);
        for v in [10.0, 10.0, 10.0, 11.0, 9.0, 10.0] {
            state.update(v);
        }
        let z = state.z_score(0.0);
        assert!(
            z < 0.0,
            "z-score for value below mean should be negative, got {z}"
        );
    }

    // ---------------------------------------------------------------
    // sanitize_alpha
    // ---------------------------------------------------------------

    #[test]
    fn sanitize_alpha_passes_valid_values() {
        assert!((sanitize_alpha(0.0) - 0.0).abs() < f64::EPSILON);
        assert!((sanitize_alpha(0.5) - 0.5).abs() < f64::EPSILON);
        assert!((sanitize_alpha(1.0) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn sanitize_alpha_clamps_out_of_range() {
        assert!((sanitize_alpha(1.5) - 1.0).abs() < f64::EPSILON);
        assert!((sanitize_alpha(-0.1) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn sanitize_alpha_returns_default_for_non_finite() {
        assert!((sanitize_alpha(f64::NAN) - DEFAULT_EWMA_ALPHA).abs() < f64::EPSILON);
        assert!((sanitize_alpha(f64::INFINITY) - DEFAULT_EWMA_ALPHA).abs() < f64::EPSILON);
        assert!((sanitize_alpha(f64::NEG_INFINITY) - DEFAULT_EWMA_ALPHA).abs() < f64::EPSILON);
    }

    // ---------------------------------------------------------------
    // Session start accessor
    // ---------------------------------------------------------------

    #[test]
    fn session_start_ms_returns_constructor_value() {
        let ts = 1_700_000_000_000i64;
        let detector = AnomalyDetector::new(ts);
        assert_eq!(detector.session_start_ms(), ts);
    }

    #[test]
    fn session_start_ms_zero() {
        let detector = AnomalyDetector::new(0);
        assert_eq!(detector.session_start_ms(), 0);
    }

    // ---------------------------------------------------------------
    // Edge case: empty / fresh detector
    // ---------------------------------------------------------------

    #[test]
    fn fresh_detector_reports_no_anomalies() {
        let detector = AnomalyDetector::new(0);
        // No observations have been made — budget check with a high limit is fine.
        assert!(detector.check_budget(1000.0).is_none());
    }

    #[test]
    fn fresh_detector_quality_single_point_no_anomaly() {
        let mut detector = AnomalyDetector::new(0);
        // A single terrible quality score should not trigger (not enough samples).
        assert!(detector.check_quality(0.0).is_none());
    }

    #[test]
    fn fresh_detector_cost_zero_no_anomaly() {
        let mut detector = AnomalyDetector::new(0);
        assert!(detector.check_cost(0.0).is_none());
    }

    // ---------------------------------------------------------------
    // Constants sanity
    // ---------------------------------------------------------------

    #[test]
    fn z_threshold_is_three_sigma() {
        assert!(
            (COST_SPIKE_Z_THRESHOLD - 3.0).abs() < f64::EPSILON,
            "cost spike threshold should be exactly 3.0"
        );
    }

    #[test]
    fn prompt_loop_window_and_threshold_are_consistent() {
        assert!(
            PROMPT_LOOP_THRESHOLD <= PROMPT_LOOP_WINDOW,
            "threshold must fit within the window"
        );
    }

    // ---------------------------------------------------------------
    // Combined / integration-style
    // ---------------------------------------------------------------

    #[test]
    fn all_anomaly_types_can_fire_independently() {
        let mut detector = AnomalyDetector::new(0);

        // Trigger prompt loop.
        for _ in 0..PROMPT_LOOP_THRESHOLD {
            detector.check_prompt(1);
        }
        assert!(matches!(
            detector.check_prompt(1),
            Some(Anomaly::PromptLoop { .. })
        ));

        // Trigger cost spike (build baseline, then spike).
        for _ in 0..10 {
            detector.check_cost(1.0);
        }
        let cost_result = detector.check_cost(100.0);
        assert!(
            matches!(cost_result, Some(Anomaly::CostSpike { .. })),
            "expected CostSpike, got {cost_result:?}"
        );

        // Trigger budget exhaustion (we've been accumulating cost).
        assert!(matches!(
            detector.check_budget(1.0),
            Some(Anomaly::BudgetExhausted { .. })
        ));

        // Trigger quality degradation.
        for _ in 0..10 {
            detector.check_quality(0.9);
        }
        let mut quality_result = None;
        for _ in 0..5 {
            quality_result = detector.check_quality(0.2);
        }
        assert!(
            matches!(quality_result, Some(Anomaly::QualityDegradation { .. })),
            "expected QualityDegradation, got {quality_result:?}"
        );
    }

    #[test]
    fn anomaly_enum_debug_and_clone() {
        let a = Anomaly::PromptLoop { repeated_count: 5 };
        let b = a.clone();
        assert_eq!(a, b);
        // Debug should not panic.
        let _ = format!("{a:?}");

        let c = Anomaly::CostSpike { z_score: 4.2 };
        let _ = format!("{c:?}");

        let d = Anomaly::QualityDegradation { avg_drop: 0.3 };
        let _ = format!("{d:?}");

        let e = Anomaly::BudgetExhausted {
            used: 10.0,
            limit: 5.0,
        };
        let _ = format!("{e:?}");
    }
}
