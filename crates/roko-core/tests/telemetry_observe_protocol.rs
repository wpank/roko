//! Contract tests for event-oriented telemetry observation.

use roko_core::{
    Body, Kind, LensScope, ObservableEvent, ObservableEventKind, Result, Signal, TelemetryObserve,
    Verdict,
};

fn signal(label: &str) -> Signal {
    Signal::builder(Kind::Metric)
        .body(Body::text(label))
        .build()
}

fn all_events() -> Vec<ObservableEvent> {
    let created = signal("created");
    vec![
        ObservableEvent::SignalCreated(created.clone()),
        ObservableEvent::SignalScored("s".into(), "score".into()),
        ObservableEvent::SignalRouted("s".into(), "route".into()),
        ObservableEvent::SignalVerified("s".into(), Verdict::pass("gate")),
        ObservableEvent::SignalComposed(vec!["a".into()], created),
        ObservableEvent::SignalDemurrageApplied("s".into(), 0.1),
        ObservableEvent::SignalPromoted("s".into(), "working".into(), "episodic".into()),
        ObservableEvent::SignalPruned("s".into()),
        ObservableEvent::CellStarted {
            block: "cell".into(),
            run: "run".into(),
            input_hash: "hash".into(),
        },
        ObservableEvent::CellCompleted {
            block: "cell".into(),
            run: "run".into(),
            duration_ms: 2,
            cost_usd: 0.01,
        },
        ObservableEvent::CellFailed {
            block: "cell".into(),
            run: "run".into(),
            error: "failed".into(),
        },
        ObservableEvent::CellRetried {
            block: "cell".into(),
            run: "run".into(),
            attempt: 2,
            reason: "retry".into(),
        },
        ObservableEvent::CellCancelled {
            block: "cell".into(),
            run: "run".into(),
        },
        ObservableEvent::CellPredictionPublished {
            block: "cell".into(),
            prediction: "prediction".into(),
        },
        ObservableEvent::CellCalibrationReceived {
            block: "cell".into(),
            error: 0.2,
        },
        ObservableEvent::GraphStarted {
            graph: "graph".into(),
            run: "run".into(),
            input_hash: "hash".into(),
        },
        ObservableEvent::GraphNodeCompleted {
            graph: "graph".into(),
            run: "run".into(),
            node: "node".into(),
            duration_ms: 3,
        },
        ObservableEvent::GraphCompleted {
            graph: "graph".into(),
            run: "run".into(),
            duration_ms: 4,
            cost_usd: 0.02,
        },
        ObservableEvent::GraphFailed {
            graph: "graph".into(),
            run: "run".into(),
            error: "failed".into(),
        },
        ObservableEvent::GraphPaused {
            graph: "graph".into(),
            run: "run".into(),
            reason: "pause".into(),
        },
        ObservableEvent::GraphResumed {
            graph: "graph".into(),
            run: "run".into(),
        },
        ObservableEvent::AgentTick {
            agent: "agent".into(),
            regime: "stable".into(),
            prediction_error: 0.1,
            vitality: 0.9,
        },
        ObservableEvent::AgentRegimeChange {
            agent: "agent".into(),
            old: "stable".into(),
            new_regime: "explore".into(),
        },
        ObservableEvent::AgentBudgetUpdate {
            agent: "agent".into(),
            spent_usd: 1.0,
            remaining_usd: 2.0,
            vitality: 0.8,
        },
        ObservableEvent::AgentModeChange {
            agent: "agent".into(),
            old: "normal".into(),
            new_mode: "focused".into(),
        },
        ObservableEvent::AgentPhaseChange {
            agent: "agent".into(),
            old: "healthy".into(),
            new_phase: "conserve".into(),
        },
        ObservableEvent::AgentStateTransition {
            agent: "agent".into(),
            old: "idle".into(),
            new_state: "running".into(),
        },
        ObservableEvent::AgentSlotUpdate {
            agent: "agent".into(),
            slot: "coder".into(),
            state: "occupied".into(),
        },
        ObservableEvent::MemoryRetrieved {
            query: "query".into(),
            results: 2,
            duration_ms: 1,
        },
        ObservableEvent::MemoryStored {
            signal: "s".into(),
            tier: "working".into(),
        },
        ObservableEvent::MemoryConsolidated {
            promoted: 1,
            demoted: 2,
            pruned: 3,
        },
        ObservableEvent::DemurrageApplied {
            count: 4,
            total_balance_lost: 0.4,
        },
        ObservableEvent::VerifyPreResult {
            block: "cell".into(),
            verdict: Verdict::pass("pre"),
            evidence: vec!["test".into()],
        },
        ObservableEvent::VerifyPostResult {
            block: "cell".into(),
            verdict: Verdict::pass("post"),
            reward: 0.9,
            evidence: vec!["test".into()],
        },
        ObservableEvent::TriggerFired {
            trigger: "cron".into(),
            graph: "graph".into(),
        },
        ObservableEvent::TriggerArmed {
            trigger: "cron".into(),
        },
        ObservableEvent::TriggerDisarmed {
            trigger: "cron".into(),
        },
        ObservableEvent::ExtensionHookCalled {
            extension: "ext".into(),
            hook: "before-run".into(),
            layer: 2,
            duration_ms: 5,
        },
        ObservableEvent::ExtensionHookFailed {
            extension: "ext".into(),
            hook: "before-run".into(),
            error: "failed".into(),
        },
    ]
}

#[test]
fn every_spec_named_event_has_exactly_one_family() {
    let events = all_events();
    assert_eq!(events.len(), 39, "the spec's named list totals 39, not 38");

    let expected = [8, 7, 6, 7, 4, 2, 3, 2];
    let families = [
        ObservableEventKind::SignalLifecycle,
        ObservableEventKind::CellLifecycle,
        ObservableEventKind::GraphLifecycle,
        ObservableEventKind::AgentLifecycle,
        ObservableEventKind::MemoryLifecycle,
        ObservableEventKind::VerifyLifecycle,
        ObservableEventKind::TriggerLifecycle,
        ObservableEventKind::ExtensionLifecycle,
    ];
    for (family, expected_count) in families.into_iter().zip(expected) {
        assert_eq!(
            events.iter().filter(|event| family.matches(event)).count(),
            expected_count
        );
    }
    assert!(
        events
            .iter()
            .all(|event| ObservableEventKind::All.matches(event))
    );
}

#[test]
fn filters_scope_and_serde_are_portable() {
    let event = ObservableEvent::CellStarted {
        block: "compile".into(),
        run: "run-7".into(),
        input_hash: "abc".into(),
    };
    assert!(event.matches_any(&[ObservableEventKind::CellLifecycle]));
    assert!(!event.matches_any(&[]));
    assert!(LensScope::Global.matches_event(&event));
    assert!(LensScope::Cell(String::new()).matches_event(&event));
    assert!(LensScope::Cell("compile".into()).matches_event(&event));
    assert!(!LensScope::Cell("review".into()).matches_event(&event));
    assert!(!LensScope::Graph("compile".into()).matches_event(&event));

    let json = serde_json::to_string(&event).expect("serialize event");
    let decoded: ObservableEvent = serde_json::from_str(&json).expect("deserialize event");
    assert_eq!(decoded, event);
}

struct TestLens;

#[async_trait::async_trait]
impl TelemetryObserve for TestLens {
    async fn observe(&self, _event: &ObservableEvent) -> Result<Vec<Signal>> {
        Ok(Vec::new())
    }

    fn observes(&self) -> &[ObservableEventKind] {
        const OBSERVES: &[ObservableEventKind] = &[ObservableEventKind::All];
        OBSERVES
    }

    fn scope(&self) -> LensScope {
        LensScope::Global
    }
}

#[test]
fn telemetry_observe_is_object_safe_send_and_sync() {
    fn assert_observer(_: &dyn TelemetryObserve) {}
    fn assert_send_sync<T: Send + Sync>() {}

    assert_observer(&TestLens);
    assert_send_sync::<TestLens>();
}
