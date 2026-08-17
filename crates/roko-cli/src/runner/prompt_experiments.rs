//! Attempt-scoped prompt-experiment lifecycle for Runner-v2.
//!
//! Prompt construction prepares a durable treatment receipt. This module owns
//! the two effectful boundaries that must not be hidden inside composition:
//! marking the exact final prompt immediately before provider launch, and
//! settling treatments only after a typed terminal event is durable. Startup
//! reconciliation reads both rotated and live event generations so a crash
//! between those two writes cannot lose learning feedback.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use roko_learn::prompt_experiment::{
    AssignmentSettlement, ExperimentStore, PromptAssignmentError, PromptAttemptKey,
};

use super::types::{RunnerEvent, TaskAttemptOutcome, TaskAttemptRef};

/// Result of replaying durable runner terminals into prompt learning.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PromptExperimentReconciliation {
    /// Unique terminal attempts found for the selected run.
    pub terminal_attempts: usize,
    /// Attempts whose treatment receipts were settled or idempotently replayed.
    pub settled_attempts: usize,
    /// Terminal attempts with no prompt assignment bucket.
    pub attempts_without_assignments: usize,
    /// Attempts with contradictory durable terminal facts. These are not
    /// settled automatically.
    pub conflicting_attempts: Vec<PromptAttemptKey>,
}

/// Construct the durable experiment key corresponding to a runner attempt.
#[must_use]
pub fn attempt_key(run_id: &str, attempt: &TaskAttemptRef) -> PromptAttemptKey {
    PromptAttemptKey::new(
        run_id,
        attempt.plan_id.clone(),
        attempt.task_id.clone(),
        attempt.attempt,
    )
}

/// Hash the exact final system and user prompts at the provider boundary.
///
/// Length prefixes make the pair unambiguous (`("ab", "c")` differs from
/// `("a", "bc")`) without allocating another full prompt copy.
#[must_use]
pub fn dispatch_prompt_hash(system_prompt: &str, user_prompt: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"roko.prompt.dispatch.v1\0");
    hasher.update(&(system_prompt.len() as u64).to_le_bytes());
    hasher.update(system_prompt.as_bytes());
    hasher.update(&(user_prompt.len() as u64).to_le_bytes());
    hasher.update(user_prompt.as_bytes());
    hasher.finalize().to_hex().to_string()
}

/// Mark a prepared attempt dispatched immediately before provider launch.
pub fn mark_dispatched(
    store_path: &Path,
    key: &PromptAttemptKey,
    system_prompt: &str,
    user_prompt: &str,
    included_assignment_ids: &[String],
) -> Result<(), PromptAssignmentError> {
    let prompt_hash = dispatch_prompt_hash(system_prompt, user_prompt);
    let included_assignment_ids = included_assignment_ids
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    ExperimentStore::mark_attempt_dispatched(
        store_path,
        key,
        &prompt_hash,
        &included_assignment_ids,
    )
    .map(|_| ())
}

/// Project a runner terminal outcome into experiment settlement semantics.
#[must_use]
pub const fn settlement_for_outcome(outcome: TaskAttemptOutcome) -> AssignmentSettlement {
    match outcome {
        TaskAttemptOutcome::Passed => AssignmentSettlement::Observed { success: true },
        TaskAttemptOutcome::Failed
        | TaskAttemptOutcome::Exhausted
        | TaskAttemptOutcome::TimedOut => AssignmentSettlement::Observed { success: false },
        TaskAttemptOutcome::Cancelled => AssignmentSettlement::Abandoned,
    }
}

/// Settle a single durable attempt after its terminal event has been appended.
///
/// `Ok(false)` means the workspace has no experiment store or the attempt had
/// no applicable experiment. This is the normal path for non-experiment runs.
pub fn settle_terminal_attempt(
    store_path: &Path,
    key: &PromptAttemptKey,
    settlement: AssignmentSettlement,
) -> Result<bool, PromptAssignmentError> {
    if !store_path.exists() {
        return Ok(false);
    }
    match ExperimentStore::settle_attempt(store_path, key, settlement) {
        Ok(_) => Ok(true),
        Err(PromptAssignmentError::AttemptNotFound(_)) => Ok(false),
        Err(error) => Err(error),
    }
}

/// Reconcile typed terminal facts from oldest archive through the live log.
///
/// Malformed log input fails before mutating the experiment store. Conflicting
/// terminal facts are reported and skipped while independent attempts settle.
pub async fn reconcile_terminal_events(
    events_jsonl: &Path,
    experiment_store: &Path,
    run_id: &str,
) -> Result<PromptExperimentReconciliation> {
    if !experiment_store.exists() {
        return Ok(PromptExperimentReconciliation::default());
    }

    let mut generations = roko_fs::log_rotation::discover_archives(events_jsonl)
        .await
        .with_context(|| format!("discovering archives for {}", events_jsonl.display()))?;
    generations.push(events_jsonl.to_path_buf());

    let mut terminals = BTreeMap::<PromptAttemptKey, AssignmentSettlement>::new();
    let mut conflicts = BTreeSet::<PromptAttemptKey>::new();
    for path in generations {
        collect_terminal_events(&path, run_id, &mut terminals, &mut conflicts)?;
    }

    let mut report = PromptExperimentReconciliation {
        terminal_attempts: terminals.len() + conflicts.len(),
        conflicting_attempts: conflicts.iter().cloned().collect(),
        ..PromptExperimentReconciliation::default()
    };
    for key in &conflicts {
        terminals.remove(key);
    }
    for (key, settlement) in terminals {
        match settle_terminal_attempt(experiment_store, &key, settlement) {
            Ok(true) => report.settled_attempts += 1,
            Ok(false) => report.attempts_without_assignments += 1,
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "settling prompt experiment for {}/{}/{}",
                        key.plan_id, key.task_id, key.attempt
                    )
                });
            }
        }
    }
    Ok(report)
}

fn collect_terminal_events(
    path: &Path,
    selected_run_id: &str,
    terminals: &mut BTreeMap<PromptAttemptKey, AssignmentSettlement>,
    conflicts: &mut BTreeSet<PromptAttemptKey>,
) -> Result<()> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error).with_context(|| format!("opening {}", path.display())),
    };
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line =
            line.with_context(|| format!("reading {} line {}", path.display(), index + 1))?;
        if line.trim().is_empty() {
            continue;
        }
        let event: RunnerEvent = serde_json::from_str(&line)
            .with_context(|| format!("parsing {} line {}", path.display(), index + 1))?;
        let terminal = match event {
            RunnerEvent::TaskAttemptCompleted {
                run_id,
                attempt,
                outcome,
                prompt_experiment_observation_eligible,
                ..
            } if run_id == selected_run_id => {
                let settlement = if prompt_experiment_observation_eligible {
                    settlement_for_outcome(outcome)
                } else {
                    AssignmentSettlement::Abandoned
                };
                Some((attempt_key(&run_id, &attempt), settlement))
            }
            RunnerEvent::TimeoutRecorded {
                run_id, timeout, ..
            } if run_id == selected_run_id => timeout.attempt.as_ref().map(|attempt| {
                (
                    attempt_key(&run_id, attempt),
                    AssignmentSettlement::Observed { success: false },
                )
            }),
            _ => None,
        };
        let Some((key, settlement)) = terminal else {
            continue;
        };
        if conflicts.contains(&key) {
            continue;
        }
        match terminals.get(&key) {
            Some(existing) if *existing != settlement => {
                terminals.remove(&key);
                conflicts.insert(key);
            }
            Some(_) => {}
            None => {
                terminals.insert(key, settlement);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_learn::prompt_experiment::{PromptExperiment, PromptVariant};

    fn experiment_store(path: &Path) {
        let variants = vec![
            PromptVariant {
                id: "a".into(),
                name: "A".into(),
                section_name: "constraints".into(),
                content: "Be concise.".into(),
                slug: None,
                active: true,
            },
            PromptVariant {
                id: "b".into(),
                name: "B".into(),
                section_name: "constraints".into(),
                content: "Be thorough.".into(),
                slug: None,
                active: true,
            },
        ];
        let mut store = ExperimentStore::new();
        store.register(PromptExperiment::new("exp", "constraints", variants));
        store.save(path).unwrap();
    }

    fn append_event(path: &Path, event: &RunnerEvent) {
        use std::io::Write as _;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        serde_json::to_writer(&mut file, event).unwrap();
        writeln!(file).unwrap();
    }

    #[test]
    fn prompt_hash_binds_both_boundaries_unambiguously() {
        assert_eq!(
            dispatch_prompt_hash("system", "user"),
            dispatch_prompt_hash("system", "user")
        );
        assert_ne!(
            dispatch_prompt_hash("ab", "c"),
            dispatch_prompt_hash("a", "bc")
        );
        assert_ne!(
            dispatch_prompt_hash("system", "user"),
            dispatch_prompt_hash("system!", "user")
        );
    }

    #[test]
    fn terminal_mapping_excludes_operator_cancellation() {
        assert_eq!(
            settlement_for_outcome(TaskAttemptOutcome::Passed),
            AssignmentSettlement::Observed { success: true }
        );
        assert_eq!(
            settlement_for_outcome(TaskAttemptOutcome::TimedOut),
            AssignmentSettlement::Observed { success: false }
        );
        assert_eq!(
            settlement_for_outcome(TaskAttemptOutcome::Cancelled),
            AssignmentSettlement::Abandoned
        );
    }

    #[tokio::test]
    async fn rotated_terminal_reconciliation_is_restart_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let store_path = dir.path().join("experiments.json");
        let live_events = dir.path().join("events.jsonl");
        let archive = dir.path().join("events.20260816T120000Z.jsonl");
        experiment_store(&store_path);

        let key = PromptAttemptKey::new("run", "plan", "task", 1);
        let prepared =
            ExperimentStore::prepare_attempt_assignments(&store_path, &key, None, &["constraints"])
                .unwrap();
        ExperimentStore::mark_attempt_dispatched(
            &store_path,
            &key,
            "prompt-hash",
            &[prepared[0].assignment_id.as_str()],
        )
        .unwrap();
        let terminal = RunnerEvent::task_attempt_completed(
            "run",
            TaskAttemptRef::new("plan", "task", 1),
            TaskAttemptOutcome::Passed,
            None,
            1,
            "model",
            "provider",
        );
        append_event(&archive, &terminal);
        append_event(&live_events, &terminal);

        let first = reconcile_terminal_events(&live_events, &store_path, "run")
            .await
            .unwrap();
        assert_eq!(first.terminal_attempts, 1);
        assert_eq!(first.settled_attempts, 1);
        let second = reconcile_terminal_events(&live_events, &store_path, "run")
            .await
            .unwrap();
        assert_eq!(second.settled_attempts, 1);

        let store = ExperimentStore::load_strict(&store_path).unwrap();
        let experiment = store.get("exp").unwrap();
        assert_eq!(
            experiment
                .stats
                .values()
                .map(|stats| stats.trials)
                .sum::<u64>(),
            1
        );
        assert_eq!(
            experiment
                .stats
                .values()
                .map(|stats| stats.successes)
                .sum::<u64>(),
            1
        );
    }

    #[tokio::test]
    async fn conflicting_terminal_facts_do_not_mutate_learning() {
        let dir = tempfile::tempdir().unwrap();
        let store_path = dir.path().join("experiments.json");
        let events = dir.path().join("events.jsonl");
        experiment_store(&store_path);
        let key = PromptAttemptKey::new("run", "plan", "task", 1);
        let prepared =
            ExperimentStore::prepare_attempt_assignments(&store_path, &key, None, &["constraints"])
                .unwrap();
        ExperimentStore::mark_attempt_dispatched(
            &store_path,
            &key,
            "prompt-hash",
            &[prepared[0].assignment_id.as_str()],
        )
        .unwrap();
        for outcome in [TaskAttemptOutcome::Passed, TaskAttemptOutcome::Failed] {
            append_event(
                &events,
                &RunnerEvent::task_attempt_completed(
                    "run",
                    TaskAttemptRef::new("plan", "task", 1),
                    outcome,
                    None,
                    1,
                    "model",
                    "provider",
                ),
            );
        }

        let report = reconcile_terminal_events(&events, &store_path, "run")
            .await
            .unwrap();
        assert_eq!(report.conflicting_attempts, vec![key]);
        let store = ExperimentStore::load_strict(&store_path).unwrap();
        assert_eq!(
            store
                .get("exp")
                .unwrap()
                .stats
                .values()
                .map(|stats| stats.trials)
                .sum::<u64>(),
            0
        );
    }

    #[tokio::test]
    async fn prelaunch_failure_terminal_abandons_dispatched_receipt() {
        let dir = tempfile::tempdir().unwrap();
        let store_path = dir.path().join("experiments.json");
        let events = dir.path().join("events.jsonl");
        experiment_store(&store_path);
        let key = PromptAttemptKey::new("run", "plan", "task", 1);
        let prepared =
            ExperimentStore::prepare_attempt_assignments(&store_path, &key, None, &["constraints"])
                .unwrap();
        ExperimentStore::mark_attempt_dispatched(
            &store_path,
            &key,
            "prompt-hash",
            &[prepared[0].assignment_id.as_str()],
        )
        .unwrap();
        let mut terminal = RunnerEvent::task_attempt_completed(
            "run",
            TaskAttemptRef::new("plan", "task", 1),
            TaskAttemptOutcome::Failed,
            None,
            1,
            "model",
            "provider",
        );
        let RunnerEvent::TaskAttemptCompleted {
            prompt_experiment_observation_eligible,
            ..
        } = &mut terminal
        else {
            unreachable!()
        };
        *prompt_experiment_observation_eligible = false;
        append_event(&events, &terminal);

        reconcile_terminal_events(&events, &store_path, "run")
            .await
            .unwrap();
        let store = ExperimentStore::load_strict(&store_path).unwrap();
        assert_eq!(
            store
                .get("exp")
                .unwrap()
                .stats
                .values()
                .map(|stats| stats.trials)
                .sum::<u64>(),
            0
        );
        assert_eq!(
            store.assignments_for_attempt(&key).unwrap()[0].state,
            roko_learn::prompt_experiment::PromptAssignmentState::Abandoned
        );
    }
}
