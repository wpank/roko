//! Property-based tests for roko-gate core types.

use proptest::prelude::*;
use roko_core::Verdict;
use roko_gate::adaptive_threshold::AdaptiveThresholds;
use roko_gate::benchmark_gate::BenchmarkComparison;
use roko_gate::verdict_publisher::VerdictSummary;

// ─── AdaptiveThresholds: EMA in [0,1] ───────────────────────────────────────

proptest! {
    /// EMA pass rate stays in [0, 1] under arbitrary pass/fail sequences.
    #[test]
    fn ema_always_in_unit_interval(
        observations in prop::collection::vec(any::<bool>(), 1..60),
        rung in 0u32..7,
    ) {
        let mut at = AdaptiveThresholds::new();
        for passed in observations {
            at.observe(rung, passed);
        }
        let rate = at.threshold_for(rung);
        prop_assert!(rate >= 0.0 && rate <= 1.0,
            "EMA pass rate {rate} is out of [0, 1] for rung {rung}");
    }

    /// CUSUM accumulators are always non-negative.
    #[test]
    fn cusum_accumulators_non_negative(
        observations in prop::collection::vec(any::<bool>(), 1..40),
        rung in 0u32..7,
    ) {
        let mut at = AdaptiveThresholds::new();
        for passed in observations {
            at.observe(rung, passed);
        }
        let (high, low) = at.cusum_values(rung);
        prop_assert!(high >= 0.0, "cusum_high {high} < 0");
        prop_assert!(low >= 0.0, "cusum_low {low} < 0");
    }

    /// suggested_max_retries is bounded in [1, 5] after enough observations.
    #[test]
    fn suggested_retries_in_valid_range(
        observations in prop::collection::vec(any::<bool>(), 5..40),
        rung in 0u32..7,
    ) {
        let mut at = AdaptiveThresholds::new();
        for passed in observations {
            at.observe(rung, passed);
        }
        let retries = at.suggested_max_retries(rung);
        prop_assert!(retries >= 1 && retries <= 5,
            "suggested_max_retries({rung}) = {retries} out of [1, 5]");
    }
}

// ─── BenchmarkComparison: sign of change_pct matches direction ───────────────

proptest! {
    /// When baseline_ns > current_ns (improvement), change_pct is negative.
    /// When baseline_ns < current_ns (regression), change_pct is positive.
    #[test]
    fn change_pct_sign_matches_direction(
        baseline_ns in 1.0f64..1_000_000.0,
        delta_pct in -0.5f64..2.0,
    ) {
        // current = baseline * (1 + delta_pct) to control the direction precisely.
        let current_ns = (baseline_ns * (1.0 + delta_pct)).max(1.0);
        let change_pct = (current_ns - baseline_ns) / baseline_ns * 100.0;
        let cmp = BenchmarkComparison {
            name: "bench".into(),
            baseline_ns,
            current_ns,
            change_pct,
        };
        if cmp.baseline_ns > cmp.current_ns {
            // Improvement: current is faster, so change_pct should be negative.
            prop_assert!(cmp.change_pct <= 0.0,
                "improvement should have negative change_pct, got {}", cmp.change_pct);
        } else if cmp.current_ns > cmp.baseline_ns {
            // Regression: current is slower, so change_pct should be positive.
            prop_assert!(cmp.change_pct >= 0.0,
                "regression should have positive change_pct, got {}", cmp.change_pct);
        }
        // equal case (within floating-point) can be either sign.
    }

    /// BenchmarkComparison serialization roundtrip.
    #[test]
    fn benchmark_comparison_serde_roundtrip(
        baseline_ns in 1.0f64..1_000_000.0,
        current_ns in 1.0f64..1_000_000.0,
    ) {
        let change_pct = (current_ns - baseline_ns) / baseline_ns * 100.0;
        let cmp = BenchmarkComparison {
            name: "bench_serde".into(),
            baseline_ns,
            current_ns,
            change_pct,
        };
        let json = serde_json::to_string(&cmp).expect("serialize");
        let parsed: BenchmarkComparison = serde_json::from_str(&json).expect("deserialize");
        prop_assert!((cmp.baseline_ns - parsed.baseline_ns).abs() < 1e-6);
        prop_assert!((cmp.current_ns - parsed.current_ns).abs() < 1e-6);
        prop_assert!((cmp.change_pct - parsed.change_pct).abs() < 1e-6);
        prop_assert_eq!(&cmp.name, &parsed.name);
    }
}

// ─── Verdict and VerdictSummary roundtrip ────────────────────────────────────

fn arb_verdict() -> impl Strategy<Value = Verdict> {
    (any::<bool>(), "[a-z]{1,8}", 0u64..300_000).prop_map(|(passed, gate, duration_ms)| {
        let mut v = if passed {
            Verdict::pass(gate)
        } else {
            Verdict::fail(gate, "property test failure")
        };
        v.duration_ms = duration_ms;
        v
    })
}

proptest! {
    /// VerdictSummary serializes without error and preserves key fields.
    #[test]
    fn verdict_summary_serializes(verdict in arb_verdict(), rung in prop::option::of(0u32..7)) {
        let summary = VerdictSummary::from_verdict(&verdict, rung);
        let json = serde_json::to_string(&summary).expect("serialize");
        // Verify the JSON contains the expected gate name.
        let gate_str = verdict.gate.as_str();
        prop_assert!(json.contains(gate_str),
            "serialized VerdictSummary does not contain gate {gate_str:?}: {json}");
        prop_assert_eq!(summary.passed, verdict.passed);
        prop_assert!((summary.score - verdict.score).abs() < 1e-6);
        prop_assert_eq!(summary.rung, rung);
    }

    /// A passing Verdict always has score = 1.0.
    #[test]
    fn passing_verdict_has_score_one(gate in "[a-z]{1,8}") {
        let verdict = Verdict::pass(gate);
        prop_assert!(verdict.passed);
        prop_assert!((verdict.score - 1.0).abs() < 1e-6,
            "pass score should be 1.0, got {}", verdict.score);
    }

    /// A failing Verdict always has score = 0.0.
    #[test]
    fn failing_verdict_has_score_zero(gate in "[a-z]{1,8}") {
        let verdict = Verdict::fail(gate, "fail");
        prop_assert!(!verdict.passed);
        prop_assert!((verdict.score - 0.0).abs() < 1e-6,
            "fail score should be 0.0, got {}", verdict.score);
    }

    /// Verdict serde roundtrip (Verdict itself has Serialize+Deserialize).
    #[test]
    fn verdict_serde_roundtrip(verdict in arb_verdict()) {
        let json = serde_json::to_string(&verdict).expect("serialize");
        let parsed: Verdict = serde_json::from_str(&json).expect("deserialize");
        prop_assert_eq!(verdict.passed, parsed.passed);
        prop_assert!((verdict.score - parsed.score).abs() < 1e-6);
        prop_assert_eq!(&verdict.gate, &parsed.gate);
        prop_assert_eq!(verdict.duration_ms, parsed.duration_ms);
    }
}
