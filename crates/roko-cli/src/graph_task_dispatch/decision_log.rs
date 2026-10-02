//! Decision and exposure records of Graph task dispatch (S01 §4.5, §5.3,
//! §5.4).
//!
//! An attempt that reaches routing writes one route decision row to its
//! run's `decisions.jsonl`: the row [`ModelRouter::decide`] made when
//! dispatch planned the attempt, keyed to the attempt. Its prompt's items go
//! to the run's `exposures.jsonl`, one row per item the prompt retrieved,
//! included or not, and the attempt's verdict counts them. Each content
//! decision point the prompt retrieved items at (knowledge, playbooks,
//! sections, error patterns) adds one content decision row, with digests of
//! the learned state it chose from (P0-10). A T0 reflex attempt and a
//! harness failure before planning write none.
//!
//! [`ModelRouter::decide`]: crate::dispatch::ModelRouter::decide

use std::collections::BTreeMap;
use std::sync::LazyLock;
use std::time::SystemTime;

use roko_learn::routing_log::DecisionState;
use roko_learn::telemetry::records::b3_digest;
use roko_learn::telemetry::{
    AttemptIdentity, ContentCandidate, ContentDecisionPoint, ContentDecisionRecord, DecisionSource,
    ExcludedReason, ExposureCounts, ExposureItemKind, ExposureRecord,
};

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
    /// with `task`'s id and the time it is written, one exposure row per item
    /// its prompt retrieved, one content decision per decision point, and an
    /// access to each knowledge entry it included.
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
        self.record_content_decisions(attempt, plan);
        self.record_knowledge_access(plan);
    }

    /// Count an access to each knowledge entry `plan`'s prompt included (S01
    /// P0-9), off the reactor, unless learning is frozen: access counts are
    /// the store's evidence that a prompt used an entry. Cited episodes are
    /// not knowledge entries. A failed count is logged, and the attempt goes
    /// on.
    fn record_knowledge_access(&self, plan: &RunnerDispatchPlan) {
        let included: Vec<String> = plan
            .prompt
            .diagnostics
            .items
            .iter()
            .filter(|item| item.kind == ExposureItemKind::Knowledge && item.included)
            .map(|item| item.id.clone())
            .collect();
        // A frozen run counts no access (decision 2218).
        if included.is_empty() || self.learning_frozen() {
            return;
        }
        let store = roko_neuro::KnowledgeStore::for_workdir(&self.workdir);
        let path = store.path().to_path_buf();
        crate::background_writes::spawn(&path, async move {
            let counted = tokio::task::spawn_blocking(move || {
                let ids: Vec<&str> = included.iter().map(String::as_str).collect();
                store.count_access(&ids)
            })
            .await;
            match counted {
                Ok(Ok(_)) => {}
                Ok(Err(error)) => {
                    tracing::warn!(%error, "knowledge access count failed (best-effort)");
                }
                Err(error) => {
                    tracing::warn!(%error, "knowledge access count task failed (best-effort)");
                }
            }
        });
    }

    /// One content decision per decision point at which `plan`'s prompt
    /// retrieved an item (S01 §4.5): the retrieved items are the candidates,
    /// the included ones the choice, made by a fixed ranking. Each row
    /// carries the digests of the learned state the candidates came from.
    fn record_content_decisions(&self, attempt: &AttemptContext, plan: &RunnerDispatchPlan) {
        let mut points: BTreeMap<ContentDecisionPoint, Vec<&PromptItemDiagnostic>> =
            BTreeMap::new();
        for item in &plan.prompt.diagnostics.items {
            points
                .entry(item.kind.decision_point())
                .or_default()
                .push(item);
        }
        if points.is_empty() {
            return;
        }
        let state = self.learned_state();
        for (point, items) in points {
            let decision = content_decision(attempt.identity(), point, &items, &state);
            attempt.record_content_decision(decision);
        }
    }

    /// The learned state the prompt's content decisions read (S01 P0-10):
    /// the knowledge store and the playbooks the prompt cache loads from this
    /// dispatcher's workdir, and the gate thresholds in force.
    fn learned_state(&self) -> LearnedState {
        let roko = self.workdir.join(".roko");
        let thresholds = self.feedback.gate_thresholds_path.as_deref();
        LearnedState {
            knowledge: store_state(&roko.join("neuro"), ".jsonl", "kn", Some(KNOWLEDGE_FILE)),
            playbooks: store_state(&roko.join("learn").join("playbooks"), ".json", "pb", None),
            thresholds: thresholds.and_then(digest_file).map(|file| file.digest),
        }
    }
}

/// The knowledge store's entries: one per line.
const KNOWLEDGE_FILE: &str = "knowledge.jsonl";

/// The learned state an attempt's content decisions were made from.
struct LearnedState {
    knowledge: DecisionState,
    playbooks: DecisionState,
    thresholds: Option<String>,
}

/// The ranking that chooses a content decision point's candidates.
const fn content_policy(point: ContentDecisionPoint) -> &'static str {
    match point {
        // The three entries holding the most task keywords; episodes join
        // them at this decision point.
        ContentDecisionPoint::Knowledge => "keyword_overlap_top3",
        // The three playbooks holding the most task keywords, then the best
        // record.
        ContentDecisionPoint::Playbooks => "keyword_outcome_top3",
        // The sections that fit the prompt's token budget.
        ContentDecisionPoint::Sections => "token_budget_composer",
        // The store's five leading patterns, in a bounded summary.
        ContentDecisionPoint::ErrorPatterns => "error_pattern_summary_top5",
        ContentDecisionPoint::Reflections | ContentDecisionPoint::DreamAdvice => "unranked",
    }
}

/// The content decision at `point`, whose candidates are `items`, made from
/// `state`. An item the role's prompt has no place for was never eligible.
fn content_decision(
    identity: &AttemptIdentity,
    point: ContentDecisionPoint,
    items: &[&PromptItemDiagnostic],
    state: &LearnedState,
) -> ContentDecisionRecord {
    let candidates = items
        .iter()
        .map(|item| ContentCandidate {
            id: item.id.clone(),
            rank: item.rank,
            score: item.score,
            eligible: item.excluded_reason != Some(ExcludedReason::RoleFilter),
            p: Some(if item.included { 1.0 } else { 0.0 }),
        })
        .collect();
    let chosen = items
        .iter()
        .filter(|item| item.included)
        .map(|item| item.id.clone())
        .collect();
    let read = match point {
        ContentDecisionPoint::Knowledge => Some(state.knowledge.clone()),
        ContentDecisionPoint::Playbooks => Some(state.playbooks.clone()),
        _ => None,
    };
    ContentDecisionRecord {
        identity: identity.clone(),
        decision_point: point,
        policy: content_policy(point).to_string(),
        candidates,
        chosen,
        // A fixed ranking chooses its set with certainty.
        chosen_propensity: Some(1.0),
        source: Some(DecisionSource::Default),
        state: read,
        thresholds_digest: state.thresholds.clone(),
    }
}

/// A file as it was when it was last digested.
#[derive(Debug, Clone)]
struct DigestedFile {
    len: u64,
    modified: SystemTime,
    digest: String,
    /// Its non-blank lines.
    lines: u64,
}

/// Every file a content decision digested, so a file is read again only
/// once its length or modification time changes: dispatch never rereads an
/// unchanged knowledge store.
static DIGESTED_FILES: LazyLock<parking_lot::Mutex<HashMap<PathBuf, DigestedFile>>> =
    LazyLock::new(Default::default);

/// `path`'s `b3:` digest and line count, from [`DIGESTED_FILES`] while its
/// length and modification time are unchanged; `None` when it cannot be
/// read.
fn digest_file(path: &Path) -> Option<DigestedFile> {
    let metadata = std::fs::metadata(path).ok()?;
    let (len, modified) = (metadata.len(), metadata.modified().ok()?);
    if let Some(seen) = DIGESTED_FILES.lock().get(path)
        && seen.len == len
        && seen.modified == modified
    {
        return Some(seen.clone());
    }
    let bytes = std::fs::read(path).ok()?;
    let lines = bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.iter().all(u8::is_ascii_whitespace))
        .count();
    let file = DigestedFile {
        len,
        modified,
        digest: b3_digest(&bytes),
        lines: u64::try_from(lines).unwrap_or(u64::MAX),
    };
    DIGESTED_FILES
        .lock()
        .insert(path.to_path_buf(), file.clone());
    Some(file)
}

/// The learned state of the store in `dir` (S01 P0-10): a `b3:` digest over
/// its files named `*{extension}`, in sorted path order (each file's name and
/// digest), labelled `{label}:n={n}`. `n` counts the lines of `counted`, or
/// the files when there is none to count. A missing store holds nothing.
fn store_state(dir: &Path, extension: &str, label: &str, counted: Option<&str>) -> DecisionState {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(extension))
        .collect();
    names.sort();
    let mut listing = String::new();
    let mut n_obs = 0;
    let mut newest: Option<SystemTime> = None;
    for name in &names {
        let Some(file) = digest_file(&dir.join(name)) else {
            continue;
        };
        listing.push_str(&format!("{name}\0{}\n", file.digest));
        n_obs += match counted {
            Some(target) if name.as_str() == target => file.lines,
            Some(_) => 0,
            None => 1,
        };
        newest = newest.max(Some(file.modified));
    }
    DecisionState {
        read: n_obs > 0,
        version: format!("{label}:n={n_obs}"),
        digest: b3_digest(listing.as_bytes()),
        age_s: newest
            .and_then(|modified| modified.elapsed().ok())
            .map(|age| age.as_secs()),
        n_obs,
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

    /// A playbook a prompt retrieved and included.
    fn playbook_item(id: &str) -> PromptItemDiagnostic {
        PromptItemDiagnostic {
            kind: ExposureItemKind::Playbook,
            ..knowledge_item(id, 1, true)
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

    /// One content decision per decision point a prompt retrieved items at
    /// (S01 §4.5): two retrieved knowledge entries, one included, are one
    /// knowledge row with both as candidates and the included one chosen,
    /// read from a store of two entries. Rows carry the digests of the state
    /// they chose from, and a changed playbook changes the playbooks' digest.
    #[tokio::test]
    async fn content_decisions_list_retrieved_and_included_ids() {
        let temp = tempdir().expect("tempdir");
        let runs_dir = temp.path().join(".roko/runs");
        seed_knowledge(
            temp.path(),
            &[("kn-1", "first entry"), ("kn-2", "second entry")],
        );
        let learn = temp.path().join(".roko/learn");
        std::fs::create_dir_all(learn.join("playbooks")).expect("create the playbook directory");
        let playbook = learn.join("playbooks/pb-1.json");
        std::fs::write(&playbook, r#"{"id":"pb-1"}"#).expect("write a playbook");
        let thresholds = learn.join("gate-thresholds.json");
        std::fs::write(&thresholds, "{}").expect("write the gate thresholds");
        let feedback = GraphFeedbackContext {
            runs_dir: Some(runs_dir.clone()),
            gate_thresholds_path: Some(thresholds),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, task) =
            make_test_dispatcher(&temp, VERIFY_PROVIDER, no_auto_fix, feedback).await;
        let spec = make_spec(&task);
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        let plan = planned(vec![
            knowledge_item("kn-1", 1, true),
            knowledge_item("kn-2", 2, false),
            playbook_item("pb-1"),
        ]);
        let mut first = dispatcher.open_attempt(&spec, &task, &ctx);
        dispatcher.record_planned_attempt(&mut first, &task, &plan);
        std::fs::write(&playbook, r#"{"id":"pb-1","goal":"changed"}"#).expect("change it");
        let mut second = dispatcher.open_attempt(&spec, &task, &ctx);
        dispatcher.record_planned_attempt(&mut second, &task, &plan);
        for attempt in [first, second] {
            let passed = Settlement::verified(&Ok(TaskGateVerdict::Passed));
            attempt.settle(passed, "stream-model", None);
        }
        dispatcher.close_run_attempts(RUN);
        // The included entry's access counts land in the background.
        crate::background_writes::settled(&temp.path().join(".roko")).await;

        let run = RunRecords::load(&runs_dir.join(RUN)).expect("load the run");
        assert!(run.invalid.is_empty(), "{:?}", run.invalid);
        assert!(run.decisions.is_empty(), "no route was decided");
        let rows = |point: ContentDecisionPoint| -> Vec<&ContentDecisionRecord> {
            run.content_decisions
                .iter()
                .map(|line| &line.record)
                .filter(|row| row.decision_point == point)
                .collect()
        };
        let knowledge = rows(ContentDecisionPoint::Knowledge);
        assert_eq!(knowledge.len(), 2, "one row per attempt");
        let row = knowledge[0];
        let candidates: Vec<(&str, Option<f64>)> = row
            .candidates
            .iter()
            .map(|candidate| (candidate.id.as_str(), candidate.p))
            .collect();
        assert_eq!(candidates, [("kn-1", Some(1.0)), ("kn-2", Some(0.0))]);
        assert_eq!(row.chosen, ["kn-1"]);
        assert_eq!(row.chosen_propensity, Some(1.0));
        assert_eq!(row.source, Some(DecisionSource::Default));
        assert_eq!(row.policy, "keyword_overlap_top3");
        let store = row.state.as_ref().expect("the knowledge store's state");
        assert_eq!((store.n_obs, store.version.as_str()), (2, "kn:n=2"));
        assert!(store.read && store.digest.starts_with("b3:"), "{store:?}");
        assert_eq!(row.thresholds_digest, Some(b3_digest(b"{}")));
        let entries = knowledge[1].state.as_ref().map(|state| state.n_obs);
        assert_eq!(entries, Some(2), "the second attempt read the same store");

        let playbooks = rows(ContentDecisionPoint::Playbooks);
        let digests: Vec<&str> = playbooks
            .iter()
            .filter_map(|row| row.state.as_ref())
            .map(|state| state.digest.as_str())
            .collect();
        assert_eq!(digests.len(), 2, "{playbooks:?}");
        assert_ne!(digests[0], digests[1], "the playbook changed");
        assert_eq!(playbooks[0].chosen, ["pb-1"]);
    }

    /// The store counts an access to each knowledge entry a prompt included
    /// (S01 P0-9): an included entry's count is 1, a dropped one's stays 0,
    /// and a cited episode is no knowledge entry, whatever its id.
    #[tokio::test]
    async fn included_knowledge_records_access() {
        let temp = tempdir().expect("tempdir");
        let roko = temp.path().join(".roko");
        seed_knowledge(
            temp.path(),
            &[
                ("kn-1", "first entry"),
                ("kn-2", "second entry"),
                ("ep-1", "an entry an episode's id names"),
            ],
        );
        let feedback = GraphFeedbackContext {
            runs_dir: Some(roko.join("runs")),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, task) =
            make_test_dispatcher(&temp, VERIFY_PROVIDER, no_auto_fix, feedback).await;
        let spec = make_spec(&task);
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        let episode = PromptItemDiagnostic {
            kind: ExposureItemKind::Episode,
            ..knowledge_item("ep-1", 3, true)
        };
        let plan = planned(vec![
            knowledge_item("kn-1", 1, true),
            knowledge_item("kn-2", 2, false),
            episode,
        ]);
        let mut attempt = dispatcher.open_attempt(&spec, &task, &ctx);
        dispatcher.record_planned_attempt(&mut attempt, &task, &plan);
        crate::background_writes::settled(&roko).await;

        let store = roko_neuro::KnowledgeStore::for_workdir(temp.path());
        let entries = store.read_all().expect("read the knowledge store");
        let accesses: Vec<(&str, u64)> = entries
            .iter()
            .map(|entry| (entry.id.as_str(), entry.access_count))
            .collect();
        assert_eq!(accesses, [("kn-1", 1), ("kn-2", 0), ("ep-1", 0)]);
        assert!(entries[0].last_accessed.is_some());
        let passed = Settlement::verified(&Ok(TaskGateVerdict::Passed));
        attempt.settle(passed, "stream-model", None);
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
            &[(
                "kn-stream",
                "Streaming graph task output flushes each chunk",
            )],
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
