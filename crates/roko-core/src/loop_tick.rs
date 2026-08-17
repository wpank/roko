//! A reusable signal selection, composition, verification, and persistence helper.
//!
//! This module does **not** own Roko's production execution loop. It composes a
//! useful subset of the core traits for callers that already own orchestration:
//! query, route, compose, verify, persist, and react. It does not launch an
//! agent/provider (ACT), publish to a Bus (BROADCAST), enforce iteration/time/
//! cost limits, or replace the CLI `WorkflowEngine`, Runner-v2, or Graph
//! runtimes.
//!
//! ```text
//!   candidates = substrate.query(q, ctx)
//!       ↓
//!   selection = router.select(candidates, ctx)
//!       ↓
//!   composed  = composer.compose([selection], budget, scorer, ctx)
//!       ↓
//!   verdict   = gate.verify(composed, ctx)
//!       ↓
//!   if passed: substrate.put(composed) + policy.decide(stream, ctx)
//! ```
//!
//! The historical [`loop_tick`] name is retained as a deprecated compatibility
//! alias. New code should call [`select_compose_verify_persist`] and should use
//! a runtime-owned coordinator when ACT, BROADCAST, cancellation, or resource
//! enforcement is required.

use serde::{Deserialize, Serialize};

use crate::{
    Budget, Compose, Context, Engram, Query, React, Route, Store, Verdict, Verify, error::Result,
};

/// Historical configuration accepted by [`loop_tick_with_config`].
///
/// These fields were never enforced by this helper. The type remains readable
/// for source compatibility; production limits belong to the runtime that owns
/// provider execution and cancellation.
#[deprecated(
    since = "0.1.0",
    note = "loop_tick is a compatibility helper and does not enforce TickConfig; use a runtime-owned coordinator"
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickConfig {
    /// Maximum number of turns (candidates examined) before stopping.
    /// `None` means unlimited.
    pub max_turns: Option<u64>,
    /// Timeout in seconds for the entire tick. `None` means no timeout.
    pub timeout_secs: Option<u64>,
    /// Budget ceiling in USD. `None` means no budget limit.
    pub budget_usd: Option<f64>,
    /// Whether to emit verbose tracing for this tick.
    pub verbose: bool,
}

#[allow(deprecated)]
impl Default for TickConfig {
    fn default() -> Self {
        Self {
            max_turns: None,
            timeout_secs: None,
            budget_usd: None,
            verbose: false,
        }
    }
}

#[allow(deprecated)]
impl TickConfig {
    /// Create a config with no limits (equivalent to `Default`).
    #[must_use]
    pub fn unlimited() -> Self {
        Self::default()
    }

    /// Set the maximum number of turns.
    #[must_use]
    pub const fn with_max_turns(mut self, max: u64) -> Self {
        self.max_turns = Some(max);
        self
    }

    /// Set the timeout in seconds.
    #[must_use]
    pub const fn with_timeout_secs(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }

    /// Set the budget ceiling in USD.
    #[must_use]
    pub fn with_budget_usd(mut self, usd: f64) -> Self {
        self.budget_usd = Some(usd);
        self
    }

    /// Enable verbose tracing.
    #[must_use]
    pub const fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
}

/// Outcome of one signal selection/composition/verification/persistence pass.
#[derive(Debug)]
pub struct SignalSelectionOutcome {
    /// How many candidates the substrate returned.
    pub candidates_examined: usize,
    /// The composed signal (if one was produced).
    pub composed: Option<Engram>,
    /// The gate's verdict (if composition happened).
    pub verdict: Option<Verdict>,
    /// Signals emitted by the policy.
    pub emitted: Vec<Engram>,
    /// Content hashes of signals written back to substrate.
    pub written: Vec<crate::ContentHash>,
}

impl SignalSelectionOutcome {
    /// Did this tick's work pass its gate?
    #[must_use]
    pub fn passed(&self) -> bool {
        self.verdict.as_ref().is_some_and(|v| v.passed)
    }

    /// Did this tick do any work (query returned candidates)?
    #[must_use]
    pub const fn did_work(&self) -> bool {
        self.candidates_examined > 0
    }
}

fn ensure_lineage(mut signal: Engram, parent: crate::ContentHash) -> Engram {
    if !signal.lineage.contains(&parent) {
        signal.lineage.push(parent);
    }
    signal
}

/// Select, compose, verify, persist, and react to one candidate Signal.
///
/// # Steps
///
/// 1. Query the substrate for candidates matching `query`.
/// 2. Ask the router to select one (returns early if none selected).
/// 3. Ask the composer to build a new signal from the selection.
/// 4. Ask the gate to verify the composed signal.
/// 5. If it passes: write it back to the substrate and run the policy.
///
/// # Errors
///
/// Propagates errors from the substrate and composer. Verify failures are
/// *not* errors — they return a failing [`Verdict`] in the outcome.
#[allow(clippy::similar_names, clippy::too_many_arguments)]
pub async fn select_compose_verify_persist(
    substrate: &dyn Store,
    scorer: &dyn crate::traits::Score,
    router: &dyn Route,
    composer: &dyn Compose,
    gate: &dyn Verify,
    policy: &dyn React,
    query: &Query,
    budget: &Budget,
    ctx: &Context,
) -> Result<SignalSelectionOutcome> {
    // 1. Query the substrate for candidates.
    let candidates = substrate.query(query, ctx).await?;
    let candidates_examined = candidates.len();

    if candidates.is_empty() {
        return Ok(SignalSelectionOutcome {
            candidates_examined: 0,
            composed: None,
            verdict: None,
            emitted: Vec::new(),
            written: Vec::new(),
        });
    }

    // 2. Route selects one candidate (or bails).
    let Some(selection) = router.select(&candidates, ctx) else {
        return Ok(SignalSelectionOutcome {
            candidates_examined,
            composed: None,
            verdict: None,
            emitted: Vec::new(),
            written: Vec::new(),
        });
    };

    // 3. Find the selected signal among candidates; feed it to the composer.
    let Some(chosen) = candidates
        .iter()
        .find(|s| s.id == selection.chosen)
        .cloned()
    else {
        return Ok(SignalSelectionOutcome {
            candidates_examined,
            composed: None,
            verdict: None,
            emitted: Vec::new(),
            written: Vec::new(),
        });
    };
    let composed = ensure_lineage(
        composer.compose(&[chosen.clone()], budget, scorer, ctx)?,
        chosen.id,
    );

    // 4. Verify verifies the composition.
    let verdict = gate.verify(&composed, ctx).await;

    // 5. If passed, persist and run policy reaction.
    let mut written = Vec::new();
    let mut emitted = Vec::new();
    if verdict.passed {
        let id = substrate.put(composed.clone()).await?;
        written.push(id);

        // React sees the new signal and may produce reactions.
        let reactions = policy.decide(std::slice::from_ref(&composed), ctx);
        for r in reactions {
            let id = substrate.put(r.clone()).await?;
            written.push(id);
            emitted.push(r);
        }
    }

    Ok(SignalSelectionOutcome {
        candidates_examined,
        composed: Some(composed),
        verdict: Some(verdict),
        emitted,
        written,
    })
}

/// Historical outcome name retained for compatibility.
#[deprecated(
    since = "0.1.0",
    note = "use SignalSelectionOutcome; this helper is not the production universal loop"
)]
pub type TickOutcome = SignalSelectionOutcome;

/// Historical helper name retained for compatibility.
///
/// # Errors
///
/// Propagates errors from the substrate and composer.
#[deprecated(
    since = "0.1.0",
    note = "use select_compose_verify_persist; production execution is owned by WorkflowEngine, Runner-v2, or Graph"
)]
#[allow(clippy::similar_names, clippy::too_many_arguments)]
pub async fn loop_tick(
    substrate: &dyn Store,
    scorer: &dyn crate::traits::Score,
    router: &dyn Route,
    composer: &dyn Compose,
    gate: &dyn Verify,
    policy: &dyn React,
    query: &Query,
    budget: &Budget,
    ctx: &Context,
) -> Result<SignalSelectionOutcome> {
    select_compose_verify_persist(
        substrate, scorer, router, composer, gate, policy, query, budget, ctx,
    )
    .await
}

/// Historical configured helper retained for compatibility.
///
/// `tick_config` is intentionally ignored because this helper does not own an
/// ACT loop, provider budget, or cancellation boundary.
///
/// # Errors
///
/// Propagates errors from the substrate and composer.
#[deprecated(
    since = "0.1.0",
    note = "TickConfig was never enforced; use select_compose_verify_persist plus a runtime-owned coordinator"
)]
#[allow(deprecated)]
#[allow(clippy::similar_names, clippy::too_many_arguments)]
pub async fn loop_tick_with_config(
    substrate: &dyn Store,
    scorer: &dyn crate::traits::Score,
    router: &dyn Route,
    composer: &dyn Compose,
    gate: &dyn Verify,
    policy: &dyn React,
    query: &Query,
    budget: &Budget,
    ctx: &Context,
    _tick_config: &TickConfig,
) -> Result<SignalSelectionOutcome> {
    select_compose_verify_persist(
        substrate, scorer, router, composer, gate, policy, query, budget, ctx,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Body, Budget, ContentHash, Context, Engram, Kind, Provenance, Query, Result, Score,
        Selection, verdict::Verdict,
    };
    use async_trait::async_trait;
    use parking_lot::Mutex;
    use std::sync::Arc;

    struct TestSubstrate {
        candidate: Engram,
        written: Arc<Mutex<Vec<Engram>>>,
    }

    #[async_trait]
    impl Store for TestSubstrate {
        async fn put(&self, signal: Engram) -> Result<ContentHash> {
            self.written.lock().push(signal.clone());
            Ok(signal.id)
        }

        async fn get(&self, _id: &ContentHash) -> Result<Option<Engram>> {
            Ok(None)
        }

        async fn query(&self, _q: &Query, _ctx: &Context) -> Result<Vec<Engram>> {
            Ok(vec![self.candidate.clone()])
        }

        async fn prune(&self, _threshold: f32, _ctx: &Context) -> Result<usize> {
            Ok(0)
        }
    }

    struct TestRouter {
        choice: Selection,
    }

    impl crate::cell::Cell for TestRouter {
        fn cell_id(&self) -> &str {
            "test-router"
        }
        fn cell_name(&self) -> &str {
            "TestRouter"
        }
        fn protocols(&self) -> Vec<crate::cell::ProtocolId> {
            vec![crate::cell::ProtocolId::Route]
        }
    }

    impl Route for TestRouter {
        fn select(&self, _candidates: &[Engram], _ctx: &Context) -> Option<Selection> {
            Some(self.choice.clone())
        }

        fn feedback(&self, _outcome: &crate::Outcome) {}

        fn name(&self) -> &'static str {
            "test_router"
        }
    }

    struct PassthroughComposer;

    impl crate::cell::Cell for PassthroughComposer {
        fn cell_id(&self) -> &str {
            "passthrough-composer"
        }
        fn cell_name(&self) -> &str {
            "PassthroughComposer"
        }
        fn protocols(&self) -> Vec<crate::cell::ProtocolId> {
            vec![crate::cell::ProtocolId::Compose]
        }
    }

    impl Compose for PassthroughComposer {
        fn compose(
            &self,
            signals: &[Engram],
            _budget: &Budget,
            _scorer: &dyn crate::traits::Score,
            _ctx: &Context,
        ) -> Result<Engram> {
            Ok(Engram::builder(Kind::Prompt)
                .body(Body::text("composed"))
                .provenance(Provenance::trusted("composer"))
                .score(Score::NEUTRAL)
                .created_at_ms(0)
                .lineage(
                    signals
                        .iter()
                        .flat_map(|signal| signal.lineage.iter().copied()),
                )
                .build())
        }

        fn name(&self) -> &'static str {
            "passthrough"
        }
    }

    struct PassGate;

    #[async_trait]
    impl crate::cell::Cell for PassGate {
        fn cell_id(&self) -> &str {
            "pass-gate"
        }
        fn cell_name(&self) -> &str {
            "PassGate"
        }
        fn protocols(&self) -> Vec<crate::cell::ProtocolId> {
            vec![crate::cell::ProtocolId::Verify]
        }
    }

    #[async_trait]
    impl Verify for PassGate {
        async fn verify(&self, _signal: &Engram, _ctx: &Context) -> Verdict {
            Verdict::pass("pass_gate")
        }

        fn name(&self) -> &'static str {
            "pass_gate"
        }
    }

    struct NoopPolicy;

    impl crate::cell::Cell for NoopPolicy {
        fn cell_id(&self) -> &str {
            "noop-policy"
        }
        fn cell_name(&self) -> &str {
            "NoopPolicy"
        }
        fn protocols(&self) -> Vec<crate::cell::ProtocolId> {
            vec![crate::cell::ProtocolId::React]
        }
    }

    impl React for NoopPolicy {
        fn decide(&self, _stream: &[Engram], _ctx: &Context) -> Vec<Engram> {
            Vec::new()
        }

        fn name(&self) -> &'static str {
            "noop_policy"
        }
    }

    struct ZeroScorer;

    impl crate::cell::Cell for ZeroScorer {
        fn cell_id(&self) -> &str {
            "zero-scorer"
        }
        fn cell_name(&self) -> &str {
            "ZeroScorer"
        }
        fn protocols(&self) -> Vec<crate::cell::ProtocolId> {
            vec![crate::cell::ProtocolId::Score]
        }
    }

    impl crate::traits::Score for ZeroScorer {
        fn score(&self, _signal: &Engram, _ctx: &Context) -> crate::Score {
            crate::Score::NEUTRAL
        }

        fn name(&self) -> &'static str {
            "zero_scorer"
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn signal_selection_adds_missing_upstream_lineage() {
        let candidate = Engram::builder(Kind::Task)
            .body(Body::text("task"))
            .provenance(Provenance::trusted("source"))
            .created_at_ms(0)
            .build();
        let substrate = TestSubstrate {
            candidate: candidate.clone(),
            written: Arc::new(Mutex::new(Vec::new())),
        };
        let router = TestRouter {
            choice: Selection::new(candidate.id, "test_router"),
        };
        let composer = PassthroughComposer;
        let gate = PassGate;
        let policy = NoopPolicy;
        let scorer = ZeroScorer;
        let budget = Budget::unlimited();
        let ctx = Context::now();

        let outcome = select_compose_verify_persist(
            &substrate,
            &scorer,
            &router,
            &composer,
            &gate,
            &policy,
            &Query::all(),
            &budget,
            &ctx,
        )
        .await
        .unwrap();

        assert!(outcome.passed());
        let composed = outcome.composed.as_ref().expect("composed signal");
        assert!(composed.lineage.contains(&candidate.id));
        assert_eq!(substrate.written.lock().len(), 1);
        assert!(substrate.written.lock()[0].lineage.contains(&candidate.id));
    }
}
