//! The audit worker (S05 §4.3, §4.9; 7123): it audits a run's selected
//! units off the critical path, one at a time, and appends each audit's
//! labels to the vault ledger as `audit.result`.
//!
//! - [`AuditWorker::start`] runs a run's worker on a thread of its own, so
//!   dispatch never waits on it. Units arrive on a bounded channel
//!   ([`AuditWorker::submit`]); when it is full a unit waits in the ledger,
//!   the queue of record, where a selected unit without an `audit.result`
//!   is still to audit. The worker first takes up the units earlier runs
//!   left, and looks again when its run closes.
//! - Phase A: A1 is `check_attempt_diff` over the unit's base and result
//!   trees plus the audit-only kinds (G); the lines the attempt added are
//!   scanned for hidden-suite canaries; A2 re-runs the task's checks in an
//!   audit worktree (Y, [`super::rerun`]). Phase B runs at π_B, 1 when
//!   phase A flagged the unit and `[audit] phase_b_rate` otherwise, drawn
//!   with the run's key, and calls the B1, B2 and B3 checks ([`PhaseB`]).
//!   B3 only corroborates, except on docs, plan and research tasks.
//! - Caps: `[audit] per_audit_usd` and `per_audit_cpu_secs` per audit, and
//!   audit spend within `budget_frac` of the run's model spend. When that
//!   budget binds, `audit.budget_exhausted` is logged once and the units
//!   stay queued.
//! - [`AuditWorker::drain`], at run close, gives the worker at most
//!   `drain_secs` to finish what is queued; it then sweeps
//!   `.roko/episodes.jsonl`, the knowledge store and the playbooks for
//!   canaries. A unit it did not reach stays selected without a result, for
//!   the next run's worker.
//!
//! A queued unit's task inputs wait in [`queue_dir`] until its result is
//! written; a unit without them is audited from its selection alone. Each
//! audited Y also feeds the gate-gaming detector ([`GamingWatch`], F1),
//! whose alert goes to the ledger as `audit.policy_change`. After each
//! result, and once it has drained, the worker closes every window that is
//! due (`roko_gate::audit::feedback`, 7131): each stratum's estimates, the
//! strictness ladder's steps and the routing trust estimates. Each result
//! with a label also writes the attempt's `vs.label` row, which teaches the
//! run's self-model (DP5, [`super::labels`]), and a confirmed false green or
//! gaming finding, or a gate-gaming alert, opens an incident in the vault
//! (DP6, [`report_incidents`]).

use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use roko_core::audit_home::AuditVault;
use roko_core::audit_types::{AuditLabels, VerifyDepth};
use roko_core::config::GatesConfig;
use roko_core::config::audit::AuditConfig;
use roko_gate::attempt_diff::{
    AttemptChange, AttemptDiffPolicy, ChangeKind, audit_only_findings, check_attempt_diff,
    scripts_run_by,
};
use roko_gate::audit::canary::{CanaryScanner, scan_diff};
use roko_gate::audit::feedback::{TrustBook, close_due_windows, trust_path};
use roko_gate::audit::hidden::HiddenStore;
use roko_gate::audit::incident::{Evidence, Incident, IncidentDraft, IncidentKind, IncidentStore};
use roko_gate::audit::ledger::{AuditEvent, AuditLedger, LedgerRecord, records};
use roko_gate::audit::policy::{EPS_FLOOR, RunKey, select};
use roko_learn::cascade_router::AUDIT_HARNESS;
use roko_learn::gate_gaming::{GamingAlert, GateGamingDetector};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use tokio_util::sync::CancellationToken;

use super::git;
use super::labels::{AuditReport, VsLearner, record_label, vs_label};
use super::rerun::{A2Outcome, ServiceContext, checks_for, prepare_build, rerun, target_dir};
use super::worktree::AuditWorktree;

/// Units a worker's channel holds; the next waits in the ledger.
pub const CHANNEL_CAPACITY: usize = 32;

/// The largest file A1 reads, in bytes.
const MAX_TEXT_BYTES: usize = 512 * 1024;

/// How long [`AuditWorker::drain`] waits for a cancelled audit to stop.
const CANCEL_GRACE: Duration = Duration::from_secs(5);

/// Task kinds where B3 is the only check (S05 §4.3).
const REVIEW_ONLY_KINDS: [&str; 3] = ["docs", "plan", "research"];

/// What an audit needs of a unit's task, noted when its attempt opens.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AuditTask {
    /// The task's title.
    pub title: String,
    /// Its description.
    pub description: String,
    /// Its goal (TSS v1).
    pub goal: String,
    /// Its acceptance criteria.
    pub acceptance: Vec<String>,
    /// `[task.hidden] suite`: `auto`, `none` or a suite id.
    pub hidden_suite: Option<String>,
    /// `[task.hidden] interface`: the public surface a hidden suite calls.
    pub interface: Vec<String>,
    /// `[task.hidden] properties`: what a hidden suite checks.
    pub properties: Vec<String>,
    /// The paths its `files` name.
    pub files: Vec<String>,
    /// Its authored verify steps, `(phase, command)`.
    pub verify: Vec<(String, String)>,
    /// `code`, `docs`, `plan`, `research`, …: its domain, else its role.
    pub kind: String,
}

/// One selected unit, as the worker audits it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditUnit {
    /// The selection.
    pub sel_id: String,
    /// S01's attempt key.
    pub attempt_key: String,
    /// The run it was drawn in.
    pub run_id: String,
    /// The plan.
    pub plan_id: String,
    /// The task.
    pub task_id: String,
    /// π_i.
    pub pi: f64,
    /// The tree the attempt started from.
    pub base_tree: Option<String>,
    /// The tree it left.
    pub result_tree: Option<String>,
    /// The implementing model.
    pub model: String,
    /// The task, when its attempt was noted.
    pub task: AuditTask,
}

impl AuditUnit {
    /// The unit a selected `audit.selection` names, without its task.
    fn from_selection(record: &LedgerRecord) -> Option<Self> {
        let AuditEvent::Selection {
            sel_id,
            attempt_key,
            run_id,
            task_id,
            stratum,
            pi,
            selected: true,
            base_tree,
            result_tree,
            ..
        } = &record.event
        else {
            return None;
        };
        Some(Self {
            sel_id: sel_id.clone(),
            attempt_key: attempt_key.clone(),
            run_id: run_id.clone(),
            plan_id: plan_of(attempt_key).to_string(),
            task_id: task_id.clone(),
            pi: *pi,
            base_tree: base_tree.clone(),
            result_tree: result_tree.clone(),
            model: stratum.model.clone(),
            task: AuditTask::default(),
        })
    }
}

/// What one phase-B check found.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CheckOutcome {
    /// Its labels: B1 gives Y, B2 W, and B3 Y and G, which count only as
    /// S05 §4.3 allows.
    pub labels: AuditLabels,
    /// What its model calls cost, in USD.
    pub cost_usd: f64,
    /// Its detail, for the result's `checks`.
    pub detail: Value,
}

/// What a phase-B check sees of the unit under audit.
#[derive(Debug)]
pub struct UnitAudit<'a> {
    /// The unit.
    pub unit: &'a AuditUnit,
    /// The workspace repository.
    pub repo: &'a Path,
    /// The audit worktree of the unit's result tree, its visible tests
    /// restored.
    pub worktree: &'a Path,
    /// The workspace's vault.
    pub vault: &'a AuditVault,
    /// Phase A's findings.
    pub findings: &'a [String],
    /// Phase A's labels.
    pub phase_a: AuditLabels,
    /// What the check may spend, in USD.
    pub usd_left: f64,
    /// How long it may run.
    pub time_left: Duration,
}

/// A phase-B check: B1 (7124), B2 (7125) or B3 (7127).
#[async_trait::async_trait]
pub trait PhaseBCheck: Send + Sync {
    /// Check the unit.
    async fn check(&self, audit: &UnitAudit<'_>) -> CheckOutcome;
}

/// The phase-B checks a worker runs; a missing one leaves its label null.
#[derive(Clone, Default)]
pub struct PhaseB {
    /// B1, the hidden suite.
    pub b1: Option<Arc<dyn PhaseBCheck>>,
    /// B2, extreme mutation.
    pub b2: Option<Arc<dyn PhaseBCheck>>,
    /// B3, the cross-family review.
    pub b3: Option<Arc<dyn PhaseBCheck>>,
}

/// What a run's worker works with.
pub struct WorkerContext {
    /// The workspace: a git repository holding the units' trees.
    pub workdir: PathBuf,
    /// Its vault.
    pub vault: AuditVault,
    /// `[audit]`.
    pub config: AuditConfig,
    /// `[gates]`, for the production gate executor.
    pub gates: GatesConfig,
    /// The workspace's audit secret, from which each run's key derives.
    pub secret: Vec<u8>,
    /// The run whose units the worker is given.
    pub run_id: String,
    /// The run's model spend so far, in USD.
    pub run_spend: Arc<parking_lot::Mutex<f64>>,
    /// The phase-B checks.
    pub phase_b: PhaseB,
    /// The workspace's gate-gaming detector.
    pub gaming: GamingWatch,
    /// The run's self-model, which audited VS labels teach (DP5).
    pub learner: Option<Arc<dyn VsLearner>>,
}

/// F1 (S05 §4.5, 7128): the gate-gaming detector over a workspace's
/// attempts and audits.
///
/// Each settled attempt adds its gate verdict at weight 1, and each audited
/// label adds quality 1 − Y at weight 1/π_i, so a pass rate that climbs
/// while audited quality falls raises an alert.
#[derive(Debug, Clone)]
pub struct GamingWatch {
    detector: Arc<parking_lot::Mutex<GateGamingDetector>>,
}

impl GamingWatch {
    /// A watch whose detector keeps its alerts in `vault`'s incidents.
    #[must_use]
    pub fn new(vault: &AuditVault) -> Self {
        let alerts = vault.incidents_dir().join("gate-gaming-alerts.jsonl");
        Self {
            detector: Arc::new(parking_lot::Mutex::new(GateGamingDetector::new(alerts))),
        }
    }

    /// Count a settled attempt's gate verdict for `model`.
    pub fn gate(&self, model: &str, passed: bool) {
        let mut detector = self.detector.lock();
        detector.observe_weighted(model, Some(passed), None, 1.0);
    }

    /// Count an audited unit's label `y` for `model`, drawn at `pi`; returns
    /// the alert the window then raises.
    pub fn audited(&self, model: &str, y: bool, pi: f64) -> Option<GamingAlert> {
        let quality = if y { 0.0 } else { 1.0 };
        let mut detector = self.detector.lock();
        detector.observe_weighted(model, None, Some(quality), 1.0 / pi.max(EPS_FLOOR));
        detector.detect(model)
    }
}

/// A run's audit worker, on a thread of its own.
pub struct AuditWorker {
    sender: Option<tokio::sync::mpsc::Sender<AuditUnit>>,
    finished: std::sync::mpsc::Receiver<()>,
    cancel: CancellationToken,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl AuditWorker {
    /// Start `context.run_id`'s worker.
    ///
    /// # Errors
    ///
    /// The vault's ledger cannot be opened, or the thread cannot start.
    pub fn start(context: WorkerContext) -> std::io::Result<Self> {
        let ledger = AuditLedger::open(&context.vault)?
            .with_mirror(context.workdir.join(".roko/audit/audits.jsonl"));
        let (sender, units) = tokio::sync::mpsc::channel(CHANNEL_CAPACITY);
        let (done, finished) = std::sync::mpsc::channel();
        let cancel = CancellationToken::new();
        let token = cancel.clone();
        let thread = std::thread::Builder::new()
            .name("roko-audit".to_string())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                let worker = Worker::new(context, ledger);
                match runtime {
                    Ok(runtime) => runtime.block_on(worker.run(units, token)),
                    Err(error) => tracing::warn!(%error, "the audit worker did not start"),
                }
                let _ = done.send(());
            })?;
        Ok(Self {
            sender: Some(sender),
            finished,
            cancel,
            thread: Some(thread),
        })
    }

    /// Queue `unit` without waiting: `false` when the channel is full or
    /// closed, and the unit waits in the ledger.
    pub fn submit(&self, unit: AuditUnit) -> bool {
        self.sender
            .as_ref()
            .is_some_and(|sender| sender.try_send(unit).is_ok())
    }

    /// Close the channel and give the worker at most `limit` to audit what
    /// is queued and sweep, then cancel what still runs. Returns whether it
    /// finished in time.
    pub fn drain(mut self, limit: Duration) -> bool {
        self.sender = None;
        let finished = self.finished.recv_timeout(limit).is_ok();
        if finished {
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        } else {
            self.cancel.cancel();
            let _ = self.finished.recv_timeout(CANCEL_GRACE);
        }
        finished
    }
}

impl Drop for AuditWorker {
    /// A worker dropped without a drain stops at once; its units stay
    /// queued.
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

/// Where a queued unit's task inputs wait until its result is written:
/// `<vault>/<workspace_id>/worktrees/queue/`.
#[must_use]
pub fn queue_dir(vault: &AuditVault) -> PathBuf {
    vault.worktrees_dir().join("queue")
}

/// Keep `unit`'s task inputs in the vault until its result is written.
///
/// # Errors
///
/// The queue cannot be written.
pub fn queue_unit(vault: &AuditVault, unit: &AuditUnit) -> std::io::Result<()> {
    let queue = queue_dir(vault);
    std::fs::create_dir_all(&queue)?;
    let text = serde_json::to_string(unit)?;
    std::fs::write(queue.join(format!("{}.json", unit.sel_id)), text)
}

/// One audit's outcome, before it is appended.
#[derive(Debug, Default)]
struct Audit {
    labels: AuditLabels,
    findings: Vec<String>,
    checks: Map<String, Value>,
    cost_usd: f64,
    secs: f64,
    /// π_B, when the unit had trees to draw phase B on.
    pi_b: Option<f64>,
}

/// The worker's state, on its thread.
struct Worker {
    context: WorkerContext,
    ledger: AuditLedger,
    hidden: Option<HiddenStore>,
    /// Selections this worker took up.
    seen: HashSet<String>,
    /// What this worker's audits spent, in USD.
    spent_usd: f64,
    /// Whether `audit.budget_exhausted` was logged.
    exhausted: bool,
    /// Models a gate-gaming alert was logged for.
    alerted: HashSet<String>,
}

impl Worker {
    fn new(context: WorkerContext, ledger: AuditLedger) -> Self {
        let hidden = HiddenStore::open(&context.vault).ok();
        Self {
            context,
            ledger,
            hidden,
            seen: HashSet::new(),
            spent_usd: 0.0,
            exhausted: false,
            alerted: HashSet::new(),
        }
    }

    /// Audit what earlier runs left, then each unit the run submits, then,
    /// once the channel closes, what waited in the ledger; then sweep.
    async fn run(
        mut self,
        mut units: tokio::sync::mpsc::Receiver<AuditUnit>,
        cancel: CancellationToken,
    ) {
        let mut queue: VecDeque<AuditUnit> = self.pending().into();
        loop {
            let unit = match queue.pop_front() {
                Some(unit) => unit,
                None => tokio::select! {
                    () = cancel.cancelled() => return,
                    unit = units.recv() => match unit {
                        Some(unit) => unit,
                        None => break,
                    },
                },
            };
            if !self.take(unit, &cancel).await {
                return;
            }
        }
        for unit in self.pending() {
            if !self.take(unit, &cancel).await {
                return;
            }
        }
        self.sweep();
        self.close_windows();
    }

    /// Audit `unit` once, unless the budget binds or another worker holds
    /// it; `false` when the worker was cancelled.
    async fn take(&mut self, unit: AuditUnit, cancel: &CancellationToken) -> bool {
        if !self.seen.insert(unit.sel_id.clone()) {
            return true;
        }
        let Some(usd_left) = self.budget_left() else {
            self.log_exhausted();
            return true;
        };
        let queue = queue_dir(&self.context.vault);
        let Some(_lock) = lock(&queue, &unit.sel_id) else {
            return true;
        };
        if !self.has_result(&unit.sel_id) {
            let audit = tokio::select! {
                () = cancel.cancelled() => return false,
                audit = self.audit(&unit, usd_left) => audit,
            };
            let y = audit.labels.y;
            self.record(&unit, audit);
            self.watch_gaming(&unit, y);
            self.close_windows();
        }
        for extension in ["json", "lock"] {
            let _ = std::fs::remove_file(queue.join(format!("{}.{extension}", unit.sel_id)));
        }
        true
    }

    /// What the next audit may spend: `budget_frac` of the run's model
    /// spend less what audits spent, at most `per_audit_usd`; `None` once
    /// audits spent the budget.
    fn budget_left(&self) -> Option<f64> {
        let config = &self.context.config;
        let limit = config.budget_frac * *self.context.run_spend.lock();
        let left = limit - self.spent_usd;
        (self.spent_usd <= 0.0 || left > 0.0).then(|| left.max(0.0).min(config.per_audit_usd))
    }

    /// Log `audit.budget_exhausted`, once.
    fn log_exhausted(&mut self) {
        if std::mem::replace(&mut self.exhausted, true) {
            return;
        }
        let event = AuditEvent::BudgetExhausted {
            run_id: self.context.run_id.clone(),
            spent_usd: self.spent_usd,
            limit_usd: self.context.config.budget_frac * *self.context.run_spend.lock(),
        };
        if let Err(error) = self.ledger.append(event) {
            tracing::warn!(%error, "audit.budget_exhausted not written");
        }
    }

    /// The selected units the ledger holds without a result, oldest first,
    /// with their task inputs when they were queued.
    fn pending(&self) -> Vec<AuditUnit> {
        let all = match records(self.ledger.dir()) {
            Ok(all) => all,
            Err(error) => {
                tracing::warn!(%error, "the audit ledger is unreadable; no queued unit is taken");
                return Vec::new();
            }
        };
        let audited: HashSet<&str> = all
            .iter()
            .filter_map(|record| match &record.event {
                AuditEvent::Result { sel_id, .. } => Some(sel_id.as_str()),
                _ => None,
            })
            .collect();
        let queue = queue_dir(&self.context.vault);
        all.iter()
            .filter_map(AuditUnit::from_selection)
            .filter(|unit| {
                !audited.contains(unit.sel_id.as_str()) && !self.seen.contains(&unit.sel_id)
            })
            .map(|unit| load_unit(&queue, &unit.sel_id).unwrap_or(unit))
            .collect()
    }

    /// Whether the ledger holds a result for `sel_id`.
    fn has_result(&self, sel_id: &str) -> bool {
        records(self.ledger.dir()).is_ok_and(|all| {
            all.iter().any(|record| {
                matches!(&record.event, AuditEvent::Result { sel_id: id, .. } if id == sel_id)
            })
        })
    }

    /// Phase A, then phase B when its draw takes the unit.
    async fn audit(&mut self, unit: &AuditUnit, usd_left: f64) -> Audit {
        let started = Instant::now();
        let cap = Duration::from_secs(self.context.config.per_audit_cpu_secs);
        let mut checks = Map::new();
        let trees = unit.base_tree.as_deref().zip(unit.result_tree.as_deref());
        let Some((base, result)) = trees else {
            checks.insert("a1".into(), json!({ "error": "no trees" }));
            return Audit {
                checks,
                ..Audit::default()
            };
        };
        let repo = self.context.workdir.clone();
        // Phase A: A1, the canary scan of the added lines, and A2.
        let checked = a1(&repo, &unit.task, &self.context.gates, base, result);
        let (findings, g, changed) = match checked {
            Ok(found) => (found.findings, Some(found.gaming), found.changed),
            Err(error) => {
                checks.insert("a1".into(), json!({ "error": error.to_string() }));
                (Vec::new(), None, Vec::new())
            }
        };
        let canaries = self.scan_added(base, result);
        let path = self.context.vault.worktrees_dir().join(&unit.sel_id);
        let opened = open_worktree(&repo, &path, base, result);
        let (y, flaky) = match &opened {
            Ok((worktree, restored)) => {
                let budget = cap.saturating_sub(started.elapsed());
                let (a2, detail) = self.a2(unit, worktree.path(), &changed, budget).await;
                checks.insert("a2".into(), detail);
                checks.insert("restored".into(), json!(restored));
                (a2.y, a2.flaky)
            }
            Err(error) => {
                checks.insert("a2".into(), json!({ "error": error.to_string() }));
                (None, false)
            }
        };
        let phase_a = AuditLabels { y, g, w: None };
        // Phase B, at π_B.
        let flagged = y == Some(true) || g == Some(true) || flaky || canaries > 0;
        let pi_b = if flagged {
            1.0
        } else {
            self.context.config.phase_b_rate
        };
        let drawn = phase_b_drawn(&self.context.secret, unit, result, pi_b);
        checks.insert("phase_b".into(), json!({ "pi_b": pi_b, "drawn": drawn }));
        let mut labels = [AuditLabels::default(); 3];
        let mut cost_usd = 0.0;
        if drawn && let Ok((worktree, _)) = &opened {
            let view = UnitAudit {
                unit,
                repo: &repo,
                worktree: worktree.path(),
                vault: &self.context.vault,
                findings: &findings,
                phase_a,
                usd_left,
                time_left: cap,
            };
            (labels, cost_usd) = self.phase_b(view, started, cap, &mut checks).await;
        }
        let [b1, b2, b3] = labels;
        Audit {
            labels: combine(phase_a, b1, b2, b3, &unit.task.kind),
            findings,
            checks,
            cost_usd,
            secs: started.elapsed().as_secs_f64(),
            pi_b: Some(pi_b),
        }
    }

    /// Phase B's checks, B1, B2 and B3, within what is left of the audit's
    /// caps: their labels and their cost. Each one's detail goes to
    /// `checks`.
    async fn phase_b(
        &self,
        mut view: UnitAudit<'_>,
        started: Instant,
        cap: Duration,
        checks: &mut Map<String, Value>,
    ) -> ([AuditLabels; 3], f64) {
        let phase_b = self.context.phase_b.clone();
        let hooks = [("b1", phase_b.b1), ("b2", phase_b.b2), ("b3", phase_b.b3)];
        let usd = view.usd_left;
        let mut labels = [AuditLabels::default(); 3];
        let mut cost_usd = 0.0;
        for (slot, (name, check)) in hooks.into_iter().enumerate() {
            let Some(check) = check else {
                checks.insert(name.into(), Value::Null);
                continue;
            };
            view.usd_left = (usd - cost_usd).max(0.0);
            view.time_left = cap.saturating_sub(started.elapsed());
            let outcome = check.check(&view).await;
            cost_usd += outcome.cost_usd;
            labels[slot] = outcome.labels;
            checks.insert(name.into(), outcome.detail);
        }
        (labels, cost_usd)
    }

    /// A2 in `worktree`, building in the workspace's audit target; returns
    /// its outcome and detail.
    async fn a2(
        &self,
        unit: &AuditUnit,
        worktree: &Path,
        changed: &[String],
        budget: Duration,
    ) -> (A2Outcome, Value) {
        let target = target_dir(&self.context.vault);
        if let Err(error) = prepare_build(worktree, &target, changed) {
            tracing::warn!(%error, "the audit target directory was not linked");
        }
        let checks = checks_for(&unit.task.verify, changed, budget);
        let service = ServiceContext {
            gates: self.context.gates.clone(),
            run_id: unit.run_id.clone(),
            plan_id: unit.plan_id.clone(),
            task_id: unit.task_id.clone(),
            changed_files: changed.to_vec(),
        };
        let outcome = rerun(worktree, &checks, budget, Some(&target), Some(&service)).await;
        let detail = json!({
            "y": outcome.y,
            "flaky": outcome.flaky,
            "passes": outcome.passes,
            "checks": checks.iter().map(|check| check.label.as_str()).collect::<Vec<_>>(),
            "secs": outcome.secs,
            "stopped": outcome.stopped,
        });
        (outcome, detail)
    }

    /// Report every hidden-suite canary on a line the attempt added, out of
    /// roko's own state (SC4); returns how many were new.
    fn scan_added(&mut self, base: &str, result: &str) -> usize {
        let Some(store) = self.hidden.as_ref() else {
            return 0;
        };
        let args = ["diff", "-U0", base, result, "--", ":(exclude).roko"];
        let hits = match git(&self.context.workdir, &args) {
            Ok(diff) => scan_diff(&diff),
            Err(_) => return 0,
        };
        if hits.is_empty() {
            return 0;
        }
        match CanaryScanner::new(store).report(&mut self.ledger, "diff", &hits) {
            Ok(reports) => reports.len(),
            Err(error) => {
                tracing::warn!(%error, "a canary in an audited diff was not logged");
                0
            }
        }
    }

    /// Append the audit's `audit.result` and, when it has a label, the
    /// attempt's `vs.label` row (DP5), and count its spend.
    fn record(&mut self, unit: &AuditUnit, audit: Audit) {
        self.spent_usd += audit.cost_usd;
        let pi_eff = audit.pi_b.map(|pi_b| unit.pi * pi_b);
        let labels = audit.labels;
        let labelled = labels.y.is_some() || labels.g.is_some() || labels.w.is_some();
        let report = AuditReport {
            labels,
            checks: &audit.checks,
            findings: &audit.findings,
            pi_eff,
            cost_usd: audit.cost_usd,
        };
        let row = vs_label(unit, &report);
        let evidence = evidence_of(unit, &audit.checks, &audit.findings);
        let id = unit.sel_id.strip_prefix("sel-").unwrap_or(&unit.sel_id);
        let event = AuditEvent::Result {
            sel_id: unit.sel_id.clone(),
            res_id: format!("res-{id}"),
            attempt_key: unit.attempt_key.clone(),
            labels,
            findings: audit.findings,
            checks: Value::Object(audit.checks),
            cost_usd: Some(audit.cost_usd),
            cpu_secs: Some(audit.secs),
            pi_eff,
        };
        if let Err(error) = self.ledger.append(event) {
            tracing::warn!(sel_id = %unit.sel_id, %error, "audit.result not written");
            return;
        }
        if !labelled {
            return;
        }
        let learner = self.context.learner.as_deref();
        let workdir = &self.context.workdir;
        if let Err(error) = record_label(&mut self.ledger, workdir, &row, learner) {
            tracing::warn!(sel_id = %unit.sel_id, %error, "vs.label not written");
        }
        // DP6: a confirmed false green or gaming finding opens an incident.
        report_incidents(
            &self.context.vault,
            &mut self.ledger,
            unit,
            labels,
            &evidence,
        );
    }

    /// F1 (7128): count the unit's audited Y in the gate-gaming detector,
    /// and log the first alert it raises for the model as
    /// `audit.policy_change`.
    fn watch_gaming(&mut self, unit: &AuditUnit, y: Option<bool>) {
        let Some(y) = y else {
            return;
        };
        let Some(alert) = self.context.gaming.audited(&unit.model, y, unit.pi) else {
            return;
        };
        if !self.alerted.insert(alert.model_slug.clone()) {
            return;
        }
        tracing::warn!(model = %alert.model_slug, "{}", alert.summary());
        let event = AuditEvent::PolicyChange {
            knob: "gaming_alert".to_string(),
            from: Value::Null,
            to: serde_json::to_value(&alert).unwrap_or(Value::Null),
            reason: alert.summary(),
        };
        if let Err(error) = self.ledger.append(event) {
            tracing::warn!(%error, "a gate-gaming alert was not logged");
        }
        // DP6: the alert opens an incident about the model.
        let draft = IncidentDraft {
            kind: IncidentKind::SpecGaming,
            attempt_key: "-".to_string(),
            run_id: unit.run_id.clone(),
            task_id: unit.task_id.clone(),
            model: alert.model_slug.clone(),
            harness: AUDIT_HARNESS.to_string(),
            files: Vec::new(),
            evidence: Evidence {
                sel_id: Some(unit.sel_id.clone()),
                note: Some(alert.summary()),
                ..Evidence::default()
            },
        };
        report_incident(&self.context.vault, &mut self.ledger, draft);
    }

    /// Close every window that is due (7131). An active M1 floor applies
    /// at dispatch, as the higher of it and the ladder's level (7132), so
    /// the ladder's own floor here is V0.
    fn close_windows(&mut self) {
        let closed = close_due_windows(
            &mut self.ledger,
            &self.context.vault,
            &self.context.config,
            VerifyDepth::V0,
            chrono::Utc::now(),
        );
        if let Err(error) = closed {
            tracing::warn!(%error, "an audit window was not closed");
        }
    }

    /// Sweep the workspace's episode log, knowledge store and playbooks
    /// for hidden-suite canaries (SC4).
    fn sweep(&mut self) {
        let Some(store) = self.hidden.as_ref() else {
            return;
        };
        let layout = roko_fs::RokoLayout::for_project(&self.context.workdir);
        let mut paths = vec![
            layout.episodes_path(),
            layout.neuro_dir().join("knowledge.jsonl"),
        ];
        if let Ok(entries) = std::fs::read_dir(layout.playbooks_dir()) {
            paths.extend(entries.filter_map(Result::ok).map(|entry| entry.path()));
        }
        paths.retain(|path| path.is_file());
        if let Err(error) = CanaryScanner::new(store).sweep(&mut self.ledger, &paths) {
            tracing::warn!(%error, "the run-close canary sweep failed");
        }
    }
}

/// What an incident about `unit`'s audit holds as evidence: its ids and
/// trees, A1's findings, B1's suite and how many of its tests failed, and
/// B3's verdict; never a hidden test's body or name.
pub(crate) fn evidence_of(
    unit: &AuditUnit,
    checks: &Map<String, Value>,
    findings: &[String],
) -> Evidence {
    let id = unit.sel_id.strip_prefix("sel-").unwrap_or(&unit.sel_id);
    let b1 = checks.get("b1");
    Evidence {
        sel_id: Some(unit.sel_id.clone()),
        res_id: Some(format!("res-{id}")),
        base_tree: unit.base_tree.clone(),
        result_tree: unit.result_tree.clone(),
        findings: findings.to_vec(),
        hidden_suite: b1
            .and_then(|b1| b1["suite_id"].as_str())
            .map(str::to_string),
        hidden_failed: b1.and_then(|b1| b1["y"].as_bool()).map(u32::from),
        b3: checks.get("b3").filter(|b3| !b3.is_null()).cloned(),
        note: None,
        immune_link: None,
    }
}

/// DP6 (S05 §4.6, §4.7; 7135): an incident for each finding `labels`
/// confirms, a false green or gaming, with its fix proposal. Each new one
/// downgrades the pair's routing trust, and repeated gaming proposes
/// isolating the pair. Returns the incidents it opened: one already open is
/// not opened again.
pub(crate) fn report_incidents(
    vault: &AuditVault,
    ledger: &mut AuditLedger,
    unit: &AuditUnit,
    labels: AuditLabels,
    evidence: &Evidence,
) -> Vec<Incident> {
    let found = [
        (labels.y, IncidentKind::FalseGreen),
        (labels.g, IncidentKind::SpecGaming),
    ];
    let mut opened = Vec::new();
    for (label, kind) in found {
        if label != Some(true) {
            continue;
        }
        let draft = IncidentDraft {
            kind,
            attempt_key: unit.attempt_key.clone(),
            run_id: unit.run_id.clone(),
            task_id: unit.task_id.clone(),
            model: unit.model.clone(),
            harness: AUDIT_HARNESS.to_string(),
            files: unit.task.files.clone(),
            evidence: evidence.clone(),
        };
        opened.extend(report_incident(vault, ledger, draft));
    }
    opened
}

/// Open `draft`'s incident; when it is new, downgrade its pair's routing
/// trust and, for gaming, propose isolating the pair once it has gamed
/// repeatedly. Returns the incident when it is new.
fn report_incident(
    vault: &AuditVault,
    ledger: &mut AuditLedger,
    draft: IncidentDraft,
) -> Option<Incident> {
    let (model, harness) = (draft.model.clone(), draft.harness.clone());
    let store = match IncidentStore::open(vault) {
        Ok(store) => store,
        Err(error) => {
            tracing::warn!(%error, "the audit incident store cannot be opened");
            return None;
        }
    };
    let (incident, new) = match store.open_incident(ledger, draft) {
        Ok(opened) => opened,
        Err(error) => {
            tracing::warn!(%error, "an audit incident was not opened");
            return None;
        }
    };
    if !new {
        return None;
    }
    tracing::warn!(
        incident = %incident.incident_id,
        kind = incident.kind.label(),
        model = %model,
        "audit incident opened"
    );
    downgrade_trust(vault, ledger, &incident);
    if incident.kind == IncidentKind::SpecGaming {
        match store.propose_isolation(&model, &harness) {
            Ok(Some(path)) => tracing::warn!(
                proposal = %path.display(),
                "repeated gaming: an isolation proposal waits for an operator"
            ),
            Ok(None) => {}
            Err(error) => tracing::warn!(%error, "the isolation proposal was not written"),
        }
    }
    Some(incident)
}

/// DP6's trust downgrade (S05 §4.6): `incident`'s pair loses routing trust
/// at once, logged as `audit.policy_change`. DP4 reads it at the next plan
/// start, and the next window's estimate replaces it.
fn downgrade_trust(vault: &AuditVault, ledger: &mut AuditLedger, incident: &Incident) {
    let path = trust_path(vault);
    let mut book = match TrustBook::load(&path) {
        Ok(book) => book,
        Err(error) => {
            tracing::warn!(%error, "routing trust was not downgraded: its file is unreadable");
            return;
        }
    };
    let (before, after) = book.downgrade(&incident.model, &incident.harness);
    if let Err(error) = book.save(&path) {
        tracing::warn!(%error, "routing trust was not downgraded");
        return;
    }
    let event = AuditEvent::PolicyChange {
        knob: format!("trust:{}/{}", incident.model, incident.harness),
        from: json!(before),
        to: json!(after),
        reason: format!(
            "trust_downgrade: incident {} ({})",
            incident.incident_id,
            incident.kind.label()
        ),
    };
    if let Err(error) = ledger.append(event) {
        tracing::warn!(%error, "the trust downgrade was not logged");
    }
}

/// What A1 found.
struct A1 {
    /// Every finding, one line each.
    findings: Vec<String>,
    /// Whether one is gaming: tampering or an audit-only kind.
    gaming: bool,
    /// The paths the attempt changed.
    changed: Vec<String>,
}

/// A1 over `base..result` in `repo`: the inline screen's diff checks, by
/// the task's `files` and verify scripts and the rung files of `gates`, and
/// the audit-only kinds. A scope finding is reported, but it is not gaming.
fn a1(
    repo: &Path,
    task: &AuditTask,
    gates: &GatesConfig,
    base: &str,
    result: &str,
) -> anyhow::Result<A1> {
    let changes = tree_changes(repo, base, result)?;
    let policy = AttemptDiffPolicy {
        task_files: task.files.clone(),
        verify_scripts: task
            .verify
            .iter()
            .flat_map(|(_, command)| scripts_run_by(command))
            .collect(),
        rung_files: gates.rung_files(),
        ..AttemptDiffPolicy::default()
    };
    let mut findings = check_attempt_diff(&changes, &policy);
    findings.extend(audit_only_findings(&changes));
    Ok(A1 {
        gaming: findings
            .iter()
            .any(|finding| finding.kind.is_tamper() || finding.kind.is_audit_only()),
        findings: findings.iter().map(ToString::to_string).collect(),
        changed: changes.into_iter().map(|change| change.path).collect(),
    })
}

/// What changed from `base` to `result` in `repo`, renames found, with
/// each side's text when it is small and not binary. Roko's own state is
/// left out.
fn tree_changes(repo: &Path, base: &str, result: &str) -> anyhow::Result<Vec<AttemptChange>> {
    let listing = git(repo, &["diff", "--name-status", "-z", "-M", base, result])?;
    let mut fields = listing.split('\0').filter(|field| !field.is_empty());
    let mut changes = Vec::new();
    while let (Some(status), Some(path)) = (fields.next(), fields.next()) {
        let kind = match status.chars().next() {
            Some('A') => ChangeKind::Added,
            Some('D') => ChangeKind::Deleted,
            Some('R' | 'C') => ChangeKind::Renamed,
            _ => ChangeKind::Modified,
        };
        let mut change = AttemptChange::new(kind, path);
        if kind == ChangeKind::Renamed
            && let Some(to) = fields.next()
        {
            change.old_path = Some(path.to_string());
            change.path = to.to_string();
        }
        if is_roko_state(&change.path) {
            continue;
        }
        if kind != ChangeKind::Added {
            change.before = blob_text(repo, base, change.old_path());
        }
        if kind != ChangeKind::Deleted {
            change.after = blob_text(repo, result, &change.path);
        }
        changes.push(change);
    }
    Ok(changes)
}

/// The text of `path` in `tree`, or `None` when it is binary, larger than
/// [`MAX_TEXT_BYTES`] or unreadable.
fn blob_text(repo: &Path, tree: &str, path: &str) -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["cat-file", "blob", &format!("{tree}:{path}")])
        .output()
        .ok()?;
    let text = output.stdout;
    (output.status.success() && text.len() <= MAX_TEXT_BYTES && !text.contains(&0))
        .then(|| String::from_utf8_lossy(&text).into_owned())
}

/// Whether `path` is roko's own state, which the harness writes during a
/// run.
fn is_roko_state(path: &str) -> bool {
    path == ".roko" || path.starts_with(".roko/")
}

/// The audit worktree of `result` at `path`, any stale one there removed
/// first, with the visible tests the attempt changed restored from `base`.
pub(crate) fn open_worktree(
    repo: &Path,
    path: &Path,
    base: &str,
    result: &str,
) -> anyhow::Result<(AuditWorktree, Vec<String>)> {
    if path.exists() {
        std::fs::remove_dir_all(path)?;
        git(repo, &["worktree", "prune"])?;
    }
    let worktree = AuditWorktree::create(repo, path, result, "roko audit")?;
    let restored = worktree.restore_tests(base, result)?;
    Ok((worktree, restored))
}

/// Whether phase B audits `unit` at `pi_b`: a keyed draw with the unit's
/// run key, which its reveal reproduces.
fn phase_b_drawn(secret: &[u8], unit: &AuditUnit, result: &str, pi_b: f64) -> bool {
    if pi_b <= 0.0 {
        return false;
    }
    let attempt = format!("{}#phase_b", unit.attempt_key);
    let pi_b = pi_b.clamp(EPS_FLOOR, 1.0);
    RunKey::derive(secret, &unit.run_id)
        .and_then(|key| select(&key, &unit.run_id, &unit.task_id, &attempt, result, pi_b))
        .is_ok_and(|selection| selection.selected)
}

/// The audit's labels (S05 §4.3): Y from A2 and B1, G from A1, W from B2.
/// B3's Y and G count only where [`b3_counts`]; elsewhere they set nothing
/// a mechanical check did not.
pub(super) fn combine(
    phase_a: AuditLabels,
    b1: AuditLabels,
    b2: AuditLabels,
    b3: AuditLabels,
    kind: &str,
) -> AuditLabels {
    let (b3_y, b3_g) = if b3_counts(kind) {
        (b3.y, b3.g)
    } else {
        (None, None)
    };
    AuditLabels {
        y: either(&[phase_a.y, b1.y, b3_y]),
        g: either(&[phase_a.g, b3_g]),
        w: b2.w,
    }
}

/// Whether B3's labels count on a task of `kind`: only on a docs, plan or
/// research task, where it is the only check (S05 §4.3).
pub(crate) fn b3_counts(kind: &str) -> bool {
    REVIEW_ONLY_KINDS.contains(&kind)
}

/// `Some(true)` when a label is 1, else `Some(false)` when one is 0, else
/// `None`.
fn either(labels: &[Option<bool>]) -> Option<bool> {
    if labels.contains(&Some(true)) {
        Some(true)
    } else if labels.contains(&Some(false)) {
        Some(false)
    } else {
        None
    }
}

/// The plan of an attempt key, `{run}:{plan}:{task}:{attempt}`.
fn plan_of(attempt_key: &str) -> &str {
    attempt_key.rsplit(':').nth(2).unwrap_or("-")
}

/// An exclusive lock on `sel_id` among the workspace's workers, or `None`
/// when another one holds it.
fn lock(queue: &Path, sel_id: &str) -> Option<std::fs::File> {
    std::fs::create_dir_all(queue).ok()?;
    let file = std::fs::File::create(queue.join(format!("{sel_id}.lock"))).ok()?;
    file.try_lock().ok()?;
    Some(file)
}

/// A queued unit's task inputs.
fn load_unit(queue: &Path, sel_id: &str) -> Option<AuditUnit> {
    let text = std::fs::read_to_string(queue.join(format!("{sel_id}.json"))).ok()?;
    serde_json::from_str(&text).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sixty rounds of one model's attempts: the gate passes every other
    /// attempt, and from round 30 every one; every third round one green
    /// unit is audited at π = 0.5, and from round 30 the audits find false
    /// greens when `gaming`. Returns the alerts raised.
    fn rounds(watch: &GamingWatch, gaming: bool) -> Vec<GamingAlert> {
        let mut alerts = Vec::new();
        for round in 0_u32..60 {
            let late = round >= 30;
            watch.gate("model-a", late || round.is_multiple_of(2));
            if round.is_multiple_of(3) {
                alerts.extend(watch.audited("model-a", gaming && late, 0.5));
            }
        }
        alerts
    }

    #[test]
    fn audited_labels_raise_a_gaming_alert_on_a_planted_stream() {
        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("repo");
        std::fs::create_dir_all(&workspace).expect("mkdir");
        let home = temp.path().join("vault");
        let vault = AuditVault::resolve_with(&workspace, Some(&home), None).expect("a vault");

        // The pass rate climbs while audits find more and more false greens.
        let alerts = rounds(&GamingWatch::new(&vault), true);
        let first = alerts.first().expect("a planted stream raises an alert");
        assert_eq!(first.model_slug, "model-a");
        assert!(first.pass_rate_delta > 0.15, "{first:?}");
        assert!(first.quality_delta < -0.10, "{first:?}");

        // The same climb, with audits that keep finding sound work, raises
        // none.
        let clean = rounds(&GamingWatch::new(&vault), false);
        assert!(clean.is_empty(), "{clean:?}");
    }

    /// DP6 (7135): an audit that confirms a false green opens an incident in
    /// the vault, logged, with a fix proposal that says only "audit failed:
    /// false_green" and names no hidden test, and the pair loses routing
    /// trust at once. The same finding again opens nothing.
    #[test]
    fn a_confirmed_false_green_opens_an_incident_with_a_fix_proposal() {
        use roko_gate::audit::incident::IncidentStatus;

        let temp = tempfile::tempdir().expect("tempdir");
        let workspace = temp.path().join("repo");
        std::fs::create_dir_all(&workspace).expect("mkdir");
        let home = temp.path().join("vault");
        let vault = AuditVault::resolve_with(&workspace, Some(&home), None).expect("a vault");
        let mut ledger = AuditLedger::open(&vault).expect("a ledger");
        let unit = AuditUnit {
            sel_id: "sel-0a1b2c3d4e5f".to_string(),
            attempt_key: "run-1:plan:t1:1".to_string(),
            run_id: "run-1".to_string(),
            plan_id: "plan".to_string(),
            task_id: "t1".to_string(),
            pi: 0.25,
            base_tree: Some("base".to_string()),
            result_tree: Some("result".to_string()),
            model: "glm-4.7".to_string(),
            task: AuditTask {
                files: vec!["src/lib.rs".to_string()],
                ..AuditTask::default()
            },
        };
        // B1's hidden suite failed the attempt. The failing test's name stays
        // in the vault's check detail, and nowhere an incident shows.
        let mut checks = Map::new();
        let b1 = json!({ "suite_id": "hs-7f2c", "y": true, "failing": ["two_is_two"] });
        checks.insert("b1".into(), b1);
        let findings = vec!["test_weakened `tests/a.rs`: an assertion was removed".to_string()];
        let labels = AuditLabels {
            y: Some(true),
            g: Some(false),
            w: None,
        };
        let evidence = evidence_of(&unit, &checks, &findings);

        let opened = report_incidents(&vault, &mut ledger, &unit, labels, &evidence);
        assert_eq!(opened.len(), 1, "a false green, and G is 0");
        let incident = &opened[0];
        assert_eq!(incident.kind, IncidentKind::FalseGreen);
        assert_eq!(incident.status(), IncidentStatus::Open);
        assert_eq!(
            incident.evidence.res_id.as_deref(),
            Some("res-0a1b2c3d4e5f")
        );
        assert_eq!(incident.evidence.hidden_failed, Some(1));
        let record = vault
            .incidents_dir()
            .join(format!("{}.json", incident.incident_id));
        let record = std::fs::read_to_string(record).expect("the incident record");
        assert!(!record.contains("two_is_two"), "{record}");

        // The fix proposal is a ready task that says only what failed.
        let name = incident.fix_proposal.as_deref().expect("a fix proposal");
        let path = vault.incidents_dir().join(name);
        let proposal = std::fs::read_to_string(path).expect("the proposal");
        let parsed: toml::Value = toml::from_str(&proposal).expect("a TOML task");
        let task = &parsed["task"][0];
        assert_eq!(task["title"].as_str(), Some("audit failed: false_green"));
        assert_eq!(
            task["description"].as_str(),
            Some("audit failed: false_green")
        );
        assert_eq!(task["files"][0].as_str(), Some("src/lib.rs"));
        for hidden in ["two_is_two", "hs-7f2c", "tests/a.rs"] {
            assert!(!proposal.contains(hidden), "{hidden} in {proposal}");
        }

        // It is logged, and the pair's routing trust falls at once.
        let all = records(ledger.dir()).expect("records");
        let opens = all
            .iter()
            .filter(|record| {
                matches!(&record.event, AuditEvent::Incident { status, .. } if status == "open")
            })
            .count();
        assert_eq!(opens, 1);
        let downgraded = all.iter().any(|record| {
            matches!(
                &record.event,
                AuditEvent::PolicyChange { knob, .. } if knob == "trust:glm-4.7/roko"
            )
        });
        assert!(downgraded);
        let trust = TrustBook::load(&trust_path(&vault)).expect("the trust book");
        let glm = &trust.estimates[0];
        assert_eq!(glm.model, "glm-4.7");
        assert!(glm.mean() > 0.05, "below the prior's 5%: {glm:?}");

        // The same finding again opens nothing.
        let again = report_incidents(&vault, &mut ledger, &unit, labels, &evidence);
        assert!(again.is_empty());
    }
}
