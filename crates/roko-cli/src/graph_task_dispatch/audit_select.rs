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

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use roko_core::audit_types::Stratum;
use roko_core::config::audit::AuditConfig;
use roko_gate::audit::canary::{CanaryScanner, scan_text};
use roko_gate::audit::hidden::HiddenStore;
use roko_gate::audit::ledger::{AuditEvent, AuditLedger};
use roko_gate::audit::policy::{
    InclusionParams, RunKey, inclusion_probability, select, workspace_secret,
};
use roko_learn::telemetry::records::{AttemptVerdictRecord, GateVerdictTag};

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
    runs: parking_lot::Mutex<HashMap<String, RunDraws>>,
}

/// One open run: its key, and its green units per stratum.
struct RunDraws {
    key: RunKey,
    strata: BTreeMap<&'static str, u64>,
}

impl AuditSelector {
    /// The lottery of the workspace at `workdir` when `[audit] enabled`, or
    /// `None`, with a warning, when the vault cannot be used.
    pub(super) fn for_config(config: &AuditConfig, workdir: &Path) -> Option<Self> {
        if !config.enabled {
            return None;
        }
        let opened = || -> Result<Self, String> {
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
                trees: parking_lot::Mutex::new(HashMap::new()),
                runs: parking_lot::Mutex::new(HashMap::new()),
            })
        };
        match opened() {
            Ok(selector) => Some(selector),
            Err(error) => {
                tracing::warn!(%error, "[audit] is enabled, but no attempt is drawn");
                None
            }
        }
    }

    /// Open run `run_id`: derive its key and commit to it, once.
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
        runs.insert(run_id.to_string(), RunDraws { key, strata });
    }

    /// Close run `run_id`: list its green units per stratum and reveal its
    /// key.
    pub(super) fn close_run(&self, run_id: &str) {
        let Some(run) = self.runs.lock().remove(run_id) else {
            return;
        };
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

    /// Note the trees the pre-verify screen saw for the attempt
    /// `attempt_key`.
    pub(super) fn note_trees(&self, attempt_key: &str, base: &str, result: &str) {
        let trees = (base.to_string(), result.to_string());
        self.trees.lock().insert(attempt_key.to_string(), trees);
    }

    /// DP1: draw `verdict`'s attempt when it is green, and scan its output
    /// for canaries. Logs and returns; never fails the attempt.
    pub(super) fn draw(&self, verdict: &AttemptVerdictRecord, output: Option<&str>) {
        let identity = &verdict.identity;
        let trees = self.trees.lock().remove(&identity.attempt_key);
        if let Some(output) = output {
            self.scan(output);
        }
        let Some((stratum, census)) = verdict.gate_verdict.map(stratum_of) else {
            return;
        };
        let mut runs = self.runs.lock();
        let Some(run) = runs.get_mut(&identity.run_id) else {
            return;
        };
        let pi = if census {
            Ok(1.0)
        } else {
            inclusion_probability(&self.params, None, None)
        };
        let (base_tree, result_tree) = trees.map_or((None, None), |(base, result)| {
            (Some(base), Some(result))
        });
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
        let model = verdict
            .executed
            .model_dispatched
            .clone()
            .or_else(|| verdict.executed.model_requested.clone())
            .unwrap_or_default();
        let event = AuditEvent::Selection {
            sel_id,
            attempt_key: attempt.clone(),
            run_id: run_id.clone(),
            task_id: task_id.clone(),
            // Settle does not know the task's type; the worker may fill it.
            stratum: Stratum {
                task_type: String::new(),
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
    use roko_gate::audit::ledger::{LedgerRecord, verify_chain};
    use roko_gate::audit::policy::{Selection, verify_reveal};
    use roko_graph::cells::NoopAttemptRecorder;

    use super::*;
    use crate::graph_task_dispatch::tests::{
        VERIFY_PROVIDER, make_spec, make_test_dispatcher, no_auto_fix, recording_feedback,
        verify_step,
    };
    use crate::graph_task_dispatch::{
        CellContext, GraphTaskDispatcher, TaskDispatcher, TaskLease, StreamingTaskDispatcher,
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
            let result = dispatcher.dispatch(&make_spec(task), Vec::new(), &ctx).await;
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
            events.extend(text.lines().map(|line| serde_json::from_str(line).expect("a record")));
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
        assert_eq!(drawn, ["T1", "T2", "T4"], "one draw per green unit, none for a failure");
        assert_eq!(strata["passed"], 2);
        assert_eq!(strata["unverified"], 1);
        assert_eq!(strata["forced_accept"], 0, "the reserved stratum is present, with no units");
        let key = RunKey::from_hex(&key_hex).expect("the revealed key");
        let mismatches = verify_reveal(&key, RUN, &committed, &selections);
        assert!(mismatches.is_empty(), "{mismatches:?}");
        let mirror = temp.path().join(".roko/audit/audits.jsonl");
        assert!(mirror.exists(), "the workspace mirror is written");
    }
}
