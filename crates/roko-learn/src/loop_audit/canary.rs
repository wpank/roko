//! Canary wire trace: probes P1–P7 localize where a loop is cut (S03 §4.7; backlog 5121).
//!
//! When a loop turns `flagged:dormant`, or on demand, the auditor writes a
//! nonce artifact through the loop's own writer and traces it: P1 written,
//! P2 loaded, P3 selected, P4 assembled or routed, P5 executed, P6 logged,
//! P7 credited. The trace stops at the first probe that fails, and one
//! `loop.canary` row records it.
//!
//! roko-learn cannot call the real writers: roko-neuro depends on
//! roko-learn, and dispatch lives in roko-cli. The driver therefore works
//! through two traits that roko-cli implements, [`CanaryWriter`] per loop
//! (backlog 5127) and [`DryRunPlanner`] over dispatch's dry-run `plan()`
//! (backlog 5128); their signatures are frozen for those tasks. P6 reads the
//! run's decision and exposure rows for the nonce. A dry-run canary skips P5
//! and spends nothing.

use std::path::Path;

use super::ledger::{CanaryRow, LOOP_AUDIT_SCHEMA, Ledger, LoopAuditRecord, LoopAuditRow, ProbeRow};
use crate::telemetry::records::{RunFile, b3_digest};

/// A loop's own writer and reader, as the canary drives them (S03 §4.7).
pub trait CanaryWriter {
    /// P1: write a `CANARY-<nonce>` artifact through the loop's own writer
    /// and read it back; the state version that holds it, or why it could
    /// not be written.
    ///
    /// # Errors
    ///
    /// Why the artifact is not in the loop's state.
    fn write(&mut self, nonce: &str) -> Result<u64, String>;

    /// P2: the state version the loop's reader loaded, if it loaded any.
    fn loaded_version(&mut self) -> Option<u64>;

    /// P3: whether the reader's output (ids, pick) contains the nonce.
    fn read_selected(&mut self, nonce: &str) -> bool;

    /// P7: settle a synthetic outcome on the nonce's artifact; whether the
    /// loop's counters for it moved.
    fn credit(&mut self, nonce: &str) -> bool;

    /// Remove the nonce's artifact. The driver calls it after every trace.
    fn cleanup(&mut self, nonce: &str);
}

/// What a dry-run plan shows of the canary (P4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanProbe {
    /// The assembled prompt contains the nonce.
    pub prompt_contains: bool,
    /// The routed model.
    pub model: String,
    /// The source of the route's model choice.
    pub source: String,
}

/// What a capped live call shows of the canary (P5).
#[derive(Debug, Clone, PartialEq)]
pub struct ExecuteProbe {
    /// The provider request contains the nonce.
    pub request_contains: bool,
    /// The model the provider was called with.
    pub model: String,
    /// What the call cost.
    pub cost_usd: f64,
}

/// Dispatch's planner, as the canary drives it (S03 §4.7).
pub trait DryRunPlanner {
    /// P4: plan `task` as dispatch would; with `dry_run`, no provider is
    /// called.
    ///
    /// # Errors
    ///
    /// Why no plan was made.
    fn plan(&mut self, task: &CanaryTask, dry_run: bool) -> Result<PlanProbe, String>;

    /// P5: run the plan live with `max_tokens` capped. `None` when the
    /// planner cannot, which skips P5; a dry-run canary never asks.
    fn execute_capped(&mut self, _task: &CanaryTask) -> Option<Result<ExecuteProbe, String>> {
        None
    }
}

/// Where the canary's artifact should show in the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanaryTarget {
    /// In the assembled prompt: knowledge, playbooks, sections, prompt
    /// variants.
    Prompt,
    /// In the route: the canary preference sends the category to `model`,
    /// chosen by `source` rather than a fallback.
    Route {
        /// The canary model.
        model: String,
        /// The honest source of the choice.
        source: String,
    },
}

impl CanaryTarget {
    /// P4's verdict on `plan`, with its evidence.
    fn plan_verdict(&self, plan: &PlanProbe) -> (bool, String) {
        match self {
            Self::Prompt => {
                let evidence = if plan.prompt_contains {
                    "the dry-run prompt carries the nonce"
                } else {
                    "the dry-run prompt lacks the nonce"
                };
                (plan.prompt_contains, evidence.to_string())
            }
            Self::Route { model, source } => (
                plan.model == *model && plan.source == *source,
                format!("routed to {} by {}", plan.model, plan.source),
            ),
        }
    }

    /// P5's verdict on `executed`, with its evidence.
    fn execute_verdict(&self, executed: &ExecuteProbe) -> (bool, String) {
        match self {
            Self::Prompt => {
                let evidence = if executed.request_contains {
                    "the provider request carries the nonce"
                } else {
                    "the provider request lacks the nonce"
                };
                (executed.request_contains, evidence.to_string())
            }
            Self::Route { model, .. } => (
                executed.model == *model,
                format!("the provider was called with {}", executed.model),
            ),
        }
    }
}

/// The synthetic task a canary traces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanaryTask {
    /// The loop under trace.
    pub loop_id: String,
    /// The nonce its artifact carries.
    pub nonce: String,
    /// The synthetic category the artifact targets, e.g. `canary`.
    pub category: String,
    /// Where the artifact should show in the plan.
    pub target: CanaryTarget,
}

/// The probes run so far, in order, and what they spent.
#[derive(Debug, Default)]
struct Probes {
    rows: Vec<ProbeRow>,
    cost_usd: f64,
}

impl Probes {
    /// Record probe `p`; whether it passed.
    fn check(&mut self, p: &str, ok: bool, evidence: String) -> bool {
        self.rows.push(ProbeRow {
            p: p.to_string(),
            ok,
            evidence: Some(evidence),
        });
        ok
    }
}

/// Trace `task` through `writer`, `planner` and the run directory
/// `run_dir`, stopping at the first probe that fails, and clean the
/// artifact up. P5 runs only when `dry_run` is false and the planner can
/// execute.
pub fn trace(
    writer: &mut dyn CanaryWriter,
    planner: &mut dyn DryRunPlanner,
    run_dir: &Path,
    task: &CanaryTask,
    dry_run: bool,
) -> CanaryRow {
    let mut probes = Probes::default();
    probe(writer, planner, run_dir, task, dry_run, &mut probes);
    writer.cleanup(&task.nonce);
    CanaryRow {
        nonce: task.nonce.clone(),
        dry_run,
        first_failure: probes
            .rows
            .iter()
            .find(|row| !row.ok)
            .map(|row| row.p.clone()),
        probes: probes.rows,
        cost_usd: probes.cost_usd,
    }
}

/// [`trace`], then append its `loop.canary` row to `ledger`, naming the run
/// by `run_dir`'s directory name.
///
/// # Errors
///
/// The ledger's write error.
pub fn run_canary(
    writer: &mut dyn CanaryWriter,
    planner: &mut dyn DryRunPlanner,
    run_dir: &Path,
    task: &CanaryTask,
    dry_run: bool,
    ledger: &Ledger,
) -> std::io::Result<CanaryRow> {
    let row = trace(writer, planner, run_dir, task, dry_run);
    let (loop_id, nonce) = (&task.loop_id, &task.nonce);
    let identity = format!("{LOOP_AUDIT_SCHEMA}|{loop_id}|loop.canary|{nonce}");
    let run_id = run_dir
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    ledger.append(&LoopAuditRecord {
        schema_version: LOOP_AUDIT_SCHEMA.to_string(),
        record_id: Some(b3_digest(identity.as_bytes())),
        ts: Some(chrono::Utc::now().to_rfc3339()),
        loop_id: task.loop_id.clone(),
        harness_sha: None,
        config_hash: None,
        audit_epoch: None,
        run_id: Some(run_id),
        row: LoopAuditRow::Canary(row.clone()),
    })?;
    Ok(row)
}

/// Run the probes in order until one fails.
fn probe(
    writer: &mut dyn CanaryWriter,
    planner: &mut dyn DryRunPlanner,
    run_dir: &Path,
    task: &CanaryTask,
    dry_run: bool,
    probes: &mut Probes,
) {
    let nonce = task.nonce.as_str();

    // P1 written: the loop's own writer holds the nonce.
    let version = match writer.write(nonce) {
        Ok(version) => version,
        Err(error) => {
            probes.check("P1", false, error);
            return;
        }
    };
    probes.check("P1", true, format!("state version {version} holds the nonce"));

    // P2 loaded: the reader loaded that version or a later one.
    let loaded = writer.loaded_version();
    let evidence = loaded.map_or_else(
        || "the reader loaded no state".to_string(),
        |loaded| format!("the reader loaded version {loaded} of {version}"),
    );
    if !probes.check("P2", loaded.is_some_and(|loaded| loaded >= version), evidence) {
        return;
    }

    // P3 selected: the reader's output contains the nonce.
    let selected = writer.read_selected(nonce);
    let evidence = if selected {
        "the reader selected the nonce"
    } else {
        "the reader's output lacks the nonce"
    };
    if !probes.check("P3", selected, evidence.to_string()) {
        return;
    }

    // P4 assembled or routed: the plan carries the nonce.
    let (planned, evidence) = match planner.plan(task, dry_run) {
        Ok(plan) => task.target.plan_verdict(&plan),
        Err(error) => (false, error),
    };
    if !probes.check("P4", planned, evidence) {
        return;
    }

    // P5 executed: a capped live call, never on a dry run.
    if !dry_run && let Some(executed) = planner.execute_capped(task) {
        let (ran, evidence) = match executed {
            Ok(executed) => {
                probes.cost_usd += executed.cost_usd;
                task.target.execute_verdict(&executed)
            }
            Err(error) => (false, error),
        };
        if !probes.check("P5", ran, evidence) {
            return;
        }
    }

    // P6 logged: the run's decision or exposure rows name the nonce.
    let logged: Vec<&str> = [RunFile::Decisions, RunFile::Exposures]
        .into_iter()
        .filter(|file| {
            std::fs::read_to_string(file.path_in(run_dir))
                .is_ok_and(|text| text.lines().any(|line| line.contains(nonce)))
        })
        .map(RunFile::file_name)
        .collect();
    let evidence = if logged.is_empty() {
        "no decision or exposure row names the nonce".to_string()
    } else {
        format!("{} names the nonce", logged.join(" and "))
    };
    if !probes.check("P6", !logged.is_empty(), evidence) {
        return;
    }

    // P7 credited: a synthetic settled outcome moved the counters.
    let credited = writer.credit(nonce);
    let evidence = if credited {
        "the synthetic outcome moved the nonce's counters"
    } else {
        "the synthetic outcome left the counters unchanged"
    };
    probes.check("P7", credited, evidence.to_string());
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use serde::{Deserialize, Serialize};

    use super::super::faults::{self, FaultKind};
    use super::super::ledger::append_row;
    use super::*;

    /// A toy loop's state: a version and each entry's credit count.
    #[derive(Debug, Default, Serialize, Deserialize)]
    struct ToyState {
        version: u64,
        entries: BTreeMap<String, u64>,
    }

    /// A toy loop whose state is a JSON file, and whose reader asks
    /// `faults::active` as a real read site does: a CUT empties it.
    #[derive(Debug, Clone)]
    struct ToyLoop {
        loop_id: String,
        state: PathBuf,
        run_dir: PathBuf,
    }

    impl ToyLoop {
        fn new(root: &Path, loop_id: &str) -> Self {
            Self {
                loop_id: loop_id.to_string(),
                state: root.join(format!("{loop_id}.json")),
                run_dir: root.join("runs").join("gr-canary"),
            }
        }

        fn load(&self) -> ToyState {
            std::fs::read_to_string(&self.state)
                .ok()
                .and_then(|text| serde_json::from_str(&text).ok())
                .unwrap_or_default()
        }

        fn save(&self, state: &ToyState) {
            let text = serde_json::to_string(state).expect("serialize the toy state");
            std::fs::write(&self.state, text).expect("write the toy state");
        }

        /// The loop's reader: no state under a CUT flag.
        fn read(&self) -> Option<ToyState> {
            (faults::active(&self.loop_id) != Some(FaultKind::Cut)).then(|| self.load())
        }

        /// A dry-run canary over this loop, appended to `ledger`.
        fn canary(&self, nonce: &str, ledger: &Ledger) -> CanaryRow {
            let task = CanaryTask {
                loop_id: self.loop_id.clone(),
                nonce: nonce.to_string(),
                category: "canary".to_string(),
                target: CanaryTarget::Prompt,
            };
            let mut writer = self.clone();
            let mut planner = ToyPlanner { toy: self.clone() };
            run_canary(&mut writer, &mut planner, &self.run_dir, &task, true, ledger)
                .expect("append the canary row")
        }
    }

    impl CanaryWriter for ToyLoop {
        fn write(&mut self, nonce: &str) -> Result<u64, String> {
            let mut state = self.load();
            state.version += 1;
            state.entries.insert(format!("CANARY-{nonce}"), 0);
            self.save(&state);
            Ok(state.version)
        }

        fn loaded_version(&mut self) -> Option<u64> {
            self.read().map(|state| state.version)
        }

        fn read_selected(&mut self, nonce: &str) -> bool {
            let entry = format!("CANARY-{nonce}");
            self.read()
                .is_some_and(|state| state.entries.contains_key(&entry))
        }

        fn credit(&mut self, nonce: &str) -> bool {
            let mut state = self.load();
            let Some(count) = state.entries.get_mut(&format!("CANARY-{nonce}")) else {
                return false;
            };
            *count += 1;
            self.save(&state);
            true
        }

        fn cleanup(&mut self, nonce: &str) {
            let mut state = self.load();
            state.entries.remove(&format!("CANARY-{nonce}"));
            self.save(&state);
        }
    }

    /// A planner whose prompt holds what the toy reader selects, and which
    /// logs a decision row listing it.
    struct ToyPlanner {
        toy: ToyLoop,
    }

    impl DryRunPlanner for ToyPlanner {
        fn plan(&mut self, task: &CanaryTask, _dry_run: bool) -> Result<PlanProbe, String> {
            let prompt: Vec<String> = self
                .toy
                .read()
                .map(|state| state.entries.into_keys().collect())
                .unwrap_or_default();
            let row = serde_json::json!({ "kind": "decision", "learned_state": prompt });
            let decisions = RunFile::Decisions.path_in(&self.toy.run_dir);
            append_row(&decisions, &row).map_err(|error| error.to_string())?;
            Ok(PlanProbe {
                prompt_contains: prompt.iter().any(|entry| entry.contains(&task.nonce)),
                model: "toy-model".to_string(),
                source: "toy".to_string(),
            })
        }
    }

    /// The probes a row ran, in order.
    fn probes_run(row: &CanaryRow) -> Vec<&str> {
        row.probes.iter().map(|probe| probe.p.as_str()).collect()
    }

    /// A healthy loop passes every probe of a dry run (P5 is skipped), and
    /// its artifact is cleaned up.
    #[test]
    fn canary_passes_a_healthy_loop() {
        let dir = tempfile::tempdir().expect("temp dir");
        let toy = ToyLoop::new(dir.path(), "L-canary-healthy");
        let ledger = Ledger::at(dir.path().join("learn/loop-audit.jsonl"));

        let row = toy.canary("c-healthy", &ledger);
        assert_eq!(row.first_failure, None, "{row:?}");
        assert_eq!(probes_run(&row), ["P1", "P2", "P3", "P4", "P6", "P7"]);
        assert!(row.dry_run);
        assert_eq!(row.cost_usd, 0.0);
        assert!(toy.load().entries.is_empty(), "the artifact is cleaned up");

        let rows = ledger.read().expect("read the ledger");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].loop_id, "L-canary-healthy");
        assert_eq!(rows[0].run_id, Some(Some("gr-canary".to_string())));
        assert_eq!(rows[0].row, LoopAuditRow::Canary(row));
    }

    /// S03 §4.7 and C3: a CUT flag on the toy loop's reader makes P2 the
    /// first failing probe; once the flag is cleared, every probe passes.
    #[cfg(feature = "fault-injection")]
    #[test]
    fn canary_localizes_injected_cut() {
        const LOOP: &str = "L-canary-cut";
        let dir = tempfile::tempdir().expect("temp dir");
        let toy = ToyLoop::new(dir.path(), LOOP);
        let ledger = Ledger::at(dir.path().join("learn/loop-audit.jsonl"));
        faults::enable(
            faults::FaultActor::Env,
            toy.run_dir.join(super::super::ledger::FAULTS_FILE),
        );
        faults::set(faults::FaultSpec {
            loop_id: LOOP.to_string(),
            kind: FaultKind::Cut,
            ttl_secs: 600,
            max_decisions: 100,
            spend_cap_usd: None,
        })
        .expect("set the CUT flag");

        let cut = toy.canary("c-cut", &ledger);
        assert_eq!(cut.first_failure.as_deref(), Some("P2"), "{cut:?}");
        assert_eq!(probes_run(&cut), ["P1", "P2"]);
        assert!(toy.load().entries.is_empty(), "the artifact is cleaned up");

        assert!(faults::clear(LOOP));
        let healthy = toy.canary("c-clear", &ledger);
        assert_eq!(healthy.first_failure, None, "{healthy:?}");
        assert_eq!(probes_run(&healthy), ["P1", "P2", "P3", "P4", "P6", "P7"]);

        let rows = ledger.read().expect("read the ledger");
        assert_eq!(rows.len(), 2);
        for (row, nonce) in rows.iter().zip(["c-cut", "c-clear"]) {
            let LoopAuditRow::Canary(canary) = &row.row else {
                panic!("not a canary row: {row:?}");
            };
            assert_eq!(canary.nonce, nonce);
            assert!(canary.dry_run);
            assert_eq!(canary.cost_usd, 0.0);
        }
        faults::disable();
    }
}
