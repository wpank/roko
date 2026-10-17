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
//! the learned state it chose from (P0-10); so does a knowledge or playbook
//! reader that had an opportunity and loaded nothing to show, whose row
//! marks the read cut or stale (gap-a13544). A T0 reflex attempt and a
//! harness failure before planning write none.
//!
//! The knowledge, playbook and sections rows also carry S03's fields
//! (A-DEC, backlog 5125; gap-6f31c3 for the sections): L-know's, L-play's or
//! L-sec's layer and the chain's draw on it, both proposals on both arms,
//! and a receipt that binds the included items' rendered digests to the
//! assembled request ([`content_audit`]).
//!
//! [`ModelRouter::decide`]: crate::dispatch::ModelRouter::decide

use std::collections::BTreeMap;
use std::sync::LazyLock;
use std::time::SystemTime;

use roko_learn::loop_audit::arm_set::{ArmSet, MAXIMIZE_CONDITION};
use roko_learn::loop_audit::faults::{self, FaultKind};
use roko_learn::routing_log::DecisionState;
use roko_learn::section_effect::{SECTION_BANDIT_PATH, SectionBandit, SectionDecision};
use roko_learn::telemetry::records::{
    AuditFields, ContentProposals, DecisionAssignment, DecisionOpportunity, DecisionReceipt,
    b3_digest,
};
use roko_learn::telemetry::{
    AttemptIdentity, AttemptTiming, ContentCandidate, ContentDecisionPoint, ContentDecisionRecord,
    DecisionSource, ExcludedReason, ExposureCounts, ExposureItemKind, ExposureRecord,
};

use super::attempt::AttemptContext;
use super::prompt_experiment::dispatch_prompt_hash;
use super::verification::error_pattern_limit;
use super::*;
use crate::dispatch::RunnerDispatchPlan;
use crate::dispatch::prompt_builder::{PromptItemDiagnostic, ReaderRead};

/// `decision`, the route row of `attempt`, as the
/// [`DashboardEvent::RouteDecided`](roko_core::DashboardEvent::RouteDecided)
/// a host follows: the pick, the stage, source and reason, and its
/// candidates, the pick first and the rest by score. A pick by rung or by
/// hint can score below the models it passed over, and must not fall out of
/// the list. It says where the attempt runs on the ladder and why (`reason`),
/// and carries the self-model's forecast of it, when there is one
/// (gap-aaaaae).
pub(super) fn route_event(
    decision: &roko_learn::routing_log::RoutingDecisionLog,
    attempt: &AttemptContext,
    reason: Option<super::ladder::TryReason>,
) -> roko_core::DashboardEvent {
    let (rung, ladder_step) = attempt.ladder_place();
    let mut candidates: Vec<&roko_learn::routing_log::CandidateEntry> =
        decision.candidates.iter().collect();
    let picked = |candidate: &roko_learn::routing_log::CandidateEntry| {
        candidate.model == decision.selected_model
    };
    candidates.sort_by(|a, b| {
        picked(b)
            .cmp(&picked(a))
            .then_with(|| b.score.total_cmp(&a.score))
    });
    let source = decision
        .source
        .and_then(|source| serde_json::to_value(source).ok())
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default();
    roko_core::DashboardEvent::RouteDecided {
        plan_id: attempt.key.plan_id.clone(),
        task_id: decision.task_id.clone(),
        attempt: attempt.key.attempt,
        model: decision.selected_model.clone(),
        provider: decision.selected_provider.clone(),
        stage: decision.routing_stage.clone(),
        reason: decision.routing_reason.clone(),
        source,
        candidates: candidates
            .into_iter()
            .take(roko_core::dashboard_snapshot::ROUTE_EVENT_CANDIDATES)
            .map(|candidate| roko_core::RouteCandidate {
                model: candidate.model.clone(),
                provider: candidate.provider.clone(),
                score: candidate.score,
                p: candidate.p,
                eligible: candidate.eligible,
                ineligible_reason: candidate.ineligible_reason.clone(),
            })
            .collect(),
        ladder: Box::new(roko_core::dashboard_snapshot::RouteLadder {
            rung,
            ladder_step,
            ladder_reason: reason.map_or_else(String::new, |reason| reason.as_str().to_string()),
            forecast: attempt.forecast().cloned(),
        }),
    }
}

/// The self-model's `prediction` of an attempt, as its route event carries
/// it (gap-aaaaae): its mode and action, the model it would choose, and per
/// candidate the model, provider, P(verified success) and median cost, the
/// likeliest first. The 90th-percentile cost stays in `predictions.jsonl`:
/// for an arm with no history it is far from anything a run spends.
pub(super) fn route_forecast(
    prediction: &roko_learn::telemetry::records::AttemptPredictionRecord,
) -> roko_core::dashboard_snapshot::RouteForecast {
    let arm = |key: &str| key.parse::<roko_learn::self_model::ArmKey>().ok();
    let mut candidates: Vec<roko_core::dashboard_snapshot::ForecastCandidate> = prediction
        .candidates
        .iter()
        .map(|candidate| {
            let (model, provider) = arm(&candidate.arm).map_or_else(
                || (candidate.arm.clone(), String::new()),
                |arm| (arm.model, arm.provider),
            );
            roko_core::dashboard_snapshot::ForecastCandidate {
                model,
                provider,
                p_vs: candidate.p_vs,
                cost_q50: candidate.cost_q50,
            }
        })
        .collect();
    candidates.sort_by(|a, b| b.p_vs.total_cmp(&a.p_vs));
    candidates.truncate(roko_core::dashboard_snapshot::ROUTE_EVENT_CANDIDATES);
    roko_core::dashboard_snapshot::RouteForecast {
        mode: prediction.predictor.mode.clone(),
        action: prediction.decision.action.clone(),
        would_choose: prediction
            .decision
            .would_choose
            .as_deref()
            .map(|key| arm(key).map_or_else(|| key.to_string(), |arm| arm.model)),
        candidates,
    }
}

/// What the prompt of attempt `attempt` of `task_id` carried, from its
/// `diagnostics`, as the [`DashboardEvent::PromptAssembled`] a host follows
/// (gap-5f6d0b): the knowledge entries, playbooks and episodes that reached
/// it, each retrieved one that did not with why, and its sections.
///
/// [`DashboardEvent::PromptAssembled`]: roko_core::DashboardEvent::PromptAssembled
pub(super) fn prompt_event(
    diagnostics: &crate::dispatch::PromptDiagnostics,
    plan_id: &str,
    task_id: &str,
    attempt: u32,
) -> roko_core::DashboardEvent {
    let carried = |kind: ExposureItemKind| {
        diagnostics
            .items
            .iter()
            .filter(|item| item.kind == kind && item.included)
            .map(|item| item.id.clone())
            .collect::<Vec<_>>()
    };
    let left_out = diagnostics
        .items
        .iter()
        .filter(|item| {
            !item.included
                && matches!(
                    item.kind,
                    ExposureItemKind::Knowledge
                        | ExposureItemKind::Playbook
                        | ExposureItemKind::Episode
                        | ExposureItemKind::Section
                )
        })
        .map(|item| roko_core::PromptLeftOut {
            kind: item.kind.as_str().to_string(),
            id: item.id.clone(),
            reason: item
                .excluded_reason
                .and_then(|reason| serde_json::to_value(reason).ok())
                .and_then(|reason| reason.as_str().map(str::to_string))
                .unwrap_or_default(),
        })
        .collect::<Vec<_>>();
    let dropped_sections = left_out
        .iter()
        .filter(|item| item.kind == ExposureItemKind::Section.as_str())
        .map(|item| item.id.clone())
        .collect();
    roko_core::DashboardEvent::PromptAssembled {
        plan_id: plan_id.to_string(),
        task_id: task_id.to_string(),
        attempt,
        knowledge_ids: carried(ExposureItemKind::Knowledge),
        playbook_ids: carried(ExposureItemKind::Playbook),
        episode_ids: carried(ExposureItemKind::Episode),
        left_out,
        included_sections: diagnostics.included_sections.clone(),
        dropped_sections,
        estimated_tokens: diagnostics.estimated_tokens,
    }
}

/// Most exposure rows one attempt writes (S01 §5.9
/// `max_exposures_per_attempt`; a constant until a `[telemetry]` key
/// exists). The attempt's counts include the items past it.
const MAX_EXPOSURES_PER_ATTEMPT: usize = 64;

impl GraphTaskDispatcher {
    /// Record what planning decided for `attempt` (S01 P0-8, P0-9): `plan`'s
    /// route decision, keyed to the attempt (its trace id too) and stamped
    /// with `task`'s id and the time it is written, one exposure row per item
    /// its prompt retrieved, one content decision per decision point, and an
    /// access to each knowledge entry it included. DP4's own count of the
    /// exclusions its routing made goes on the attempt's verdict, apart from
    /// the route row (gap-595e28).
    pub(super) fn record_planned_attempt(
        &self,
        attempt: &mut AttemptContext,
        task: &TaskDef,
        plan: &RunnerDispatchPlan,
    ) {
        attempt.record_trust_exclusions(plan.trust_exclusions);
        if let Some(mut decision) = plan.route_decision.clone() {
            let attempt_key = attempt.key.attempt_key();
            decision.trace_id.clone_from(&attempt_key);
            decision.attempt_key = Some(attempt_key);
            decision.task_id.clone_from(&task.id);
            decision.timestamp = chrono::Utc::now().to_rfc3339();
            // The decision as it happens, for whoever follows the run
            // (gap-2a3298); the attempt's `agent_spawned` follows it.
            if let Some(tui) = &self.tui_bridge {
                let reason = self.routed_try_reason(&attempt.key.attempt_key());
                tui.publish_event(route_event(&decision, attempt, reason));
            }
            attempt.record_decision(decision);
        }
        // What its prompt carried and left out, for the same host
        // (gap-5f6d0b).
        if let Some(tui) = &self.tui_bridge {
            tui.publish_event(prompt_event(
                &plan.prompt.diagnostics,
                &attempt.key.plan_id,
                &task.id,
                attempt.key.attempt,
            ));
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
        let included = included_knowledge_ids(plan);
        // A frozen run counts no access (decision 2218).
        if included.is_empty() || self.learning_frozen() {
            return;
        }
        // A store of its own, which shares the file's write gate with every
        // other store of the file (bug-c4f0ed): the count and the knowledge
        // lifecycle's rewrites never lose each other's update.
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

    /// One content decision per decision point of `plan`'s prompt
    /// ([`decision_points`]; S01 §4.5): the retrieved items are the
    /// candidates, the included ones the choice, made by a fixed ranking.
    /// Each row carries the digests of the learned state the candidates came
    /// from, and the error-pattern row's policy names how many patterns the
    /// θ the attempt runs shows (gap-26c055).
    fn record_content_decisions(&self, attempt: &AttemptContext, plan: &RunnerDispatchPlan) {
        if decision_points(plan).is_empty() {
            return;
        }
        let state = self.learned_state();
        let times = decision_times(attempt.timing());
        let arm_set = attempt.arm_set();
        let shown = error_pattern_limit(attempt.harness_params());
        let identity = attempt.identity();
        let decisions = content_decisions(plan, identity, &state, arm_set.as_deref(), times, shown);
        for decision in decisions {
            attempt.record_content_decision(decision);
        }
    }

    /// The learned state the prompt's content decisions read (S01 P0-10):
    /// the knowledge store, the playbooks and the section bandit the prompt
    /// cache loads from this dispatcher's workdir, and the gate thresholds
    /// in force. A plan run's prompts are built from one prompt-cache
    /// snapshot, so its decisions name that snapshot, not the stores as they
    /// are now (backlog 4214).
    fn learned_state(&self) -> LearnedState {
        let roko = self.workdir.join(".roko");
        let thresholds = self.feedback.gate_thresholds_path.as_deref();
        let (knowledge, playbooks, sections) = match self.factory.prompt_snapshot() {
            Some(snapshot) => (
                snapshot_state(&snapshot.knowledge, "kn"),
                snapshot_state(&snapshot.playbooks, "pb"),
                snapshot_state(&snapshot.sections, "sb"),
            ),
            None => (
                store_state(&roko.join("neuro"), ".jsonl", "kn", Some(KNOWLEDGE_FILE)),
                store_state(&roko.join("learn").join("playbooks"), ".json", "pb", None),
                bandit_state(&self.workdir.join(SECTION_BANDIT_PATH)),
            ),
        };
        LearnedState {
            knowledge,
            playbooks,
            sections,
            thresholds: thresholds.and_then(digest_file).map(|file| file.digest),
        }
    }
}

/// The knowledge entries `plan`'s prompt included (S01 P0-9): those it
/// retrieved that reached the prompt, never a cited episode. The store counts
/// an access to each, and the attempt's `model_call` row names them
/// (bug-eaa318).
pub(super) fn included_knowledge_ids(plan: &RunnerDispatchPlan) -> Vec<String> {
    plan.prompt
        .diagnostics
        .items
        .iter()
        .filter(|item| item.kind == ExposureItemKind::Knowledge && item.included)
        .map(|item| item.id.clone())
        .collect()
}

/// The knowledge store's entries: one per line.
const KNOWLEDGE_FILE: &str = "knowledge.jsonl";

/// The learned state an attempt's content decisions were made from.
struct LearnedState {
    knowledge: DecisionState,
    playbooks: DecisionState,
    /// The section bandit's posteriors (L-sec).
    sections: DecisionState,
    thresholds: Option<String>,
}

/// The section bandit's draws on one prompt (S02 L9): those the prompt
/// applied, on the chain's learned arm, and those it only proposed, on the
/// arm that runs π⁰ ([`PromptDiagnostics::section_proposals`]). At most one
/// of the two holds draws.
///
/// [`PromptDiagnostics::section_proposals`]: crate::dispatch::PromptDiagnostics::section_proposals
#[derive(Clone, Copy)]
pub(super) struct BanditDraws<'a> {
    applied: &'a [SectionDecision],
    proposed: &'a [SectionDecision],
}

/// The ranking that chooses a content decision point's candidates, as the
/// prompt builder runs it (gap-a40021). The error-pattern policy names as
/// many patterns as the attempt's prompt shows at most, `shown`: the
/// `error_patterns_k` of the θ it runs (M1's B4), which a fixed number would
/// misstate (gap-26c055).
fn content_policy(point: ContentDecisionPoint, shown: usize) -> String {
    match point {
        // The three entries sharing the most of the task's topic terms, two
        // at least, at a confidence of 0.3 or more; episodes join them at
        // this decision point (backlogs 4211, 4213).
        ContentDecisionPoint::Knowledge => "topic_overlap_top3".to_string(),
        // The three playbooks sharing the most topic terms, two at least,
        // then the best record (backlog 4212).
        ContentDecisionPoint::Playbooks => "topic_overlap_outcome_top3".to_string(),
        // The sections that fit the prompt's token budget.
        ContentDecisionPoint::Sections => "token_budget_composer".to_string(),
        // The patterns seen on the task or failing one of its verify
        // commands, in a bounded summary (backlogs 4209, 4210).
        ContentDecisionPoint::ErrorPatterns => format!("task_or_command_keyed_top{shown}"),
        ContentDecisionPoint::Reflections | ContentDecisionPoint::DreamAdvice => {
            "unranked".to_string()
        }
    }
}

/// The policy of a sections decision the section bandit drew at (S02 L9):
/// its draws, then the token budget.
const SECTION_BANDIT_POLICY: &str = "section_bandit_token_budget";

/// The items `plan`'s prompt retrieved, by content decision point, with a
/// point whose reader had an opportunity but loaded nothing to show, a cut
/// or stale read, holding none (gap-a13544).
fn decision_points(
    plan: &RunnerDispatchPlan,
) -> BTreeMap<ContentDecisionPoint, Vec<&PromptItemDiagnostic>> {
    let mut points: BTreeMap<ContentDecisionPoint, Vec<&PromptItemDiagnostic>> = BTreeMap::new();
    for item in &plan.prompt.diagnostics.items {
        points
            .entry(item.kind.decision_point())
            .or_default()
            .push(item);
    }
    for read in &plan.prompt.diagnostics.reads {
        if read.opportunity {
            points.entry(read.point).or_default();
        }
    }
    points
}

/// The content decision rows of `plan`, one per point of
/// [`decision_points`], made from `state` for the attempt `identity`, whose
/// chain drew `arm_set`, at `times` (assigned, decided); `shown` is the most
/// error patterns its prompt shows. Each row's S03 fields read the loops'
/// fault flags as the attempt's decision, as its prompt did (bug-9d23ed).
fn content_decisions(
    plan: &RunnerDispatchPlan,
    identity: &AttemptIdentity,
    state: &LearnedState,
    arm_set: Option<&ArmSet>,
    times: (i64, i64),
    shown: usize,
) -> Vec<ContentDecisionRecord> {
    let diagnostics = &plan.prompt.diagnostics;
    let draws = &diagnostics.section_decisions;
    let bandit = BanditDraws {
        applied: draws,
        proposed: &diagnostics.section_proposals,
    };
    let prompt = &plan.prompt;
    let request_hash = dispatch_prompt_hash(&prompt.system_prompt, &prompt.user_prompt);
    let mut decisions = Vec::new();
    for (point, items) in decision_points(plan) {
        let read = diagnostics.reads.iter().find(|read| read.point == point);
        let mut decision = content_decision(identity, point, &items, read, state, draws, shown);
        let draw = (arm_set, times);
        let audited = faults::decision(&identity.attempt_key, || {
            content_audit(point, &items, read, identity, draw, &request_hash, bandit)
        });
        if let Some((proposals, audit)) = audited {
            decision.proposals = Some(proposals);
            decision.audit = audit;
        }
        if point == ContentDecisionPoint::Knowledge && diagnostics.knowledge_section_off {
            controller_decided(&mut decision);
        }
        decisions.push(decision);
    }
    decisions
}

/// The opportunity reason of a knowledge row whose prompt ran a θ with the
/// knowledge section off (M1's B4, gap-6bb6d8).
const KNOWLEDGE_SECTION_OFF: &str = "knowledge_section_off";

/// Mark `decision`, the knowledge row of a prompt whose θ had the knowledge
/// section off (M1's B4, gap-6bb6d8), as the controller's: θ chose what
/// reached the prompt, so the row names the controller as its source, and it
/// is no opportunity of L-know, so the census credits neither of L-know's
/// arms, its holdout included, with what θ did. Whatever arm L-know drew, a
/// chain whose θ turns the section off drops out alike. The proposals stay
/// the reader's: what it would have shown, and π⁰'s none.
fn controller_decided(decision: &mut ContentDecisionRecord) {
    decision.source = Some(DecisionSource::Control);
    decision.audit.opportunity = Some(DecisionOpportunity {
        eligible: false,
        reason: KNOWLEDGE_SECTION_OFF.to_string(),
    });
}

/// The content decision rows dispatch writes for `plan`, the plan of the
/// attempt `identity` in the workspace `workdir`, whose chain drew
/// `arm_set`, at `times` (assigned, decided): the rows of a planned
/// attempt, made from the learned state of the workspace's stores, each
/// with the chain's arms. Pure but for reading the stores, for E1 (5130;
/// gap-a13544).
pub fn planned_content_decisions(
    workdir: &Path,
    plan: &RunnerDispatchPlan,
    identity: &AttemptIdentity,
    arm_set: Option<&ArmSet>,
    times: (i64, i64),
) -> Vec<ContentDecisionRecord> {
    let roko = workdir.join(".roko");
    let state = LearnedState {
        knowledge: store_state(&roko.join("neuro"), ".jsonl", "kn", Some(KNOWLEDGE_FILE)),
        playbooks: store_state(&roko.join("learn").join("playbooks"), ".json", "pb", None),
        sections: bandit_state(&workdir.join(SECTION_BANDIT_PATH)),
        thresholds: None,
    };
    let shown = error_pattern_limit(None);
    let mut decisions = content_decisions(plan, identity, &state, arm_set, times, shown);
    for decision in &mut decisions {
        decision.arm_set = arm_set.cloned();
    }
    decisions
}

/// `state`, the store's learned state at the decision, as its reader `read`
/// loaded it (gap-a13544): not read when the reader loaded none of it, a
/// cut, and stale when it loaded an older part of it.
fn reader_state(mut state: DecisionState, read: Option<&ReaderRead>) -> DecisionState {
    if let Some(read) = read {
        state.read &= read.loaded > 0;
        state.stale = read.loaded > 0 && read.loaded < read.available;
    }
    state
}

/// The content decision at `point`, whose candidates are `items`, made from
/// `state` as its reader `read` loaded it. An item the role's prompt has no
/// place for was never eligible. At the sections point, `draws` are the
/// section bandit's: the row gives each drawn section its odds of staying
/// in, the propensity of the bandit's draws, and the draws themselves,
/// which `--srm` checks (bug-2410e1). `shown` is the most error patterns
/// the prompt shows.
fn content_decision(
    identity: &AttemptIdentity,
    point: ContentDecisionPoint,
    items: &[&PromptItemDiagnostic],
    read: Option<&ReaderRead>,
    state: &LearnedState,
    draws: &[SectionDecision],
    shown: usize,
) -> ContentDecisionRecord {
    let draws: &[SectionDecision] = if point == ContentDecisionPoint::Sections {
        draws
    } else {
        &[]
    };
    let candidates = items
        .iter()
        .map(|item| ContentCandidate {
            id: item.id.clone(),
            rank: item.rank,
            score: item.score,
            eligible: item.excluded_reason != Some(ExcludedReason::RoleFilter),
            p: Some(inclusion_probability(item, draws)),
        })
        .collect();
    let chosen = items
        .iter()
        .filter(|item| item.included)
        .map(|item| item.id.clone())
        .collect();
    let learned = match point {
        ContentDecisionPoint::Knowledge => Some(state.knowledge.clone()),
        ContentDecisionPoint::Playbooks => Some(state.playbooks.clone()),
        ContentDecisionPoint::Sections => Some(state.sections.clone()),
        _ => None,
    };
    let (policy, chosen_propensity, source) = if draws.is_empty() {
        // A fixed ranking chooses its set with certainty.
        (content_policy(point, shown), 1.0, DecisionSource::Default)
    } else {
        // Given the bandit's draws, the token budget's cut is fixed.
        let propensity: f64 = draws.iter().map(|draw| draw.propensity).product();
        (
            SECTION_BANDIT_POLICY.to_string(),
            propensity,
            DecisionSource::Explore,
        )
    };
    ContentDecisionRecord {
        identity: identity.clone(),
        decision_point: point,
        policy,
        candidates,
        chosen,
        chosen_propensity: Some(chosen_propensity),
        source: Some(source),
        state: learned.map(|learned| reader_state(learned, read)),
        thresholds_digest: state.thresholds.clone(),
        arm_set: None,
        section_draws: draws.to_vec(),
        proposals: None,
        audit: Default::default(),
    }
}

/// When the chain's arms were drawn and when the prompt's content decisions
/// were made: the attempt's open and its prompt's assembly. A decision never
/// shares its draw's millisecond.
fn decision_times(timing: &AttemptTiming) -> (i64, i64) {
    let decided_at = timing
        .prompt_assembled_at
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
    let assigned_at = timing.attempt_started_at.unwrap_or(decided_at - 1);
    (assigned_at, decided_at.max(assigned_at + 1))
}

/// S03's fields and proposals of the content decision at `point` (A-DEC, S01
/// §5.3; backlog 5125), for L-know at `knowledge`, L-play at `playbooks` and
/// L-sec at `sections` (gap-6f31c3); the other points carry none yet. The
/// proposals and the opportunity are the loop's ([`reader_proposals`],
/// [`section_proposals`], whose draws `bandit` holds). The draw is the
/// chain's arm set's on the point's layer, with `(assigned_at,
/// decided_at)`. The receipt names the included items' rendered digests,
/// which prompt assembly found in the system prompt dispatch launches as is,
/// bound to the hash of the assembled request. Pure, for E1 (5130).
pub(super) fn content_audit(
    point: ContentDecisionPoint,
    items: &[&PromptItemDiagnostic],
    read: Option<&ReaderRead>,
    identity: &AttemptIdentity,
    draw: (Option<&ArmSet>, (i64, i64)),
    request_hash: &str,
    bandit: BanditDraws<'_>,
) -> Option<(ContentProposals, AuditFields)> {
    let (loop_id, layer) = match point {
        ContentDecisionPoint::Knowledge => ("L-know", "knowledge"),
        ContentDecisionPoint::Playbooks => ("L-play", "playbooks"),
        ContentDecisionPoint::Sections => ("L-sec", "sections"),
        _ => return None,
    };
    let (arm_set, (assigned_at, decided_at)) = draw;
    // A fault flag on the loop (S03 §4.9; fault-injection builds only):
    // UNLOGGED drops the receipt, LABEL_ONLY draws the arm at the decision.
    let fault = faults::active(loop_id);
    let label_only = fault == Some(FaultKind::LabelOnly);
    let assigned_at = if label_only { decided_at } else { assigned_at };
    let (proposals, opportunity) = if point == ContentDecisionPoint::Sections {
        let maximize = arm_set.is_some_and(|arms| arms.condition_id == MAXIMIZE_CONDITION);
        section_proposals(items, bandit, maximize)
    } else {
        reader_proposals(items, read)
    };
    let key = identity.key();
    let assignment = arm_set
        .and_then(|arms| arms.get(layer))
        .map(|draw| DecisionAssignment::new(draw.clone(), &key, assigned_at));
    let receipt = DecisionReceipt {
        kind: "content".to_string(),
        ok: true,
        request_hash: Some(request_hash.to_string()),
        exposure_hashes: items
            .iter()
            .filter(|item| item.included)
            .map(|item| item.rendered_sha256.clone())
            .collect(),
    };
    let audit = AuditFields {
        loop_id: Some(loop_id.to_string()),
        loop_ids: vec![loop_id.to_string()],
        layer: Some(layer.to_string()),
        opportunity: Some(opportunity),
        assignment,
        decided_at: Some(decided_at),
        receipt: (fault != Some(FaultKind::Unlogged)).then_some(receipt),
    };
    Some((proposals, audit))
}

/// L-know's or L-play's proposals at its reader's point, and whether the
/// point was the loop's opportunity: the learned proposal is the ids the
/// reader would include, on the withheld arm too, under a θ that turns
/// the knowledge section off (gap-6bb6d8), and when the section bandit
/// leaves out the section they render into (bug-550909), so that what L-sec
/// removed counts as proposed and not reached; π⁰ includes none. The point
/// is an opportunity when it offered an item or its reader `read` had one,
/// though it loaded nothing to show (gap-a13544).
fn reader_proposals(
    items: &[&PromptItemDiagnostic],
    read: Option<&ReaderRead>,
) -> (ContentProposals, DecisionOpportunity) {
    let learned = items
        .iter()
        .filter(|item| {
            item.included
                || matches!(
                    item.excluded_reason,
                    Some(
                        ExcludedReason::WithheldArm
                            | ExcludedReason::HarnessPolicy
                            | ExcludedReason::BanditExcluded
                    )
                )
        })
        .map(|item| item.id.clone())
        .collect();
    let retrieved = items
        .iter()
        .any(|item| item.excluded_reason != Some(ExcludedReason::RoleFilter));
    let offered = retrieved || read.is_some_and(|read| read.opportunity);
    let reason = if offered {
        "items_retrieved"
    } else {
        "no_item_for_the_role"
    };
    let proposals = ContentProposals {
        learned: Some(learned),
        default: Some(Vec::new()),
    };
    let opportunity = DecisionOpportunity {
        eligible: offered,
        reason: reason.to_string(),
    };
    (proposals, opportunity)
}

/// L-sec's proposals at the `sections` point, and whether the point was its
/// opportunity (S03 §4.2; gap-6f31c3). Both are the sections a prompt
/// includes: under the section bandit, and under π⁰, which leaves none out.
/// Where the prompt applied the bandit's draws (`bandit.applied`, the
/// chain's learned arm), its sections are the bandit's, and π⁰'s add the
/// ones the bandit left out; where it only logged them (`bandit.proposed`,
/// the arm that runs π⁰), its sections are π⁰'s, and the bandit's leave out
/// the ones it drew out. The token budget's cut is taken to fall alike on
/// both. The point is an opportunity when the bandit drew a droppable
/// section, which it does on every arm outside `maximize` mode, where the
/// bandit is off.
fn section_proposals(
    items: &[&PromptItemDiagnostic],
    bandit: BanditDraws<'_>,
    maximize: bool,
) -> (ContentProposals, DecisionOpportunity) {
    let included: Vec<String> = items
        .iter()
        .filter(|item| item.included)
        .map(|item| item.id.clone())
        .collect();
    let left_out = |draws: &[SectionDecision]| -> Vec<String> {
        draws
            .iter()
            .filter(|draw| draw.excluded)
            .map(|draw| draw.section.clone())
            .collect()
    };
    let (learned, default) = if bandit.applied.is_empty() {
        let dropped = left_out(bandit.proposed);
        let learned = included
            .iter()
            .filter(|id| !dropped.contains(id))
            .cloned()
            .collect();
        (learned, included)
    } else {
        let mut default = included.clone();
        default.extend(left_out(bandit.applied));
        (included, default)
    };
    let drew = !bandit.applied.is_empty() || !bandit.proposed.is_empty();
    let reason = match (drew, maximize) {
        (true, _) => "droppable_sections",
        (false, true) => "maximize",
        (false, false) => "no_droppable_section",
    };
    let proposals = ContentProposals {
        learned: Some(learned),
        default: Some(default),
    };
    let opportunity = DecisionOpportunity {
        eligible: drew,
        reason: reason.to_string(),
    };
    (proposals, opportunity)
}

/// The section bandit's learned state in its file at `path` (S01 P0-10;
/// gap-6f31c3): the file's digest, labelled `sb:n={n}`, `n` the labelled
/// outcomes its posteriors hold. A bandit that has learned from none, a
/// missing file among them, draws from its uniform prior, so its state
/// reads as empty.
fn bandit_state(path: &Path) -> DecisionState {
    let file = digest_file(path);
    let n_obs = SectionBandit::load(path).map_or(0, |bandit| bandit.labelled_outcomes());
    DecisionState {
        read: n_obs > 0,
        version: format!("sb:n={n_obs}"),
        digest: file
            .as_ref()
            .map_or_else(|| b3_digest(b""), |file| file.digest.clone()),
        age_s: file
            .and_then(|file| file.modified.elapsed().ok())
            .map(|age| age.as_secs()),
        n_obs,
        stale: false,
    }
}

/// The probability the logging policy included `item`: 1 − p_ex for a
/// section the bandit drew (`draws`), else 1 or 0, as a fixed ranking chose.
fn inclusion_probability(item: &PromptItemDiagnostic, draws: &[SectionDecision]) -> f64 {
    let drawn = draws
        .iter()
        .find(|draw| item.kind == ExposureItemKind::Section && draw.section == item.id);
    match drawn {
        Some(draw) => 1.0 - draw.p_exclude,
        None if item.included => 1.0,
        None => 0.0,
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

/// The learned state of a prompt-cache snapshot's `part` (S01 P0-10),
/// labelled `{label}:n={n}`: what the prompts were built from, whatever the
/// store holds now (backlog 4214).
fn snapshot_state(
    part: &crate::dispatch::prompt_cache::SnapshotPart,
    label: &str,
) -> DecisionState {
    let n_obs = u64::try_from(part.count).unwrap_or(u64::MAX);
    DecisionState {
        read: n_obs > 0,
        version: format!("{label}:n={n_obs}"),
        digest: part.digest.clone(),
        age_s: None,
        n_obs,
        stale: false,
    }
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
        stale: false,
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
        VERIFY_PROVIDER, jsonl_rows_where, make_spec, make_test_dispatcher, no_auto_fix,
        no_auto_fix_maximize, verify_step,
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

    /// A section a prompt offered: in the prompt, or left out for `reason`.
    fn section_item(id: &str, reason: Option<ExcludedReason>) -> PromptItemDiagnostic {
        PromptItemDiagnostic {
            kind: ExposureItemKind::Section,
            id: id.to_string(),
            section: id.to_string(),
            rank: None,
            score: Some(0.5),
            tokens: 40,
            rendered_sha256: format!("{id}-digest"),
            included: reason.is_none(),
            excluded_reason: reason,
        }
    }

    /// The section bandit's draw on `section` at p_ex = 0.2, which left it
    /// out when `excluded`.
    fn bandit_draw(section: &str, excluded: bool) -> SectionDecision {
        SectionDecision {
            section: section.to_string(),
            p_exclude: 0.2,
            excluded,
            propensity: if excluded { 0.2 } else { 0.8 },
        }
    }

    /// gap-6f31c3 (S03 §4.2, §4.4): the sections row carries L-sec's fields,
    /// both proposals on both arms. On the learned arm the prompt applied
    /// the bandit's draws: its sections are the learned proposal, and π⁰'s
    /// add the one the bandit left out. On the arm that runs π⁰ the bandit
    /// only proposed its draws: the prompt's sections are π⁰'s, and the
    /// learned proposal leaves out the one it drew out. Each is an
    /// opportunity with the chain's draw on the `sections` layer and a
    /// receipt of the included sections; maximize mode turns the bandit off,
    /// so its rows are none. A bandit that has learned nothing reads as an
    /// empty state, one taught two outcomes as read.
    #[test]
    fn sections_row_carries_l_sec_fields() {
        use std::collections::BTreeSet;

        use roko_learn::loop_audit::Registry;
        use roko_learn::loop_audit::arm_set::{ArmDraws, ArmMode};
        use roko_learn::telemetry::{Arm, AttemptKey};

        let loops = Registry::embedded().expect("the embedded loop registry");
        let draws = ArmDraws::new(0, "2026-10-16");
        let arms_of = |mode: &ArmMode, index: usize| {
            let key = AttemptKey::new("gr-sections", "plan", format!("t{index}"), 1);
            (ArmSet::assign(&key, &loops, mode, &draws), key)
        };
        let on_arm = |learned: bool| {
            (0..1_000)
                .map(|index| arms_of(&ArmMode::Normal, index))
                .find(|(arms, _)| arms.takes_default("sections") != learned)
                .expect("a chain on the arm")
        };
        let audit = |(arms, key): &(ArmSet, AttemptKey),
                     items: &[&PromptItemDiagnostic],
                     bandit: BanditDraws<'_>| {
            let identity = AttemptIdentity::new(key);
            let draw = (Some(arms), (1, 2));
            let point = ContentDecisionPoint::Sections;
            content_audit(point, items, None, &identity, draw, "b3:request", bandit)
                .expect("L-sec's fields")
        };
        let set = |ids: Option<Vec<String>>| -> BTreeSet<String> {
            ids.unwrap_or_default().into_iter().collect()
        };
        let named =
            |ids: &[&str]| -> BTreeSet<String> { ids.iter().map(|id| (*id).to_string()).collect() };
        let role = section_item("role_identity", None);
        let conventions = section_item("conventions", None);

        // The learned arm: the bandit left `tool_hints` out of the prompt.
        let chain = on_arm(true);
        let hints = section_item("tool_hints", Some(ExcludedReason::BanditExcluded));
        let applied = [
            bandit_draw("conventions", false),
            bandit_draw("tool_hints", true),
        ];
        let bandit = BanditDraws {
            applied: &applied,
            proposed: &[],
        };
        let (proposals, fields) = audit(&chain, &[&role, &conventions, &hints], bandit);
        assert_eq!(fields.loop_id.as_deref(), Some("L-sec"));
        assert_eq!(fields.loop_ids, ["L-sec"]);
        assert_eq!(fields.layer.as_deref(), Some("sections"));
        let opportunity = fields.opportunity.as_ref().expect("the opportunity");
        let seen = (opportunity.eligible, opportunity.reason.as_str());
        assert_eq!(seen, (true, "droppable_sections"));
        let assignment = fields.assignment.as_ref().expect("the chain's draw");
        assert_eq!(assignment.draw.layer, "sections");
        assert_eq!(assignment.draw.arm, Arm::Learned);
        assert_eq!(assignment.unit_key, chain.1.chain_key());
        assert!(assignment.assigned_at < fields.decided_at.expect("decided_at"));
        let both = named(&["role_identity", "conventions"]);
        assert_eq!(set(proposals.learned), both);
        let all = named(&["role_identity", "conventions", "tool_hints"]);
        assert_eq!(set(proposals.default), all);
        let receipt = fields.receipt.as_ref().expect("the receipt");
        assert_eq!(receipt.request_hash.as_deref(), Some("b3:request"));
        assert_eq!(
            receipt.exposure_hashes,
            ["role_identity-digest", "conventions-digest"]
        );

        // The arm that runs π⁰: every section in, and the bandit proposed
        // leaving `conventions` out.
        let chain = on_arm(false);
        let hints = section_item("tool_hints", None);
        let proposed = [
            bandit_draw("conventions", true),
            bandit_draw("tool_hints", false),
        ];
        let bandit = BanditDraws {
            applied: &[],
            proposed: &proposed,
        };
        let (proposals, fields) = audit(&chain, &[&role, &conventions, &hints], bandit);
        let opportunity = fields.opportunity.as_ref().expect("the opportunity");
        assert!(opportunity.eligible, "{opportunity:?}");
        let assignment = fields.assignment.as_ref().expect("the chain's draw");
        assert_ne!(assignment.draw.arm, Arm::Learned);
        assert_eq!(
            set(proposals.learned),
            named(&["role_identity", "tool_hints"])
        );
        assert_eq!(set(proposals.default), all);

        // Maximize mode: the bandit is off, and the point is no opportunity.
        let chain = arms_of(&ArmMode::Maximize, 0);
        let bandit = BanditDraws {
            applied: &[],
            proposed: &[],
        };
        let (proposals, fields) = audit(&chain, &[&role, &conventions, &hints], bandit);
        let opportunity = fields.opportunity.as_ref().expect("the opportunity");
        let seen = (opportunity.eligible, opportunity.reason.as_str());
        assert_eq!(seen, (false, "maximize"));
        assert_eq!(proposals.learned, proposals.default);

        // The bandit's state: empty until it learns from a labelled outcome.
        let temp = tempdir().expect("tempdir");
        let path = temp.path().join(SECTION_BANDIT_PATH);
        let cold = bandit_state(&path);
        assert_eq!((cold.read, cold.n_obs), (false, 0), "{cold:?}");
        SectionBandit::transaction(&path, |bandit| {
            bandit.record("conventions", false, Some(1));
            bandit.record("tool_hints", true, Some(0));
            bandit.record("tool_hints", false, None);
        })
        .expect("teach the bandit");
        let taught = bandit_state(&path);
        let seen = (taught.read, taught.n_obs, taught.version.as_str());
        assert_eq!(seen, (true, 2, "sb:n=2"), "{taught:?}");
        assert_ne!(taught.digest, cold.digest);
    }

    /// bug-550909: when the section bandit leaves out `domain_context`, the
    /// knowledge and playbook readers' items never reach the prompt, and
    /// their rows still propose them. What L-sec removed is L-know's or
    /// L-play's learned proposal and not its choice, so the census counts
    /// the opportunity as not reached. An item the token budget cut stays
    /// out of the proposal, and the receipt names only what the prompt
    /// included.
    #[test]
    fn reader_rows_propose_what_the_section_bandit_left_out() {
        use roko_learn::loop_audit::Registry;
        use roko_learn::loop_audit::arm_set::{ArmDraws, ArmMode};
        use roko_learn::telemetry::AttemptKey;

        let loops = Registry::embedded().expect("the embedded loop registry");
        let key = AttemptKey::new("gr-readers", "plan", "t1", 1);
        let draws = ArmDraws::new(0, "2026-10-17");
        let arms = ArmSet::assign(&key, &loops, &ArmMode::Normal, &draws);
        let identity = AttemptIdentity::new(&key);
        let audit = |point: ContentDecisionPoint, items: &[&PromptItemDiagnostic]| {
            let bandit = BanditDraws {
                applied: &[],
                proposed: &[],
            };
            let draw = (Some(&arms), (1, 2));
            content_audit(point, items, None, &identity, draw, "b3:request", bandit)
                .expect("the reader's fields")
        };
        let left_out = |item: PromptItemDiagnostic| PromptItemDiagnostic {
            included: false,
            excluded_reason: Some(ExcludedReason::BanditExcluded),
            ..item
        };

        let lesson = left_out(knowledge_item("kn-1", 1, true));
        let cut = knowledge_item("kn-2", 2, false);
        let (proposals, fields) = audit(ContentDecisionPoint::Knowledge, &[&lesson, &cut]);
        assert_eq!(fields.loop_id.as_deref(), Some("L-know"));
        assert_eq!(proposals.learned, Some(vec!["kn-1".to_string()]));
        assert_eq!(proposals.default, Some(Vec::new()));
        let receipt = fields.receipt.expect("the receipt");
        assert!(receipt.exposure_hashes.is_empty(), "{receipt:?}");

        let removed = left_out(playbook_item("pb-1"));
        let shown = playbook_item("pb-2");
        let (proposals, fields) = audit(ContentDecisionPoint::Playbooks, &[&removed, &shown]);
        assert_eq!(fields.loop_id.as_deref(), Some("L-play"));
        let both = vec!["pb-1".to_string(), "pb-2".to_string()];
        assert_eq!(proposals.learned, Some(both));
        let receipt = fields.receipt.expect("the receipt");
        assert_eq!(receipt.exposure_hashes, ["pb-2-digest"]);
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
            trust_exclusions: None,
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

    /// gap-595e28: an attempt's verdict keeps DP4's own count of the
    /// exclusions its routing made, though its plan wrote no route row to
    /// name them, and the verify depth DP3 checked it at.
    #[tokio::test]
    async fn verdict_keeps_dp4_count_and_verify_depth() {
        use roko_core::audit_types::VerifyDepth;
        use roko_learn::telemetry::records::VerifyDepthRecord;

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
        let mut plan = planned(Vec::new());
        plan.trust_exclusions = Some(2);
        dispatcher.record_planned_attempt(&mut attempt, &task, &plan);
        let depth = VerifyDepthRecord {
            task_type: "code".to_string(),
            depth: VerifyDepth::V2,
            ladder: Some(VerifyDepth::V2),
            floor: VerifyDepth::V0,
        };
        attempt.record_verify_depth(Some(depth.clone()));
        let passed = Settlement::verified(&Ok(TaskGateVerdict::Passed));
        attempt.settle(passed, "stream-model", None);
        dispatcher.close_run_attempts(RUN);

        let run = RunRecords::load(&runs_dir.join(RUN)).expect("load the run");
        assert!(run.invalid.is_empty(), "{:?}", run.invalid);
        assert!(run.decisions.is_empty(), "the plan wrote no route row");
        let verdict = &run.verdicts[0].record;
        assert_eq!(verdict.trust_exclusions, Some(2));
        assert_eq!(verdict.verify_depth, Some(depth));
    }

    /// S03 §5 A-DEC (backlog 5125): the knowledge row carries L-know's
    /// layer and the chain's draw, made before the prompt was assembled, both
    /// proposals, and a receipt that binds the included entry's digest to
    /// the assembled request. The playbook row carries L-play's.
    #[tokio::test]
    async fn graph_decisions_log_arm_before_plan_and_receipt() {
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
        let chain_key = attempt.key.chain_key();
        let mut plan = planned(vec![
            knowledge_item("kn-1", 1, true),
            knowledge_item("kn-2", 2, false),
            playbook_item("pb-1"),
        ]);
        plan.prompt.system_prompt = "## Domain Context\nkn-1 rendered".to_string();
        plan.prompt.user_prompt = "# Task Request\nt".to_string();
        let request = dispatch_prompt_hash(&plan.prompt.system_prompt, &plan.prompt.user_prompt);
        attempt.prompt_assembled();
        dispatcher.record_planned_attempt(&mut attempt, &task, &plan);
        let passed = Settlement::verified(&Ok(TaskGateVerdict::Passed));
        attempt.settle(passed, "stream-model", None);
        dispatcher.close_run_attempts(RUN);

        let run = RunRecords::load(&runs_dir.join(RUN)).expect("load the run");
        assert!(run.invalid.is_empty(), "{:?}", run.invalid);
        let row = |point: ContentDecisionPoint| {
            run.content_decisions
                .iter()
                .map(|line| &line.record)
                .find(|row| row.decision_point == point)
                .expect("a row at the point")
        };
        let knowledge = row(ContentDecisionPoint::Knowledge);
        let audit = &knowledge.audit;
        assert_eq!(audit.layer.as_deref(), Some("knowledge"));
        assert_eq!(audit.loop_id.as_deref(), Some("L-know"));
        let opportunity = audit.opportunity.as_ref().expect("the opportunity");
        assert!(opportunity.eligible, "{opportunity:?}");
        let assignment = audit.assignment.as_ref().expect("the chain's draw");
        assert_eq!(assignment.unit_key, chain_key);
        assert_eq!(assignment.draw.layer, "knowledge");
        assert!(assignment.assigned_at < audit.decided_at.expect("decided_at"));
        let proposals = knowledge.proposals.as_ref().expect("the proposals");
        assert_eq!(proposals.learned, Some(vec!["kn-1".to_string()]));
        assert_eq!(proposals.default, Some(Vec::new()));
        let receipt = audit.receipt.as_ref().expect("the receipt");
        assert!(receipt.ok);
        assert_eq!(receipt.request_hash.as_deref(), Some(request.as_str()));
        assert_eq!(receipt.exposure_hashes, ["kn-1-digest"]);

        let playbooks = row(ContentDecisionPoint::Playbooks);
        assert_eq!(playbooks.audit.loop_id.as_deref(), Some("L-play"));
        let learned = playbooks.proposals.as_ref().and_then(|p| p.learned.clone());
        assert_eq!(learned, Some(vec!["pb-1".to_string()]));
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
        assert_eq!(row.policy, "topic_overlap_top3");
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

    /// gap-5f6d0b: `prompt_assembled` names what the prompt carried by kind,
    /// and each retrieved knowledge entry, playbook, episode or section it
    /// left out with why. The error-pattern block is none of them.
    #[test]
    fn prompt_event_names_what_it_carried_and_left_out() {
        let left_out = |item: PromptItemDiagnostic, reason: ExcludedReason| PromptItemDiagnostic {
            included: false,
            excluded_reason: Some(reason),
            ..item
        };
        let kind = |kind: ExposureItemKind, id: &str| PromptItemDiagnostic {
            kind,
            ..knowledge_item(id, 1, true)
        };
        let diagnostics = PromptDiagnostics {
            items: vec![
                knowledge_item("kn-1", 1, true),
                left_out(
                    knowledge_item("kn-2", 2, true),
                    ExcludedReason::BanditExcluded,
                ),
                playbook_item("pb-1"),
                left_out(playbook_item("pb-2"), ExcludedReason::WithheldArm),
                kind(ExposureItemKind::Episode, "ep-1"),
                left_out(
                    kind(ExposureItemKind::ErrorPattern, "test::abc"),
                    ExcludedReason::TokenBudget,
                ),
                section_item("role_identity", None),
                section_item("domain_context", Some(ExcludedReason::BanditExcluded)),
            ],
            included_sections: vec!["role_identity".to_string()],
            estimated_tokens: 900,
            ..PromptDiagnostics::default()
        };

        let event = prompt_event(&diagnostics, "plan", "T1", 2);

        assert_eq!(
            serde_json::to_value(&event).expect("the event as JSON"),
            serde_json::json!({
                "type": "prompt_assembled",
                "plan_id": "plan",
                "task_id": "T1",
                "attempt": 2,
                "knowledge_ids": ["kn-1"],
                "playbook_ids": ["pb-1"],
                "episode_ids": ["ep-1"],
                "left_out": [
                    {"kind": "knowledge", "id": "kn-2", "reason": "bandit_excluded"},
                    {"kind": "playbook", "id": "pb-2", "reason": "withheld_arm"},
                    {"kind": "section", "id": "domain_context", "reason": "bandit_excluded"},
                ],
                "included_sections": ["role_identity"],
                "dropped_sections": ["domain_context"],
                "estimated_tokens": 900,
            })
        );
    }

    /// gap-5f6d0b: an attempt publishes `prompt_assembled` as it is planned,
    /// after its `route_decided`, naming the stored lesson its prompt carried
    /// because it shares the task's words. The run is in maximize mode, so no
    /// arm withholds the entry.
    #[tokio::test]
    async fn an_attempt_publishes_what_its_prompt_carried() {
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
        let hub = crate::state_hub::shared_state_hub();
        let mut events = hub.subscribe_events();
        let bridge = TuiBridge::new(hub.sender());
        let (dispatcher, mut task) = crate::graph_task_dispatch::tests::make_test_dispatcher_with(
            &temp,
            VERIFY_PROVIDER,
            no_auto_fix_maximize,
            feedback,
            |dispatcher| dispatcher.with_tui_bridge(bridge),
        )
        .await;
        task.verify = vec![verify_step("structural", "true")];
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        dispatcher
            .dispatch(&make_spec(&task), Vec::new(), &ctx)
            .await
            .expect("the verified attempt passes");

        let mut order = Vec::new();
        let mut carried = Vec::new();
        loop {
            match events.try_recv() {
                Ok(envelope) => match envelope.payload {
                    roko_core::DashboardEvent::RouteDecided { .. } => order.push("route_decided"),
                    roko_core::DashboardEvent::PromptAssembled {
                        task_id,
                        attempt,
                        knowledge_ids,
                        included_sections,
                        ..
                    } => {
                        order.push("prompt_assembled");
                        carried.push((task_id, attempt, knowledge_ids, included_sections));
                    }
                    _ => {}
                },
                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => {}
                Err(_) => break,
            }
        }
        let [(task_id, attempt, knowledge_ids, included_sections)] = carried.as_slice() else {
            panic!("one prompt_assembled: {carried:?}");
        };
        assert_eq!((task_id.as_str(), *attempt), (task.id.as_str(), 1));
        assert_eq!(knowledge_ids, &["kn-stream"]);
        assert!(
            included_sections
                .iter()
                .any(|section| section == "domain_context"),
            "{included_sections:?}"
        );
        let prompt = order.iter().position(|kind| *kind == "prompt_assembled");
        if let Some(route) = order.iter().position(|kind| *kind == "route_decided") {
            assert!(Some(route) < prompt, "the route comes first: {order:?}");
        }
    }

    /// bug-eaa318: the provider bridge's `model_call` row of an attempt whose
    /// prompt included a knowledge entry names the entry, so the loop census
    /// finds knowledge ids on the efficiency log. A frozen run's row names it
    /// too, but the run records no knowledge feedback (decision 2218).
    #[tokio::test]
    async fn dispatch_with_knowledge_produces_an_efficiency_row_with_knowledge_ids() {
        use roko_learn::loop_audit::{Registry, census};

        for frozen in [false, true] {
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
            let configure = |config: &mut RokoConfig| {
                no_auto_fix_maximize(config);
                config.learning.frozen = frozen;
            };
            let (dispatcher, mut task) =
                make_test_dispatcher(&temp, VERIFY_PROVIDER, configure, feedback).await;
            task.verify = vec![verify_step("structural", "true")];
            let ctx = CellContext::new().with_run_id(RUN.to_string());
            dispatcher
                .dispatch(&make_spec(&task), Vec::new(), &ctx)
                .await
                .expect("the verified attempt passes");
            drop(dispatcher);

            let log = roko.join("learn/efficiency.jsonl");
            let calls = jsonl_rows_where(&log, 1, |row| row["kind"] == "model_call").await;
            assert_eq!(
                calls[0]["knowledge_ids"],
                serde_json::json!(["kn-stream"]),
                "frozen {frozen}: {}",
                calls[0]
            );
            let knowledge_feedback = roko.join("learn/knowledge-feedback.jsonl");
            assert_eq!(knowledge_feedback.exists(), !frozen, "frozen {frozen}");
            let registry = Registry::embedded().expect("the embedded loop registry");
            let report = census::run(temp.path(), &registry, None);
            let facts = &report.row("L-know").expect("L-know's census row").facts;
            let efficiency = facts
                .iter()
                .find(|fact| fact.contains("efficiency rows carry"))
                .expect("L-know's log fact");
            assert!(!efficiency.contains(" and 0/"), "{efficiency}");
        }
    }

    /// bug-0a7ec1: a Graph attempt's verdict settles the knowledge outcome of
    /// the stored entry its prompt carried. A passing verify leaves one
    /// `success` row, with the run id and the attempt key, and a score of 1;
    /// a failing one leaves `failure` and -1. The provider call leaves no
    /// `partial` row of its own.
    #[tokio::test]
    async fn graph_attempt_verdict_settles_its_knowledge_outcome() {
        for (check, outcome, score) in [("true", "success", 1), ("false", "failure", -1)] {
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
                make_test_dispatcher(&temp, VERIFY_PROVIDER, no_auto_fix_maximize, feedback).await;
            task.verify = vec![verify_step("structural", check)];
            let ctx = CellContext::new().with_run_id(RUN.to_string());
            let dispatched = dispatcher
                .dispatch(&make_spec(&task), Vec::new(), &ctx)
                .await;
            assert_eq!(dispatched.is_ok(), check == "true", "{check}");
            drop(dispatcher);
            crate::background_writes::settled(&roko).await;

            let run = RunRecords::load(&roko.join("runs").join(RUN)).expect("load the run");
            let [verdict] = run.verdicts.as_slice() else {
                panic!("one verdict: {:?}", run.verdicts);
            };
            let log = std::fs::read_to_string(roko.join("learn/knowledge-feedback.jsonl"))
                .expect("the knowledge feedback log");
            let rows: Vec<serde_json::Value> = log
                .lines()
                .map(|line| serde_json::from_str(line).expect("a JSON row"))
                .collect();
            let [row] = rows.as_slice() else {
                panic!("one row: {rows:?}");
            };
            assert_eq!(row["type"], "knowledge_outcome", "{row}");
            assert_eq!(row["outcome"], outcome, "{row}");
            assert_eq!(row["run_id"], RUN, "{row}");
            assert_eq!(
                row["request_id"], verdict.record.identity.attempt_key,
                "{row}"
            );
            assert_eq!(row["knowledge_ids"], serde_json::json!(["kn-stream"]));
            let scores = std::fs::read_to_string(roko.join("learn/knowledge-scores.json"))
                .expect("the knowledge scores");
            let scores: serde_json::Value = serde_json::from_str(&scores).expect("JSON scores");
            assert_eq!(scores["scores"]["kn-stream"], score, "{scores}");
        }
    }

    /// G29: a dispatch whose prompt retrieved a matching knowledge entry logs
    /// the entry as included, and its verdict counts the inclusion. The run
    /// is in maximize mode, so no arm withholds the entry.
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
            make_test_dispatcher(&temp, VERIFY_PROVIDER, no_auto_fix_maximize, feedback).await;
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

    /// gap-6bb6d8 (M1's B4): the θ an attempt runs decides whether its prompt
    /// carries the knowledge section. Every chain runs the θ of an `on`-mode
    /// controller. Under θ₀ the matching entry reaches the prompt. Under a θ
    /// whose `knowledge_section` is off the entry is still retrieved, but the
    /// prompt has no `# Neuro knowledge` section, the exposure row says θ
    /// left the entry out, nothing credits it, and the knowledge decision row
    /// names the controller as its source and is no opportunity of L-know,
    /// whose proposal stands. The run is in maximize mode, so no L-know arm
    /// withholds the entry.
    #[tokio::test]
    async fn knowledge_section_knob_off_omits_the_knowledge_section() {
        use roko_core::config::harness_params::{HarnessLadders, HarnessParams, Knob, Step};
        use roko_core::config::homeostasis::{HomeostasisConfig, HomeostasisMode};
        use roko_learn::homeostasis::controller::Controller;
        use roko_learn::homeostasis::detect::Baseline;
        use roko_learn::homeostasis::policy::ViabilityPolicy;

        use crate::graph_task_dispatch::tests::make_test_dispatcher_with;
        use crate::runtime_feedback::HomeostasisSink;

        const POLICY: &str = "policy_version = 1\nholdout = 0.0\n\
            ev.pass_rate = { lo = 0.70 }\nev.usd_per_verified_success = { hi = 0.12 }\n\
            ev.false_green = { hi = 0.10 }\nev.latency_p90_s = { hi = 900 }\n";

        for knowledge_section in [true, false] {
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
            let configure = |config: &mut RokoConfig| {
                no_auto_fix_maximize(config);
                config.runner.log_prompts = true;
            };
            // The θ the attempt runs: θ₀, or θ₀ with the section off.
            let mut applied = None;
            let finish = |dispatcher: GraphTaskDispatcher| {
                let theta0 = HarnessParams::baseline(&dispatcher.config);
                let ladders = HarnessLadders::from_config(&dispatcher.config);
                let settings = HomeostasisConfig {
                    mode: HomeostasisMode::On,
                    ..HomeostasisConfig::default()
                };
                let baseline = Baseline {
                    pass_rate: 0.80,
                    usd_per_resolution: 0.05,
                    wall_ms: 300_000.0,
                };
                let policy = ViabilityPolicy::parse(POLICY).expect("the policy parses");
                let controller = Controller::new(
                    &settings,
                    policy,
                    theta0.clone(),
                    ladders.clone(),
                    baseline,
                    0,
                );
                // Every chain runs the controller's θ, whatever the day's draws.
                let sink = HomeostasisSink::new(temp.path(), Some(controller), None);
                let sink = Arc::new(sink.with_holdout(0.0));
                let theta = if knowledge_section {
                    theta0
                } else {
                    let off = theta0
                        .step(Knob::KnowledgeSection, Step::Down, &ladders)
                        .expect("the section switches off");
                    assert_eq!(sink.handle().swap(off.clone(), "section off"), 1);
                    off
                };
                assert_eq!(theta.knowledge_section, knowledge_section);
                applied = Some(theta);
                dispatcher.with_feedback(GraphFeedbackContext {
                    runs_dir: Some(roko.join("runs")),
                    homeostasis: Some(sink),
                    ..GraphFeedbackContext::default()
                })
            };
            let feedback = GraphFeedbackContext::default();
            let (dispatcher, mut task) =
                make_test_dispatcher_with(&temp, VERIFY_PROVIDER, configure, feedback, finish)
                    .await;
            let applied = applied.expect("the sink's θ");
            task.verify = vec![verify_step("structural", "true")];
            let ctx = CellContext::new().with_run_id(RUN.to_string());
            dispatcher
                .dispatch(&make_spec(&task), Vec::new(), &ctx)
                .await
                .expect("the verified attempt passes");
            drop(dispatcher);
            crate::background_writes::settled(&roko).await;

            // The system prompt the agent got.
            let logs: Vec<PathBuf> = std::fs::read_dir(roko.join("prompt-logs"))
                .expect("the prompt log directory")
                .map(|entry| entry.expect("a prompt log").path())
                .collect();
            assert_eq!(logs.len(), 1, "{logs:?}");
            let log = std::fs::read_to_string(&logs[0]).expect("read the prompt log");
            let (system, _) = log
                .split_once("=== USER PROMPT ===")
                .expect("the log holds both prompts");
            let on = knowledge_section;
            assert_eq!(system.contains("# Neuro knowledge"), on, "{system}");
            assert_eq!(system.contains("[kn-stream]"), on, "{system}");

            let run = RunRecords::load(&roko.join("runs").join(RUN)).expect("load the run");
            assert!(run.invalid.is_empty(), "{:?}", run.invalid);
            // The attempt ran θ, on the controller's learned arm.
            let harness = &run.harness_decisions[0].record;
            assert_eq!(harness.params_digest, applied.params_digest(), "on {on}");
            // The entry was retrieved either way; θ alone kept it out.
            let entry = run
                .exposures
                .iter()
                .map(|line| &line.record)
                .find(|row| row.item_id == "kn-stream")
                .expect("the entry's exposure row");
            let reason = (!on).then_some(ExcludedReason::HarnessPolicy);
            assert_eq!((entry.included, entry.excluded_reason), (on, reason));
            let counts = run.verdicts[0]
                .record
                .exposures
                .expect("the verdict counts");
            assert!(counts.retrieved > 0, "{counts:?}");
            // The knowledge row: the reader's proposal stands; θ's choice is
            // the controller's, and no opportunity of L-know.
            let row = run
                .content_decisions
                .iter()
                .map(|line| &line.record)
                .find(|row| row.decision_point == ContentDecisionPoint::Knowledge)
                .expect("the knowledge row");
            let learned = row.proposals.as_ref().and_then(|p| p.learned.clone());
            assert_eq!(learned, Some(vec!["kn-stream".to_string()]), "on {on}");
            assert_eq!(row.audit.loop_id.as_deref(), Some("L-know"));
            let opportunity = row.audit.opportunity.as_ref().expect("the opportunity");
            let seen = (opportunity.eligible, opportunity.reason.as_str());
            if on {
                assert_eq!(row.chosen, ["kn-stream"]);
                assert_eq!(row.source, Some(DecisionSource::Default));
                assert_eq!(seen, (true, "items_retrieved"));
            } else {
                assert!(row.chosen.is_empty(), "{row:?}");
                assert_eq!(row.source, Some(DecisionSource::Control));
                assert_eq!(seen, (false, KNOWLEDGE_SECTION_OFF));
            }
        }
    }

    /// gap-a40021: the error-pattern decision lists one candidate per pattern
    /// the prompt shows, named by its key, and chooses each one that reached
    /// the prompt, where it used to list one candidate for the whole block.
    #[tokio::test]
    async fn error_pattern_content_decision_lists_one_candidate_per_key() {
        use roko_learn::error_pattern_store::{GateFailureObservation, GateFailureSource};

        let temp = tempdir().expect("tempdir");
        let roko = temp.path().join(".roko");
        let feedback = GraphFeedbackContext {
            runs_dir: Some(roko.join("runs")),
            ..GraphFeedbackContext::default()
        };
        let (dispatcher, mut task) =
            make_test_dispatcher(&temp, VERIFY_PROVIDER, no_auto_fix_maximize, feedback).await;
        task.verify = vec![verify_step("structural", "true")];
        let spec = make_spec(&task);
        // Two patterns that earlier attempts of the task left.
        for (key, digest) in [
            ("verify::E0425", "cannot find value `total` in this scope"),
            ("verify::E0599", "no method named `len` found"),
        ] {
            dispatcher
                .factory
                .error_pattern_store()
                .write()
                .expect("error pattern store")
                .observe_gate_failure(GateFailureObservation::new(
                    key,
                    spec.plan_id.clone(),
                    Some(task.id.clone()),
                    "cargo test -p crate-a",
                    "verify",
                    digest,
                    GateFailureSource::GateClassification,
                ));
        }
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        dispatcher
            .dispatch(&spec, Vec::new(), &ctx)
            .await
            .expect("the verified attempt passes");
        drop(dispatcher);
        crate::background_writes::settled(&roko).await;

        let run = RunRecords::load(&roko.join("runs").join(RUN)).expect("load the run");
        assert!(run.invalid.is_empty(), "{:?}", run.invalid);
        let row = run
            .content_decisions
            .iter()
            .map(|line| &line.record)
            .find(|row| row.decision_point == ContentDecisionPoint::ErrorPatterns)
            .expect("the error-pattern decision");
        let mut candidates: Vec<&str> = row
            .candidates
            .iter()
            .map(|candidate| candidate.id.as_str())
            .collect();
        candidates.sort_unstable();
        assert_eq!(candidates, ["verify::E0425", "verify::E0599"]);
        let mut chosen: Vec<&str> = row.chosen.iter().map(String::as_str).collect();
        chosen.sort_unstable();
        assert_eq!(chosen, candidates, "both patterns reached the prompt");
        assert!(
            row.policy.starts_with("task_or_command_keyed_top"),
            "{}",
            row.policy
        );
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

    /// The run records of `dispatches` passing attempts of one task, with
    /// `[experiments] maximize` set to `maximize`. Each prompt includes a
    /// knowledge entry, so each attempt writes a content decision too.
    async fn arm_set_run(maximize: bool, dispatches: usize) -> RunRecords {
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
        let configure = |config: &mut RokoConfig| {
            no_auto_fix(config);
            config.experiments.maximize = maximize;
        };
        let (dispatcher, mut task) =
            make_test_dispatcher(&temp, VERIFY_PROVIDER, configure, feedback).await;
        task.verify = vec![verify_step("structural", "true")];
        let spec = make_spec(&task);
        let ctx = CellContext::new().with_run_id(RUN.to_string());
        for _ in 0..dispatches {
            dispatcher
                .dispatch(&spec, Vec::new(), &ctx)
                .await
                .expect("the verified attempt passes");
        }
        drop(dispatcher);
        crate::background_writes::settled(&roko).await;
        RunRecords::load(&roko.join("runs").join(RUN)).expect("load the run")
    }

    /// S02.P1-14: the attempts of one task share their chain's arm set,
    /// drawn at attempt open from the chain's key, and every decision row of
    /// each attempt carries it; maximize mode (`[experiments] maximize`)
    /// leaves every loop on its learned arm.
    #[tokio::test]
    async fn arm_set_is_chain_stable_and_logged() {
        use roko_learn::loop_audit::Registry;
        use roko_learn::loop_audit::arm_set::{ArmDraws, ArmMode, ArmSet, MAXIMIZE_CONDITION};
        use roko_learn::telemetry::{Arm, AttemptKey};

        let run = arm_set_run(false, 2).await;
        assert!(run.invalid.is_empty(), "{:?}", run.invalid);
        assert_eq!(run.decisions.len(), 2, "one route row per attempt");
        assert!(!run.content_decisions.is_empty());
        let route = |index: usize| run.decisions[index].record.arm_set.clone();
        let first = route(0).expect("the first route row's arms");
        assert_eq!(route(1), Some(first.clone()), "retries inherit");
        for row in &run.content_decisions {
            assert_eq!(row.record.arm_set.as_ref(), Some(&first));
        }

        // The set is the chain's draw, so a resumed run re-derives it.
        let identity = &run.verdicts[0].record.identity;
        assert_eq!(first.chain_key, identity.chain_key);
        let key = AttemptKey::new(&identity.run_id, &identity.plan_id, &identity.task_id, 1);
        let salt = &first.arms["placebo"].salt_id;
        let epoch = salt
            .split_once('@')
            .map(|(_, epoch)| epoch)
            .expect("salt_id names the epoch");
        let loops = Registry::embedded().expect("the embedded loop registry");
        let drawn = ArmSet::assign(&key, &loops, &ArmMode::Normal, &ArmDraws::new(0, epoch));
        assert_eq!(drawn, first);

        let run = arm_set_run(true, 1).await;
        let row = &run.decisions[0].record;
        let arms = row.arm_set.as_ref().expect("the route row's arms");
        assert_eq!(arms.condition_id, MAXIMIZE_CONDITION);
        for assignment in arms.arms.values() {
            assert_eq!(assignment.arm, Arm::Learned, "{arms:?}");
            assert!((assignment.propensity - 1.0).abs() < 1e-12, "{arms:?}");
        }
    }

    /// S02 L12, S03 §4.3: every attempt writes one placebo decision, its
    /// chain's placebo assignment with identical proposals, and a retry
    /// inherits the arm; across 1,000 chains the placebo's h of 0.5 splits
    /// its arms evenly.
    #[tokio::test]
    async fn placebo_decisions_have_identical_proposals() {
        use roko_learn::loop_audit::Registry;
        use roko_learn::loop_audit::arm_set::{ArmDraws, ArmMode, ArmSet};
        use roko_learn::telemetry::{Arm, AttemptKey};

        let run = arm_set_run(false, 2).await;
        assert!(run.invalid.is_empty(), "{:?}", run.invalid);
        let placebo = &run.placebo_decisions;
        let keys: Vec<&str> = placebo
            .iter()
            .map(|row| row.record.identity.attempt_key.as_str())
            .collect();
        let attempts: Vec<&str> = run
            .verdicts
            .iter()
            .map(|row| row.record.identity.attempt_key.as_str())
            .collect();
        assert_eq!(keys, attempts, "one placebo row per attempt");
        let first = &placebo[0].record.assignment;
        for row in placebo {
            let record = &row.record;
            assert_eq!(record.decision_point, "placebo");
            assert_eq!(record.loop_id, "L-placebo");
            assert_eq!(record.proposals.learned, record.proposals.default);
            assert_eq!(&record.assignment, first, "retries inherit");
            assert!((record.chosen_propensity - record.assignment.propensity).abs() < 1e-12);
        }
        let arms = run.decisions[0].record.arm_set.as_ref().expect("the arms");
        assert_eq!(arms.placebo(), Some(first));

        let loops = Registry::embedded().expect("the embedded loop registry");
        let draws = ArmDraws::new(0, "2026-10-03");
        let (mut learned, mut withheld) = (0_u32, 0_u32);
        for index in 0..1_000 {
            let key = AttemptKey::new("gr-placebo", "plan", format!("t{index}"), 1);
            let set = ArmSet::assign(&key, &loops, &ArmMode::Normal, &draws);
            match set.placebo().expect("the placebo").arm {
                Arm::Learned => learned += 1,
                Arm::Default => withheld += 1,
                _ => {}
            }
        }
        let share = f64::from(learned) / f64::from(learned + withheld);
        assert!((0.45..=0.55).contains(&share), "learned share {share}");
    }

    /// gap-a40021: the policies name the rankings the prompt builder runs:
    /// knowledge and playbooks by the task's topic terms (backlogs
    /// 4211-4213), error patterns by the task and its verify commands
    /// (backlogs 4209, 4210).
    #[test]
    fn content_policies_name_the_topic_and_key_rankings() {
        use ContentDecisionPoint::{ErrorPatterns, Knowledge, Playbooks, Sections};

        let policies =
            [Knowledge, Playbooks, Sections, ErrorPatterns].map(|point| content_policy(point, 5));

        assert_eq!(
            policies,
            [
                "topic_overlap_top3",
                "topic_overlap_outcome_top3",
                "token_budget_composer",
                "task_or_command_keyed_top5",
            ]
        );
    }

    /// gap-26c055: the error-pattern policy names as many patterns as the
    /// attempt's prompt may show: θ₀'s five, or the `error_patterns_k` of
    /// the θ it runs (M1's B4).
    #[test]
    fn error_pattern_policy_names_the_live_pattern_count() {
        let point = ContentDecisionPoint::ErrorPatterns;
        let theta0 = error_pattern_limit(None);
        assert_eq!(content_policy(point, theta0), "task_or_command_keyed_top5");
        let config = RokoConfig::default();
        let mut theta = roko_core::config::harness_params::HarnessParams::baseline(&config);
        assert_eq!(error_pattern_limit(Some(&theta)), theta0);
        theta.error_patterns_k = 10;
        let shown = error_pattern_limit(Some(&theta));
        assert_eq!(content_policy(point, shown), "task_or_command_keyed_top10");
        assert_eq!(
            content_policy(ContentDecisionPoint::Knowledge, shown),
            "topic_overlap_top3"
        );
    }
}
