//! Standalone gate combinators: ParallelGate, VotingGate, FallbackGate (GATE-04).
//!
//! Each combinator wraps inner gates and itself implements [`Verify`], enabling
//! algebraic composition of verification pipelines.
//!
//! | Combinator | Strategy | Aggregate |
//! |---|---|---|
//! | [`ParallelGate`] | Run all gates concurrently | min score; fail if any fails |
//! | [`VotingGate`] | Run all gates, require N-of-M pass | mean of passing scores |
//! | [`FallbackGate`] | Try primary; on failure try fallback | first passing verdict |

use async_trait::async_trait;
use futures::stream::{FuturesUnordered, StreamExt};
use roko_core::{Context, Signal, Verdict, Verify};
use std::fmt;
use std::num::NonZeroUsize;

// ─── ParallelGate ────────────────────────────────────────────────────────────

/// Runs N gates concurrently and aggregates verdicts by taking the minimum score.
///
/// If any gate fails, the aggregate fails. Use when inner gates are independent
/// and can safely run simultaneously (e.g., CompileGate + LintGate).
///
/// Inner gates are dispatched concurrently via [`tokio::task::JoinSet`] with an
/// optional concurrency cap. Results are always returned in **declaration
/// order** regardless of completion order. Dropping the `ParallelGate` future
/// cancels all in-flight inner gates through standard `JoinSet` drop semantics.
pub struct ParallelGate {
    gates: Vec<Box<dyn Verify>>,
    name: String,
    /// Maximum number of inner gates that may execute simultaneously.
    /// `None` means all gates run at once (`inner.len()`).
    max_concurrency: Option<NonZeroUsize>,
}

impl ParallelGate {
    /// Create a new parallel gate with the given name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            gates: Vec::new(),
            name: name.into(),
            max_concurrency: None,
        }
    }

    /// Append an inner gate.
    pub fn push(&mut self, gate: Box<dyn Verify>) {
        self.gates.push(gate);
    }

    /// Chainable gate append.
    #[must_use]
    pub fn with_gate(mut self, gate: Box<dyn Verify>) -> Self {
        self.push(gate);
        self
    }

    /// Set the maximum number of gates that may execute concurrently.
    ///
    /// `None` (the default) resolves to `inner.len().max(1)` -- all gates
    /// execute at once. `Some(cap)` resolves to `min(cap, inner.len().max(1))`.
    #[must_use]
    pub fn with_max_concurrency(mut self, cap: NonZeroUsize) -> Self {
        self.max_concurrency = Some(cap);
        self
    }

    /// The configured concurrency cap, or `None` for unbounded.
    #[must_use]
    pub fn max_concurrency(&self) -> Option<NonZeroUsize> {
        self.max_concurrency
    }

    /// Number of inner gates.
    #[must_use]
    pub fn len(&self) -> usize {
        self.gates.len()
    }

    /// Whether no inner gates are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.gates.is_empty()
    }

    /// Effective concurrency for the current gate set: `min(cap, len.max(1))`.
    fn effective_concurrency(&self) -> usize {
        let n = self.gates.len().max(1);
        match self.max_concurrency {
            None => n,
            Some(cap) => cap.get().min(n),
        }
    }
}

impl fmt::Debug for ParallelGate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ParallelGate")
            .field("name", &self.name)
            .field("gates", &self.gates.len())
            .field("max_concurrency", &self.max_concurrency)
            .finish()
    }
}

impl roko_core::Cell for ParallelGate {
    fn cell_id(&self) -> &str {
        "parallel-gate"
    }
    fn cell_name(&self) -> &str {
        "ParallelGate"
    }
    fn protocols(&self) -> Vec<roko_core::ProtocolId> {
        vec![roko_core::ProtocolId::Verify]
    }
}

#[async_trait]

impl Verify for ParallelGate {
    async fn verify(&self, signal: &Signal, ctx: &Context) -> Verdict {
        let started = std::time::Instant::now();

        if self.gates.is_empty() {
            return Verdict::pass(&self.name)
                .with_detail("ParallelGate: no inner gates")
                .with_duration(elapsed_ms(started));
        }

        // Run all inner gates concurrently using FuturesUnordered, bounded
        // by a semaphore when max_concurrency is set. Each future carries
        // its declaration-order index so results are reassembled in the
        // original push order regardless of completion order.
        //
        // Dropping this future (cancellation) drops the FuturesUnordered,
        // which drops all incomplete inner futures -- no work leaks.
        let concurrency = self.effective_concurrency();
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(concurrency));

        let mut futs = FuturesUnordered::new();
        for (idx, gate) in self.gates.iter().enumerate() {
            let sem = semaphore.clone();
            futs.push(async move {
                let _permit = sem.acquire().await;
                let verdict = gate.verify(signal, ctx).await;
                (idx, verdict)
            });
        }

        // Collect results as they complete, then sort by index.
        let mut indexed: Vec<(usize, Verdict)> = Vec::with_capacity(self.gates.len());
        while let Some((idx, verdict)) = futs.next().await {
            indexed.push((idx, verdict));
        }
        indexed.sort_by_key(|(idx, _)| *idx);
        let verdicts: Vec<Verdict> = indexed.into_iter().map(|(_, v)| v).collect();

        // Aggregate: min score, fail if any failed.
        let min_score = verdicts
            .iter()
            .map(|v| v.score)
            .fold(f32::INFINITY, f32::min);

        let failed: Vec<&str> = verdicts
            .iter()
            .filter(|v| !v.passed)
            .map(|v| v.gate.as_str())
            .collect();

        let detail = verdicts
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let status = if v.passed { "pass" } else { "fail" };
                format!("  {}. [{status}] {} (score={:.2})", i + 1, v.gate, v.score)
            })
            .collect::<Vec<_>>()
            .join("\n");

        let elapsed = elapsed_ms(started);

        if failed.is_empty() {
            Verdict::pass(&self.name)
                .with_score(min_score)
                .with_detail(format!(
                    "ParallelGate: {}/{} passed\n{detail}",
                    verdicts.len(),
                    verdicts.len()
                ))
                .with_duration(elapsed)
        } else {
            let reason = format!(
                "{} of {} gates failed: {}",
                failed.len(),
                verdicts.len(),
                failed.join(", ")
            );
            Verdict::fail(&self.name, reason)
                .with_score(min_score)
                .with_detail(format!(
                    "ParallelGate: {}/{} passed\n{detail}",
                    verdicts.len() - failed.len(),
                    verdicts.len()
                ))
                .with_duration(elapsed)
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// ─── VotingGate ──────────────────────────────────────────────────────────────

/// Runs M gates and requires N-of-M to pass.
///
/// Aggregate score = mean of passing verdicts' scores. Use when multiple
/// reviewers must agree (e.g., 2-of-3 code review gates).
pub struct VotingGate {
    gates: Vec<Box<dyn Verify>>,
    required_passes: usize,
    name: String,
}

impl VotingGate {
    /// Create a new voting gate requiring `required_passes` out of M gates to pass.
    #[must_use]
    pub fn new(name: impl Into<String>, required_passes: usize) -> Self {
        Self {
            gates: Vec::new(),
            required_passes: required_passes.max(1),
            name: name.into(),
        }
    }

    /// Append an inner gate.
    pub fn push(&mut self, gate: Box<dyn Verify>) {
        self.gates.push(gate);
    }

    /// Chainable gate append.
    #[must_use]
    pub fn with_gate(mut self, gate: Box<dyn Verify>) -> Self {
        self.push(gate);
        self
    }

    /// Number of inner gates.
    #[must_use]
    pub fn len(&self) -> usize {
        self.gates.len()
    }

    /// Whether no inner gates are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.gates.is_empty()
    }
}

impl fmt::Debug for VotingGate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VotingGate")
            .field("name", &self.name)
            .field("gates", &self.gates.len())
            .field("required_passes", &self.required_passes)
            .finish()
    }
}

impl roko_core::Cell for VotingGate {
    fn cell_id(&self) -> &str {
        "voting-gate"
    }
    fn cell_name(&self) -> &str {
        "VotingGate"
    }
    fn protocols(&self) -> Vec<roko_core::ProtocolId> {
        vec![roko_core::ProtocolId::Verify]
    }
}

#[async_trait]

impl Verify for VotingGate {
    async fn verify(&self, signal: &Signal, ctx: &Context) -> Verdict {
        let started = std::time::Instant::now();

        if self.gates.is_empty() {
            return Verdict::pass(&self.name)
                .with_detail("VotingGate: no inner gates")
                .with_duration(elapsed_ms(started));
        }

        // Run all gates and collect verdicts.
        let mut verdicts = Vec::with_capacity(self.gates.len());
        for gate in &self.gates {
            verdicts.push(gate.verify(signal, ctx).await);
        }

        let pass_count = verdicts.iter().filter(|v| v.passed).count();
        let passing_scores: Vec<f32> = verdicts
            .iter()
            .filter(|v| v.passed)
            .map(|v| v.score)
            .collect();

        let mean_passing_score = if passing_scores.is_empty() {
            0.0
        } else {
            passing_scores.iter().sum::<f32>() / passing_scores.len() as f32
        };

        let detail = verdicts
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let status = if v.passed { "pass" } else { "fail" };
                format!("  {}. [{status}] {} (score={:.2})", i + 1, v.gate, v.score)
            })
            .collect::<Vec<_>>()
            .join("\n");

        let elapsed = elapsed_ms(started);
        let overall_passed = pass_count >= self.required_passes;

        if overall_passed {
            Verdict::pass(&self.name)
                .with_score(mean_passing_score)
                .with_detail(format!(
                    "VotingGate: {pass_count}/{total} passed (required {required})\n{detail}",
                    total = self.gates.len(),
                    required = self.required_passes,
                ))
                .with_duration(elapsed)
        } else {
            let reason = format!(
                "voting failed: {pass_count}/{total} passed, required {required}",
                total = self.gates.len(),
                required = self.required_passes,
            );
            Verdict::fail(&self.name, reason)
                .with_score(mean_passing_score)
                .with_detail(format!(
                    "VotingGate: {pass_count}/{total} passed (required {required})\n{detail}",
                    total = self.gates.len(),
                    required = self.required_passes,
                ))
                .with_duration(elapsed)
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

// ─── FallbackGate ────────────────────────────────────────────────────────────

/// Tries a primary gate; if it fails, tries a fallback.
///
/// The first passing verdict wins. Use when you want to try a fast check
/// first and fall back to a more thorough one on failure.
pub struct FallbackGate {
    primary: Box<dyn Verify>,
    fallback: Box<dyn Verify>,
    name: String,
}

impl FallbackGate {
    /// Create a fallback gate with the given primary and fallback.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        primary: Box<dyn Verify>,
        fallback: Box<dyn Verify>,
    ) -> Self {
        Self {
            primary,
            fallback,
            name: name.into(),
        }
    }
}

impl fmt::Debug for FallbackGate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FallbackGate")
            .field("name", &self.name)
            .field("primary", &self.primary.name())
            .field("fallback", &self.fallback.name())
            .finish()
    }
}

impl roko_core::Cell for FallbackGate {
    fn cell_id(&self) -> &str {
        "fallback-gate"
    }
    fn cell_name(&self) -> &str {
        "FallbackGate"
    }
    fn protocols(&self) -> Vec<roko_core::ProtocolId> {
        vec![roko_core::ProtocolId::Verify]
    }
}

#[async_trait]

impl Verify for FallbackGate {
    async fn verify(&self, signal: &Signal, ctx: &Context) -> Verdict {
        let started = std::time::Instant::now();

        // Try primary first.
        let primary_verdict = self.primary.verify(signal, ctx).await;
        if primary_verdict.passed {
            return primary_verdict.with_duration(elapsed_ms(started));
        }

        // Primary failed — try fallback.
        let fallback_verdict = self.fallback.verify(signal, ctx).await;
        let elapsed = elapsed_ms(started);

        if fallback_verdict.passed {
            let mut v = fallback_verdict;
            v = v.with_detail(format!(
                "FallbackGate '{}': primary '{}' failed, fallback '{}' passed\nPrimary reason: {}",
                self.name,
                self.primary.name(),
                self.fallback.name(),
                primary_verdict.reason,
            ));
            v.with_duration(elapsed)
        } else {
            // Both failed.
            Verdict::fail(
                &self.name,
                format!(
                    "both primary '{}' and fallback '{}' failed",
                    self.primary.name(),
                    self.fallback.name(),
                ),
            )
            .with_score(fallback_verdict.score.min(primary_verdict.score))
            .with_detail(format!(
                "FallbackGate '{}': both gates failed\n  Primary ({}): {}\n  Fallback ({}): {}",
                self.name,
                self.primary.name(),
                primary_verdict.reason,
                self.fallback.name(),
                fallback_verdict.reason,
            ))
            .with_duration(elapsed)
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

fn elapsed_ms(started: std::time::Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use roko_core::{Body, Context, Kind, Signal, Verdict, Verify};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockGate {
        gate_name: String,
        pass: bool,
        calls: Arc<AtomicUsize>,
    }

    impl MockGate {
        fn new(name: &str, pass: bool) -> Self {
            Self {
                gate_name: name.to_string(),
                pass,
                calls: Arc::new(AtomicUsize::new(0)),
            }
        }

        fn calls_handle(&self) -> Arc<AtomicUsize> {
            Arc::clone(&self.calls)
        }
    }

    impl roko_core::Cell for MockGate {
        fn cell_id(&self) -> &str {
            "mock-gate-comp-test"
        }
        fn cell_name(&self) -> &str {
            "MockGate"
        }
        fn protocols(&self) -> Vec<roko_core::ProtocolId> {
            vec![roko_core::ProtocolId::Verify]
        }
    }

    #[async_trait]

    impl Verify for MockGate {
        async fn verify(&self, _signal: &Signal, _ctx: &Context) -> Verdict {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.pass {
                Verdict::pass(&self.gate_name).with_score(0.9)
            } else {
                Verdict::fail(&self.gate_name, "mock failure").with_score(0.2)
            }
        }

        fn name(&self) -> &str {
            &self.gate_name
        }
    }

    fn signal() -> Signal {
        Signal::builder(Kind::Task).body(Body::empty()).build()
    }

    fn ctx() -> Context {
        Context::at(0)
    }

    // ─── ParallelGate tests ──────────────────────────────────────────

    #[tokio::test]
    async fn parallel_all_pass() {
        let a = MockGate::new("a", true);
        let b = MockGate::new("b", true);
        let ac = a.calls_handle();
        let bc = b.calls_handle();
        let gate = ParallelGate::new("par")
            .with_gate(Box::new(a))
            .with_gate(Box::new(b));
        let v = gate.verify(&signal(), &ctx()).await;
        assert!(v.passed);
        assert_eq!(ac.load(Ordering::SeqCst), 1);
        assert_eq!(bc.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn parallel_any_fail_causes_failure() {
        let a = MockGate::new("a", true);
        let b = MockGate::new("b", false);
        let c = MockGate::new("c", true);
        let ac = a.calls_handle();
        let bc = b.calls_handle();
        let cc = c.calls_handle();
        let gate = ParallelGate::new("par")
            .with_gate(Box::new(a))
            .with_gate(Box::new(b))
            .with_gate(Box::new(c));
        let v = gate.verify(&signal(), &ctx()).await;
        assert!(!v.passed);
        // All gates should have run.
        assert_eq!(ac.load(Ordering::SeqCst), 1);
        assert_eq!(bc.load(Ordering::SeqCst), 1);
        assert_eq!(cc.load(Ordering::SeqCst), 1);
        assert!(v.reason.contains("b"));
    }

    #[tokio::test]
    async fn parallel_aggregate_uses_min_score() {
        let a = MockGate::new("a", true);
        let b = MockGate::new("b", true);
        let gate = ParallelGate::new("par")
            .with_gate(Box::new(a))
            .with_gate(Box::new(b));
        let v = gate.verify(&signal(), &ctx()).await;
        assert!(v.passed);
        assert_eq!(v.score, 0.9); // both gates return 0.9
    }

    #[tokio::test]
    async fn parallel_empty_passes() {
        let gate = ParallelGate::new("empty");
        let v = gate.verify(&signal(), &ctx()).await;
        assert!(v.passed);
        assert!(gate.is_empty());
    }

    #[tokio::test]
    async fn parallel_max_concurrency_builder() {
        let gate = ParallelGate::new("cap")
            .with_max_concurrency(NonZeroUsize::new(2).unwrap())
            .with_gate(Box::new(MockGate::new("a", true)))
            .with_gate(Box::new(MockGate::new("b", true)))
            .with_gate(Box::new(MockGate::new("c", true)));
        assert_eq!(gate.max_concurrency().unwrap().get(), 2);
        let v = gate.verify(&signal(), &ctx()).await;
        assert!(v.passed);
    }

    #[tokio::test]
    async fn parallel_max_concurrency_caps_at_gate_count() {
        // Cap of 10 with only 2 gates -> effective concurrency is 2.
        let gate = ParallelGate::new("cap")
            .with_max_concurrency(NonZeroUsize::new(10).unwrap())
            .with_gate(Box::new(MockGate::new("a", true)))
            .with_gate(Box::new(MockGate::new("b", true)));
        assert_eq!(gate.effective_concurrency(), 2);
        let v = gate.verify(&signal(), &ctx()).await;
        assert!(v.passed);
    }

    #[tokio::test]
    async fn parallel_default_unbounded_concurrency() {
        let gate = ParallelGate::new("unb");
        assert!(gate.max_concurrency().is_none());
    }

    /// A gate that records the instant it started executing and waits on a
    /// barrier. This proves that multiple gates were executing concurrently.
    struct BarrierGate {
        gate_name: String,
        pass: bool,
        started: Arc<std::sync::Mutex<Vec<std::time::Instant>>>,
        barrier: Arc<tokio::sync::Barrier>,
    }

    impl roko_core::Cell for BarrierGate {
        fn cell_id(&self) -> &str {
            "barrier-gate-test"
        }
        fn cell_name(&self) -> &str {
            "BarrierGate"
        }
        fn protocols(&self) -> Vec<roko_core::ProtocolId> {
            vec![roko_core::ProtocolId::Verify]
        }
    }

    #[async_trait]
    impl Verify for BarrierGate {
        async fn verify(&self, _signal: &Signal, _ctx: &Context) -> Verdict {
            // Record that we started.
            if let Ok(mut started) = self.started.lock() {
                started.push(std::time::Instant::now());
            }
            // Wait for all peers to arrive -- proves concurrent execution.
            self.barrier.wait().await;
            if self.pass {
                Verdict::pass(&self.gate_name).with_score(0.9)
            } else {
                Verdict::fail(&self.gate_name, "barrier fail").with_score(0.2)
            }
        }
        fn name(&self) -> &str {
            &self.gate_name
        }
    }

    #[tokio::test]
    async fn parallel_proves_concurrent_start() {
        // Use a Barrier(3) -- all 3 gates must reach the barrier before any
        // can proceed. If execution were serial, this would deadlock.
        let barrier = Arc::new(tokio::sync::Barrier::new(3));
        let started = Arc::new(std::sync::Mutex::new(Vec::new()));

        let gate = ParallelGate::new("overlap")
            .with_gate(Box::new(BarrierGate {
                gate_name: "a".into(),
                pass: true,
                started: started.clone(),
                barrier: barrier.clone(),
            }))
            .with_gate(Box::new(BarrierGate {
                gate_name: "b".into(),
                pass: true,
                started: started.clone(),
                barrier: barrier.clone(),
            }))
            .with_gate(Box::new(BarrierGate {
                gate_name: "c".into(),
                pass: true,
                started: started.clone(),
                barrier: barrier.clone(),
            }));

        let v = gate.verify(&signal(), &ctx()).await;
        assert!(v.passed, "all gates should pass");
        let starts = started.lock().unwrap();
        assert_eq!(starts.len(), 3, "all three gates must have started");
    }

    #[tokio::test]
    async fn parallel_declaration_order_preserved() {
        // Three gates with different sleep durations -- the one pushed first
        // should appear first in the detail output regardless of when it
        // completed.
        struct SlowGate {
            gate_name: String,
            delay: std::time::Duration,
        }

        impl roko_core::Cell for SlowGate {
            fn cell_id(&self) -> &str {
                "slow-gate-test"
            }
            fn cell_name(&self) -> &str {
                "SlowGate"
            }
            fn protocols(&self) -> Vec<roko_core::ProtocolId> {
                vec![roko_core::ProtocolId::Verify]
            }
        }

        #[async_trait]
        impl Verify for SlowGate {
            async fn verify(&self, _signal: &Signal, _ctx: &Context) -> Verdict {
                tokio::time::sleep(self.delay).await;
                Verdict::pass(&self.gate_name).with_score(1.0)
            }
            fn name(&self) -> &str {
                &self.gate_name
            }
        }

        let gate = ParallelGate::new("order")
            .with_gate(Box::new(SlowGate {
                gate_name: "first".into(),
                delay: std::time::Duration::from_millis(30),
            }))
            .with_gate(Box::new(SlowGate {
                gate_name: "second".into(),
                delay: std::time::Duration::from_millis(10),
            }))
            .with_gate(Box::new(SlowGate {
                gate_name: "third".into(),
                delay: std::time::Duration::from_millis(20),
            }));

        let v = gate.verify(&signal(), &ctx()).await;
        assert!(v.passed);
        let detail = v.detail.as_deref().unwrap_or("");
        let pos_first = detail.find("first").expect("first in detail");
        let pos_second = detail.find("second").expect("second in detail");
        let pos_third = detail.find("third").expect("third in detail");
        assert!(
            pos_first < pos_second && pos_second < pos_third,
            "verdicts must be in declaration order: first < second < third\n{detail}"
        );
    }

    #[tokio::test]
    async fn parallel_failure_aggregation_all_run() {
        // When one gate fails, the others must still run and their results
        // must still appear in the aggregate.
        let barrier = Arc::new(tokio::sync::Barrier::new(3));
        let started = Arc::new(std::sync::Mutex::new(Vec::new()));

        let gate = ParallelGate::new("fail-agg")
            .with_gate(Box::new(BarrierGate {
                gate_name: "ok1".into(),
                pass: true,
                started: started.clone(),
                barrier: barrier.clone(),
            }))
            .with_gate(Box::new(BarrierGate {
                gate_name: "bad".into(),
                pass: false,
                started: started.clone(),
                barrier: barrier.clone(),
            }))
            .with_gate(Box::new(BarrierGate {
                gate_name: "ok2".into(),
                pass: true,
                started: started.clone(),
                barrier: barrier.clone(),
            }));

        let v = gate.verify(&signal(), &ctx()).await;
        assert!(!v.passed);
        assert!(v.reason.contains("bad"));
        // All three must have run.
        let starts = started.lock().unwrap();
        assert_eq!(starts.len(), 3);
    }

    // ─── VotingGate tests ────────────────────────────────────────────

    #[tokio::test]
    async fn voting_passes_with_enough_votes() {
        let a = MockGate::new("a", true);
        let b = MockGate::new("b", false);
        let c = MockGate::new("c", true);
        // 2-of-3 required.
        let gate = VotingGate::new("vote", 2)
            .with_gate(Box::new(a))
            .with_gate(Box::new(b))
            .with_gate(Box::new(c));
        let v = gate.verify(&signal(), &ctx()).await;
        assert!(v.passed, "2/3 should pass with required=2");
    }

    #[tokio::test]
    async fn voting_fails_without_enough_votes() {
        let a = MockGate::new("a", true);
        let b = MockGate::new("b", false);
        let c = MockGate::new("c", false);
        // 2-of-3 required but only 1 passes.
        let gate = VotingGate::new("vote", 2)
            .with_gate(Box::new(a))
            .with_gate(Box::new(b))
            .with_gate(Box::new(c));
        let v = gate.verify(&signal(), &ctx()).await;
        assert!(!v.passed, "1/3 should fail with required=2");
    }

    #[tokio::test]
    async fn voting_score_is_mean_of_passing() {
        let a = MockGate::new("a", true);
        let b = MockGate::new("b", true);
        let c = MockGate::new("c", false);
        let gate = VotingGate::new("vote", 2)
            .with_gate(Box::new(a))
            .with_gate(Box::new(b))
            .with_gate(Box::new(c));
        let v = gate.verify(&signal(), &ctx()).await;
        assert!(v.passed);
        // Both passing mocks return 0.9, mean = 0.9.
        assert!((v.score - 0.9).abs() < 0.01);
    }

    #[tokio::test]
    async fn voting_empty_passes() {
        let gate = VotingGate::new("empty", 1);
        let v = gate.verify(&signal(), &ctx()).await;
        assert!(v.passed);
    }

    // ─── FallbackGate tests ──────────────────────────────────────────

    #[tokio::test]
    async fn fallback_uses_primary_when_it_passes() {
        let primary = MockGate::new("primary", true);
        let fallback = MockGate::new("fallback", true);
        let fc = fallback.calls_handle();
        let gate = FallbackGate::new("fb", Box::new(primary), Box::new(fallback));
        let v = gate.verify(&signal(), &ctx()).await;
        assert!(v.passed);
        assert_eq!(fc.load(Ordering::SeqCst), 0, "fallback should not run");
    }

    #[tokio::test]
    async fn fallback_uses_fallback_on_primary_failure() {
        let primary = MockGate::new("primary", false);
        let fallback = MockGate::new("fallback", true);
        let fc = fallback.calls_handle();
        let gate = FallbackGate::new("fb", Box::new(primary), Box::new(fallback));
        let v = gate.verify(&signal(), &ctx()).await;
        assert!(v.passed, "fallback should rescue");
        assert_eq!(fc.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn fallback_both_fail() {
        let primary = MockGate::new("primary", false);
        let fallback = MockGate::new("fallback", false);
        let gate = FallbackGate::new("fb", Box::new(primary), Box::new(fallback));
        let v = gate.verify(&signal(), &ctx()).await;
        assert!(!v.passed);
        assert!(v.reason.contains("primary"));
        assert!(v.reason.contains("fallback"));
    }
}
