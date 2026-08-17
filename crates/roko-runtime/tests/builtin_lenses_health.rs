//! Focused behavioral coverage for the built-in health telemetry Lenses.

use std::collections::BTreeMap;

use roko_core::{
    AlertLevel, Body, DriftPayload, Kind, LensConfig, LensScope, ObservableEvent,
    ObservableEventKind, Signal, TelemetryObserve,
};
pub use roko_runtime::{LensPayload, LensSignalEnvelope};

#[path = "../src/builtin_lenses_health.rs"]
mod builtin_lenses_health;

use builtin_lenses_health::{
    BUDGET_LENS_BLOCK_ALIASES, BudgetLens, DRIFT_LENS_BLOCK_ALIASES, DriftLens,
    ERROR_LENS_BLOCK_ALIASES, ErrorLens, create_builtin_health_lens,
};

fn config(name: &str, block: &str, scope: &str) -> LensConfig {
    LensConfig {
        name: name.into(),
        block: block.into(),
        scope: scope.into(),
        params: BTreeMap::new(),
    }
}

fn payload(signals: &[Signal]) -> LensPayload {
    assert_eq!(signals.len(), 1, "expected one canonical Lens envelope");
    LensSignalEnvelope::from_signal(&signals[0])
        .expect("valid Lens envelope")
        .payload
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-12,
        "expected {expected}, got {actual}"
    );
}

fn drift_payload(payload: LensPayload) -> DriftPayload {
    let LensPayload::Drift(payload) = payload else {
        panic!("expected Drift payload");
    };
    payload
}

async fn observe_payload(lens: &dyn TelemetryObserve, event: ObservableEvent) -> LensPayload {
    let signals = lens.observe(&event).await.expect("observe event");
    payload(&signals)
}

fn completed(block: &str, run: &str) -> ObservableEvent {
    ObservableEvent::CellCompleted {
        block: block.into(),
        run: run.into(),
        duration_ms: 10,
        cost_usd: 0.0,
    }
}

fn failed(block: &str, run: &str, error: &str) -> ObservableEvent {
    ObservableEvent::CellFailed {
        block: block.into(),
        run: run.into(),
        error: error.into(),
    }
}

#[test]
fn factory_aliases_versions_and_fail_closed_configuration() {
    assert_eq!(ERROR_LENS_BLOCK_ALIASES, ["error-lens", "roko:error-lens"]);
    assert_eq!(DRIFT_LENS_BLOCK_ALIASES, ["drift-lens", "roko:drift-lens"]);
    assert_eq!(
        BUDGET_LENS_BLOCK_ALIASES,
        ["budget-lens", "roko:budget-lens"]
    );

    let error = config("errors", "roko:error-lens@^1.0", "graph:build");
    assert!(
        create_builtin_health_lens(&error)
            .expect("factory result")
            .is_some()
    );
    let unrelated = config("cost", "roko:cost-lens@1", "graph");
    assert!(
        create_builtin_health_lens(&unrelated)
            .expect("factory result")
            .is_none()
    );

    let mut unknown = error.clone();
    unknown.params.insert("typo".into(), true.into());
    assert!(ErrorLens::from_config(&unknown).is_err());

    let mut conflicting_interval = error.clone();
    conflicting_interval
        .params
        .insert("interval".into(), "1s".into());
    conflicting_interval
        .params
        .insert("interval_ms".into(), 1_000_i64.into());
    assert!(ErrorLens::from_config(&conflicting_interval).is_err());

    let bad_drift_scope = config("drift", "drift-lens", "cell:compile");
    assert!(DriftLens::from_config(&bad_drift_scope).is_err());

    let mut bad_thresholds = config("budget", "budget-lens", "agent:alice");
    bad_thresholds.params.insert("info_pct".into(), 0.9.into());
    bad_thresholds
        .params
        .insert("warning_pct".into(), 0.8.into());
    assert!(BudgetLens::from_config(&bad_thresholds).is_err());

    let mut bad_agent_bound = config("budget", "budget-lens", "agent:alice");
    bad_agent_bound
        .params
        .insert("max_agents".into(), 0_i64.into());
    assert!(BudgetLens::from_config(&bad_agent_bound).is_err());

    let wrong_constructor = config("errors", "drift-lens", "agent");
    assert!(ErrorLens::from_config(&wrong_constructor).is_err());
}

#[tokio::test]
async fn error_lens_classifies_every_category_and_uses_a_rolling_outcome_window() {
    let mut config = config("errors", "roko:error-lens@1", "graph:build");
    config.params.insert("window_events".into(), 3_i64.into());
    config.params.insert("interval".into(), "2s".into());
    let lens = ErrorLens::from_config(&config).expect("ErrorLens config");

    assert_eq!(lens.scope(), LensScope::Graph("build".into()));
    assert_eq!(
        lens.observes(),
        [
            ObservableEventKind::CellLifecycle,
            ObservableEventKind::GraphLifecycle,
            ObservableEventKind::ExtensionLifecycle,
        ]
    );

    lens.observe(&completed("compile", "r0"))
        .await
        .expect("success event");
    lens.observe(&failed("compile", "r1", "request timeout"))
        .await
        .expect("timeout event");
    lens.observe(&failed("parse", "r2", "invalid input payload"))
        .await
        .expect("input event");
    let rolling = observe_payload(&lens, completed("tests", "r3")).await;
    let LensPayload::Error(rolling) = rolling else {
        panic!("expected Error payload");
    };
    assert_eq!(rolling.target, "graph:build");
    assert_eq!(rolling.interval_ms, 2_000);
    assert_eq!(rolling.total_errors, 2);
    assert_close(rolling.error_rate, 2.0 / 3.0);
    assert_eq!(rolling.by_category["Timeout"], 1);
    assert_eq!(rolling.by_category["InputInvalid"], 1);
    assert!(!rolling.by_block.contains_key("compile") || rolling.by_block["compile"] == 1);

    let cases = [
        ("permission denied by capability policy", "CapabilityDenied"),
        ("provider network connection failed", "External"),
        ("invariant violated", "LogicError"),
        ("operation cancelled", "Cancelled"),
    ];
    for (index, (message, category)) in cases.into_iter().enumerate() {
        let payload = observe_payload(&lens, failed("worker", &format!("c{index}"), message)).await;
        let LensPayload::Error(payload) = payload else {
            panic!("expected Error payload");
        };
        assert_eq!(payload.by_category[category], 1);
    }

    let extension = observe_payload(
        &lens,
        ObservableEvent::ExtensionHookFailed {
            extension: "audit".into(),
            hook: "before_run".into(),
            error: "opaque failure".into(),
        },
    )
    .await;
    let LensPayload::Error(extension) = extension else {
        panic!("expected Error payload");
    };
    assert_eq!(extension.by_category["External"], 1);
    assert_eq!(extension.by_block["audit:before_run"], 1);
}

#[tokio::test]
async fn error_lens_resolves_retry_attempts_without_counting_pending_as_failure() {
    let mut config = config("errors", "error-lens", "graph");
    config.params.insert("window_events".into(), 10_i64.into());
    let lens = ErrorLens::from_config(&config).expect("ErrorLens config");

    let first_retry = observe_payload(
        &lens,
        ObservableEvent::CellRetried {
            block: "compile".into(),
            run: "r1".into(),
            attempt: 2,
            reason: "failed".into(),
        },
    )
    .await;
    let LensPayload::Error(first_retry) = first_retry else {
        panic!("expected Error payload");
    };
    assert_eq!(first_retry.retry_count, 1);
    assert_close(first_retry.retry_success_rate, 0.0);

    lens.observe(&failed("compile", "r1", "logic fault"))
        .await
        .expect("failed retry");
    lens.observe(&ObservableEvent::CellRetried {
        block: "compile".into(),
        run: "r1".into(),
        attempt: 3,
        reason: "retry again".into(),
    })
    .await
    .expect("second retry");
    let resolved = observe_payload(&lens, completed("compile", "r1")).await;
    let LensPayload::Error(resolved) = resolved else {
        panic!("expected Error payload");
    };
    assert_eq!(resolved.retry_count, 2);
    assert_close(resolved.retry_success_rate, 0.5);
}

#[tokio::test]
async fn drift_lens_materializes_only_correlated_balance_tier_and_metadata_evidence() {
    let mut config = config("drift", "roko:drift-lens@^1", "agent:alice");
    config
        .params
        .insert("cold_balance_threshold".into(), 0.05.into());
    config.params.insert("window_events".into(), 2_i64.into());
    let lens = DriftLens::from_config(&config).expect("DriftLens config");

    assert_eq!(lens.scope(), LensScope::Agent("alice".into()));
    assert_eq!(
        lens.observes(),
        [
            ObservableEventKind::MemoryLifecycle,
            ObservableEventKind::SignalLifecycle,
        ]
    );

    let heuristic = Signal::builder(Kind::Custom("heuristic".into()))
        .body(Body::text("prefer deterministic reducers"))
        .balance(0.8)
        .tag("heuristic.calibration", "0.9")
        .build();
    assert!(
        lens.observe(&ObservableEvent::SignalCreated(heuristic.clone()))
            .await
            .expect("signal evidence")
            .is_empty()
    );
    let stored = observe_payload(
        &lens,
        ObservableEvent::MemoryStored {
            signal: heuristic.id.to_hex(),
            tier: "Transient".into(),
        },
    )
    .await;
    let stored = drift_payload(stored);
    assert_eq!(stored.memory, "agent:alice");
    assert_eq!(stored.total_entries, 1);
    assert_eq!(stored.tier_distribution["transient"], 1);
    assert_close(stored.avg_balance, 0.8);
    assert_close(stored.heuristic_calibration_avg, 0.9);

    let demurrage = observe_payload(
        &lens,
        ObservableEvent::SignalDemurrageApplied(heuristic.id.to_hex(), 0.2),
    )
    .await;
    let demurrage = drift_payload(demurrage);
    assert!((demurrage.avg_balance - 0.6).abs() < 1e-12);
    assert!((demurrage.balance_delta + 0.2).abs() < 1e-12);

    let promoted = observe_payload(
        &lens,
        ObservableEvent::SignalPromoted(
            heuristic.id.to_hex(),
            "Transient".into(),
            "Working".into(),
        ),
    )
    .await;
    let promoted = drift_payload(promoted);
    assert_eq!(promoted.tier_distribution["working"], 1);
    assert_close(promoted.promotion_rate, 1.0);

    let anti = Signal::builder(Kind::Custom("anti_knowledge".into()))
        .body(Body::text("known failure"))
        .balance(0.01)
        .build();
    lens.observe(&ObservableEvent::SignalCreated(anti.clone()))
        .await
        .expect("anti-knowledge evidence");
    let anti_payload = observe_payload(
        &lens,
        ObservableEvent::MemoryStored {
            signal: anti.id.to_hex(),
            tier: "Transient".into(),
        },
    )
    .await;
    let anti_payload = drift_payload(anti_payload);
    assert_eq!(anti_payload.anti_knowledge_count, 1);
    assert_eq!(anti_payload.cold_entries, 1);

    let consolidated = observe_payload(
        &lens,
        ObservableEvent::MemoryConsolidated {
            promoted: 1,
            demoted: 2,
            pruned: 1,
        },
    )
    .await;
    let consolidated = drift_payload(consolidated);
    assert_close(consolidated.promotion_rate, 0.5);
    assert_close(consolidated.demotion_rate, 0.5);

    let pruned = observe_payload(&lens, ObservableEvent::SignalPruned(anti.id.to_hex())).await;
    let pruned = drift_payload(pruned);
    assert_eq!(pruned.total_entries, 1);
    assert_eq!(pruned.anti_knowledge_count, 0);
}

#[tokio::test]
async fn drift_lens_does_not_invent_balance_for_uncorrelated_memory_entries() {
    let lens =
        DriftLens::from_config(&config("drift", "drift-lens", "global")).expect("DriftLens config");
    let payload = observe_payload(
        &lens,
        ObservableEvent::MemoryStored {
            signal: "external-ref".into(),
            tier: "Cold".into(),
        },
    )
    .await;
    let LensPayload::Drift(payload) = payload else {
        panic!("expected Drift payload");
    };
    assert_eq!(payload.total_entries, 1);
    assert_close(payload.avg_balance, 0.0);
    assert_eq!(payload.cold_entries, 1);
    assert_close(payload.heuristic_calibration_avg, 0.0);
}

#[tokio::test]
async fn drift_lens_bounds_pending_and_entry_state_and_resolves_id_aliases() {
    let mut config = config("drift", "drift-lens", "agent:alice");
    config.params.insert("window_events".into(), 2_i64.into());
    let lens = DriftLens::from_config(&config).expect("DriftLens config");

    let anti = Signal::builder(Kind::Custom("anti_knowledge".into()))
        .body(Body::text("known failure"))
        .balance(0.4)
        .build();
    lens.observe(&ObservableEvent::SignalCreated(anti.clone()))
        .await
        .expect("signal evidence");
    let stored = observe_payload(
        &lens,
        ObservableEvent::MemoryStored {
            signal: anti.id.short(),
            tier: "Transient".into(),
        },
    )
    .await;
    let LensPayload::Drift(stored) = stored else {
        panic!("expected Drift payload");
    };
    assert_eq!(stored.total_entries, 1);
    assert_eq!(stored.anti_knowledge_count, 1);

    let duplicate = observe_payload(
        &lens,
        ObservableEvent::MemoryStored {
            signal: anti.id.to_hex(),
            tier: "Working".into(),
        },
    )
    .await;
    let LensPayload::Drift(duplicate) = duplicate else {
        panic!("expected Drift payload");
    };
    assert_eq!(duplicate.total_entries, 1);
    assert_eq!(duplicate.tier_distribution["working"], 1);

    let pruned = observe_payload(&lens, ObservableEvent::SignalPruned(anti.id.short())).await;
    let LensPayload::Drift(pruned) = pruned else {
        panic!("expected Drift payload");
    };
    assert_eq!(pruned.total_entries, 0);

    let first = Signal::builder(Kind::Custom("fact".into()))
        .body(Body::text("first"))
        .balance(0.2)
        .build();
    let second = Signal::builder(Kind::Custom("fact".into()))
        .body(Body::text("second"))
        .balance(0.4)
        .build();
    let third = Signal::builder(Kind::Custom("fact".into()))
        .body(Body::text("third"))
        .balance(0.6)
        .build();
    for signal in [&first, &second, &third] {
        lens.observe(&ObservableEvent::SignalCreated(signal.clone()))
            .await
            .expect("signal evidence");
    }

    let evicted_pending = observe_payload(
        &lens,
        ObservableEvent::MemoryStored {
            signal: first.id.to_hex(),
            tier: "Cold".into(),
        },
    )
    .await;
    let LensPayload::Drift(evicted_pending) = evicted_pending else {
        panic!("expected Drift payload");
    };
    assert_close(evicted_pending.avg_balance, 0.0);

    lens.observe(&ObservableEvent::MemoryStored {
        signal: second.id.short(),
        tier: "Transient".into(),
    })
    .await
    .expect("store second");
    let bounded = observe_payload(
        &lens,
        ObservableEvent::MemoryStored {
            signal: third.id.to_hex(),
            tier: "Working".into(),
        },
    )
    .await;
    let LensPayload::Drift(bounded) = bounded else {
        panic!("expected Drift payload");
    };
    assert_eq!(bounded.total_entries, 2);
    assert_close(bounded.avg_balance, 0.5);

    let alias_pruned =
        observe_payload(&lens, ObservableEvent::SignalPruned(second.id.to_hex())).await;
    let LensPayload::Drift(alias_pruned) = alias_pruned else {
        panic!("expected Drift payload");
    };
    assert_eq!(alias_pruned.total_entries, 1);
}

#[tokio::test]
async fn budget_lens_emits_only_threshold_transitions_with_vitality_and_phase() {
    let mut config = config("budget", "roko:budget-lens@1", "agent:alice");
    config.params.insert("interval".into(), "60s".into());
    config.params.insert("info_pct".into(), 0.5.into());
    config.params.insert("budget_warn_pct".into(), 0.8.into());
    config
        .params
        .insert("budget_critical_pct".into(), 0.95.into());
    let lens = BudgetLens::from_config(&config).expect("BudgetLens config");

    assert_eq!(lens.scope(), LensScope::Agent("alice".into()));
    assert_eq!(
        lens.observes(),
        [
            ObservableEventKind::AgentLifecycle,
            ObservableEventKind::CellLifecycle,
        ]
    );
    assert!(
        lens.observe(&ObservableEvent::AgentPhaseChange {
            agent: "alice".into(),
            old: "normal".into(),
            new_phase: "conserve".into(),
        })
        .await
        .expect("phase update")
        .is_empty()
    );

    async fn update(
        lens: &BudgetLens,
        spent: f64,
        remaining: f64,
    ) -> roko_core::Result<Vec<Signal>> {
        lens.observe(&ObservableEvent::AgentBudgetUpdate {
            agent: "alice".into(),
            spent_usd: spent,
            remaining_usd: remaining,
            vitality: 0.6,
        })
        .await
    }

    assert!(
        update(&lens, 4.0, 6.0)
            .await
            .expect("below threshold")
            .is_empty()
    );
    let info = payload(&update(&lens, 5.0, 5.0).await.expect("info transition"));
    let LensPayload::BudgetAlert(info) = info else {
        panic!("expected BudgetAlert payload");
    };
    assert_eq!(info.level, AlertLevel::Info);
    assert_eq!(info.target, "agent:alice");
    assert_close(info.vitality, 0.6);
    assert_eq!(info.vitality_phase, "conserve");
    assert_close(info.burn_rate, 60.0);
    assert_eq!(info.projected_exhaustion_ms, None);

    assert!(
        update(&lens, 6.0, 4.0)
            .await
            .expect("same info level")
            .is_empty()
    );
    let warning = payload(&update(&lens, 8.0, 2.0).await.expect("warning transition"));
    let LensPayload::BudgetAlert(warning) = warning else {
        panic!("expected BudgetAlert payload");
    };
    assert_eq!(warning.level, AlertLevel::Warning);
    assert_close(warning.burn_rate, 120.0);

    let critical = payload(&update(&lens, 9.5, 0.5).await.expect("critical transition"));
    let LensPayload::BudgetAlert(critical) = critical else {
        panic!("expected BudgetAlert payload");
    };
    assert_eq!(critical.level, AlertLevel::Critical);
    assert!(
        update(&lens, 9.8, 0.2)
            .await
            .expect("deduplicated critical")
            .is_empty()
    );

    assert!(
        update(&lens, 1.0, 9.0)
            .await
            .expect("budget reset")
            .is_empty()
    );
    let recrossed = payload(&update(&lens, 5.0, 5.0).await.expect("recrossed info"));
    let LensPayload::BudgetAlert(recrossed) = recrossed else {
        panic!("expected BudgetAlert payload");
    };
    assert_eq!(recrossed.level, AlertLevel::Info);
    assert_close(recrossed.burn_rate, 240.0);
}

#[tokio::test]
async fn budget_lens_rejects_non_finite_event_values_and_keeps_unavailable_fields_empty() {
    let lens = BudgetLens::from_config(&config("budget", "budget-lens", "space:alpha"))
        .expect("BudgetLens config");
    assert!(
        lens.observe(&ObservableEvent::AgentBudgetUpdate {
            agent: "alice".into(),
            spent_usd: f64::NAN,
            remaining_usd: 1.0,
            vitality: 0.5,
        })
        .await
        .is_err()
    );
    let first = observe_payload(
        &lens,
        ObservableEvent::AgentBudgetUpdate {
            agent: "alice".into(),
            spent_usd: 5.0,
            remaining_usd: 5.0,
            vitality: 0.5,
        },
    )
    .await;
    let LensPayload::BudgetAlert(first) = first else {
        panic!("expected BudgetAlert payload");
    };
    assert_eq!(first.vitality_phase, "");
    assert_close(first.burn_rate, 0.0);
    assert_eq!(first.projected_exhaustion_ms, None);
}

#[tokio::test]
async fn budget_lens_bounds_agent_state_with_deterministic_fifo_eviction() {
    let mut config = config("budget", "budget-lens", "global");
    config.params.insert("max_agents".into(), 2_i64.into());
    let lens = BudgetLens::from_config(&config).expect("BudgetLens config");

    for agent in ["alice", "bob"] {
        lens.observe(&ObservableEvent::AgentPhaseChange {
            agent: agent.into(),
            old: "normal".into(),
            new_phase: format!("{agent}-phase"),
        })
        .await
        .expect("phase evidence");
    }
    let first = observe_payload(
        &lens,
        ObservableEvent::AgentBudgetUpdate {
            agent: "alice".into(),
            spent_usd: 5.0,
            remaining_usd: 5.0,
            vitality: 0.7,
        },
    )
    .await;
    let LensPayload::BudgetAlert(first) = first else {
        panic!("expected BudgetAlert payload");
    };
    assert_eq!(first.vitality_phase, "alice-phase");

    lens.observe(&ObservableEvent::AgentPhaseChange {
        agent: "carol".into(),
        old: "normal".into(),
        new_phase: "carol-phase".into(),
    })
    .await
    .expect("third agent evicts first");
    let reintroduced = observe_payload(
        &lens,
        ObservableEvent::AgentBudgetUpdate {
            agent: "alice".into(),
            spent_usd: 5.0,
            remaining_usd: 5.0,
            vitality: 0.7,
        },
    )
    .await;
    let LensPayload::BudgetAlert(reintroduced) = reintroduced else {
        panic!("expected BudgetAlert payload");
    };
    assert_eq!(reintroduced.vitality_phase, "");
    assert_close(reintroduced.burn_rate, 0.0);
}
