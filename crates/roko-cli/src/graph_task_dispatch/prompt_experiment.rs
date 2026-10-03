//! Prompt-experiment treatments for Graph task attempts.
//!
//! The prompt builder prepares durable, attempt-scoped treatments whenever
//! [`DispatchContext::prompt_experiment`](crate::dispatch::DispatchContext)
//! is set, and swaps each assigned variant into its canonical section. This
//! module owns the Graph side of the lifecycle around that:
//!
//! 1. [`context`] names the attempt by its durable attempt key (S01: run,
//!    plan, task and 1-based ordinal, so a resumed run never reuses one) and
//!    the root workspace store, when the store holds a prompt experiment.
//! 2. [`LaunchedTreatments::bind`] marks the treatments that survived
//!    composition dispatched with the hash of the exact final prompt,
//!    immediately before provider launch.
//! 3. The attempt's feedback settles them with its learning label
//!    ([`settle`], [`settlement`]). An attempt that ends before feedback
//!    abandons them when its [`LaunchedTreatments`] drops, so no reservation
//!    stays open and no trial is counted.

use std::path::Path;

use roko_learn::prompt_experiment::{
    AssignmentSettlement, ExperimentStore, PromptAssignmentError, PromptAttemptKey,
};
use roko_learn::telemetry::AttemptKey;

use crate::dispatch::{PromptExperimentAssignmentDiagnostic, PromptExperimentContext};

/// Experiment context for the attempt `key`, or `None` when the store at
/// `store_path` holds no prompt experiment (the retrieval-strategy
/// experiment is not one) or cannot be read. An unreadable store is skipped
/// rather than failing prompt assembly.
pub(super) fn context(store_path: &Path, key: &AttemptKey) -> Option<PromptExperimentContext> {
    if !store_path.is_file() {
        return None;
    }
    let store = match ExperimentStore::load_strict(store_path) {
        Ok(store) => store,
        Err(error) => {
            tracing::warn!(
                path = %store_path.display(),
                %error,
                "prompt experiment store is unreadable; dispatching without treatments"
            );
            return None;
        }
    };
    store
        .iter()
        .any(|experiment| {
            experiment.experiment_id != ExperimentStore::RETRIEVAL_STRATEGY_EXPERIMENT_ID
        })
        .then(|| PromptExperimentContext {
            attempt_key: key.to_prompt_attempt_key(),
            store_path: store_path.to_path_buf(),
        })
}

/// Hash of the exact final system and user prompts at the provider boundary.
///
/// Length prefixes make the pair unambiguous (`("ab", "c")` differs from
/// `("a", "bc")`).
pub(super) fn dispatch_prompt_hash(system_prompt: &str, user_prompt: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"roko.prompt.dispatch.v1\0");
    hasher.update(&(system_prompt.len() as u64).to_le_bytes());
    hasher.update(system_prompt.as_bytes());
    hasher.update(&(user_prompt.len() as u64).to_le_bytes());
    hasher.update(user_prompt.as_bytes());
    hasher.finalize().to_hex().to_string()
}

/// How an attempt settles its treatments, from its learning label (S01
/// §4.1): a pass, and a failure of the agent's work (a verify failure, a
/// turn-cap stop, a timeout after output), observe the prompt. An attempt
/// without a label (unverified, a provider or harness failure) says nothing
/// about the prompt and abandons the treatments without counting a trial.
pub(super) fn settlement(learning: Option<bool>) -> AssignmentSettlement {
    match learning {
        Some(success) => AssignmentSettlement::Observed { success },
        None => AssignmentSettlement::Abandoned,
    }
}

/// Settle the treatments of attempt `key`, if it received any.
pub(super) async fn settle(
    store_path: &Path,
    key: PromptAttemptKey,
    settlement: AssignmentSettlement,
) {
    if !store_path.is_file() {
        return;
    }
    let path = store_path.to_path_buf();
    let settled = tokio::task::spawn_blocking(move || {
        ExperimentStore::settle_attempt(&path, &key, settlement).map(|_| key)
    })
    .await;
    match settled {
        Ok(Ok(key)) => tracing::debug!(
            plan_id = %key.plan_id,
            task_id = %key.task_id,
            attempt = key.attempt,
            ?settlement,
            "prompt experiment treatments settled"
        ),
        // An attempt that received no treatment has no bucket.
        Ok(Err(PromptAssignmentError::AttemptNotFound(_))) => {}
        Ok(Err(error)) => {
            tracing::warn!(%error, "prompt experiment settlement failed (best-effort)")
        }
        Err(error) => tracing::warn!(%error, "prompt experiment settlement task failed"),
    }
}

/// Treatments bound to a launched prompt; abandons them on drop unless the
/// attempt's feedback settled them first.
#[must_use = "dropping the guard abandons the attempt's treatments"]
pub(super) struct LaunchedTreatments {
    context: Option<PromptExperimentContext>,
}

impl LaunchedTreatments {
    /// Mark the treatments included in the final prompt dispatched, bound to
    /// its hash. Without a context, or when the attempt received no
    /// treatment, the guard does nothing.
    pub(super) async fn bind(
        context: Option<PromptExperimentContext>,
        assignments: &[PromptExperimentAssignmentDiagnostic],
        system_prompt: &str,
        user_prompt: &str,
    ) -> Self {
        let Some(context) = context.filter(|_| !assignments.is_empty()) else {
            return Self { context: None };
        };
        let mut included = assignments
            .iter()
            .filter(|assignment| assignment.included)
            .map(|assignment| assignment.assignment_id.clone())
            .collect::<Vec<_>>();
        included.sort();
        included.dedup();
        let prompt_hash = dispatch_prompt_hash(system_prompt, user_prompt);
        let store_path = context.store_path.clone();
        let key = context.attempt_key.clone();
        let marked = tokio::task::spawn_blocking(move || {
            let included = included.iter().map(String::as_str).collect::<Vec<_>>();
            ExperimentStore::mark_attempt_dispatched(&store_path, &key, &prompt_hash, &included)
                .map(|assignments| assignments.len())
        })
        .await;
        match marked {
            Ok(Ok(treatments)) => tracing::debug!(
                plan_id = %context.attempt_key.plan_id,
                task_id = %context.attempt_key.task_id,
                attempt = context.attempt_key.attempt,
                treatments,
                "prompt experiment treatments bound to the launched prompt"
            ),
            // Unmarked treatments stay prepared, and settlement abandons
            // prepared treatments without counting a trial.
            Ok(Err(error)) => tracing::warn!(
                %error,
                "marking prompt experiment treatments dispatched failed (best-effort)"
            ),
            Err(error) => tracing::warn!(%error, "prompt experiment dispatch task failed"),
        }
        Self {
            context: Some(context),
        }
    }
}

impl Drop for LaunchedTreatments {
    fn drop(&mut self) {
        let Some(context) = self.context.take() else {
            return;
        };
        // A settled attempt rejects a different settlement, which is the
        // normal case here; only an unsettled attempt is abandoned.
        let abandon = move || {
            let _ = ExperimentStore::settle_attempt(
                &context.store_path,
                &context.attempt_key,
                AssignmentSettlement::Abandoned,
            );
        };
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                handle.spawn_blocking(abandon);
            }
            Err(_) => abandon(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_learn::prompt_experiment::{PromptAssignmentState, PromptExperiment, PromptVariant};

    /// Register the RAG-11 retrieval-strategy experiment, as stores written
    /// before 4105 still hold it.
    fn register_retrieval_experiment(store: &mut ExperimentStore) {
        let id = ExperimentStore::RETRIEVAL_STRATEGY_EXPERIMENT_ID;
        store.register(PromptExperiment::new(
            id,
            id,
            ["keyword", "hdc-only", "hybrid"]
                .into_iter()
                .map(|arm| PromptVariant {
                    id: arm.into(),
                    name: arm.into(),
                    section_name: id.into(),
                    content: arm.into(),
                    slug: None,
                    active: true,
                })
                .collect(),
        ));
    }

    fn save_store(path: &Path) {
        let mut store = ExperimentStore::new();
        register_retrieval_experiment(&mut store);
        store.register(PromptExperiment::new(
            "constraints-exp",
            "constraints",
            ["a", "b"]
                .into_iter()
                .map(|id| PromptVariant {
                    id: id.into(),
                    name: id.into(),
                    section_name: "constraints".into(),
                    content: format!("Variant {id}."),
                    slug: None,
                    active: true,
                })
                .collect(),
        ));
        store.save(path).unwrap();
    }

    fn key(task_id: &str) -> AttemptKey {
        AttemptKey::new("graph-p-run", "p", task_id, 1)
    }

    #[test]
    fn prompt_keys_are_the_attempt_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("experiments.json");
        save_store(&path);
        let retry = AttemptKey::new("graph-p-run", "p", "T1", 3);
        let ctx = context(&path, &retry).expect("context");
        assert_eq!(
            ctx.attempt_key,
            PromptAttemptKey::new("graph-p-run", "p", "T1", 3)
        );
    }

    #[test]
    fn only_a_prompt_experiment_store_gives_a_context() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("experiments.json");
        assert!(context(&path, &key("T1")).is_none(), "no store");

        let mut retrieval_only = ExperimentStore::new();
        register_retrieval_experiment(&mut retrieval_only);
        retrieval_only.save(&path).unwrap();
        assert!(context(&path, &key("T1")).is_none());

        std::fs::write(&path, "{ not json").unwrap();
        assert!(context(&path, &key("T1")).is_none(), "unreadable");

        save_store(&path);
        let ctx = context(&path, &key("T1")).expect("context");
        assert_eq!(ctx.store_path, path);
        assert_eq!(ctx.attempt_key.task_id, "T1");
    }

    #[test]
    fn prompt_hash_binds_both_prompts_unambiguously() {
        assert_eq!(
            dispatch_prompt_hash("system", "user"),
            dispatch_prompt_hash("system", "user")
        );
        assert_ne!(
            dispatch_prompt_hash("ab", "c"),
            dispatch_prompt_hash("a", "bc")
        );
    }

    #[test]
    fn experiments_skip_attempts_without_a_learning_label() {
        use roko_learn::telemetry::{AttemptIdentity, AttemptOutcome, AttemptVerdictRecord};

        let observed = |success| AssignmentSettlement::Observed { success };
        let abandoned = AssignmentSettlement::Abandoned;
        let cases = [
            (AttemptOutcome::Passed, false, observed(true)),
            (AttemptOutcome::GateFailed, false, observed(false)),
            (AttemptOutcome::TurnCap, false, observed(false)),
            (AttemptOutcome::Timeout, true, observed(false)),
            (AttemptOutcome::Unverified, false, abandoned),
            (AttemptOutcome::Timeout, false, abandoned),
            (AttemptOutcome::ProviderError, true, abandoned),
            (AttemptOutcome::ProviderExhausted, false, abandoned),
            (AttemptOutcome::HarnessError, false, abandoned),
        ];
        for (outcome, first_token_seen, expected) in cases {
            let identity = AttemptIdentity::new(&key("T1"));
            let verdict = AttemptVerdictRecord::settle(identity, outcome, first_token_seen);
            assert_eq!(
                settlement(verdict.learning_success()),
                expected,
                "{outcome:?}"
            );
        }
    }

    fn prepare(path: &Path, key: &PromptAttemptKey) -> Vec<PromptExperimentAssignmentDiagnostic> {
        ExperimentStore::prepare_attempt_assignments(path, key, None, &["constraints"])
            .unwrap()
            .into_iter()
            .map(|assignment| PromptExperimentAssignmentDiagnostic {
                assignment_id: assignment.assignment_id,
                experiment_id: assignment.experiment_id,
                variant_id: assignment.variant_id,
                section_name: assignment.section_name,
                content_hash: assignment.content_hash,
                included: true,
            })
            .collect()
    }

    fn states(path: &Path, key: &PromptAttemptKey) -> Vec<PromptAssignmentState> {
        ExperimentStore::load_strict(path)
            .unwrap()
            .assignments_for_attempt(key)
            .unwrap()
            .iter()
            .map(|assignment| assignment.state)
            .collect()
    }

    fn trials(path: &Path) -> (u64, u64) {
        let store = ExperimentStore::load_strict(path).unwrap();
        let stats = store.get("constraints-exp").unwrap().stats.values();
        stats.fold((0, 0), |(trials, successes), stats| {
            (trials + stats.trials, successes + stats.successes)
        })
    }

    #[tokio::test]
    async fn bound_treatments_are_observed_once_settled() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("experiments.json");
        save_store(&path);
        let ctx = context(&path, &key("T1")).unwrap();
        let assignments = prepare(&path, &ctx.attempt_key);
        assert_eq!(assignments.len(), 1);

        let guard =
            LaunchedTreatments::bind(Some(ctx.clone()), &assignments, "system", "user").await;
        assert_eq!(
            states(&path, &ctx.attempt_key),
            [PromptAssignmentState::Dispatched]
        );
        settle(&path, ctx.attempt_key.clone(), settlement(Some(true))).await;
        drop(guard);
        tokio::task::yield_now().await;

        assert_eq!(
            states(&path, &ctx.attempt_key),
            [PromptAssignmentState::Observed]
        );
        assert_eq!(trials(&path), (1, 1));
    }

    #[tokio::test]
    async fn an_attempt_ending_before_feedback_abandons_its_treatments() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("experiments.json");
        save_store(&path);
        let ctx = context(&path, &key("T2")).unwrap();
        let assignments = prepare(&path, &ctx.attempt_key);

        let guard = LaunchedTreatments::bind(Some(ctx.clone()), &assignments, "s", "u").await;
        drop(guard);
        // The abandonment runs on the blocking pool.
        for _ in 0..50 {
            if states(&path, &ctx.attempt_key) == [PromptAssignmentState::Abandoned] {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        assert_eq!(
            states(&path, &ctx.attempt_key),
            [PromptAssignmentState::Abandoned]
        );
        assert_eq!(trials(&path), (0, 0));
    }
}
