//! Guarded commit for the cascade router (P21, 8136).
//!
//! What a run teaches the router reaches `.roko/learn/cascade-router.json`
//! through [`ModelCallJournal::save`], a locked read-merge-write. Once per
//! run, at its end (decision 8103), [`ModelCallJournal::save_guarded`]
//! proposes the merged snapshot to the `router` [`GuardedStore`] instead,
//! under the same lock, so the decision covers exactly the bytes written.
//! [`RouterChecks`] are the store's checks:
//!
//! - held-out: the candidate's prequential log-loss on settled attempts it
//!   was not fitted on ([`held_out_attempts`]) is no worse than the LKG's by
//!   more than [`HELD_OUT_TOLERANCE`];
//! - anchors: the `[[router]]` cases of the human-edited
//!   `.roko/policy/anchors.toml` (task features to acceptable models, and
//!   never an unconfigured or disabled model), and invariants (finite
//!   statistics, no negative counts, no more successes than trials).
//!
//! In enforce mode a failed check writes the LKG back, into the file and
//! into the router; in observe mode the merge is written and the row
//! records the rollback that would have happened. A rollback also drops
//! the unchecked deltas other processes merged into the file since the
//! LKG; they are re-learned, and a failed check's reason says so.
//!
//! [`ModelCallJournal::save`]: crate::model_call_feedback::ModelCallJournal::save
//! [`ModelCallJournal::save_guarded`]: crate::model_call_feedback::ModelCallJournal::save_guarded

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use roko_core::agent::AgentRole;
use roko_core::task::TaskCategory;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::cascade::persistence::CascadeSnapshot;
use crate::cascade_router::CascadeRouter;
use crate::error::LearnError;
use crate::guarded_commit::{
    CheckOutcome, CommitCheck, CommitDecision, GuardError, GuardMode, GuardedState, GuardedStore,
    Proposer,
};

/// The router's store under `.roko/learn/commits/`.
pub const ROUTER_STORE: &str = "router";
/// How far above the LKG's the candidate's held-out log-loss may be.
pub const HELD_OUT_TOLERANCE: f64 = 0.02;
/// The human-edited anchors file under `.roko/` (decision 8103).
pub const ANCHORS_FILE: &str = "policy/anchors.toml";
/// What a failed check's reason adds.
const ROLLBACK_NOTE: &str = "a rollback also drops the unchecked deltas other processes merged \
     into cascade-router.json since the last-known-good version; they are re-learned";
/// Count fields of a router snapshot, which are never below zero.
const COUNT_FIELDS: [&str; 4] = ["trials", "successes", "total_observations", "observations"];

/// A settled attempt the held-out check scores.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettledOutcome {
    /// The model it ran on.
    pub model: String,
    /// Its task category, when known.
    #[serde(default)]
    pub category: Option<TaskCategory>,
    /// Whether its outcome passed.
    pub success: bool,
    /// Whether it ran on M2's holdout arm.
    #[serde(default)]
    pub holdout: bool,
}

/// The settled attempts of a run, in order, that the held-out check
/// scores (decision 8103): those on M2's holdout arm, else the run's last
/// 20%.
#[must_use]
pub fn held_out_attempts(settled: &[SettledOutcome]) -> Vec<SettledOutcome> {
    let holdout: Vec<SettledOutcome> = settled
        .iter()
        .filter(|attempt| attempt.holdout)
        .cloned()
        .collect();
    if !holdout.is_empty() {
        return holdout;
    }
    let tail = settled.len().div_ceil(5);
    settled[settled.len() - tail..].to_vec()
}

/// One `[[router]]` case of the anchors file: the task features it covers
/// and the models acceptable for them.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouterAnchor {
    /// The role it covers; every role when absent.
    #[serde(default)]
    pub role: Option<AgentRole>,
    /// The task category it covers; every category when absent.
    #[serde(default)]
    pub category: Option<TaskCategory>,
    /// The models acceptable for it.
    pub acceptable: Vec<String>,
}

impl RouterAnchor {
    /// The case in words, for a check's reason.
    fn case(&self) -> String {
        let category = self.category.map_or("every category", TaskCategory::label);
        match self.role {
            Some(role) => format!("{role:?} on {category}"),
            None => format!("every role on {category}"),
        }
    }
}

/// The anchors file's tables the router reads; the knowledge store reads
/// its own.
#[derive(Debug, Default, Deserialize)]
struct AnchorsFile {
    #[serde(default)]
    router: Vec<RouterAnchor>,
}

/// The `[[router]]` cases of `.roko/policy/anchors.toml` under `roko_dir`;
/// none without the file.
///
/// # Errors
///
/// The read or parse error of a file that is there.
pub fn load_router_anchors(roko_dir: &Path) -> Result<Vec<RouterAnchor>, String> {
    let path = roko_dir.join(ANCHORS_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    let file: AnchorsFile =
        toml::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(file.router)
}

/// The router's commit checks (decision 8103).
#[derive(Debug, Clone, Default)]
pub struct RouterChecks {
    /// Settled attempts neither version was fitted on, in order.
    pub held_out: Vec<SettledOutcome>,
    /// The anchors file's `[[router]]` cases.
    pub anchors: Vec<RouterAnchor>,
    /// The configured, enabled models; empty skips that part of the anchor
    /// check.
    pub configured: BTreeSet<String>,
}

impl CommitCheck for RouterChecks {
    fn held_out(&self, candidate: &[u8], lkg: Option<&[u8]>) -> CheckOutcome {
        const CHECK: &str = "held_out";
        let Some(lkg) = lkg else {
            return CheckOutcome::pass(CHECK, "the first version: no last-known-good to compare");
        };
        if self.held_out.is_empty() {
            return CheckOutcome::pass(CHECK, "no settled attempts held out");
        }
        let Ok(candidate) = serde_json::from_slice::<CascadeSnapshot>(candidate) else {
            return failed(CHECK, "the candidate is not a router snapshot");
        };
        let Ok(lkg) = serde_json::from_slice::<CascadeSnapshot>(lkg) else {
            return CheckOutcome::pass(CHECK, "the last-known-good version does not parse");
        };
        let candidate_loss = prequential_log_loss(&candidate, &self.held_out);
        let lkg_loss = prequential_log_loss(&lkg, &self.held_out);
        let outcome = if candidate_loss <= lkg_loss + HELD_OUT_TOLERANCE {
            let reason = format!(
                "held-out log-loss {candidate_loss:.4}, the last-known-good's {lkg_loss:.4}"
            );
            CheckOutcome::pass(CHECK, reason)
        } else {
            let problem = format!(
                "held-out log-loss {candidate_loss:.4} is more than {HELD_OUT_TOLERANCE} above \
                 the last-known-good's {lkg_loss:.4}"
            );
            failed(CHECK, &problem)
        };
        outcome
            .with("candidate_log_loss", candidate_loss)
            .with("lkg_log_loss", lkg_loss)
            .with("held_out_attempts", self.held_out.len() as f64)
    }

    fn anchors(&self, candidate: &[u8]) -> CheckOutcome {
        const CHECK: &str = "anchors";
        let Ok(value) = serde_json::from_slice::<Value>(candidate) else {
            return failed(CHECK, "the candidate is not JSON");
        };
        if let Err(problem) = invariants(&value, "$") {
            return failed(CHECK, &problem);
        }
        let Ok(snapshot) = serde_json::from_value::<CascadeSnapshot>(value) else {
            return failed(CHECK, "the candidate is not a router snapshot");
        };
        for anchor in &self.anchors {
            for model in choices(&snapshot, anchor) {
                let configured = self.configured.is_empty() || self.configured.contains(&model);
                let why = if !anchor.acceptable.contains(&model) {
                    "its anchor rejects"
                } else if !configured {
                    "is not a configured, enabled model"
                } else {
                    continue;
                };
                let problem = format!("{} routes to {model}, which {why}", anchor.case());
                return failed(CHECK, &problem);
            }
        }
        let reason = format!("{} router anchors hold, and the invariants", self.anchors.len());
        CheckOutcome::pass(CHECK, reason)
    }
}

/// A failed check named `check`, whose reason ends with what a rollback
/// drops.
fn failed(check: &str, problem: &str) -> CheckOutcome {
    CheckOutcome::fail(check, format!("{problem}; {ROLLBACK_NOTE}"))
}

/// The mean log-loss of `snapshot`'s success estimates on `attempts`, in
/// order, each predicted before its outcome is counted: the
/// Laplace-smoothed pass rate of the attempt's model on its category, or on
/// all its tasks without trials there.
fn prequential_log_loss(snapshot: &CascadeSnapshot, attempts: &[SettledOutcome]) -> f64 {
    let mut counts: HashMap<(&str, Option<TaskCategory>), (u64, u64)> = HashMap::new();
    let mut total = 0.0;
    for attempt in attempts {
        let key = (attempt.model.as_str(), attempt.category);
        let (trials, successes) = counts
            .entry(key)
            .or_insert_with(|| model_counts(snapshot, &attempt.model, attempt.category));
        let rate = smoothed_rate(*trials, *successes).clamp(1e-6, 1.0 - 1e-6);
        let likelihood = if attempt.success { rate } else { 1.0 - rate };
        total -= likelihood.ln();
        *trials += 1;
        *successes += u64::from(attempt.success);
    }
    total / attempts.len().max(1) as f64
}

/// `model`'s trials and successes in `snapshot`: on `category` when it has
/// trials there, else on all its tasks.
fn model_counts(
    snapshot: &CascadeSnapshot,
    model: &str,
    category: Option<TaskCategory>,
) -> (u64, u64) {
    let on_category = category
        .and_then(|category| snapshot.category_stats.get(model)?.get(&category))
        .filter(|stats| stats.trials > 0)
        .map(|stats| (stats.trials, stats.successes));
    on_category.unwrap_or_else(|| {
        snapshot
            .confidence_stats
            .get(model)
            .map_or((0, 0), |stats| (stats.trials, stats.successes))
    })
}

/// The Laplace-smoothed pass rate of `successes` in `trials`.
fn smoothed_rate(trials: u64, successes: u64) -> f64 {
    (successes as f64 + 1.0) / (trials as f64 + 2.0)
}

/// The models `snapshot` routes `anchor`'s case to: the stage-1 role
/// table's entry for its role, and the model with the best smoothed pass
/// rate on its category among those with trials.
fn choices(snapshot: &CascadeSnapshot, anchor: &RouterAnchor) -> Vec<String> {
    let mut choices: Vec<String> = anchor
        .role
        .and_then(|role| snapshot.role_table.get(&role))
        .cloned()
        .into_iter()
        .collect();
    let models: BTreeSet<&String> = snapshot
        .confidence_stats
        .keys()
        .chain(snapshot.category_stats.keys())
        .collect();
    let mut best: Option<(&String, f64)> = None;
    for model in models {
        let (trials, successes) = model_counts(snapshot, model, anchor.category);
        let rate = smoothed_rate(trials, successes);
        if trials > 0 && best.is_none_or(|(_, top)| rate > top) {
            best = Some((model, rate));
        }
    }
    choices.extend(best.map(|(model, _)| model.clone()));
    choices
}

/// The first broken invariant of a router snapshot, at its JSON path: a
/// count below zero, more successes than trials, or a statistic that is
/// not a finite number (JSON writes NaN and infinity as `null`).
fn invariants(value: &Value, at: &str) -> Result<(), String> {
    match value {
        Value::Object(fields) => {
            let count = |name: &str| fields.get(name).and_then(Value::as_f64);
            if let (Some(trials), Some(successes)) = (count("trials"), count("successes"))
                && successes > trials
            {
                return Err(format!("{at} has {successes} successes in {trials} trials"));
            }
            for (name, field) in fields {
                let negative = field.as_f64().is_some_and(|number| number < 0.0);
                if negative && COUNT_FIELDS.contains(&name.as_str()) {
                    return Err(format!("{at}.{name} is below zero"));
                }
                invariants(field, &format!("{at}.{name}"))?;
            }
            Ok(())
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                if item.is_null() {
                    return Err(format!("{at}[{index}] is not a finite number"));
                }
                invariants(item, &format!("{at}[{index}]"))?;
            }
            Ok(())
        }
        Value::Number(number) if !number.as_f64().is_some_and(f64::is_finite) => {
            Err(format!("{at} is not a finite number"))
        }
        _ => Ok(()),
    }
}

impl GuardedState for CascadeRouter {
    /// The persisted JSON, as [`CascadeRouter::save`] writes it.
    fn snapshot(&self) -> Vec<u8> {
        self.snapshot_json().into_bytes()
    }

    fn restore(&mut self, snapshot: &[u8]) -> Result<(), GuardError> {
        self.restore_learned_json(snapshot)
            .map_err(|error| GuardError::BadSnapshot {
                store: ROUTER_STORE.to_string(),
                reason: error.to_string(),
            })
    }
}

/// The bytes a guarded save writes: the merged snapshot, or the LKG a
/// rollback puts back.
struct RouterFile(Vec<u8>);

impl GuardedState for RouterFile {
    fn snapshot(&self) -> Vec<u8> {
        self.0.clone()
    }

    fn restore(&mut self, snapshot: &[u8]) -> Result<(), GuardError> {
        self.0 = snapshot.to_vec();
        Ok(())
    }
}

/// A guarded save's store, checks and proposer.
#[derive(Debug, Clone)]
pub struct RouterGuard {
    /// `.roko/learn`, whose `commits/router/` keeps the versions.
    pub learn_dir: PathBuf,
    /// What a failed check does.
    pub mode: GuardMode,
    /// The checks.
    pub checks: RouterChecks,
    /// Who proposes, and why.
    pub proposer: Proposer,
}

/// Save `router` to the snapshot at `path` under `guard`, and return the
/// decision: the merge of what it learned into the file is proposed to the
/// router's guarded store under the file's lock, and a rollback writes the
/// LKG back, into the file and into `router`.
///
/// # Errors
///
/// As [`CascadeRouter::save`], and guarded store errors, which leave the
/// file as it was: among them a check that fails in enforce mode before the
/// store has a version.
pub fn save_guarded(
    router: &CascadeRouter,
    path: &Path,
    guard: &RouterGuard,
) -> Result<CommitDecision, LearnError> {
    router.save_deciding(path, |merged| {
        let mut store = GuardedStore::open(&guard.learn_dir, ROUTER_STORE, guard.mode)
            .map_err(std::io::Error::other)?;
        let mut file = RouterFile(merged.to_vec());
        let decision = store
            .propose(&mut file, &guard.checks, &guard.proposer)
            .map_err(std::io::Error::other)?;
        let lkg = (decision == CommitDecision::RolledBack).then_some(file.0);
        Ok((decision, lkg))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_call_feedback::ModelCallJournal;
    use crate::model_router::RoutingContext;

    const A: &str = "model-a";
    const B: &str = "model-b";

    /// `router` observes `times` attempts of `model` on implementation tasks,
    /// each passing or failing as `success` says.
    fn teach(router: &CascadeRouter, model: &str, success: bool, times: usize) {
        let ctx = RoutingContext::default();
        let reward = if success { 1.0 } else { 0.0 };
        for _ in 0..times {
            router.record_observation(&ctx, model, reward, success);
        }
    }

    /// Settled attempts as they went: A passes, B fails.
    fn truth(times: usize) -> Vec<SettledOutcome> {
        let settled = |model: &str, success| SettledOutcome {
            model: model.to_string(),
            category: Some(TaskCategory::Implementation),
            success,
            holdout: false,
        };
        (0..times)
            .flat_map(|_| [settled(A, true), settled(B, false)])
            .collect()
    }

    /// P21 (8136): a candidate fitted on inverted labels is rolled back, in
    /// the file and in the router, and a neutral one then commits.
    #[test]
    fn router_commit_rolls_back_on_held_out_regression() {
        let temp = tempfile::tempdir().expect("tempdir");
        let learn_dir = temp.path().join(".roko/learn");
        let journal = ModelCallJournal::for_learn_dir(&learn_dir);
        let guard = RouterGuard {
            learn_dir: learn_dir.clone(),
            mode: GuardMode::Enforce,
            checks: RouterChecks {
                held_out: truth(10),
                anchors: Vec::new(),
                configured: [A, B].map(String::from).into(),
            },
            proposer: Proposer::evolved("router", ROUTER_STORE, "router:test-run"),
        };
        let router = CascadeRouter::new(vec![A.to_string(), B.to_string()]);
        let implementation = |model: &str| (model.to_string(), "implementation".to_string());

        // The LKG, true to the outcomes.
        teach(&router, A, true, 18);
        teach(&router, A, false, 2);
        teach(&router, B, true, 2);
        teach(&router, B, false, 18);
        let first = journal.save_guarded(&router, &guard).expect("saved");
        assert_eq!(first, CommitDecision::Committed);

        // A run on inverted labels: the file and the router keep the LKG.
        teach(&router, A, false, 40);
        teach(&router, B, true, 40);
        let inverted = journal.save_guarded(&router, &guard).expect("saved");
        assert_eq!(inverted, CommitDecision::RolledBack);
        let bytes = std::fs::read(journal.snapshot_path()).expect("the snapshot");
        let file: CascadeSnapshot = serde_json::from_slice(&bytes).expect("a router snapshot");
        assert_eq!(file.confidence_stats[A].trials, 20);
        assert_eq!(file.confidence_stats[B].successes, 2);
        let learned = router.category_stats_snapshot();
        assert_eq!(learned[&implementation(A)], (20, 18));
        assert_eq!(learned[&implementation(B)], (20, 2));

        // A neutral run commits on top of it.
        teach(&router, A, true, 2);
        teach(&router, B, false, 2);
        let neutral = journal.save_guarded(&router, &guard).expect("saved");
        assert_eq!(neutral, CommitDecision::Committed);
        let bytes = std::fs::read(journal.snapshot_path()).expect("the snapshot");
        let file: CascadeSnapshot = serde_json::from_slice(&bytes).expect("a router snapshot");
        assert_eq!(file.confidence_stats[A].trials, 22);

        let store = GuardedStore::open(&learn_dir, ROUTER_STORE, GuardMode::Enforce)
            .expect("the router's store");
        let rows = store.rows().expect("its commit rows");
        let decisions: Vec<CommitDecision> = rows.iter().map(|row| row.decision).collect();
        assert_eq!(
            decisions,
            [
                CommitDecision::Committed,
                CommitDecision::RolledBack,
                CommitDecision::Committed,
            ]
        );
        let held_out = &rows[1].checks[0];
        assert!(!held_out.passed, "{held_out:?}");
        let candidate_loss = held_out.numbers["candidate_log_loss"];
        let lkg_loss = held_out.numbers["lkg_log_loss"];
        assert!(
            candidate_loss > lkg_loss + HELD_OUT_TOLERANCE,
            "{held_out:?}"
        );
        assert!(held_out.reason.contains("re-learned"));
        assert_eq!(store.versions().expect("versions"), [1, 2]);
    }
}
