#![cfg(feature = "fault-injection")]

//! E1, the structural fault replay (S03 §4.9 and §7; backlog 5130;
//! gap-a13544).
//!
//! `structural_faults_detected_and_localized` replays [`EPOCHS`] epochs of
//! [`TASKS`] dry-run contexts, 500 in all, through dispatch's `plan()` in a
//! workspace seeded with knowledge and a cascade router: once healthy, and
//! once with each structural fault's flag set on its loop. Each context's
//! rows come from dispatch's own builders (the route row `plan()` decides,
//! and `planned_content_decisions`), with a verdict whose provider report
//! names the routed model. The loop-audit census folds them as an audit
//! tick does, and a fresh auditor evaluates the loops after every context. A
//! fault is detected when its loop is flagged with the fault's reason. Its
//! time to detection is the loop's opportunities then: on the learned arm
//! for the faults exposure reveals (S03's N_ε), on both arms for the
//! structural pre-checks. With the flag still set, the loop's dry canary
//! localizes the fault at its first failing probe. Nothing calls a provider.
//!
//! - C2: at least 95% of the CUT, MASK, STALE and UNLOGGED injections are
//!   detected, at a median of at most 30 opportunities; DEGENERATE within
//!   50; LABEL_ONLY at the first.
//! - C3: the canary's first failing probe is the injected site in at least
//!   90% of the injections it can see: CUT and STALE at P2, MASK at P4, and
//!   UNLOGGED at P6, where every dry trace that gets past P4 stops, since a
//!   dry canary writes no decision rows.
//! - A4: the placebo never moves, and no injection breaks the auditor.
//!
//! The healthy replay flags no loop, and a fault flags no loop but its own.

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use chrono::Utc;
use roko_cli::dispatch::{DispatchContext, Dispatcher, PromptAssembler, WarmPool};
use roko_cli::graph_task_dispatch::planned_content_decisions;
use roko_cli::loop_canary::DryCanaryRunner;
use roko_cli::task_parser::TaskDef;
use roko_core::config::learning::LearningAuditConfig;
use roko_learn::cascade_router::CascadeRouter;
use roko_learn::loop_audit::arm_set::{ArmDraws, ArmMode, ArmSet};
use roko_learn::loop_audit::census::CensusState;
use roko_learn::loop_audit::faults::{self, FaultActor, FaultKind, FaultSpec};
use roko_learn::loop_audit::ledger::LoopAuditRow;
use roko_learn::loop_audit::{LoopAuditor, ReasonCode, Registry, RowOrigin};
use roko_learn::model_router::RoutingContext;
use roko_learn::telemetry::records::{
    AttemptOutcome, AttemptVerdictRecord, DECISION_SCHEMA, PlaceboDecisionRecord, RunFile, Stamped,
    VERDICT_SCHEMA,
};
use roko_learn::telemetry::report::RunRecords;
use roko_learn::telemetry::{AttemptIdentity, AttemptKey};

/// The plan the replayed attempts belong to.
const PLAN: &str = "e1";

/// Dry-run contexts per epoch: one task each, a chain of its own, cycling
/// through the topics.
const TASKS: usize = 100;

/// Epochs per case, each a replay whose chains draw arms of their own: with
/// [`TASKS`], 500 dry-run contexts.
const EPOCHS: usize = 5;

/// The five topics the seeded knowledge covers, three words each, which no
/// other topic shares.
const TOPICS: [[&str; 3]; 5] = [
    ["cache", "eviction", "policy"],
    ["parser", "token", "stream"],
    ["retry", "backoff", "budget"],
    ["schema", "migration", "column"],
    ["socket", "handshake", "timeout"],
];

/// The cascade router's models, which dispatch routes among.
const MODELS: [&str; 3] = ["claude-haiku-4-5", "claude-opus-4-1", "claude-sonnet-4-6"];

/// The cascade router's state: a little learned, so its state reads as
/// loaded, and still at its static stage, which sends implementers to the
/// premium model rather than dispatch's default.
const ROUTER: &str = r#"{
    "model_slugs": ["claude-haiku-4-5", "claude-opus-4-1", "claude-sonnet-4-6"],
    "role_table": {"implementer": "claude-opus-4-1"},
    "confidence_stats": {},
    "total_observations": 10
}"#;

/// The detection table's header, over [`Outcome::row`]'s columns.
const HEADER: &str = "case               reason               n_L n_opp probe collateral";

/// A workspace whose knowledge covers each topic with a strong entry, all
/// three of its words, and a weak one, two of them, so a healthy reader
/// scores a task's candidates apart; and whose cascade router is
/// [`ROUTER`].
fn workspace() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp dir");
    let roko = dir.path().join(".roko");
    std::fs::create_dir_all(roko.join("neuro")).expect("the knowledge store's directory");
    std::fs::create_dir_all(roko.join("learn")).expect("the learn directory");
    let mut knowledge = String::new();
    for (index, words) in TOPICS.into_iter().enumerate() {
        knowledge.push_str(&topic_entries(index, words));
    }
    std::fs::write(roko.join("neuro/knowledge.jsonl"), knowledge).expect("seed the knowledge");
    std::fs::write(roko.join("learn/cascade-router.json"), ROUTER).expect("seed the router");
    dir
}

/// The knowledge store's lines for topic `index`: its strong entry and its
/// weak one. Distinct confidences rank a degenerate reader's entries one
/// way.
fn topic_entries(index: usize, [first, second, third]: [&str; 3]) -> String {
    let strong = serde_json::json!({
        "id": format!("kn-{index}-strong"),
        "content": format!("Tune {first} {second} {third} together"),
        "confidence": 0.8 - index as f64 / 100.0,
    });
    let weak = serde_json::json!({
        "id": format!("kn-{index}-weak"),
        "content": format!("{first} {second} notes"),
        "confidence": 0.6,
    });
    format!("{strong}\n{weak}\n")
}

/// A production dispatcher over [`ROUTER`].
fn dispatcher() -> Dispatcher {
    let models = MODELS.iter().map(ToString::to_string).collect();
    let router = CascadeRouter::from_snapshot_json(ROUTER, models).expect("a cascade router");
    Dispatcher::new(
        Some(Arc::new(router)),
        PromptAssembler::new(),
        WarmPool::new(0),
        HashSet::new(),
    )
}

/// The task of context `context`: its topic's three words, and no model
/// hint, so the cascade router routes it.
fn topic_task(context: usize) -> TaskDef {
    let [first, second, third] = TOPICS[context % TOPICS.len()];
    let task = serde_json::json!({
        "id": format!("t{context}"),
        "title": format!("{first} {second} {third}"),
        "role": "implementer",
    });
    serde_json::from_value(task).expect("a valid task")
}

/// The dispatch context of the attempt `key` in the workspace `workdir`,
/// whose chain drew `arms`: a first try, routed by the cascade router, with
/// no budget pressure and no enrichment.
fn dispatch_context(workdir: &Path, key: &AttemptKey, arms: Arc<ArmSet>) -> DispatchContext {
    DispatchContext {
        plan_id: PLAN.to_string(),
        role: "implementer".to_string(),
        workdir: workdir.to_path_buf(),
        model_hint: None,
        force_backend: None,
        budget_remaining_usd: f64::MAX,
        attempt: 0,
        ladder_step: 0,
        prompt_experiment: None,
        gate_feedback: None,
        routing_context: Some(RoutingContext::default()),
        dependency_outputs: Vec::new(),
        error_patterns: Default::default(),
        cached_workspace_map: String::new(),
        cached_workspace_context: String::new(),
        concurrent_plans: Vec::new(),
        attempt_key: Some(key.clone()),
        arm_set: Some(arms),
        self_model_rung: None,
        skip_enrichment: true,
    }
}

/// The reason a loop broken by `kind` is flagged with (S03 §4.6).
const fn flagged_as(kind: FaultKind) -> ReasonCode {
    match kind {
        FaultKind::Cut => ReasonCode::Cut,
        FaultKind::Mask => ReasonCode::Mask,
        FaultKind::Degenerate => ReasonCode::Degenerate,
        FaultKind::LabelOnly => ReasonCode::LabelOnly,
        FaultKind::Stale => ReasonCode::Stale,
        FaultKind::Unlogged => ReasonCode::Unlogged,
        FaultKind::Harmful => ReasonCode::Harm,
    }
}

/// One epoch's run, as an audit tick reads it, and its last `seq`.
struct Replay {
    records: RunRecords,
    seq: u64,
}

impl Replay {
    fn new(run_id: String) -> Self {
        Self {
            records: RunRecords {
                run_id,
                ..RunRecords::default()
            },
            seq: 0,
        }
    }

    /// `record` as the run's next line of `file`, in the writer's envelope.
    fn line<T>(&mut self, file: RunFile, schema: &str, record: T) -> Stamped<T> {
        self.seq += 1;
        self.records.seqs.push((file, self.seq));
        Stamped {
            schema_version: schema.to_string(),
            record_id: format!("{}:{}", self.records.run_id, self.seq),
            seq: self.seq,
            ts: "2026-10-05T00:00:00Z".to_string(),
            record,
        }
    }

    /// Plan context `context` dry, as its chain's first attempt, with the
    /// arms its chain draws over `registry` in `epoch`, and add the rows
    /// dispatch writes for it (`record_planned_attempt`): its route row, its
    /// content rows and its placebo row; then its verdict, whose provider
    /// report names the model it routed to.
    fn context(
        &mut self,
        dispatcher: &Dispatcher,
        workdir: &Path,
        registry: &Registry,
        epoch: &str,
        context: usize,
    ) {
        let task = topic_task(context);
        let key = AttemptKey::new(&self.records.run_id, PLAN, &task.id, 1);
        let identity = AttemptIdentity::new(&key);
        let draws = ArmDraws::new(0, epoch);
        let arms = Arc::new(ArmSet::assign(&key, registry, &ArmMode::Normal, &draws));
        let ctx = dispatch_context(workdir, &key, Arc::clone(&arms));
        let plan = faults::dry_run(|| dispatcher.plan(&task, &ctx)).expect("a dry-run plan");
        let mut route = plan.route_decision.clone().expect("a route row");
        route.attempt_key = Some(key.attempt_key());
        route.trace_id = key.attempt_key();
        route.task_id.clone_from(&task.id);
        route.arm_set = Some(ArmSet::clone(&arms));
        let route = self.line(RunFile::Decisions, DECISION_SCHEMA, route);
        self.records.decisions.push(route);
        let times = (1_000, 1_001);
        let contents = faults::dry_run(|| {
            planned_content_decisions(workdir, &plan, &identity, Some(arms.as_ref()), times)
        });
        for decision in contents {
            let decision = self.line(RunFile::Decisions, DECISION_SCHEMA, decision);
            self.records.content_decisions.push(decision);
        }
        if let Some(assignment) = arms.placebo() {
            let placebo = PlaceboDecisionRecord::new(identity.clone(), assignment.clone());
            let placebo = self.line(RunFile::Decisions, DECISION_SCHEMA, placebo);
            self.records.placebo_decisions.push(placebo);
        }
        let mut verdict = AttemptVerdictRecord::settle(identity, AttemptOutcome::Passed, true);
        verdict.executed.model_reported = Some(plan.model.slug.clone());
        let verdict = self.line(RunFile::Attempts, VERDICT_SCHEMA, verdict);
        self.records.verdicts.push(verdict);
    }
}

/// What one epoch's replay showed.
#[derive(Debug, Default)]
struct Outcome {
    /// The reason the faulted loop was flagged with, once it was.
    reason: Option<ReasonCode>,
    /// The faulted loop's opportunities then: on the learned arm, and on
    /// both arms.
    ttd: Option<(u64, u64)>,
    /// The other loops that moved, each with its reason.
    collateral: Vec<String>,
    /// The canary's first failing probe, with the flag still set.
    first_failure: Option<String>,
    /// Whether the placebo moved, or the auditor broke.
    broken: bool,
}

impl Outcome {
    /// Its row of the detection table, under `case`.
    fn row(&self, case: &str) -> String {
        let reason = self.reason.map_or("-", ReasonCode::as_str);
        let (learned, all) = self.ttd.map_or_else(
            || ("-".to_string(), "-".to_string()),
            |(learned, all)| (learned.to_string(), all.to_string()),
        );
        let probe = self.first_failure.as_deref().unwrap_or("-");
        let collateral = self.collateral.join(", ");
        format!("{case:<18} {reason:<18} {learned:>5} {all:>5} {probe:<5} {collateral}")
    }
}

/// Replay epoch `epoch` of [`TASKS`] contexts in `workdir`, with the fault
/// flag `fault` set if there is one, evaluating the loops after each context
/// until the faulted loop moves; then trace its canary.
fn replay_epoch(workdir: &Path, fault: Option<(&str, FaultKind)>, epoch: usize) -> Outcome {
    let registry = Registry::embedded().expect("the embedded loop registry");
    let config = LearningAuditConfig::default();
    let template = LoopAuditor::from_records(registry.clone(), &config, &[]);
    let mut census = CensusState::new(template.params().loop_alpha());
    let dispatcher = dispatcher();
    let target = fault.map(|(loop_id, _)| loop_id);
    let label = fault.map_or_else(
        || "healthy".to_string(),
        |(loop_id, kind)| format!("{loop_id}-{kind:?}"),
    );
    let mut replay = Replay::new(format!("e1-{label}-{epoch}"));
    let draws_epoch = format!("e1-{epoch}");
    let mut outcome = Outcome::default();
    for context in 0..TASKS {
        replay.context(&dispatcher, workdir, &registry, &draws_epoch, context);
        census.fold(&replay.records, u64::MAX);
        let mut auditor = template.clone();
        for observation in auditor.observe_census(&census, &RowOrigin::default(), Utc::now()) {
            let LoopAuditRow::Health(health) = &observation.health.row else {
                continue;
            };
            let loop_id = observation.health.loop_id.as_str();
            let moved = observation.transition.is_some();
            outcome.broken |= !health.placebo_ok || (loop_id == "L-placebo" && moved);
            if !moved {
                continue;
            }
            if target != Some(loop_id) {
                let reason = health.reason.map_or("-", ReasonCode::as_str);
                outcome.collateral.push(format!("{loop_id} {reason}"));
            } else if outcome.reason.is_none() {
                outcome.reason = health.reason;
                outcome.ttd = Some((health.n_learned, health.n_opp));
            }
        }
        if outcome.reason.is_some() {
            break;
        }
    }
    if let Some(loop_id) = target {
        let runner = DryCanaryRunner::new(workdir);
        let row = runner.trace(loop_id).expect("a canary row");
        outcome.first_failure = row.first_failure;
    }
    outcome
}

/// [`replay_epoch`] with `loop_id` broken by `kind`, on a fault registry of
/// this test's own: the flag affects each context's plan and rows as one
/// decision, and the canary's trace.
fn inject(workdir: &Path, loop_id: &str, kind: FaultKind, epoch: usize) -> Outcome {
    let file = workdir.join(format!(".roko/faults-{loop_id}-{kind:?}-{epoch}.jsonl"));
    faults::isolated(FaultActor::Env, file, || {
        let spec = FaultSpec {
            loop_id: loop_id.to_string(),
            kind,
            ttl_secs: faults::MAX_TTL_SECS,
            max_decisions: 1000,
            spend_cap_usd: None,
        };
        faults::set(spec).expect("set the fault flag");
        let outcome = replay_epoch(workdir, Some((loop_id, kind)), epoch);
        assert!(faults::clear(loop_id), "the flag lasted the whole replay");
        outcome
    })
}

/// The median of `values`, the lower middle one of an even count; 0 when
/// there are none.
fn median(values: &[u64]) -> u64 {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let middle = sorted.len().saturating_sub(1) / 2;
    sorted.get(middle).copied().unwrap_or(0)
}

/// S03 C2, C3 and A4 on E1's dry-run replay (backlog 5130): each structural
/// fault is detected with its own reason, in time, on its own loop, and its
/// canary stops at the injected site; the placebo never moves. It prints the
/// detection table.
#[test]
fn structural_faults_detected_and_localized() {
    let dir = workspace();
    let workdir = dir.path();
    // Each fault: its loop, its kind, and the probe its canary should stop
    // at, where the canary can see it.
    let cases = [
        ("L-know", FaultKind::Cut, Some("P2")),
        ("L-know", FaultKind::Stale, Some("P2")),
        ("L-know", FaultKind::Unlogged, Some("P6")),
        ("L-route", FaultKind::Mask, Some("P4")),
        ("L-know", FaultKind::Degenerate, None),
        ("L-know", FaultKind::LabelOnly, None),
    ];
    let healthy: Vec<Outcome> = (0..EPOCHS)
        .map(|epoch| replay_epoch(workdir, None, epoch))
        .collect();
    let injected: Vec<Vec<Outcome>> = cases
        .iter()
        .map(|&(loop_id, kind, _)| {
            (0..EPOCHS)
                .map(|epoch| inject(workdir, loop_id, kind, epoch))
                .collect()
        })
        .collect();

    let mut table = vec![HEADER.to_string()];
    for outcome in &healthy {
        table.push(outcome.row("healthy"));
    }
    for (&(loop_id, kind, _), outcomes) in cases.iter().zip(&injected) {
        let case = format!("{loop_id} {kind:?}");
        table.extend(outcomes.iter().map(|outcome| outcome.row(&case)));
    }
    println!("{}", table.join("\n"));

    // A healthy replay flags no loop.
    for outcome in &healthy {
        assert!(outcome.collateral.is_empty(), "{outcome:?}");
        assert!(!outcome.broken, "a healthy replay broke the auditor");
    }
    for (&(loop_id, kind, probe), outcomes) in cases.iter().zip(&injected) {
        let case = format!("{loop_id} {kind:?}");
        // C2: detected with the fault's reason, in time.
        let expected = Some(flagged_as(kind));
        let detected = outcomes
            .iter()
            .filter(|outcome| outcome.reason == expected)
            .count();
        let rate_ok = detected * 100 >= outcomes.len() * 95;
        assert!(rate_ok, "{case}: {outcomes:?}");
        let structural = matches!(kind, FaultKind::Degenerate | FaultKind::LabelOnly);
        let ttds: Vec<u64> = outcomes
            .iter()
            .filter_map(|outcome| outcome.ttd)
            .map(|(learned, all)| if structural { all } else { learned })
            .collect();
        let bound = match kind {
            FaultKind::LabelOnly => 1,
            FaultKind::Degenerate => 50,
            _ => 30,
        };
        assert!(median(&ttds) <= bound, "{case}: {ttds:?}");
        let at_once = kind != FaultKind::LabelOnly || ttds.iter().all(|ttd| *ttd == 1);
        assert!(at_once, "{case}: {ttds:?}");
        // C3: the canary stops at the injected site.
        if let Some(probe) = probe {
            let localized = outcomes
                .iter()
                .filter(|outcome| outcome.first_failure.as_deref() == Some(probe))
                .count();
            let rate_ok = localized * 100 >= outcomes.len() * 90;
            assert!(rate_ok, "{case}: {outcomes:?}");
        }
        // The flag lands on its own loop, and A4: the placebo holds.
        for outcome in outcomes {
            assert!(outcome.collateral.is_empty(), "{case}: {outcome:?}");
            assert!(!outcome.broken, "{case}: the auditor broke");
        }
    }
}
