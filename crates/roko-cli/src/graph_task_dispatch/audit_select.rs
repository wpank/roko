//! DP1, the audit lottery at settle (S05 §4.2, G26). With `[audit] enabled`,
//! every green attempt draws one keyed lottery ticket when its verdict
//! settles, and the draw is only logged: audits run later, off the critical
//! path (7123), and never change the verdict. Nothing here reaches a prompt,
//! and nothing waits on an audit.
//!
//! - When a run's first attempt opens, its key K_run derives from the
//!   workspace audit secret in the vault, and `audit.key_commit` goes to the
//!   vault ledger. When the run closes, `audit.estimate` lists its green
//!   units per stratum, `forced_accept` included at 0, and `audit.key_reveal`
//!   reveals the key, so anyone can recompute every draw
//!   (`roko_gate::audit::policy::verify_reveal`).
//! - The stratum is the green verdict: `passed`,
//!   `passed_with_preexisting_failures` and `already_satisfied` are drawn at
//!   the policy's π (ρ from `[audit]`), `unverified` and the reserved
//!   `forced_accept` at π = 1 (decisions 7102 and 7103). A failed attempt
//!   draws nothing.
//! - The draw is keyed on the run, the task, the attempt and the attempt's
//!   result tree, which stands in for S05's accepted commit (Graph attempts
//!   do not commit). The pre-verify screen notes the trees ([`AuditSelector::note_trees`]);
//!   a selected unit's tree is pinned under `refs/roko/audit/<sel_id>`, so
//!   git gc keeps it for the audit worker.
//! - The attempt's output is scanned for hidden-suite canaries
//!   (`audit.leak_canary`).
//! - Each run has an audit worker (`crate::audit::worker`, 7123), started
//!   when the run opens: a selected unit goes to it with the task inputs
//!   noted when its attempt opened ([`AuditSelector::note_task`]), and the
//!   run's close waits for it to drain, for at most `[audit] drain_secs`,
//!   before the key is revealed.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use roko_core::audit_home::AuditVault;
use roko_core::audit_types::Stratum;
use roko_core::config::GatesConfig;
use roko_core::config::audit::AuditConfig;
use roko_gate::audit::canary::{CanaryScanner, scan_text};
use roko_gate::audit::hidden::HiddenStore;
use roko_gate::audit::ledger::{AuditEvent, AuditLedger};
use roko_gate::audit::policy::{
    InclusionParams, RunKey, inclusion_probability, select, workspace_secret,
};
use roko_learn::telemetry::records::{AttemptOutcome, AttemptVerdictRecord, GateVerdictTag};

use crate::audit::b1::{B1, FactoryAuthor, SuiteAuthor};
use crate::audit::b2::B2;
use crate::audit::b3::{B3, Reviewer};
use crate::audit::worker::{
    AuditTask, AuditUnit, AuditWorker, GamingWatch, PhaseB, WorkerContext, queue_unit,
};
use crate::task_parser::TaskDef;

/// Every stratum a run reports, in order.
const STRATA: [&str; 5] = [
    "passed",
    "passed_with_preexisting_failures",
    "already_satisfied",
    "unverified",
    "forced_accept",
];

/// The lottery of one workspace's runs.
pub(super) struct AuditSelector {
    workdir: PathBuf,
    secret: Vec<u8>,
    params: InclusionParams,
    ledger: parking_lot::Mutex<AuditLedger>,
    hidden: Option<HiddenStore>,
    /// Attempt key → (base tree, result tree), from the pre-verify screen.
    trees: parking_lot::Mutex<HashMap<String, (String, String)>>,
    /// Attempt key → what an audit needs of its task, noted at open.
    tasks: parking_lot::Mutex<HashMap<String, AuditTask>>,
    runs: parking_lot::Mutex<HashMap<String, RunDraws>>,
    vault: AuditVault,
    config: AuditConfig,
    gates: GatesConfig,
    /// The phase-B checks each run's worker runs.
    phase_b: PhaseB,
    /// The workspace's gate-gaming detector (F1), fed every settled attempt.
    gaming: GamingWatch,
    /// Draw every green unit at π = 1 ([`Self::census`]).
    census: AtomicBool,
}

/// One open run: its key, its green units per stratum, its model spend and
/// its audit worker.
struct RunDraws {
    key: RunKey,
    strata: BTreeMap<&'static str, u64>,
    spend: Arc<parking_lot::Mutex<f64>>,
    worker: Option<AuditWorker>,
}

impl AuditSelector {
    /// The lottery of the workspace at `workdir` when `[audit] enabled`, or
    /// `None`, with a warning, when the vault cannot be used.
    pub(super) fn for_config(
        config: &AuditConfig,
        gates: &GatesConfig,
        workdir: &Path,
    ) -> Option<Self> {
        if !config.enabled {
            return None;
        }
        match Self::open(config, gates, workdir) {
            Ok(selector) => Some(selector),
            Err(error) => {
                tracing::warn!(%error, "[audit] is enabled, but no attempt is drawn");
                None
            }
        }
    }

    /// The lottery's vault, secret and ledger.
    fn open(config: &AuditConfig, gates: &GatesConfig, workdir: &Path) -> Result<Self, String> {
        let vault = config.vault(workdir).map_err(|error| error.to_string())?;
        let secret = workspace_secret(&vault).map_err(|error| error.to_string())?;
        let ledger = AuditLedger::open(&vault)
            .map_err(|error| error.to_string())?
            .with_mirror(workdir.join(".roko/audit/audits.jsonl"));
        Ok(Self {
            workdir: workdir.to_path_buf(),
            secret,
            params: InclusionParams {
                rho: config.rho,
                ..InclusionParams::default()
            },
            ledger: parking_lot::Mutex::new(ledger),
            hidden: HiddenStore::open(&vault).ok(),
            gaming: GamingWatch::new(&vault),
            trees: parking_lot::Mutex::new(HashMap::new()),
            tasks: parking_lot::Mutex::new(HashMap::new()),
            runs: parking_lot::Mutex::new(HashMap::new()),
            vault,
            config: config.clone(),
            gates: gates.clone(),
            phase_b: PhaseB::default(),
            census: AtomicBool::new(false),
        })
    }

    /// The lottery, whose runs' workers run `phase_b`.
    #[must_use]
    pub(super) fn with_phase_b(mut self, phase_b: PhaseB) -> Self {
        self.phase_b = phase_b;
        self
    }

    /// Draw every green unit at π = 1, as a test's ρ = 1 would; config
    /// cannot set ρ above `RHO_MAX`.
    #[cfg(test)]
    pub(super) fn census(&self) {
        self.census.store(true, Ordering::Relaxed);
    }

    /// Open run `run_id`: derive its key, commit to it and start its audit
    /// worker, once.
    pub(super) fn open_run(&self, run_id: &str) {
        let mut runs = self.runs.lock();
        if runs.contains_key(run_id) {
            return;
        }
        let key = match RunKey::derive(&self.secret, run_id) {
            Ok(key) => key,
            Err(error) => {
                tracing::warn!(
                    run_id,
                    %error,
                    "no audit key for the run; its attempts are not drawn"
                );
                return;
            }
        };
        if let Err(error) = self.ledger.lock().commit_key(run_id, &key) {
            tracing::warn!(run_id, %error, "audit.key_commit not written; the run is not drawn");
            return;
        }
        let strata = STRATA.iter().map(|stratum| (*stratum, 0)).collect();
        let spend = Arc::new(parking_lot::Mutex::new(0.0));
        let worker = self.start_worker(run_id, &spend);
        let draws = RunDraws {
            key,
            strata,
            spend,
            worker,
        };
        runs.insert(run_id.to_string(), draws);
    }

    /// Start run `run_id`'s audit worker; without one, the run's selected
    /// units wait in the ledger for a later run's.
    fn start_worker(
        &self,
        run_id: &str,
        spend: &Arc<parking_lot::Mutex<f64>>,
    ) -> Option<AuditWorker> {
        let context = WorkerContext {
            workdir: self.workdir.clone(),
            vault: self.vault.clone(),
            config: self.config.clone(),
            gates: self.gates.clone(),
            secret: self.secret.clone(),
            run_id: run_id.to_string(),
            run_spend: Arc::clone(spend),
            phase_b: self.phase_b.clone(),
            gaming: self.gaming.clone(),
        };
        match AuditWorker::start(context) {
            Ok(worker) => Some(worker),
            Err(error) => {
                tracing::warn!(
                    run_id,
                    %error,
                    "no audit worker: the run's selected units wait in the ledger"
                );
                None
            }
        }
    }

    /// Close run `run_id`: let its audit worker drain, list its green units
    /// per stratum and reveal its key.
    pub(super) fn close_run(&self, run_id: &str) {
        let Some(mut run) = self.runs.lock().remove(run_id) else {
            return;
        };
        if let Some(worker) = run.worker.take()
            && !worker.drain(Duration::from_secs(self.config.drain_secs))
        {
            tracing::warn!(
                run_id,
                "the audit drain timed out; the units it did not reach wait in the ledger"
            );
        }
        let mut ledger = self.ledger.lock();
        let summary = AuditEvent::Estimate {
            window: format!("run:{run_id}"),
            stratum: None,
            estimate: serde_json::json!({ "green_units": run.strata }),
        };
        if let Err(error) = ledger.append(summary) {
            tracing::warn!(run_id, %error, "the run's audit strata were not written");
        }
        if let Err(error) = ledger.reveal_key(run_id, &run.key) {
            tracing::warn!(run_id, %error, "audit.key_reveal not written");
        }
    }

    /// Note what an audit needs of `task`, which the attempt `attempt_key`
    /// runs.
    pub(super) fn note_task(&self, attempt_key: &str, task: &TaskDef) {
        let task = audit_task(task);
        self.tasks.lock().insert(attempt_key.to_string(), task);
    }

    /// Note the trees the pre-verify screen saw for the attempt
    /// `attempt_key`.
    pub(super) fn note_trees(&self, attempt_key: &str, base: &str, result: &str) {
        let trees = (base.to_string(), result.to_string());
        self.trees.lock().insert(attempt_key.to_string(), trees);
    }

    /// DP1: draw `verdict`'s attempt when it is green, and scan its output
    /// for canaries; a selected unit goes to the run's audit worker. Logs
    /// and returns; never fails the attempt.
    pub(super) fn draw(&self, verdict: &AttemptVerdictRecord, output: Option<&str>) {
        let identity = &verdict.identity;
        let trees = self.trees.lock().remove(&identity.attempt_key);
        let task = self.tasks.lock().remove(&identity.attempt_key);
        if let Some(output) = output {
            self.scan(output);
        }
        let model = verdict
            .executed
            .model_dispatched
            .clone()
            .or_else(|| verdict.executed.model_requested.clone())
            .unwrap_or_default();
        // F1: the gaming detector counts every gate verdict.
        match verdict.outcome {
            AttemptOutcome::GateFailed => self.gaming.gate(&model, false),
            _ if verdict.gate_verdict.is_some() => self.gaming.gate(&model, true),
            _ => {}
        }
        let mut runs = self.runs.lock();
        let Some(run) = runs.get_mut(&identity.run_id) else {
            return;
        };
        *run.spend.lock() += attempt_usd(verdict);
        let Some((stratum, census)) = verdict.gate_verdict.map(stratum_of) else {
            return;
        };
        let pi = if census || self.census.load(Ordering::Relaxed) {
            Ok(1.0)
        } else {
            inclusion_probability(&self.params, None, None)
        };
        let (base_tree, result_tree) =
            trees.map_or((None, None), |(base, result)| (Some(base), Some(result)));
        let commit = result_tree.as_deref().unwrap_or("-");
        let (run_id, task_id) = (&identity.run_id, &identity.task_id);
        let attempt = &identity.attempt_key;
        let drawn = pi.and_then(|pi| select(&run.key, run_id, task_id, attempt, commit, pi));
        let selection = match drawn {
            Ok(selection) => selection,
            Err(error) => {
                tracing::warn!(attempt_key = %attempt, %error, "the audit draw failed");
                return;
            }
        };
        *run.strata.entry(stratum).or_default() += 1;
        drop(runs);
        let sel_id = format!("sel-{}", &selection.prf_u[2..14]);
        if selection.selected
            && let Some(tree) = result_tree.as_deref()
        {
            self.pin(&sel_id, tree);
        }
        let task = task.unwrap_or_default();
        let task_type = task.kind.clone();
        let unit = selection.selected.then(|| AuditUnit {
            sel_id: sel_id.clone(),
            attempt_key: attempt.clone(),
            run_id: run_id.clone(),
            plan_id: identity.plan_id.clone(),
            task_id: task_id.clone(),
            pi: selection.pi,
            base_tree: base_tree.clone(),
            result_tree: result_tree.clone(),
            model: model.clone(),
            task,
        });
        let event = AuditEvent::Selection {
            sel_id,
            attempt_key: attempt.clone(),
            run_id: run_id.clone(),
            task_id: task_id.clone(),
            // The task's kind, noted when its attempt opened.
            stratum: Stratum {
                task_type,
                model,
                arm: "prod".to_string(),
                verdict: stratum.to_string(),
            },
            pi: selection.pi,
            prf_u: selection.prf_u,
            selected: selection.selected,
            base_tree,
            result_tree,
        };
        if let Err(error) = self.ledger.lock().append(event) {
            tracing::warn!(attempt_key = %attempt, %error, "audit.selection not written");
        }
        if let Some(unit) = unit {
            self.queue(unit);
        }
    }

    /// Hand a selected unit to its run's worker, once its task inputs are
    /// kept in the vault; while the worker's channel is full, the unit
    /// waits in the ledger.
    fn queue(&self, unit: AuditUnit) {
        if let Err(error) = queue_unit(&self.vault, &unit) {
            tracing::warn!(sel_id = %unit.sel_id, %error, "the unit's task inputs were not kept");
        }
        let runs = self.runs.lock();
        let worker = runs.get(&unit.run_id).and_then(|run| run.worker.as_ref());
        if !worker.is_some_and(|worker| worker.submit(unit)) {
            tracing::debug!("the audit worker is busy; the selected unit waits in the ledger");
        }
    }

    /// Log every hidden-suite canary in `output` and expose its suite.
    fn scan(&self, output: &str) {
        let hits = scan_text(output);
        let Some(store) = self.hidden.as_ref().filter(|_| !hits.is_empty()) else {
            return;
        };
        let mut ledger = self.ledger.lock();
        if let Err(error) = CanaryScanner::new(store).report(&mut ledger, "output", &hits) {
            tracing::warn!(%error, "a hidden-suite canary in an agent's output was not logged");
        }
    }

    /// Keep `tree` from git gc under `refs/roko/audit/<sel_id>`.
    fn pin(&self, sel_id: &str, tree: &str) {
        let pinned = std::process::Command::new("git")
            .arg("-C")
            .arg(&self.workdir)
            .args(["update-ref", &format!("refs/roko/audit/{sel_id}"), tree])
            .output();
        if !pinned.is_ok_and(|output| output.status.success()) {
            tracing::warn!(sel_id, tree, "the selected unit's tree was not pinned");
        }
    }
}

impl super::GraphTaskDispatcher {
    /// The audit workers' phase-B checks: B1 and B3, with every configured
    /// model a candidate author or reviewer, in `[models]` order, and B2.
    pub(super) fn audit_phase_b(&self) -> PhaseB {
        let timeout_ms = self
            .config
            .timeouts
            .llm_call_secs
            .max(1)
            .saturating_mul(1_000);
        let authors = self
            .config
            .models
            .iter()
            .map(|(key, profile)| {
                let author = FactoryAuthor::new(
                    Arc::clone(&self.factory),
                    key.clone(),
                    profile.slug.clone(),
                    timeout_ms,
                );
                Arc::new(author) as Arc<dyn SuiteAuthor>
            })
            .collect();
        let reviewers = self
            .config
            .models
            .iter()
            .map(|(key, profile)| Reviewer {
                model: profile.slug.clone(),
                agent: Arc::new(super::routing_context::CheapFactoryAgent {
                    factory: Arc::clone(&self.factory),
                    model_key: key.clone(),
                    workdir: self.workdir.clone(),
                    timeout_ms,
                }),
            })
            .collect();
        PhaseB {
            b1: Some(Arc::new(B1::new(authors, self.config.audit.clone()))),
            b2: Some(Arc::new(B2)),
            b3: Some(Arc::new(B3::new(reviewers, self.config.audit.clone()))),
        }
    }
}

/// What an audit needs of `task`; its kind is its domain, else its role
/// (`scribe` writes docs, `researcher` research, `strategist` plans).
fn audit_task(task: &TaskDef) -> AuditTask {
    let kind = match (&task.domain, task.role.as_deref().unwrap_or("implementer")) {
        (Some(domain), _) => domain.label(),
        (None, "scribe") => "docs",
        (None, "researcher") => "research",
        (None, "strategist" | "planner") => "plan",
        (None, role) => role,
    };
    let hidden = task.spec.hidden.clone().unwrap_or_default();
    AuditTask {
        title: task.title.clone(),
        description: task.description.clone().unwrap_or_default(),
        goal: task.spec.goal.clone().unwrap_or_default(),
        acceptance: task.acceptance.clone(),
        hidden_suite: hidden.suite,
        interface: hidden.interface,
        properties: hidden.properties,
        files: task.files.clone(),
        verify: task
            .verify
            .iter()
            .map(|step| (step.phase.clone(), step.command.clone()))
            .collect(),
        kind: kind.to_string(),
    }
}

/// What an attempt's model calls cost, in USD: their API-equivalent price,
/// else the vendor's or the billed figure.
fn attempt_usd(verdict: &AttemptVerdictRecord) -> f64 {
    let cost = &verdict.cost;
    cost.api_equiv_usd
        .or(cost.vendor_usd)
        .or(cost.billed_usd)
        .unwrap_or(0.0)
        .max(0.0)
}

/// A green verdict's stratum, and whether it is a census stratum drawn at
/// π = 1.
fn stratum_of(tag: GateVerdictTag) -> (&'static str, bool) {
    match tag {
        GateVerdictTag::Passed => ("passed", false),
        GateVerdictTag::PassedWithPreexistingFailures => {
            ("passed_with_preexisting_failures", false)
        }
        GateVerdictTag::AlreadySatisfied => ("already_satisfied", false),
        GateVerdictTag::Unverified => ("unverified", true),
        GateVerdictTag::ForcedAccept => ("forced_accept", true),
    }
}

#[cfg(test)]
mod tests {
    use roko_gate::audit::ledger::{LedgerRecord, records, verify_chain};
    use roko_gate::audit::policy::{Selection, verify_reveal};
    use roko_graph::cells::NoopAttemptRecorder;

    use super::*;
    use crate::graph_task_dispatch::diff_snapshot::tests::commit_repo;
    use crate::graph_task_dispatch::tests::{
        VERIFY_PROVIDER, make_spec, make_test_dispatcher, no_auto_fix, recording_feedback,
        verify_step,
    };
    use crate::graph_task_dispatch::{
        CellContext, GraphTaskDispatcher, StreamingTaskDispatcher, TaskDispatcher, TaskLease,
        streaming_event_channel_capacity,
    };

    const RUN: &str = "graph-audit-run";

    /// S05 DP1: with `[audit] enabled`, five tasks of one run (three green,
    /// one of them unverified, and two failing verify), through `dispatch`
    /// and `dispatch_streaming`, give one `audit.selection` per green unit
    /// and none for a failed one, between the run's key commit and reveal;
    /// the revealed key reproduces every draw.
    #[tokio::test]
    async fn every_green_attempt_draws_one_audit_selection() {
        let temp = tempfile::tempdir().expect("tempdir");
        let vault_home = tempfile::tempdir().expect("vault home");
        let home = vault_home.path().join("audit");
        let (dispatcher, task) = make_test_dispatcher(
            &temp,
            VERIFY_PROVIDER,
            |config| {
                no_auto_fix(config);
                config.audit.enabled = true;
                config.audit.home = Some(home.clone());
            },
            recording_feedback(temp.path()),
        )
        .await;
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        let with = |id: &str, verify: Option<&str>| {
            let mut task = task.clone();
            task.id = id.to_string();
            task.verify = verify
                .map(|command| vec![verify_step("check", command)])
                .unwrap_or_default();
            task
        };
        let batch = [
            (with("T1", Some("true")), true),
            (with("T2", None), true),
            (with("T3", Some("false")), false),
        ];
        for (task, green) in &batch {
            let result = dispatcher
                .dispatch(&make_spec(task), Vec::new(), &ctx)
                .await;
            assert_eq!(result.is_ok(), *green, "{}: {result:?}", task.id);
        }
        let lease = TaskLease {
            path: temp.path().to_path_buf(),
            fingerprint: "test-fingerprint".to_string(),
        };
        for task in [with("T4", Some("true")), with("T5", Some("false"))] {
            let (event_tx, _event_rx) =
                tokio::sync::mpsc::channel(streaming_event_channel_capacity());
            let spec = make_spec(&task);
            let recorder = NoopAttemptRecorder;
            let _ = dispatcher
                .dispatch_streaming(&spec, Vec::new(), &ctx, &lease, event_tx, &recorder)
                .await;
        }
        let _ = GraphTaskDispatcher::close_run_attempts(&dispatcher, RUN);
        drop(dispatcher);

        let vault = roko_core::config::audit::AuditConfig {
            home: Some(home),
            ..Default::default()
        }
        .vault(temp.path())
        .expect("the vault");
        let ledger = vault.ledger_dir();
        let records = verify_chain(&ledger).expect("an unbroken chain");
        let mut events: Vec<LedgerRecord> = Vec::new();
        let mut files: Vec<_> = std::fs::read_dir(&ledger)
            .expect("the ledger")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
            .collect();
        files.sort();
        for file in files {
            let text = std::fs::read_to_string(file).expect("a day file");
            events.extend(
                text.lines()
                    .map(|line| serde_json::from_str(line).expect("a record")),
            );
        }
        assert_eq!(events.len() as u64, records);

        let position = |name: &str| events.iter().position(|record| record.event.name() == name);
        let commit_at = position("audit.key_commit").expect("a key commit");
        let first_selection = position("audit.selection").expect("a selection");
        let reveal_at = position("audit.key_reveal").expect("a key reveal");
        assert!(commit_at < first_selection && first_selection < reveal_at);

        let mut selections = Vec::new();
        let mut committed = String::new();
        let mut key_hex = String::new();
        let mut strata = serde_json::Value::Null;
        for record in &events {
            match &record.event {
                AuditEvent::KeyCommit { commitment, .. } => committed = commitment.clone(),
                AuditEvent::KeyReveal { key_hex: hex, .. } => key_hex = hex.clone(),
                AuditEvent::Estimate { estimate, .. } => strata = estimate["green_units"].clone(),
                AuditEvent::Selection {
                    run_id,
                    task_id,
                    attempt_key,
                    stratum,
                    pi,
                    prf_u,
                    selected,
                    result_tree,
                    ..
                } => {
                    assert!(*pi >= 0.05, "{task_id}: pi {pi}");
                    if stratum.verdict == "unverified" {
                        assert!((*pi - 1.0).abs() < f64::EPSILON, "{task_id}: pi {pi}");
                        assert!(*selected, "pi = 1 always selects");
                    }
                    selections.push(Selection {
                        run_id: run_id.clone(),
                        task_id: task_id.clone(),
                        attempt_id: attempt_key.clone(),
                        accepted_commit: result_tree.clone().unwrap_or_else(|| "-".to_string()),
                        pi: *pi,
                        prf_u: prf_u.clone(),
                        selected: *selected,
                    });
                }
                _ => {}
            }
        }
        let mut drawn: Vec<&str> = selections.iter().map(|row| row.task_id.as_str()).collect();
        drawn.sort_unstable();
        assert_eq!(
            drawn,
            ["T1", "T2", "T4"],
            "one draw per green unit, none for a failure"
        );
        assert_eq!(strata["passed"], 2);
        assert_eq!(strata["unverified"], 1);
        assert_eq!(
            strata["forced_accept"], 0,
            "the reserved stratum is present, with no units"
        );
        let key = RunKey::from_hex(&key_hex).expect("the revealed key");
        let mismatches = verify_reveal(&key, RUN, &committed, &selections);
        assert!(mismatches.is_empty(), "{mismatches:?}");
        let mirror = temp.path().join(".roko/audit/audits.jsonl");
        assert!(mirror.exists(), "the workspace mirror is written");
    }

    /// 7123: the implementer passes verify by adding a `cfg!(test)` branch
    /// to product code, and is selected (every unit is, as at ρ = 1). The
    /// run's audit worker gives the ledger an `audit.result` with G = 1 and
    /// the A1 finding, Y = 0 from three clean passes, and the chain holds.
    #[tokio::test]
    async fn a_planted_test_detection_yields_an_audit_result_with_an_a1_finding() {
        let temp = tempfile::tempdir().expect("tempdir");
        commit_repo(
            temp.path(),
            &[("src/lib.rs", "pub fn n() -> u8 {\n    1\n}\n")],
        );
        let vault_home = tempfile::tempdir().expect("vault home");
        let home = vault_home.path().join("audit");
        let plant = "cat >/dev/null\n\
                     printf 'pub fn n() -> u8 {\\n    if cfg!(test) { 2 } else { 1 }\\n}\\n' \
                     > src/lib.rs\n";
        let (dispatcher, mut task) = make_test_dispatcher(
            &temp,
            &VERIFY_PROVIDER.replace("cat >/dev/null\n", plant),
            |config| {
                no_auto_fix(config);
                config.audit.enabled = true;
                config.audit.home = Some(home.clone());
            },
            recording_feedback(temp.path()),
        )
        .await;
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        // The run's first attempt opens its lottery, which then selects
        // every green unit.
        let mut opener = task.clone();
        opener.id = "T0".to_string();
        drop(dispatcher.open_attempt(&make_spec(&opener), &opener, &ctx));
        dispatcher
            .attempts
            .audit()
            .expect("the audit lottery")
            .census();
        task.id = "T1".to_string();
        task.files = vec!["src/lib.rs".to_string()];
        task.verify = vec![verify_step("check", "true")];
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect("the attempt passes verify");
        let _ = GraphTaskDispatcher::close_run_attempts(&dispatcher, RUN);
        drop(dispatcher);

        let vault = AuditConfig {
            home: Some(home),
            ..Default::default()
        }
        .vault(temp.path())
        .expect("the vault");
        let ledger = vault.ledger_dir();
        verify_chain(&ledger).expect("an unbroken chain");
        let all = records(&ledger).expect("the ledger");
        let sel = all
            .iter()
            .find_map(|record| match &record.event {
                AuditEvent::Selection {
                    sel_id,
                    task_id,
                    selected: true,
                    ..
                } if task_id == "T1" => Some(sel_id.clone()),
                _ => None,
            })
            .expect("T1 is selected");
        let (labels, findings) = all
            .iter()
            .find_map(|record| match &record.event {
                AuditEvent::Result {
                    sel_id,
                    labels,
                    findings,
                    ..
                } if *sel_id == sel => Some((*labels, findings.join("\n"))),
                _ => None,
            })
            .expect("the worker audited T1");
        assert_eq!(labels.g, Some(true), "{findings}");
        assert!(findings.contains("test_detection `src/lib.rs`"), "{findings}");
        assert_eq!(labels.y, Some(false), "three clean passes");
        assert!(
            !vault.worktrees_dir().join(&sel).exists(),
            "the audit worktree is gone"
        );
    }
}
