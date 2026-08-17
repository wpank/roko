//! End-to-end contract tests for routed telemetry Lens execution.

#![allow(clippy::float_cmp, clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use roko_core::dashboard_snapshot::DiagnosisSeverity;
use roko_core::lens_circuit_breaker::{LensBreakerAction, LensBreakerStage};
use roko_core::{
    Body, Kind, LensConfig, LensRegistry, LensScope, ObservableEvent, ObservableEventKind, Result,
    RokoError, Signal, TelemetryEventSink, TelemetryObserve,
};
use roko_runtime::{
    LensBackpressurePolicy, LensBreakerConfig, LensExecutionOutcome, LensExecutor, LensPayload,
    LensQueueConfig, LensSignalEnvelope, StateHub,
};
use tokio::sync::Semaphore;

fn config(name: &str, block: &str, scope: &str) -> LensConfig {
    LensConfig {
        name: name.to_string(),
        block: block.to_string(),
        scope: scope.to_string(),
        params: BTreeMap::new(),
    }
}

fn completed(cost_usd: f64, duration_ms: u64) -> ObservableEvent {
    ObservableEvent::CellCompleted {
        block: "compile".into(),
        run: "run-1".into(),
        duration_ms,
        cost_usd,
    }
}

#[tokio::test]
async fn built_in_cost_lens_accumulates_and_updates_statehub() {
    let mut registry = LensRegistry::new();
    registry
        .register(config("cost-main", "roko:cost-lens@^1.0", "graph:build"))
        .unwrap();
    let hub = StateHub::default_capacity();
    let executor = LensExecutor::from_registry(&registry, hub.sender()).unwrap();

    let first = executor
        .dispatch(
            &completed(0.25, 100),
            &[
                LensScope::Cell("compile".into()),
                LensScope::Graph("build".into()),
            ],
        )
        .await;
    assert_eq!(first.records.len(), 1);
    assert_eq!(first.records[0].outcome, LensExecutionOutcome::Succeeded);
    assert!(first.projection_errors.is_empty());
    let envelope = LensSignalEnvelope::from_signal(&first.signals[0]).unwrap();
    match envelope.payload {
        LensPayload::CostReport(payload) => {
            assert_eq!(payload.target, "cell:compile");
            assert_eq!(payload.total_usd, 0.25);
            assert_eq!(payload.cumulative_usd, 0.25);
        }
        other => panic!("unexpected payload: {other:?}"),
    }

    let emitted = executor
        .emit(
            &completed(0.5, 100),
            &[
                LensScope::Cell("compile".into()),
                LensScope::Graph("build".into()),
            ],
        )
        .await
        .unwrap();
    assert_eq!(emitted.len(), 1);
    let graph = executor
        .dispatch(
            &ObservableEvent::GraphCompleted {
                graph: "build".into(),
                run: "run-1".into(),
                duration_ms: 300,
                cost_usd: 0.75,
            },
            &[LensScope::Graph("build".into())],
        )
        .await;
    assert!(graph.signals.is_empty());
    assert_eq!(graph.records[0].outcome, LensExecutionOutcome::Succeeded);
    let projection = hub.get_projection("cost_meter").unwrap();
    assert_eq!(projection.version, 2);
    assert_eq!(projection.data["total_usd"], 0.75);
    assert_eq!(projection.source_lenses, ["cost-main"]);
}

#[test]
fn factory_rejects_unsupported_blocks_instead_of_leaving_runtime_holes() {
    let mut registry = LensRegistry::new();
    registry
        .register(config("mystery", "plugin:mystery-lens@1", "global"))
        .unwrap();
    let hub = StateHub::default_capacity();
    let error = match LensExecutor::from_registry(&registry, hub.sender()) {
        Ok(_) => panic!("unsupported block unexpectedly constructed an executor"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("unsupported runtime block"));
    assert!(error.to_string().contains("plugin:mystery-lens@1"));
}

#[test]
fn factory_constructs_the_complete_versioned_builtin_catalog() {
    let mut configs = vec![
        config("cost", "roko:cost-lens@^1.0", "graph:build"),
        config("latency", "latency-lens@1", "graph:build"),
        config("quality", "roko:quality-lens@1", "graph:build"),
        config("efficiency", "efficiency-lens@1", "agent:alice"),
        config("errors", "roko:error-lens@1", "graph:build"),
        config("drift", "drift-lens@1", "agent:alice"),
        config("budget", "roko:budget-lens@1", "agent:alice"),
        config("usage", "usage-lens@1", "space:default"),
        config("collective", "roko:c-factor-lens@1", "space:default"),
    ];
    let mut trend = config("cost-trend", "roko:trend-lens@1", "lens:cost");
    trend.params.insert(
        "metric".into(),
        serde_json::from_value(serde_json::json!("total_usd")).unwrap(),
    );
    configs.push(trend);
    configs.push(config("cost-anomaly", "anomaly-lens@1", "lens:cost-trend"));

    let mut registry = LensRegistry::new();
    for lens in configs {
        registry.register(lens).unwrap();
    }
    let hub = StateHub::default_capacity();
    let executor = LensExecutor::from_registry(&registry, hub.sender()).unwrap();

    assert_eq!(executor.statuses().len(), 11);
    assert!(
        executor
            .statuses()
            .iter()
            .all(|status| status.invocations == 0)
    );
}

#[tokio::test]
async fn mixed_raw_and_chained_builtins_materialize_statehub_in_one_event_cycle() {
    let mut registry = LensRegistry::new();
    registry
        .register(config("cost", "roko:cost-lens@1", "graph:build"))
        .unwrap();
    registry
        .register(config("latency", "roko:latency-lens@1", "graph:build"))
        .unwrap();
    let mut trend = config("cost-trend", "roko:trend-lens@1", "lens:cost");
    trend.params.insert(
        "metric".into(),
        serde_json::from_value(serde_json::json!("total_usd")).unwrap(),
    );
    trend.params.insert(
        "min_data_points".into(),
        serde_json::from_value(serde_json::json!(2)).unwrap(),
    );
    registry.register(trend).unwrap();

    let hub = StateHub::default_capacity();
    let executor = LensExecutor::from_registry(&registry, hub.sender()).unwrap();
    let ancestry = [
        LensScope::Cell("compile".into()),
        LensScope::Graph("build".into()),
    ];

    let first = executor.dispatch(&completed(0.25, 100), &ancestry).await;
    assert_eq!(
        first
            .records
            .iter()
            .map(|record| record.lens.as_str())
            .collect::<Vec<_>>(),
        ["cost", "latency", "cost-trend"]
    );
    assert_eq!(first.signals.len(), 2);
    assert!(first.projection_errors.is_empty());

    let second = executor.dispatch(&completed(0.50, 200), &ancestry).await;
    assert_eq!(second.signals.len(), 3);
    assert!(second.projection_errors.is_empty());
    assert!(
        second
            .records
            .iter()
            .all(|record| record.outcome == LensExecutionOutcome::Succeeded)
    );

    let cost = hub.get_projection("cost_meter").unwrap();
    assert_eq!(cost.data["total_usd"], 0.75);
    assert_eq!(cost.data["cost_trend"], "rising");
    assert_eq!(cost.source_lenses, ["cost", "cost-trend"]);
    let latency = hub.get_projection("active_tasks").unwrap();
    assert_eq!(latency.data["avg_task_duration_ms"], 150);
    assert_eq!(latency.source_lenses, ["latency"]);
}

#[derive(Clone)]
enum Behavior {
    Emit(&'static str),
    Fail,
}

struct ConcurrencyProbe {
    current: AtomicUsize,
    maximum: AtomicUsize,
}

impl ConcurrencyProbe {
    fn enter(&self) {
        let current = self.current.fetch_add(1, Ordering::SeqCst) + 1;
        self.maximum.fetch_max(current, Ordering::SeqCst);
    }

    fn leave(&self) {
        self.current.fetch_sub(1, Ordering::SeqCst);
    }
}

struct TestLens {
    scope: LensScope,
    observes: Vec<ObservableEventKind>,
    behavior: Behavior,
    delay: Duration,
    probe: Option<Arc<ConcurrencyProbe>>,
}

#[async_trait::async_trait]
impl TelemetryObserve for TestLens {
    async fn observe(&self, _event: &ObservableEvent) -> Result<Vec<Signal>> {
        if let Some(probe) = &self.probe {
            probe.enter();
        }
        tokio::time::sleep(self.delay).await;
        if let Some(probe) = &self.probe {
            probe.leave();
        }
        match self.behavior {
            Behavior::Emit(label) => Ok(vec![
                Signal::builder(Kind::Metric)
                    .body(Body::text(label))
                    .build(),
            ]),
            Behavior::Fail => Err(RokoError::config("intentional Lens failure")),
        }
    }

    fn observes(&self) -> &[ObservableEventKind] {
        &self.observes
    }

    fn scope(&self) -> LensScope {
        self.scope.clone()
    }
}

fn test_lens(
    scope: LensScope,
    observes: Vec<ObservableEventKind>,
    behavior: Behavior,
    probe: Option<Arc<ConcurrencyProbe>>,
) -> Arc<dyn TelemetryObserve> {
    Arc::new(TestLens {
        scope,
        observes,
        behavior,
        delay: Duration::from_millis(10),
        probe,
    })
}

#[tokio::test]
async fn stacked_lenses_are_concurrent_failures_are_isolated_and_chains_finish_in_order() {
    let mut registry = LensRegistry::new();
    registry
        .register_with_observes(
            config("producer", "plugin:producer@1", "global"),
            vec![ObservableEventKind::All],
        )
        .unwrap();
    registry
        .register_with_observes(
            config("failing", "plugin:failing@1", "global"),
            vec![ObservableEventKind::All],
        )
        .unwrap();
    registry
        .register_with_observes(
            config("downstream", "plugin:downstream@1", "lens:producer"),
            vec![ObservableEventKind::SignalLifecycle],
        )
        .unwrap();

    let probe = Arc::new(ConcurrencyProbe {
        current: AtomicUsize::new(0),
        maximum: AtomicUsize::new(0),
    });
    let hub = StateHub::default_capacity();
    let mut executor = LensExecutor::new(registry)
        .unwrap()
        .with_projection(hub.sender());
    executor
        .register(
            "producer",
            test_lens(
                LensScope::Global,
                vec![ObservableEventKind::All],
                Behavior::Emit("raw"),
                Some(Arc::clone(&probe)),
            ),
        )
        .unwrap();
    executor
        .register(
            "failing",
            test_lens(
                LensScope::Global,
                vec![ObservableEventKind::All],
                Behavior::Fail,
                Some(Arc::clone(&probe)),
            ),
        )
        .unwrap();
    executor
        .register(
            "downstream",
            test_lens(
                LensScope::Lens("producer".into()),
                vec![ObservableEventKind::SignalLifecycle],
                Behavior::Emit("derived"),
                None,
            ),
        )
        .unwrap();
    executor.validate().unwrap();

    let report = executor
        .dispatch(&completed(0.0, 2_000), &[LensScope::Cell("compile".into())])
        .await;
    assert_eq!(probe.maximum.load(Ordering::SeqCst), 2);
    assert_eq!(
        report
            .records
            .iter()
            .map(|record| record.lens.as_str())
            .collect::<Vec<_>>(),
        ["producer", "failing", "downstream"]
    );
    assert_eq!(
        report.records[1].outcome,
        LensExecutionOutcome::Failed("config error: intentional Lens failure".into())
    );
    assert_eq!(report.records[2].source_lens.as_deref(), Some("producer"));
    assert_eq!(
        report.records[2].breaker_action,
        Some(LensBreakerAction::Allow),
        "chained Lens must inherit the raw event's overhead denominator"
    );
    assert_eq!(report.signals.len(), 2);
    assert!(report.projection_errors.is_empty());
    assert!(hub.projections().is_empty());
    assert!(report.has_failures());
}

#[tokio::test]
async fn breaker_samples_half_then_disables_and_reports_every_action() {
    let mut registry = LensRegistry::new();
    registry
        .register_with_observes(
            config("slow", "plugin:slow@1", "global"),
            vec![ObservableEventKind::CellLifecycle],
        )
        .unwrap();
    let hub = StateHub::default_capacity();
    let mut executor = LensExecutor::with_breaker_config(
        registry,
        LensBreakerConfig {
            overhead_budget_pct: 0.01,
            sample_threshold: 1,
            disable_threshold: 3,
        },
    )
    .unwrap()
    .with_projection(hub.sender());
    executor
        .register(
            "slow",
            test_lens(
                LensScope::Global,
                vec![ObservableEventKind::CellLifecycle],
                Behavior::Emit("slow"),
                None,
            ),
        )
        .unwrap();

    let marker = executor
        .dispatch(
            &ObservableEvent::CellStarted {
                block: "compile".into(),
                run: "run-1".into(),
                input_hash: "abc".into(),
            },
            &[],
        )
        .await;
    assert_eq!(
        marker.records[0].stage_after,
        Some(LensBreakerStage::Active)
    );
    assert_eq!(marker.records[0].breaker_action, None);

    let first = executor.dispatch(&completed(0.0, 100), &[]).await;
    assert_eq!(
        first.records[0].stage_after,
        Some(LensBreakerStage::Sampled)
    );
    assert_eq!(
        first.records[0].breaker_action,
        Some(LensBreakerAction::Skip)
    );
    let second = executor.dispatch(&completed(0.0, 100), &[]).await;
    assert_eq!(second.records[0].outcome, LensExecutionOutcome::Succeeded);
    let third = executor.dispatch(&completed(0.0, 100), &[]).await;
    assert_eq!(third.records[0].outcome, LensExecutionOutcome::SampledOut);
    assert_eq!(
        third.records[0].breaker_action,
        Some(LensBreakerAction::Skip)
    );
    let fourth = executor.dispatch(&completed(0.0, 100), &[]).await;
    assert_eq!(
        fourth.records[0].stage_after,
        Some(LensBreakerStage::Disabled)
    );
    assert_eq!(
        fourth.records[0].breaker_action,
        Some(LensBreakerAction::Disable)
    );
    let fifth = executor.dispatch(&completed(0.0, 100), &[]).await;
    assert_eq!(fifth.records[0].outcome, LensExecutionOutcome::Disabled);
    assert_eq!(
        fifth.records[0].breaker_action,
        Some(LensBreakerAction::Disable)
    );

    let status = executor.statuses().pop().unwrap();
    assert_eq!(status.invocations, 4);
    assert_eq!(status.sampled_out, 1);
    assert_eq!(status.total_violations, 3);
    assert!(hub.current_snapshot().diagnoses.iter().any(|diagnosis| {
        diagnosis.severity == DiagnosisSeverity::Alert
            && diagnosis.subject == "Lens circuit breaker: slow"
    }));
    executor.reset_lens("slow").unwrap();
    assert_eq!(executor.statuses()[0].stage, LensBreakerStage::Sampled);
    assert!(executor.statuses()[0].enabled);

    executor.set_lens_enabled("slow", false).unwrap();
    assert!(!executor.statuses()[0].enabled);
    let disabled = executor.dispatch(&completed(0.0, 100), &[]).await;
    assert_eq!(disabled.records[0].outcome, LensExecutionOutcome::Disabled);
    executor.set_lens_enabled("slow", true).unwrap();
    assert!(executor.statuses()[0].enabled);
    assert_eq!(executor.statuses()[0].stage, LensBreakerStage::Sampled);
}

struct BlockingLens {
    observes: Vec<ObservableEventKind>,
    calls: AtomicUsize,
    started: Arc<Semaphore>,
    release: Arc<Semaphore>,
    seen: Arc<Mutex<Vec<String>>>,
}

#[async_trait::async_trait]
impl TelemetryObserve for BlockingLens {
    async fn observe(&self, event: &ObservableEvent) -> Result<Vec<Signal>> {
        let ObservableEvent::CellCompleted { block, .. } = event else {
            return Ok(Vec::new());
        };
        self.seen.lock().unwrap().push(block.clone());
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            self.started.add_permits(1);
            self.release.acquire().await.unwrap().forget();
        }
        Ok(Vec::new())
    }

    fn observes(&self) -> &[ObservableEventKind] {
        &self.observes
    }

    fn scope(&self) -> LensScope {
        LensScope::Global
    }
}

fn named_completed(block: &str) -> ObservableEvent {
    ObservableEvent::CellCompleted {
        block: block.into(),
        run: "queue-run".into(),
        duration_ms: 100,
        cost_usd: 0.0,
    }
}

#[tokio::test]
async fn queued_runtime_registers_for_statehub_operator_control() {
    let mut registry = LensRegistry::new();
    registry
        .register(config("cost-main", "roko:cost-lens@1", "global"))
        .unwrap();
    let hub = StateHub::default_capacity();
    let _queue = LensExecutor::from_registry(&registry, hub.sender())
        .unwrap()
        .into_queued("operator-test", LensQueueConfig::default())
        .unwrap();

    let runtimes = hub.lens_runtime_snapshots();
    assert_eq!(runtimes.len(), 1);
    assert_eq!(runtimes[0].runtime_id, "operator-test");
    assert!(runtimes[0].lenses[0].enabled);
    assert_eq!(runtimes[0].lenses[0].breaker_stage, "active");

    hub.set_lens_runtime_enabled("operator-test", "cost-main", false)
        .unwrap();
    let disabled = hub.lens_runtime_snapshot("operator-test").unwrap();
    assert!(!disabled.lenses[0].enabled);

    hub.reset_lens_runtime("operator-test", "cost-main")
        .unwrap();
    let recovered = hub.lens_runtime_snapshot("operator-test").unwrap();
    assert!(recovered.lenses[0].enabled);
    assert_eq!(recovered.lenses[0].breaker_stage, "sampled");
}

#[tokio::test]
async fn statehub_fans_producer_observations_into_registered_queues() {
    let mut registry = LensRegistry::new();
    registry
        .register(config("cost-main", "roko:cost-lens@1", "global"))
        .unwrap();
    let hub = StateHub::default_capacity();
    let queue = LensExecutor::from_registry(&registry, hub.sender())
        .unwrap()
        .into_queued("producer-ingress-test", LensQueueConfig::default())
        .unwrap();

    let errors = hub.emit_observable(&completed(0.25, 100), &[LensScope::Global]);

    assert!(errors.is_empty());
    assert!(queue.wait_idle(Duration::from_secs(2)).await);
    let snapshot = queue.snapshot();
    assert_eq!(snapshot.queue.enqueued, 1);
    assert_eq!(snapshot.queue.processed, 1);
    let projection = hub.get_projection("cost_meter").unwrap();
    assert_eq!(projection.data["total_usd"], 0.25);
}

#[tokio::test]
async fn queued_delivery_is_non_blocking_and_drops_oldest_pending_event() {
    let mut registry = LensRegistry::new();
    registry
        .register_with_observes(
            config("blocking", "plugin:blocking@1", "global"),
            vec![ObservableEventKind::CellLifecycle],
        )
        .unwrap();
    let started = Arc::new(Semaphore::new(0));
    let release = Arc::new(Semaphore::new(0));
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut executor = LensExecutor::new(registry).unwrap();
    executor
        .register(
            "blocking",
            Arc::new(BlockingLens {
                observes: vec![ObservableEventKind::CellLifecycle],
                calls: AtomicUsize::new(0),
                started: Arc::clone(&started),
                release: Arc::clone(&release),
                seen: Arc::clone(&seen),
            }),
        )
        .unwrap();
    let queue = executor
        .into_queued(
            "queue-test",
            LensQueueConfig {
                capacity: 2,
                backpressure: LensBackpressurePolicy::DropOldest,
            },
        )
        .unwrap();

    tokio::time::timeout(
        Duration::from_millis(50),
        queue.emit(&named_completed("first"), &[]),
    )
    .await
    .expect("enqueue must not wait for Lens execution")
    .unwrap();
    started.acquire().await.unwrap().forget();
    queue.emit(&named_completed("dropped"), &[]).await.unwrap();
    queue.emit(&named_completed("third"), &[]).await.unwrap();
    queue.emit(&named_completed("fourth"), &[]).await.unwrap();

    let backed_up = queue.snapshot();
    assert_eq!(backed_up.queue.depth, 2);
    assert_eq!(backed_up.queue.in_flight, 1);
    assert_eq!(backed_up.queue.dropped_oldest, 1);
    assert_eq!(backed_up.queue.backpressure, "drop_oldest");

    release.add_permits(1);
    assert!(queue.wait_idle(Duration::from_secs(2)).await);
    assert_eq!(&*seen.lock().unwrap(), &["first", "third", "fourth"]);
    let drained = queue.snapshot();
    assert_eq!(drained.queue.enqueued, 4);
    assert_eq!(drained.queue.processed, 3);
    assert_eq!(drained.queue.depth, 0);
    assert_eq!(drained.queue.in_flight, 0);
    assert_eq!(drained.queue.failed_dispatches, 0);
}
