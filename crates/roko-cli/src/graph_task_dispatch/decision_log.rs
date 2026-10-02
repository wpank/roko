//! Decision and exposure records of Graph task dispatch (S01 §4.5, §5.3,
//! §5.4).
//!
//! An attempt that reaches routing writes one route decision row to its
//! run's `decisions.jsonl`: the row [`ModelRouter::decide`] made when
//! dispatch planned the attempt, keyed to the attempt. Its prompt's items go
//! to the run's `exposures.jsonl`, one row per item the prompt retrieved,
//! included or not, and the attempt's verdict counts them. A T0 reflex
//! attempt and a harness failure before planning write none.
//!
//! [`ModelRouter::decide`]: crate::dispatch::ModelRouter::decide

use roko_learn::telemetry::{AttemptIdentity, ExposureCounts, ExposureItemKind, ExposureRecord};

use super::attempt::AttemptContext;
use super::*;
use crate::dispatch::RunnerDispatchPlan;
use crate::dispatch::prompt_builder::PromptItemDiagnostic;

/// Most exposure rows one attempt writes (S01 §5.9
/// `max_exposures_per_attempt`; a constant until a `[telemetry]` key
/// exists). The attempt's counts include the items past it.
const MAX_EXPOSURES_PER_ATTEMPT: usize = 64;

impl GraphTaskDispatcher {
    /// Record what planning decided for `attempt` (S01 P0-8, P0-9): `plan`'s
    /// route decision, keyed to the attempt (its trace id too) and stamped
    /// with `task`'s id and the time it is written, and one exposure row per
    /// item its prompt retrieved.
    pub(super) fn record_planned_attempt(
        &self,
        attempt: &mut AttemptContext,
        task: &TaskDef,
        plan: &RunnerDispatchPlan,
    ) {
        if let Some(mut decision) = plan.route_decision.clone() {
            let attempt_key = attempt.key.attempt_key();
            decision.trace_id.clone_from(&attempt_key);
            decision.attempt_key = Some(attempt_key);
            decision.task_id.clone_from(&task.id);
            decision.timestamp = chrono::Utc::now().to_rfc3339();
            attempt.record_decision(decision);
        }
        record_exposures(attempt, plan);
    }
}

/// Write one exposure row per item `plan`'s prompt retrieved, up to
/// [`MAX_EXPOSURES_PER_ATTEMPT`], and count the content items it retrieved
/// and included for `attempt`'s verdict. Sections are the prompt's own
/// parts, not retrieved content: they have rows, and the counts leave them
/// out.
fn record_exposures(attempt: &mut AttemptContext, plan: &RunnerDispatchPlan) {
    let items = &plan.prompt.diagnostics.items;
    let mut counts = ExposureCounts::default();
    for (index, item) in items.iter().enumerate() {
        if item.kind != ExposureItemKind::Section {
            counts.retrieved = counts.retrieved.saturating_add(1);
            counts.included = counts.included.saturating_add(u32::from(item.included));
        }
        if index < MAX_EXPOSURES_PER_ATTEMPT {
            attempt.record_exposure(exposure_row(attempt.identity(), item));
        }
    }
    if items.len() > MAX_EXPOSURES_PER_ATTEMPT {
        tracing::debug!(
            attempt_key = %attempt.key,
            items = items.len(),
            written = MAX_EXPOSURES_PER_ATTEMPT,
            "exposure rows capped; the verdict still counts every item"
        );
    }
    attempt.record_exposure_counts(counts);
}

/// The exposure row of `item`, retrieved for the attempt `identity` names.
/// It holds the item's digest, never its text.
fn exposure_row(identity: &AttemptIdentity, item: &PromptItemDiagnostic) -> ExposureRecord {
    let mut row = ExposureRecord::new(identity.clone(), item.kind, &item.id);
    row.rank = item.rank;
    row.score = item.score;
    row.included = item.included;
    row.excluded_reason = item.excluded_reason;
    row.section_id = Some(item.section.clone());
    row.tokens = Some(item.tokens);
    row.rendered_sha256 = Some(item.rendered_sha256.clone()).filter(|digest| !digest.is_empty());
    row
}

#[cfg(test)]
mod tests {
    use roko_core::agent::ModelSpec;
    use roko_fs::layout::RokoLayout;
    use roko_learn::telemetry::report::{LegacyRows, RunRecords, check};
    use roko_learn::telemetry::{ContentDecisionPoint, DecisionSource, ExcludedReason};
    use tempfile::tempdir;

    use super::*;
    use crate::dispatch::{AssembledPrompt, PromptDiagnostics};
    use crate::graph_task_dispatch::tests::{
        VERIFY_PROVIDER, make_spec, make_test_dispatcher, no_auto_fix, verify_step,
    };
    use crate::runtime_feedback::EpisodeSink;

    const RUN: &str = "graph-decision-run";

    /// Write `entries` (id and content) to `workdir`'s knowledge store.
    fn seed_knowledge(workdir: &Path, entries: &[(&str, &str)]) {
        let neuro = workdir.join(".roko/neuro");
        std::fs::create_dir_all(&neuro).expect("create the knowledge store's directory");
        let lines: String = entries
            .iter()
            .map(|(id, content)| {
                let entry = serde_json::json!({
                    "id": id,
                    "content": content,
                    "confidence": 0.8,
                    "created_at": chrono::Utc::now(),
                });
                format!("{entry}\n")
            })
            .collect();
        std::fs::write(neuro.join("knowledge.jsonl"), lines).expect("write the knowledge store");
    }

    /// A knowledge entry a prompt retrieved, at `rank`.
    fn knowledge_item(id: &str, rank: u32, included: bool) -> PromptItemDiagnostic {
        PromptItemDiagnostic {
            kind: ExposureItemKind::Knowledge,
            id: id.to_string(),
            section: "domain_context".to_string(),
            rank: Some(rank),
            score: Some(1.0),
            tokens: 12,
            rendered_sha256: format!("{id}-digest"),
            included,
            excluded_reason: (!included).then_some(ExcludedReason::TokenBudget),
        }
    }

    /// A dispatch plan whose prompt retrieved `items`, with no route
    /// decision.
    fn planned(items: Vec<PromptItemDiagnostic>) -> RunnerDispatchPlan {
        RunnerDispatchPlan {
            model: ModelSpec::from_slug("stream-model"),
            forced: false,
            source: ModelChoiceSource::TaskHint,
            prompt: AssembledPrompt {
                system_prompt: String::new(),
                user_prompt: String::new(),
                tool_allowlist: None,
                diagnostics: PromptDiagnostics {
                    items,
                    ..PromptDiagnostics::default()
                },
            },
            route_decision: None,
        }
    }

    /// The exposure log keeps what a prompt retrieved apart from what it
    /// included (S01 §4.5): two retrieved knowledge entries, one of them cut
    /// by the token budget, are two rows, and the verdict counts 2 and 1.
    #[tokio::test]
    async fn exposure_log_distinguishes_retrieved_from_included() {
        let temp = tempdir().expect("tempdir");
        let runs_dir = temp.path().join(".roko/runs");
        let feedback = GraphFeedbackContext {
            runs_dir: Some(runs_dir.clone()),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, task) =
            make_test_dispatcher(&temp, VERIFY_PROVIDER, no_auto_fix, feedback).await;
        let spec = make_spec(&task);
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        let mut attempt = dispatcher.open_attempt(&spec, &task, &ctx);
        let key = attempt.key.attempt_key();
        let items = vec![
            knowledge_item("kn-1", 1, true),
            knowledge_item("kn-2", 2, false),
        ];
        dispatcher.record_planned_attempt(&mut attempt, &task, &planned(items));
        let passed = Settlement::verified(&Ok(TaskGateVerdict::Passed));
        let settled = attempt.settle(passed, "stream-model", None);
        let counts = ExposureCounts {
            retrieved: 2,
            included: 1,
        };
        assert_eq!(settled.verdict.exposures, Some(counts));
        dispatcher.close_run_attempts(RUN);

        let run = RunRecords::load(&runs_dir.join(RUN)).expect("load the run");
        assert!(run.invalid.is_empty(), "{:?}", run.invalid);
        let rows: Vec<(&str, bool, Option<ExcludedReason>)> = run
            .exposures
            .iter()
            .map(|line| {
                let row = &line.record;
                (row.item_id.as_str(), row.included, row.excluded_reason)
            })
            .collect();
        assert_eq!(
            rows,
            [
                ("kn-1", true, None),
                ("kn-2", false, Some(ExcludedReason::TokenBudget)),
            ]
        );
        for line in &run.exposures {
            let row = &line.record;
            assert_eq!(row.identity.attempt_key, key);
            assert_eq!(row.decision_point, ContentDecisionPoint::Knowledge);
            assert!(row.retrieved, "{row:?}");
            assert_eq!(row.section_id.as_deref(), Some("domain_context"));
            assert!(line.seq < run.verdicts[0].seq, "exposed before settled");
        }
        assert_eq!(run.verdicts[0].record.exposures, Some(counts));
    }

    /// G29: a dispatch whose prompt retrieved a matching knowledge entry logs
    /// the entry as included, and its verdict counts the inclusion.
    #[tokio::test]
    async fn attempt_record_fills_exposures() {
        let temp = tempdir().expect("tempdir");
        let roko = temp.path().join(".roko");
        // The task is "Streaming graph task": the entry shares its words.
        seed_knowledge(
            temp.path(),
            &[("kn-stream", "Streaming graph task output flushes each chunk")],
        );
        let feedback = GraphFeedbackContext {
            runs_dir: Some(roko.join("runs")),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, mut task) =
            make_test_dispatcher(&temp, VERIFY_PROVIDER, no_auto_fix, feedback).await;
        task.verify = vec![verify_step("structural", "true")];
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect("the verified attempt passes");
        drop(dispatcher);
        crate::background_writes::settled(&roko).await;

        let run = RunRecords::load(&roko.join("runs").join(RUN)).expect("load the run");
        assert!(run.invalid.is_empty(), "{:?}", run.invalid);
        let verdict = &run.verdicts[0].record;
        let counts = verdict.exposures.expect("the verdict counts its exposures");
        assert!(counts.included > 0, "{counts:?}");
        let entry = run
            .exposures
            .iter()
            .map(|line| &line.record)
            .find(|row| row.item_id == "kn-stream")
            .expect("the entry's exposure row");
        assert_eq!(entry.item_kind, ExposureItemKind::Knowledge);
        assert!(entry.included, "{entry:?}");
        assert_eq!(entry.identity.attempt_key, verdict.identity.attempt_key);
        assert!(entry.rendered_sha256.is_some(), "{entry:?}");
        // Sections have rows too, and the counts leave them out.
        let sections = run
            .exposures
            .iter()
            .filter(|line| line.record.item_kind == ExposureItemKind::Section)
            .count();
        assert!(sections > 0);
        let content = run.exposures.len() - sections;
        assert_eq!(counts.retrieved as usize, content);
    }

    /// One dispatched attempt writes one route decision row to its run's
    /// `decisions.jsonl`, keyed to the attempt and written before its
    /// verdict, and the run passes `roko learn telemetry check`.
    #[tokio::test]
    async fn graph_route_writes_decision_row() {
        let temp = tempdir().expect("tempdir");
        let roko = temp.path().join(".roko");
        let episodes_path = roko.join("episodes.jsonl");
        let facade = FeedbackFacade::new().with_sink(Arc::new(EpisodeSink::at(&episodes_path)));
        let feedback = GraphFeedbackContext {
            feedback_facade: Some(Arc::new(facade)),
            efficiency_path: Some(roko.join("learn/efficiency.jsonl")),
            costs_path: Some(roko.join("learn/costs.jsonl")),
            runs_dir: Some(roko.join("runs")),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, mut task) =
            make_test_dispatcher(&temp, VERIFY_PROVIDER, no_auto_fix, feedback).await;
        task.verify = vec![verify_step("structural", "true")];
        let spec = make_spec(&task);
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("the verified attempt passes");
        // Closing the run's writer flushes its lines.
        drop(dispatcher);
        crate::background_writes::settled(&roko).await;

        let run = RunRecords::load(&roko.join("runs").join(RUN)).expect("load the run");
        assert_eq!(run.decisions.len(), 1, "one route row per attempt");
        assert_eq!(run.verdicts.len(), 1);
        let (decision, verdict) = (&run.decisions[0], &run.verdicts[0]);
        let key = verdict.record.identity.attempt_key.as_str();
        assert_eq!(decision.record.attempt_key.as_deref(), Some(key));
        assert_eq!(decision.record.trace_id, key);
        assert_eq!(decision.record.task_id, task.id);
        assert!(decision.seq < verdict.seq, "decided before settled");
        // The task's model hint picked its model.
        assert_eq!(decision.record.source, Some(DecisionSource::TaskHint));
        assert_eq!(decision.record.selected_model, "stream-model");
        let total: f64 = decision.record.candidates.iter().filter_map(|c| c.p).sum();
        assert!((total - 1.0).abs() < 1e-9, "candidate p sums to {total}");

        let legacy = LegacyRows::load(&RokoLayout::new(roko), RUN).expect("legacy rows");
        let report = check(&run, &legacy);
        assert_eq!(report.failures(), Vec::<String>::new());
        assert_eq!(report.decisions, 1);
    }
}
