//! Demo workspace seeding for `roko init --demo`.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use roko_core::OperatingFrequency;
use roko_core::agent::{AgentBackend, AgentRole};
use roko_core::config::schema::RokoConfig;
use roko_fs::{RokoLayout, atomic_write_bytes, atomic_write_json};
use roko_learn::cfactor::{
    AgentCFactorContribution, CFactor, CFactorComponents, CollectivePathology,
};
use roko_learn::costs_db::CostRecord;
use roko_learn::efficiency::{AgentEfficiencyEvent, PromptSectionMeta, ToolCallMeta};
use roko_learn::episode_logger::{Episode, EpisodeGateVerdict, Usage};
use roko_learn::runtime_feedback::{
    KnowledgeSeedEvidence, KnowledgeSeedRecord, LearningPaths, RUNTIME_FEEDBACK_SCHEMA_VERSION,
    project_episode_paths,
};
use roko_learn::telemetry::{
    AttemptIdentity, AttemptKey, AttemptKeyed, AttemptOpenRecord, AttemptOutcome,
    AttemptVerdictRecord, CostSource, RunFile, Stamped, TelemetryRecord,
};
use roko_neuro::{KnowledgeEntry, KnowledgeKind, KnowledgeTier};
use serde::Serialize;
use serde_json::{Value, json};

/// Summary of what was seeded into the workspace.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DemoSeedReport {
    /// Human-readable descriptions of the groups that were written.
    pub seeded_groups: Vec<String>,
    /// Human-readable descriptions of the groups that already existed.
    pub skipped_groups: Vec<String>,
}

impl DemoSeedReport {
    /// Returns `true` when at least one demo artifact was written.
    #[must_use]
    pub fn any_seeded(&self) -> bool {
        !self.seeded_groups.is_empty()
    }

    /// Render a compact summary for `roko init` output.
    #[must_use]
    pub fn summary(&self) -> String {
        if !self.any_seeded() {
            if self.skipped_groups.is_empty() {
                return "demo data already present; left untouched.".to_string();
            }

            let mut out = String::from("demo data already present; left untouched.\n");
            for group in &self.skipped_groups {
                let _ = writeln!(out, "  - {group}");
            }
            return out.trim_end().to_string();
        }

        let mut out = String::from("seeded demo data:\n");
        for group in &self.seeded_groups {
            let _ = writeln!(out, "  - {group}");
        }
        if !self.skipped_groups.is_empty() {
            let _ = writeln!(out, "preserved existing data:");
            for group in &self.skipped_groups {
                let _ = writeln!(out, "  - {group}");
            }
        }
        out.trim_end().to_string()
    }

    fn record_seeded(&mut self, label: impl Into<String>) {
        self.seeded_groups.push(label.into());
    }

    fn record_skipped(&mut self, label: impl Into<String>) {
        self.skipped_groups.push(label.into());
    }
}

#[derive(Debug, Clone)]
struct DemoTaskSpec {
    plan_id: &'static str,
    task_id: &'static str,
    title: &'static str,
    domain: &'static str,
    role: AgentRole,
    category: &'static str,
    complexity_band: &'static str,
    gate: &'static str,
    success: bool,
    failure_reason: Option<&'static str>,
    age_hours: i64,
    age_minutes: i64,
    model_slot: usize,
    seed_kind: KnowledgeKind,
    seed_content: &'static str,
    seed_tags: &'static [&'static str],
    seed_source_indices: &'static [usize],
    seed_confidence: f64,
    seed_confidence_weight: f64,
    seed_model_generality: f64,
}

#[derive(Debug, Clone)]
struct DemoKnowledgeEntrySpec {
    id: &'static str,
    kind: KnowledgeKind,
    topic: &'static str,
    content: &'static str,
    tags: &'static [&'static str],
    source_indices: &'static [usize],
    source_model_slot: usize,
    age_hours: i64,
    confidence: f64,
    confidence_weight: f64,
    model_generality: f64,
    tier: KnowledgeTier,
    refuted_insight_id: Option<&'static str>,
    refutation_evidence: Option<&'static str>,
    balance: f64,
    confirmation_count: u32,
    catalytic_score: u32,
}

#[derive(Debug, Serialize)]
struct SeedEnvelopeRef<'a, T> {
    source: &'static str,
    #[serde(flatten)]
    item: &'a T,
}

#[derive(Debug, Clone, Serialize)]
struct SeededCascadeObservation {
    source: &'static str,
    domain: String,
    task_id: String,
    role: String,
    model: String,
    passed: bool,
    confidence: f64,
    timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
struct SeededCascadeModelStats {
    trials: u64,
    successes: u64,
}

#[derive(Debug, Clone, Serialize)]
struct SeededCascadeSnapshot {
    source: &'static str,
    model_slugs: Vec<String>,
    role_table: HashMap<AgentRole, String>,
    confidence_stats: BTreeMap<String, SeededCascadeModelStats>,
    total_observations: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    observations: Vec<SeededCascadeObservation>,
}

/// The demo run whose attempt ledger the dashboard's Trends page reads.
const DEMO_RUN_ID: &str = "seed-run-20260429";

/// One line of the demo run's `attempts.jsonl`.
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum SeededAttemptLine {
    Open(Box<Stamped<AttemptOpenRecord>>),
    Verdict(Box<Stamped<AttemptVerdictRecord>>),
}

/// Seed a new workspace with demo-only data.
pub fn seed_demo_workspace(
    workdir: impl AsRef<Path>,
    config: Option<&RokoConfig>,
) -> Result<DemoSeedReport> {
    let workdir = workdir.as_ref();
    let roko_dir = workdir.join(".roko");
    let layout = RokoLayout::for_project(workdir);
    let learn_paths = LearningPaths::for_project(workdir);

    let model_pool = demo_model_pool(config);
    let task_specs = demo_task_specs();
    let now = Utc::now();

    let mut planned_episodes = build_episodes(&task_specs, &model_pool, now);
    let episode_ids = existing_episode_ids(workdir);
    let episode_reference_ids = if episode_ids.is_empty() {
        planned_episodes
            .iter()
            .map(|episode| episode.id.clone())
            .collect::<Vec<_>>()
    } else {
        episode_ids
    };
    // A retry has an episode of its own, which the seeded knowledge does not
    // cite.
    let retries = build_retry_episodes(&task_specs, &planned_episodes);
    planned_episodes.extend(retries);
    planned_episodes.sort_by_key(|episode| episode.timestamp);

    let efficiency_events = build_efficiency_events(&task_specs, &model_pool, now);
    let attempt_lines = build_attempt_ledger(&task_specs, &model_pool, now);
    let cost_rows = build_cost_rows(&task_specs, &attempt_lines);
    let cfactors = build_cfactor_snapshots(&task_specs, now);
    let knowledge_seeds =
        build_knowledge_seeds(&task_specs, &episode_reference_ids, &model_pool, now);
    let knowledge_entries = build_knowledge_entries(
        &demo_knowledge_entry_specs(),
        &episode_reference_ids,
        &model_pool,
        now,
    );
    let cascade_snapshot = build_cascade_snapshot(&task_specs, &model_pool, now);

    let mut report = DemoSeedReport::default();

    let episode_paths = project_episode_paths(workdir);
    if episode_paths.iter().any(|path| path.exists()) {
        report.record_skipped("episodes (existing episode logs preserved)");
    } else {
        write_jsonl_if_absent(
            &layout.episodes_path(),
            &planned_episodes,
            "episodes",
            &mut report,
            |episode| seeded_jsonl_line(episode),
        )?;
    }

    // The dashboard reads the newest runs' ledgers, so a demo run beside real
    // runs would mix into their numbers.
    if has_attempt_records(&layout) {
        report.record_skipped("attempt ledger (existing runs preserved)");
    } else {
        write_jsonl_if_absent(
            &RunFile::Attempts.path_in(&layout.run_dir(DEMO_RUN_ID)),
            &attempt_lines,
            "attempt ledger",
            &mut report,
            |line| seeded_jsonl_line(line),
        )?;
        // The demo run's spend, beside its ledger.
        write_jsonl_if_absent(
            &learn_paths.costs_jsonl,
            &cost_rows,
            "cost rows",
            &mut report,
            |row| seeded_jsonl_line(row),
        )?;
    }
    write_jsonl_if_absent(
        &learn_paths.efficiency_jsonl,
        &efficiency_events,
        "efficiency events",
        &mut report,
        |event| seeded_jsonl_line(event),
    )?;
    write_jsonl_if_absent(
        &learn_paths.cfactor_jsonl,
        &cfactors,
        "c-factor snapshots",
        &mut report,
        |snapshot| seeded_jsonl_line(snapshot),
    )?;
    write_json_if_absent(
        &learn_paths.cascade_router_json,
        &cascade_snapshot,
        "cascade router snapshot",
        &mut report,
    )?;
    write_jsonl_if_absent(
        &learn_paths.knowledge_seeds_jsonl,
        &knowledge_seeds,
        "knowledge seeds",
        &mut report,
        |seed| seeded_jsonl_line(seed),
    )?;
    write_jsonl_if_absent(
        &roko_dir.join("neuro").join("knowledge.jsonl"),
        &knowledge_entries,
        "knowledge entries",
        &mut report,
        |entry| serde_json::to_string(entry).context("serialize knowledge entry"),
    )?;

    Ok(report)
}

fn demo_task_specs() -> Vec<DemoTaskSpec> {
    vec![
        DemoTaskSpec {
            plan_id: "seed-auth-flow",
            task_id: "t1",
            title: "Harden the login flow",
            domain: "auth",
            role: AgentRole::Implementer,
            category: "implementation",
            complexity_band: "simple",
            gate: "test",
            success: true,
            failure_reason: None,
            age_hours: 156,
            age_minutes: 0,
            model_slot: 0,
            seed_kind: KnowledgeKind::Insight,
            seed_content: "Login fixes stay stable when the happy path is locked in before edge-case retries.",
            seed_tags: &["auth", "login", "tests"],
            seed_source_indices: &[0],
            seed_confidence: 0.89,
            seed_confidence_weight: 0.74,
            seed_model_generality: 0.95,
        },
        DemoTaskSpec {
            plan_id: "seed-router-tuning",
            task_id: "t2",
            title: "Tighten cascade routing",
            domain: "routing",
            role: AgentRole::QuickReviewer,
            category: "analysis",
            complexity_band: "standard",
            gate: "lint",
            success: false,
            failure_reason: Some("lint signature showed naming drift in the router table"),
            age_hours: 124,
            age_minutes: 0,
            model_slot: 1,
            seed_kind: KnowledgeKind::Warning,
            seed_content: "Cascade routing stays noisy until the cold-start sample size grows beyond the first few guesses.",
            seed_tags: &["routing", "bandit", "confidence"],
            seed_source_indices: &[1, 2],
            seed_confidence: 0.78,
            seed_confidence_weight: 0.55,
            seed_model_generality: 0.85,
        },
        DemoTaskSpec {
            plan_id: "seed-cache-pass",
            task_id: "t3",
            title: "Trim cache-heavy prompts",
            domain: "cache",
            role: AgentRole::Strategist,
            category: "optimization",
            complexity_band: "complex",
            gate: "test",
            success: true,
            failure_reason: None,
            age_hours: 92,
            age_minutes: 0,
            model_slot: 2,
            seed_kind: KnowledgeKind::Heuristic,
            seed_content: "Cache-heavy work benefits from one compact workspace map instead of several redundant context sections.",
            seed_tags: &["cache", "prompt", "tokens"],
            seed_source_indices: &[2, 3],
            seed_confidence: 0.84,
            seed_confidence_weight: 0.68,
            seed_model_generality: 0.90,
        },
        DemoTaskSpec {
            plan_id: "seed-observability",
            task_id: "t4",
            title: "Fix observability spans",
            domain: "observability",
            role: AgentRole::Researcher,
            category: "diagnostics",
            complexity_band: "standard",
            gate: "compile",
            success: false,
            failure_reason: Some("compile signature pointed at a span export mismatch"),
            age_hours: 60,
            age_minutes: 0,
            model_slot: 3,
            seed_kind: KnowledgeKind::CausalLink,
            seed_content: "Span export mismatches tend to show up after a task rewrites its event shape.",
            seed_tags: &["observability", "spans", "metrics"],
            seed_source_indices: &[3, 4],
            seed_confidence: 0.81,
            seed_confidence_weight: 0.60,
            seed_model_generality: 0.88,
        },
        DemoTaskSpec {
            plan_id: "seed-deploy-hardening",
            task_id: "t5",
            title: "Stage deployment hardening",
            domain: "deployment",
            role: AgentRole::Conductor,
            category: "operations",
            complexity_band: "complex",
            gate: "test",
            success: true,
            failure_reason: None,
            age_hours: 8,
            age_minutes: 0,
            model_slot: 4,
            seed_kind: KnowledgeKind::StrategyFragment,
            seed_content: "Deployment hardening works best when rollback, validation, and webhook checks are staged together.",
            seed_tags: &["deploy", "rollback", "webhooks"],
            seed_source_indices: &[4, 0],
            seed_confidence: 0.87,
            seed_confidence_weight: 0.72,
            seed_model_generality: 0.96,
        },
    ]
}

fn demo_knowledge_entry_specs() -> Vec<DemoKnowledgeEntrySpec> {
    vec![
        DemoKnowledgeEntrySpec {
            id: "seed-knowledge-001",
            kind: KnowledgeKind::Insight,
            topic: "auth flow",
            content: "Login fixes are safest when auth state is validated before UI retries.",
            tags: &["auth", "login", "stability"],
            source_indices: &[0],
            source_model_slot: 0,
            age_hours: 150,
            confidence: 0.90,
            confidence_weight: 0.86,
            model_generality: 0.95,
            tier: KnowledgeTier::Working,
            refuted_insight_id: None,
            refutation_evidence: None,
            balance: 0.96,
            confirmation_count: 2,
            catalytic_score: 2,
        },
        DemoKnowledgeEntrySpec {
            id: "seed-knowledge-002",
            kind: KnowledgeKind::Heuristic,
            topic: "cache budget",
            content: "Cache-intensive work stays cheaper when the workspace map is compact and the prompt sections stay few.",
            tags: &["cache", "budget", "prompt"],
            source_indices: &[2, 3],
            source_model_slot: 2,
            age_hours: 136,
            confidence: 0.84,
            confidence_weight: 0.70,
            model_generality: 0.90,
            tier: KnowledgeTier::Consolidated,
            refuted_insight_id: None,
            refutation_evidence: None,
            balance: 0.91,
            confirmation_count: 2,
            catalytic_score: 1,
        },
        DemoKnowledgeEntrySpec {
            id: "seed-knowledge-003",
            kind: KnowledgeKind::Warning,
            topic: "routing confidence",
            content: "Routing confidence looks stable only after the cold-start sample size stops moving the decision boundary.",
            tags: &["routing", "confidence", "cold-start"],
            source_indices: &[1],
            source_model_slot: 1,
            age_hours: 112,
            confidence: 0.78,
            confidence_weight: 0.59,
            model_generality: 0.82,
            tier: KnowledgeTier::Working,
            refuted_insight_id: None,
            refutation_evidence: None,
            balance: 0.87,
            confirmation_count: 1,
            catalytic_score: 1,
        },
        DemoKnowledgeEntrySpec {
            id: "seed-knowledge-004",
            kind: KnowledgeKind::CausalLink,
            topic: "observability spans",
            content: "Longer prompt sections increase tool-call redundancy when they duplicate file context.",
            tags: &["observability", "tool-calls", "sections"],
            source_indices: &[3],
            source_model_slot: 3,
            age_hours: 88,
            confidence: 0.80,
            confidence_weight: 0.63,
            model_generality: 0.88,
            tier: KnowledgeTier::Working,
            refuted_insight_id: None,
            refutation_evidence: None,
            balance: 0.90,
            confirmation_count: 1,
            catalytic_score: 1,
        },
        DemoKnowledgeEntrySpec {
            id: "seed-knowledge-005",
            kind: KnowledgeKind::StrategyFragment,
            topic: "deployment hardening",
            content: "Deployment hardening should pair rollback, validation, and webhook verification in one pass.",
            tags: &["deploy", "rollback", "validation"],
            source_indices: &[4],
            source_model_slot: 4,
            age_hours: 64,
            confidence: 0.86,
            confidence_weight: 0.73,
            model_generality: 0.96,
            tier: KnowledgeTier::Consolidated,
            refuted_insight_id: None,
            refutation_evidence: None,
            balance: 0.94,
            confirmation_count: 2,
            catalytic_score: 2,
        },
        DemoKnowledgeEntrySpec {
            id: "seed-knowledge-006",
            kind: KnowledgeKind::AntiKnowledge,
            topic: "demo labeling",
            content: "Seeded demo data should never be mistaken for live telemetry because the source label keeps the synthetic path explicit.",
            tags: &["demo", "source", "truthfulness"],
            source_indices: &[4, 0],
            source_model_slot: 0,
            age_hours: 40,
            confidence: 0.88,
            confidence_weight: -0.80,
            model_generality: 0.98,
            tier: KnowledgeTier::Working,
            refuted_insight_id: Some("seed-knowledge-005"),
            refutation_evidence: Some(
                "demo data is synthetic and must remain visibly distinct from production telemetry",
            ),
            balance: 0.85,
            confirmation_count: 1,
            catalytic_score: 1,
        },
        DemoKnowledgeEntrySpec {
            id: "seed-knowledge-007",
            kind: KnowledgeKind::Insight,
            topic: "model pool",
            content: "A small pool of realistic model slugs is enough for demo dashboards as long as the naming stays consistent.",
            tags: &["models", "dashboard", "routing"],
            source_indices: &[2, 4],
            source_model_slot: 2,
            age_hours: 24,
            confidence: 0.85,
            confidence_weight: 0.69,
            model_generality: 0.93,
            tier: KnowledgeTier::Consolidated,
            refuted_insight_id: None,
            refutation_evidence: None,
            balance: 0.92,
            confirmation_count: 2,
            catalytic_score: 2,
        },
        DemoKnowledgeEntrySpec {
            id: "seed-knowledge-008",
            kind: KnowledgeKind::Heuristic,
            topic: "observability task cadence",
            content: "Observability tasks move faster when span names, token budgets, and tool outputs are kept aligned.",
            tags: &["observability", "tokens", "tool-output"],
            source_indices: &[3, 1],
            source_model_slot: 3,
            age_hours: 8,
            confidence: 0.82,
            confidence_weight: 0.66,
            model_generality: 0.90,
            tier: KnowledgeTier::Working,
            refuted_insight_id: None,
            refutation_evidence: None,
            balance: 0.90,
            confirmation_count: 1,
            catalytic_score: 1,
        },
    ]
}

fn build_episodes(
    specs: &[DemoTaskSpec],
    model_pool: &[String],
    now: DateTime<Utc>,
) -> Vec<Episode> {
    let mut episodes = Vec::with_capacity(specs.len());
    for spec in specs {
        let model = model_for_slot(model_pool, spec.model_slot);
        let backend = backend_for_model(&model).to_string();
        let completed_at = seeded_datetime(now, spec.age_hours, spec.age_minutes);
        let wall_ms = wall_time_for_task(spec);
        let started_at = completed_at - Duration::milliseconds(wall_ms as i64);
        let mut episode = Episode::new(
            format!("seed-{}-{}", spec.role.label(), spec.task_id),
            spec.task_id,
        );
        episode.kind = "agent_turn".to_string();
        episode.id = format!("seed-episode-{}", spec.task_id);
        episode.episode_id = episode.id.clone();
        episode.timestamp = completed_at;
        episode.agent_id = format!("seed-{}-{}", spec.role.label(), spec.task_id);
        episode.task_id = spec.task_id.to_string();
        episode.input_signal_hash = format!("seed-input-{}", spec.task_id);
        episode.output_signal_hash = format!("seed-output-{}", spec.task_id);
        episode.agent_template = spec.role.label().to_string();
        episode.model = model.clone();
        episode.backend = backend;
        episode.trigger_kind = "plan_task".to_string();
        episode.trigger_signal_hash = format!("seed-trigger-{}", spec.task_id);
        episode.started_at = started_at;
        episode.completed_at = completed_at;
        episode.duration_secs = wall_ms as f64 / 1000.0;
        episode.gate_verdicts = episode_gate_verdicts(spec);
        episode.usage = Usage {
            input_tokens: episode_input_tokens(spec),
            output_tokens: episode_output_tokens(spec),
            cache_read_tokens: episode_cache_tokens(spec),
            cache_write_tokens: episode_cache_write_tokens(spec),
            cost_usd: episode_cost(spec, true),
            cost_usd_without_cache: episode_cost(spec, false),
            wall_ms,
        };
        episode.success = spec.success;
        episode.turns = if spec.success { 2 } else { 3 };
        episode.tokens_used = episode.usage.input_tokens + episode.usage.output_tokens;
        episode.external_actions = episode_external_actions(spec);
        episode.failure_reason = spec.failure_reason.map(str::to_string);
        episode.reflection = Some(episode_reflection(spec));
        episode.reasoning_summary = Some(episode_reasoning_summary(spec));
        episode.hdc_fingerprint = Some(format!("seed-hdc-{}", spec.task_id));
        episode.headline = spec.success;
        episode.prompt_composition = Some(json!({
            "source": "seed",
            "sections": prompt_sections_json(spec, true),
        }));
        episode
            .extra
            .insert("source".to_string(), Value::String("seed".to_string()));
        episode.extra.insert(
            "plan_id".to_string(),
            Value::String(spec.plan_id.to_string()),
        );
        episode
            .extra
            .insert("domain".to_string(), Value::String(spec.domain.to_string()));
        episode.extra.insert(
            "task_title".to_string(),
            Value::String(spec.title.to_string()),
        );
        episode.extra.insert(
            "category".to_string(),
            Value::String(spec.category.to_string()),
        );
        // The attempt it records, as a run's episodes name theirs (S01).
        episode.extra.insert(
            "attempt_key".to_string(),
            Value::String(demo_attempt_key(spec, 1).attempt_key()),
        );
        episodes.push(episode);
    }

    episodes.sort_by_key(|episode| episode.timestamp);
    episodes
}

/// The episodes of the demo's retries: a task that failed was retried once
/// and failed again ([`build_attempt_ledger`]), and each attempt has an
/// episode. A retry's is its first attempt's, ten minutes on.
fn build_retry_episodes(specs: &[DemoTaskSpec], first_attempts: &[Episode]) -> Vec<Episode> {
    let mut retries = Vec::new();
    for spec in specs {
        let Some(first) = first_attempts
            .iter()
            .find(|episode| episode.task_id == spec.task_id)
        else {
            continue;
        };
        for attempt in 2..=demo_attempts(spec) {
            let later = Duration::minutes(i64::from(attempt - 1) * 10);
            let mut episode = first.clone();
            episode.id = format!("{}-{attempt}", first.id);
            episode.episode_id = episode.id.clone();
            episode.timestamp += later;
            episode.started_at += later;
            episode.completed_at += later;
            episode.extra.insert(
                "attempt_key".to_string(),
                Value::String(demo_attempt_key(spec, attempt).attempt_key()),
            );
            retries.push(episode);
        }
    }
    retries
}

/// Two efficiency rows per task, each keyed by its attempt: the primary
/// turn is the task's first attempt, and the follow-up its last, another
/// turn of an attempt that passed or the retry of one that failed.
fn build_efficiency_events(
    specs: &[DemoTaskSpec],
    model_pool: &[String],
    now: DateTime<Utc>,
) -> Vec<AttemptKeyed<AgentEfficiencyEvent>> {
    let mut events = Vec::with_capacity(specs.len() * 2);
    for (index, spec) in specs.iter().enumerate() {
        let primary_model = model_for_slot(model_pool, spec.model_slot);
        let secondary_model = model_for_slot(model_pool, spec.model_slot + 1);
        let primary_time = seeded_datetime(now, spec.age_hours, spec.age_minutes);
        let secondary_time =
            seeded_datetime(now, spec.age_hours.saturating_sub(1), spec.age_minutes + 17);
        let primary = build_efficiency_event(spec, index, &primary_model, primary_time, true);
        let followup =
            build_efficiency_event(spec, index + 100, &secondary_model, secondary_time, false);

        events.push(AttemptKeyed {
            attempt_key: demo_attempt_key(spec, 1).attempt_key(),
            row: primary,
        });
        events.push(AttemptKeyed {
            attempt_key: demo_attempt_key(spec, demo_attempts(spec)).attempt_key(),
            row: followup,
        });
    }

    events.sort_by(|left, right| left.row.timestamp.cmp(&right.row.timestamp));
    events
}

/// The demo run's attempt ledger: an open line and a verdict for each
/// attempt. A task that failed was retried once, and failed again.
fn build_attempt_ledger(
    specs: &[DemoTaskSpec],
    model_pool: &[String],
    now: DateTime<Utc>,
) -> Vec<SeededAttemptLine> {
    let mut lines = Vec::with_capacity(specs.len() * 4);
    let mut seq = 0;
    for spec in specs {
        let model = model_for_slot(model_pool, spec.model_slot);
        let completed_at =
            seeded_datetime(now, spec.age_hours.saturating_sub(1), spec.age_minutes + 58);
        let wall_ms = wall_time_for_task(spec);
        let attempts = demo_attempts(spec);
        let outcome = if spec.success {
            AttemptOutcome::Passed
        } else {
            AttemptOutcome::GateFailed
        };
        for attempt in 1..=attempts {
            // An attempt settles ten minutes before the retry that follows it.
            let later_attempts = i64::from(attempts - attempt);
            let settled_at = completed_at - Duration::minutes(later_attempts * 10);
            let started_at = settled_at - Duration::milliseconds(wall_ms as i64);
            let started_ms = started_at.timestamp_millis();
            let key = demo_attempt_key(spec, attempt);
            let identity = AttemptIdentity::new(&key);

            let mut open = AttemptOpenRecord::new(identity.clone(), started_ms);
            open.role = Some(spec.role.label().to_string());
            seq += 1;
            let open = stamped(open, seq, started_at);
            lines.push(SeededAttemptLine::Open(Box::new(open)));

            let mut verdict = AttemptVerdictRecord::settle(identity, outcome, false);
            verdict.timing.attempt_started_at = Some(started_ms);
            verdict.timing.settled_at = Some(settled_at.timestamp_millis());
            verdict.executed.provider = Some(backend_for_model(&model).to_string());
            verdict.executed.model_requested = Some(model.clone());
            verdict.executed.model_dispatched = Some(model.clone());
            let cached = attempt_cache_tokens(spec);
            verdict.usage.tokens_in = Some(attempt_input_tokens(spec).saturating_sub(cached));
            verdict.usage.tokens_out = Some(attempt_output_tokens(spec));
            verdict.usage.tokens_cache_read = Some(cached);
            verdict.cost.api_equiv_usd = Some(attempt_cost(spec));
            verdict.cost.source = CostSource::Mock;
            seq += 1;
            let verdict = stamped(verdict, seq, settled_at);
            lines.push(SeededAttemptLine::Verdict(Box::new(verdict)));
        }
    }
    lines
}

/// How many attempts the demo run made at `spec`'s task: one when it
/// passed, and two when it failed, was retried and failed again.
fn demo_attempts(spec: &DemoTaskSpec) -> u32 {
    if spec.success { 1 } else { 2 }
}

/// The key of attempt `attempt` at `spec`'s task in the demo run, which its
/// ledger lines and its efficiency, cost and episode rows carry (gap-ad93ce).
fn demo_attempt_key(spec: &DemoTaskSpec, attempt: u32) -> AttemptKey {
    AttemptKey::new(DEMO_RUN_ID, spec.plan_id, spec.task_id, attempt)
}

/// The demo run's `learn/costs.jsonl`: a row for each attempt that settled
/// in `ledger`, keyed by it and costed as its verdict is.
fn build_cost_rows(
    specs: &[DemoTaskSpec],
    ledger: &[SeededAttemptLine],
) -> Vec<AttemptKeyed<CostRecord>> {
    let mut rows = Vec::new();
    for line in ledger {
        let SeededAttemptLine::Verdict(settled) = line else {
            continue;
        };
        let verdict = &settled.record;
        let identity = &verdict.identity;
        let Some(spec) = specs.iter().find(|spec| spec.task_id == identity.task_id) else {
            continue;
        };
        let (executed, usage, cost) = (&verdict.executed, &verdict.usage, &verdict.cost);
        let row = CostRecord {
            timestamp: settled.ts.clone(),
            model: executed.model_dispatched.clone().unwrap_or_default(),
            provider: executed.provider.clone().unwrap_or_default(),
            role: spec.role.label().to_string(),
            plan_id: identity.plan_id.clone(),
            task_id: identity.task_id.clone(),
            complexity_band: spec.complexity_band.to_string(),
            input_tokens: usage.tokens_in.unwrap_or_default(),
            output_tokens: usage.tokens_out.unwrap_or_default(),
            cached_tokens: usage.tokens_cache_read.unwrap_or_default(),
            cost_usd: cost.api_equiv_usd.unwrap_or_default(),
            duration_ms: wall_time_for_task(spec),
            success: verdict.outcome == AttemptOutcome::Passed,
            session_id: String::new(),
            cost_source: cost.source,
            priced: Some(true),
            api_equiv_usd: cost.api_equiv_usd,
            price_snapshot_id: cost.price_snapshot_id.clone(),
        };
        rows.push(AttemptKeyed {
            attempt_key: identity.attempt_key.clone(),
            row,
        });
    }
    rows
}

/// `record` as the line a run's telemetry writer appends at `at`.
fn stamped<R: TelemetryRecord>(record: R, seq: u64, at: DateTime<Utc>) -> Stamped<R> {
    Stamped {
        schema_version: R::SCHEMA.to_string(),
        record_id: record.record_id(),
        seq,
        ts: at.to_rfc3339_opts(SecondsFormat::Millis, true),
        record,
    }
}

/// Whether a run under `.roko/runs` already has attempt records.
fn has_attempt_records(layout: &RokoLayout) -> bool {
    std::fs::read_dir(layout.runs_dir()).is_ok_and(|entries| {
        entries
            .flatten()
            .any(|entry| RunFile::Attempts.path_in(&entry.path()).exists())
    })
}

fn build_cfactor_snapshots(specs: &[DemoTaskSpec], now: DateTime<Utc>) -> Vec<CFactor> {
    let mut snapshots = Vec::with_capacity(3);

    snapshots.push(build_cfactor_snapshot(
        seeded_datetime(now, 144, 0),
        2,
        0.54,
        CFactorComponents {
            gate_pass_rate: 0.50,
            cost_efficiency: 0.44,
            speed: 0.49,
            information_flow_rate: 0.42,
            first_try_rate: 0.48,
            knowledge_growth: 0.34,
            knowledge_integration_rate: 0.20,
            hdc_diversity: 0.33,
            convergence_velocity: 0.22,
            turn_taking_equality: 0.64,
            social_perceptiveness: 0.39,
        },
        vec![
            AgentCFactorContribution {
                agent_id: format!("seed-{}-{}", specs[0].role.label(), specs[0].task_id),
                episode_count: 1,
                without_agent_overall: 0.46,
                contribution_score: 0.08,
            },
            AgentCFactorContribution {
                agent_id: format!("seed-{}-{}", specs[1].role.label(), specs[1].task_id),
                episode_count: 1,
                without_agent_overall: 0.58,
                contribution_score: -0.04,
            },
        ],
        vec![CollectivePathology::Groupthink {
            diversity_score: 0.41,
        }],
    ));

    snapshots.push(build_cfactor_snapshot(
        seeded_datetime(now, 72, 0),
        4,
        0.63,
        CFactorComponents {
            gate_pass_rate: 0.62,
            cost_efficiency: 0.57,
            speed: 0.60,
            information_flow_rate: 0.55,
            first_try_rate: 0.56,
            knowledge_growth: 0.49,
            knowledge_integration_rate: 0.39,
            hdc_diversity: 0.48,
            convergence_velocity: 0.41,
            turn_taking_equality: 0.71,
            social_perceptiveness: 0.50,
        },
        vec![
            AgentCFactorContribution {
                agent_id: format!("seed-{}-{}", specs[2].role.label(), specs[2].task_id),
                episode_count: 1,
                without_agent_overall: 0.58,
                contribution_score: 0.05,
            },
            AgentCFactorContribution {
                agent_id: format!("seed-{}-{}", specs[3].role.label(), specs[3].task_id),
                episode_count: 1,
                without_agent_overall: 0.67,
                contribution_score: -0.04,
            },
        ],
        vec![CollectivePathology::Cascade {
            trigger_agent: format!("seed-{}-{}", specs[1].role.label(), specs[1].task_id),
            affected_count: 1,
        }],
    ));

    snapshots.push(build_cfactor_snapshot(
        seeded_datetime(now, 4, 0),
        5,
        0.74,
        CFactorComponents {
            gate_pass_rate: 0.75,
            cost_efficiency: 0.70,
            speed: 0.72,
            information_flow_rate: 0.69,
            first_try_rate: 0.73,
            knowledge_growth: 0.61,
            knowledge_integration_rate: 0.56,
            hdc_diversity: 0.66,
            convergence_velocity: 0.60,
            turn_taking_equality: 0.79,
            social_perceptiveness: 0.66,
        },
        vec![
            AgentCFactorContribution {
                agent_id: format!("seed-{}-{}", specs[0].role.label(), specs[0].task_id),
                episode_count: 1,
                without_agent_overall: 0.69,
                contribution_score: 0.05,
            },
            AgentCFactorContribution {
                agent_id: format!("seed-{}-{}", specs[4].role.label(), specs[4].task_id),
                episode_count: 1,
                without_agent_overall: 0.68,
                contribution_score: 0.06,
            },
        ],
        Vec::new(),
    ));

    snapshots
}

fn build_knowledge_seeds(
    specs: &[DemoTaskSpec],
    episode_ids: &[String],
    model_pool: &[String],
    now: DateTime<Utc>,
) -> Vec<KnowledgeSeedRecord> {
    let mut seeds = Vec::with_capacity(specs.len());
    for spec in specs {
        let source_episodes = episode_ids_for_indices(episode_ids, spec.seed_source_indices);
        let source_episode = source_episodes
            .first()
            .cloned()
            .unwrap_or_else(|| format!("seed-episode-{}", spec.task_id));
        let model = model_for_slot(model_pool, spec.model_slot);
        seeds.push(KnowledgeSeedRecord {
            schema_version: RUNTIME_FEEDBACK_SCHEMA_VERSION,
            seed_id: format!("seed-knowledge-seed-{}", spec.task_id),
            created_at: seeded_datetime(
                now,
                spec.age_hours.saturating_sub(1),
                spec.age_minutes + 35,
            )
            .to_rfc3339(),
            kind: spec.seed_kind.as_str().to_string(),
            content: spec.seed_content.to_string(),
            confidence: spec.seed_confidence,
            confidence_weight: spec.seed_confidence_weight,
            source_episodes: source_episodes.clone(),
            source_model: Some(model),
            model_generality: spec.seed_model_generality,
            tags: spec
                .seed_tags
                .iter()
                .map(|tag| (*tag).to_string())
                .collect(),
            plan_id: spec.plan_id.to_string(),
            task_id: spec.task_id.to_string(),
            evidence: vec![KnowledgeSeedEvidence {
                source_type: "episode".to_string(),
                source_id: source_episode,
                outcome: if spec.success {
                    "passed".to_string()
                } else {
                    "failed".to_string()
                },
                weight: spec.seed_confidence_weight.abs().min(1.0),
            }],
            metadata: json!({
                "source": "seed",
                "domain": spec.domain,
                "task_title": spec.title,
            }),
        });
    }

    seeds.sort_by(|left, right| left.created_at.cmp(&right.created_at));
    seeds
}

fn build_knowledge_entries(
    specs: &[DemoKnowledgeEntrySpec],
    episode_ids: &[String],
    model_pool: &[String],
    now: DateTime<Utc>,
) -> Vec<KnowledgeEntry> {
    let mut entries = Vec::with_capacity(specs.len());
    for spec in specs {
        let source_episodes = episode_ids_for_indices(episode_ids, spec.source_indices);
        let source_model = model_for_slot(model_pool, spec.source_model_slot);
        let created_at = seeded_datetime(now, spec.age_hours, 0);
        let mut entry = KnowledgeEntry::default();
        entry.id = spec.id.to_string();
        entry.kind = spec.kind;
        entry.source = Some("seed".to_string());
        entry.content = spec.content.to_string();
        entry.confidence = spec.confidence;
        entry.confidence_weight = spec.confidence_weight;
        entry.source_episodes = source_episodes.clone();
        entry.tags = spec
            .tags
            .iter()
            .map(|tag| (*tag).to_string())
            .chain(std::iter::once(spec.topic.to_string()))
            .collect();
        entry.source_model = Some(source_model);
        entry.model_generality = spec.model_generality;
        entry.created_at = created_at;
        entry.half_life_days = spec.kind.default_half_life_days();
        entry.tier = spec.tier;
        entry.refuted_insight_id = spec.refuted_insight_id.map(str::to_string);
        entry.refutation_evidence = spec.refutation_evidence.map(str::to_string);
        entry.confirmation_count = spec.confirmation_count;
        entry.distinct_contexts = source_episodes
            .iter()
            .map(|episode_id| format!("{}:{episode_id}", spec.id))
            .collect();
        entry.balance = spec.balance;
        entry.catalytic_score = spec.catalytic_score;
        entries.push(entry);
    }

    entries.sort_by(|left, right| left.created_at.cmp(&right.created_at));
    entries
}

fn build_cascade_snapshot(
    specs: &[DemoTaskSpec],
    model_pool: &[String],
    now: DateTime<Utc>,
) -> SeededCascadeSnapshot {
    let mut observations = Vec::new();
    let mut confidence_counts: BTreeMap<String, SeededCascadeModelStats> = BTreeMap::new();

    let demo_observations = demo_router_observations(specs, model_pool, now);
    for observation in demo_observations {
        let stats = confidence_counts
            .entry(observation.model.clone())
            .or_default();
        stats.trials += 1;
        if observation.passed {
            stats.successes += 1;
        }
        observations.push(observation);
    }
    observations.sort_by_key(|observation| observation.timestamp);

    let mut model_slugs = Vec::new();
    let mut seen_models = HashSet::new();
    for observation in &observations {
        if seen_models.insert(observation.model.clone()) {
            model_slugs.push(observation.model.clone());
        }
    }

    SeededCascadeSnapshot {
        source: "seed",
        model_slugs,
        role_table: demo_role_table(model_pool),
        confidence_stats: confidence_counts,
        total_observations: observations.len() as u64,
        observations,
    }
}

fn demo_router_observations(
    specs: &[DemoTaskSpec],
    model_pool: &[String],
    now: DateTime<Utc>,
) -> Vec<SeededCascadeObservation> {
    let names = [
        "auth",
        "routing",
        "cache",
        "observability",
        "deployment",
        "docs",
        "analysis",
        "planning",
        "triage",
        "validation",
        "handoff",
        "followup",
    ];
    let mut observations = Vec::new();
    for (index, domain) in names.iter().enumerate() {
        let spec = &specs[index % specs.len()];
        let model = model_for_slot(model_pool, (spec.model_slot + index) % model_pool.len());
        let passed = match index {
            1 | 6 | 10 => false,
            _ => true,
        };
        observations.push(SeededCascadeObservation {
            source: "seed",
            domain: (*domain).to_string(),
            task_id: spec.task_id.to_string(),
            role: alternate_role(spec.role).label().to_string(),
            model,
            passed,
            confidence: match passed {
                true => 0.72 + (index as f64 * 0.01),
                false => 0.41 + (index as f64 * 0.01),
            }
            .min(0.95),
            timestamp: seeded_datetime(now, 6 + index as i64 / 2, (index as i64 * 11) % 60),
        });
    }
    observations
}

fn demo_role_table(model_pool: &[String]) -> HashMap<AgentRole, String> {
    let roles = [
        AgentRole::Conductor,
        AgentRole::Strategist,
        AgentRole::Implementer,
        AgentRole::Researcher,
        AgentRole::Auditor,
    ];
    let mut table = HashMap::new();
    for (index, role) in roles.into_iter().enumerate() {
        table.insert(role, model_for_slot(model_pool, index));
    }
    table
}

fn build_efficiency_event(
    spec: &DemoTaskSpec,
    event_index: usize,
    model: &str,
    timestamp: DateTime<Utc>,
    primary: bool,
) -> AgentEfficiencyEvent {
    let prompt_sections = prompt_sections_for_task(spec, primary);
    let tool_calls = tool_calls_for_task(spec, primary);
    let total_prompt_tokens: u64 = prompt_sections.iter().map(|section| section.tokens).sum();
    let input_tokens = efficiency_input_tokens(spec, primary);
    let output_tokens = efficiency_output_tokens(spec, primary);
    let cache_read_tokens = efficiency_cache_read_tokens(spec, primary);
    let cache_write_tokens = efficiency_cache_write_tokens(spec, primary);
    let cost_usd = efficiency_cost_usd(spec, primary);

    AgentEfficiencyEvent {
        agent_id: format!(
            "seed-{}-{}-{}-{}",
            spec.role.label(),
            spec.task_id,
            if primary { "primary" } else { "followup" },
            event_index
        ),
        role: if primary {
            spec.role.label().to_string()
        } else {
            alternate_role(spec.role).label().to_string()
        },
        backend: backend_for_model(model).to_string(),
        model: model.to_string(),
        plan_id: spec.plan_id.to_string(),
        task_id: spec.task_id.to_string(),
        attempt_id: format!(
            "seed-{}-{}-{}-{}",
            spec.role.label(),
            spec.task_id,
            if primary { "primary" } else { "followup" },
            event_index
        ),
        input_tokens,
        output_tokens,
        reasoning_tokens: if primary { 96 } else { 64 },
        cache_read_tokens,
        cache_write_tokens,
        cost_usd,
        cost_usd_without_cache: cost_usd + if primary { 0.06 } else { 0.04 },
        api_equiv_usd: None,
        price_snapshot_id: None,
        prompt_sections,
        total_prompt_tokens,
        system_prompt_tokens: if primary { 240 } else { 200 },
        tools_available: 6,
        tools_used: if primary { 2 } else { 1 },
        tool_calls,
        wall_time_ms: if primary { 1_780 } else { 980 },
        duration_ms: if primary { 1_780 } else { 980 },
        time_to_first_token_ms: if primary { 420 } else { 260 },
        was_warm_start: !primary,
        iteration: if primary { 1 } else { 2 },
        turn_number: 0,
        is_final_turn: true,
        // A failed task's follow-up is its retry, which failed as well.
        gate_passed: Some(spec.success),
        outcome: if !spec.success {
            "blocked".to_string()
        } else if primary {
            "pass".to_string()
        } else {
            "recovered".to_string()
        },
        gate_errors: if !spec.success {
            vec![format!("seed-{}-{}", spec.task_id, spec.gate)]
        } else {
            Vec::new()
        },
        model_used: model.to_string(),
        frequency: if spec.success {
            OperatingFrequency::Theta
        } else if primary {
            OperatingFrequency::Delta
        } else {
            OperatingFrequency::Gamma
        },
        strategy_attempted: if primary {
            if spec.success {
                "none".to_string()
            } else {
                "retry_with_smaller_context".to_string()
            }
        } else {
            "resume_after_review".to_string()
        },
        timestamp: timestamp.to_rfc3339(),
    }
}

fn build_cfactor_snapshot(
    computed_at: DateTime<Utc>,
    episode_count: usize,
    overall: f64,
    components: CFactorComponents,
    agent_contributions: Vec<AgentCFactorContribution>,
    pathologies: Vec<CollectivePathology>,
) -> CFactor {
    CFactor {
        overall,
        components,
        agent_contributions,
        pathologies,
        computed_at,
        episode_count,
    }
}

fn prompt_sections_for_task(spec: &DemoTaskSpec, primary: bool) -> Vec<PromptSectionMeta> {
    let base = if primary { 1_100 } else { 760 };
    vec![
        PromptSectionMeta {
            name: "workspace_map".to_string(),
            tokens: base / 4,
            priority: 0,
            was_truncated: false,
            was_dropped: false,
        },
        PromptSectionMeta {
            name: "task_brief".to_string(),
            tokens: base / 5,
            priority: 1,
            was_truncated: false,
            was_dropped: false,
        },
        PromptSectionMeta {
            name: format!("{}_notes", spec.domain),
            tokens: base / 6,
            priority: 2,
            was_truncated: !spec.success && !primary,
            was_dropped: false,
        },
        PromptSectionMeta {
            name: "tool_history".to_string(),
            tokens: base / 8,
            priority: 3,
            was_truncated: false,
            was_dropped: primary && !spec.success,
        },
    ]
}

fn prompt_sections_json(spec: &DemoTaskSpec, primary: bool) -> Vec<Value> {
    prompt_sections_for_task(spec, primary)
        .into_iter()
        .map(|section| {
            json!({
                "name": section.name,
                "tokens": section.tokens,
                "priority": section.priority,
                "was_truncated": section.was_truncated,
                "was_dropped": section.was_dropped,
            })
        })
        .collect()
}

fn existing_episode_ids(workdir: &Path) -> Vec<String> {
    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    for path in project_episode_paths(workdir) {
        if !path.exists() {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Ok(episode) = serde_json::from_str::<Episode>(line) else {
                continue;
            };
            let id = if episode.id.trim().is_empty() {
                episode.episode_id
            } else {
                episode.id
            };
            if !id.is_empty() && seen.insert(id.clone()) {
                ids.push(id);
            }
        }
    }
    ids
}

fn episode_ids_for_indices(episode_ids: &[String], indices: &[usize]) -> Vec<String> {
    if episode_ids.is_empty() {
        return indices
            .iter()
            .map(|index| format!("seed-episode-{}", index + 1))
            .collect();
    }
    indices
        .iter()
        .map(|index| episode_ids[index % episode_ids.len()].clone())
        .collect()
}

fn write_jsonl_if_absent<T>(
    path: &Path,
    records: &[T],
    label: &str,
    report: &mut DemoSeedReport,
    mut render: impl FnMut(&T) -> Result<String>,
) -> Result<()>
where
    T: Serialize,
{
    if path.exists() {
        report.record_skipped(format!("{label} (existing file preserved)"));
        return Ok(());
    }

    let payload = jsonl_payload(records, &mut render)?;
    atomic_write_bytes(path, payload.as_bytes())
        .with_context(|| format!("write {}", path.display()))?;
    report.record_seeded(format!("{label}: {} records", records.len()));
    Ok(())
}

fn write_json_if_absent<T>(
    path: &Path,
    value: &T,
    label: &str,
    report: &mut DemoSeedReport,
) -> Result<()>
where
    T: Serialize,
{
    if path.exists() {
        report.record_skipped(format!("{label} (existing file preserved)"));
        return Ok(());
    }

    atomic_write_json(path, value).with_context(|| format!("write {}", path.display()))?;
    report.record_seeded(format!("{label}: 1 snapshot"));
    Ok(())
}

fn jsonl_payload<T>(
    records: &[T],
    render: &mut impl FnMut(&T) -> Result<String>,
) -> Result<String> {
    let mut out = String::new();
    for record in records {
        out.push_str(&render(record)?);
        out.push('\n');
    }
    Ok(out)
}

fn seeded_jsonl_line<T: Serialize>(item: &T) -> Result<String> {
    serde_json::to_string(&SeedEnvelopeRef {
        source: "seed",
        item,
    })
    .context("serialize seeded JSONL line")
}

fn demo_model_pool(config: Option<&RokoConfig>) -> Vec<String> {
    let mut pool = Vec::new();
    let mut seen = HashSet::new();

    if let Some(config) = config {
        push_model(
            &mut pool,
            &mut seen,
            config.agent.default_model.trim().to_string(),
        );
        let mut configured: Vec<String> = config
            .effective_models()
            .values()
            .map(|profile| profile.slug.trim().to_string())
            .filter(|slug| !slug.is_empty())
            .collect();
        configured.sort();
        for model in configured {
            push_model(&mut pool, &mut seen, model);
        }
    }

    for fallback in [
        "claude-sonnet-4-6",
        "claude-haiku-4-5",
        "claude-opus-4-6",
        "gpt-5.4-mini",
        "gemini-2.5-pro",
        "kimi-k2.5",
    ] {
        push_model(&mut pool, &mut seen, fallback.to_string());
    }

    if pool.is_empty() {
        pool.push("claude-sonnet-4-6".to_string());
    }

    while pool.len() < 5 {
        let next = format!("demo-model-{}", pool.len() + 1);
        push_model(&mut pool, &mut seen, next);
    }

    pool
}

fn push_model(pool: &mut Vec<String>, seen: &mut HashSet<String>, model: String) {
    let model = model.trim();
    if model.is_empty() {
        return;
    }
    if seen.insert(model.to_string()) {
        pool.push(model.to_string());
    }
}

fn model_for_slot(model_pool: &[String], slot: usize) -> String {
    let index = slot % model_pool.len();
    model_pool[index].clone()
}

#[allow(deprecated)]
fn backend_for_model(model: &str) -> &'static str {
    match AgentBackend::from_model(model) {
        AgentBackend::Claude => "claude",
        AgentBackend::Codex => "codex",
        AgentBackend::Cursor => "cursor",
        AgentBackend::Ollama => "ollama",
        AgentBackend::OpenAi => "openai",
        AgentBackend::Perplexity => "perplexity",
        AgentBackend::Cerebras => "cerebras",
        AgentBackend::Hermes => "hermes",
        AgentBackend::OpenClaw => "openclaw",
        _ => "unknown",
    }
}

fn alternate_role(role: AgentRole) -> AgentRole {
    match role {
        AgentRole::Conductor => AgentRole::Strategist,
        AgentRole::Strategist => AgentRole::Implementer,
        AgentRole::Implementer => AgentRole::QuickReviewer,
        AgentRole::QuickReviewer => AgentRole::Implementer,
        AgentRole::Researcher => AgentRole::Auditor,
        AgentRole::Auditor => AgentRole::Researcher,
        AgentRole::Scribe => AgentRole::QuickReviewer,
        AgentRole::Critic => AgentRole::Implementer,
        AgentRole::AutoFixer => AgentRole::QuickReviewer,
        AgentRole::Refactorer => AgentRole::Implementer,
        AgentRole::PrePlanner => AgentRole::Strategist,
        AgentRole::DocVerifier => AgentRole::Scribe,
        AgentRole::IntegrationTester => AgentRole::Auditor,
        AgentRole::MergeResolver => AgentRole::Implementer,
        AgentRole::TerminalValidator => AgentRole::Auditor,
        AgentRole::LifecycleTester => AgentRole::Researcher,
        AgentRole::SpecDriftDetector => AgentRole::Auditor,
        AgentRole::RegressionDetector => AgentRole::Researcher,
        AgentRole::PerformanceSentinel => AgentRole::Strategist,
        AgentRole::CoverageTracker => AgentRole::QuickReviewer,
        AgentRole::PlanLifecycleManager => AgentRole::Conductor,
        AgentRole::CrossSystemTester => AgentRole::Researcher,
        AgentRole::ErrorDiagnoser => AgentRole::Researcher,
        AgentRole::DependencyValidator => AgentRole::Auditor,
        AgentRole::PatternExtractor => AgentRole::Scribe,
        AgentRole::SnapshotComparator => AgentRole::Auditor,
        AgentRole::FullLoopValidator => AgentRole::Conductor,
        _ => AgentRole::Implementer,
    }
}

fn seeded_datetime(now: DateTime<Utc>, hours_ago: i64, minutes_ago: i64) -> DateTime<Utc> {
    now - Duration::hours(hours_ago.max(0)) - Duration::minutes(minutes_ago.max(0))
}

fn wall_time_for_task(spec: &DemoTaskSpec) -> u64 {
    match spec.complexity_band {
        "simple" => 1_380,
        "standard" => 1_920,
        "complex" => 2_880,
        _ => 1_640,
    }
}

fn episode_input_tokens(spec: &DemoTaskSpec) -> u64 {
    match spec.complexity_band {
        "simple" => 860,
        "standard" => 1_240,
        "complex" => 1_620,
        _ => 1_020,
    }
}

fn episode_output_tokens(spec: &DemoTaskSpec) -> u64 {
    match spec.complexity_band {
        "simple" => 260,
        "standard" => 340,
        "complex" => 540,
        _ => 300,
    }
}

fn episode_cache_tokens(spec: &DemoTaskSpec) -> u64 {
    match spec.complexity_band {
        "simple" => 180,
        "standard" => 240,
        "complex" => 360,
        _ => 210,
    }
}

fn episode_cache_write_tokens(spec: &DemoTaskSpec) -> u64 {
    match spec.complexity_band {
        "simple" => 40,
        "standard" => 70,
        "complex" => 110,
        _ => 55,
    }
}

fn episode_cost(spec: &DemoTaskSpec, cached: bool) -> f64 {
    let base = match spec.complexity_band {
        "simple" => 0.12,
        "standard" => 0.19,
        "complex" => 0.31,
        _ => 0.16,
    };
    if cached { base } else { base + 0.04 }
}

fn efficiency_input_tokens(spec: &DemoTaskSpec, primary: bool) -> u64 {
    let base = match spec.complexity_band {
        "simple" => 920,
        "standard" => 1_220,
        "complex" => 1_680,
        _ => 1_080,
    };
    if primary {
        base
    } else {
        base.saturating_sub(260)
    }
}

fn efficiency_output_tokens(spec: &DemoTaskSpec, primary: bool) -> u64 {
    let base = match spec.complexity_band {
        "simple" => 300,
        "standard" => 360,
        "complex" => 560,
        _ => 320,
    };
    if primary {
        base
    } else {
        base.saturating_sub(120)
    }
}

fn efficiency_cache_read_tokens(spec: &DemoTaskSpec, primary: bool) -> u64 {
    let base = match spec.complexity_band {
        "simple" => 140,
        "standard" => 220,
        "complex" => 420,
        _ => 160,
    };
    if primary { base } else { base / 2 }
}

fn efficiency_cache_write_tokens(spec: &DemoTaskSpec, primary: bool) -> u64 {
    let base = match spec.complexity_band {
        "simple" => 60,
        "standard" => 88,
        "complex" => 120,
        _ => 70,
    };
    if primary { base } else { base / 2 }
}

fn efficiency_cost_usd(spec: &DemoTaskSpec, primary: bool) -> f64 {
    let base = match spec.complexity_band {
        "simple" => 0.04,
        "standard" => 0.08,
        "complex" => 0.15,
        _ => 0.06,
    };
    if primary { base } else { base * 0.65 }
}

fn attempt_input_tokens(spec: &DemoTaskSpec) -> u64 {
    match spec.complexity_band {
        "simple" => 1_020,
        "standard" => 1_330,
        "complex" => 1_920,
        _ => 1_160,
    }
}

fn attempt_output_tokens(spec: &DemoTaskSpec) -> u64 {
    match spec.complexity_band {
        "simple" => 280,
        "standard" => 360,
        "complex" => 620,
        _ => 320,
    }
}

fn attempt_cache_tokens(spec: &DemoTaskSpec) -> u64 {
    match spec.complexity_band {
        "simple" => 200,
        "standard" => 280,
        "complex" => 460,
        _ => 220,
    }
}

fn attempt_cost(spec: &DemoTaskSpec) -> f64 {
    match spec.complexity_band {
        "simple" => 0.07,
        "standard" => 0.11,
        "complex" => 0.20,
        _ => 0.09,
    }
}

fn tool_calls_for_task(spec: &DemoTaskSpec, primary: bool) -> Vec<ToolCallMeta> {
    let mut calls = Vec::new();
    calls.push(ToolCallMeta {
        tool_name: "Read".to_string(),
        duration_ms: if primary { 230 } else { 180 },
        result_tokens: if primary { 420 } else { 260 },
        succeeded: Some(true),
        advanced_task: true,
        was_redundant: false,
        error_category: None,
    });
    calls.push(ToolCallMeta {
        tool_name: if spec.success { "Write" } else { "Search" }.to_string(),
        duration_ms: if primary { 320 } else { 140 },
        result_tokens: if spec.success { 180 } else { 110 },
        succeeded: Some(spec.success || !primary),
        advanced_task: spec.success || !primary,
        was_redundant: !primary && !spec.success,
        error_category: if spec.success && !primary {
            Some("duplicate_patch".to_string())
        } else if !spec.success && primary {
            Some("validation_gap".to_string())
        } else {
            None
        },
    });
    calls
}

fn episode_gate_verdicts(spec: &DemoTaskSpec) -> Vec<EpisodeGateVerdict> {
    let mut verdicts = vec![EpisodeGateVerdict::new("compile", true)];
    if spec.success {
        verdicts.push(EpisodeGateVerdict::new(spec.gate, true));
    } else {
        verdicts.push(
            EpisodeGateVerdict::new(spec.gate, false)
                .with_signature(format!("seed-{}-{}", spec.task_id, spec.gate)),
        );
    }
    verdicts
}

fn episode_external_actions(spec: &DemoTaskSpec) -> Vec<Value> {
    if spec.success {
        vec![
            json!({"kind": "write", "path": format!("src/{}/patch.rs", spec.domain)}),
            json!({"kind": "validate", "gate": spec.gate}),
        ]
    } else {
        vec![
            json!({"kind": "inspect", "path": format!("src/{}/mod.rs", spec.domain)}),
            json!({"kind": "retry", "strategy": "smaller_context"}),
        ]
    }
}

fn episode_reflection(spec: &DemoTaskSpec) -> String {
    if spec.success {
        format!(
            "The {} task converged after the {} gate passed.",
            spec.domain, spec.gate
        )
    } else {
        format!(
            "The {} task needed a second pass because the {} gate reported drift.",
            spec.domain, spec.gate
        )
    }
}

fn episode_reasoning_summary(spec: &DemoTaskSpec) -> String {
    if spec.success {
        format!(
            "Chose the {} model path and kept the prompt tight.",
            spec.domain
        )
    } else {
        format!(
            "Reviewed the {} failure and rerouted to a safer repair path.",
            spec.domain
        )
    }
}

#[cfg(test)]
mod tests {
    use roko_core::metric::compute_headlines;
    use roko_learn::telemetry::report::{LegacyRows, RunRecords, check};

    use super::*;

    #[test]
    fn demo_seed_feeds_the_attempt_ledger_dashboard() {
        let workdir = tempfile::tempdir().expect("tempdir");
        let report = seed_demo_workspace(workdir.path(), None).expect("seed");
        let ledger = "attempt ledger: 14 records".to_string();
        assert!(report.seeded_groups.contains(&ledger), "{report:?}");
        let layout = RokoLayout::for_project(workdir.path());
        assert!(
            !layout.memory_dir().join("task-metrics.jsonl").exists(),
            "nothing reads task-metrics.jsonl"
        );

        let run_dir = layout.run_dir(DEMO_RUN_ID);
        let run = RunRecords::load(&run_dir).expect("the demo run");
        assert!(run.invalid.is_empty(), "{:?}", run.invalid);
        assert_eq!((run.opens.len(), run.verdicts.len()), (7, 7));

        // Five tasks: three passed first time, two failed twice.
        let metrics = crate::tui::dashboard::attempt_ledger_metrics(workdir.path());
        assert_eq!(metrics.len(), 7, "{metrics:?}");
        for metric in &metrics {
            assert!(!metric.model.is_empty(), "{metric:?}");
            assert!(!metric.role.is_empty(), "{metric:?}");
            assert!(metric.cost_usd > 0.0, "{metric:?}");
            assert!(metric.cache_hit_rate > 0.0, "{metric:?}");
        }
        let headlines = compute_headlines(&metrics);
        assert_eq!(headlines.n_plans, 5, "{headlines:?}");
        assert!(
            (headlines.first_attempt_pass_rate - 0.6).abs() < 1e-9,
            "{headlines:?}"
        );
        assert!(
            (headlines.avg_iterations_per_plan - 1.4).abs() < 1e-9,
            "{headlines:?}"
        );

        let again = seed_demo_workspace(workdir.path(), None).expect("reseed");
        let kept = "attempt ledger (existing runs preserved)".to_string();
        assert!(again.skipped_groups.contains(&kept), "{again:?}");
    }

    /// gap-ad93ce: each of the demo run's seven attempts has an efficiency
    /// row, a cost row and an episode keyed by it, and no row names an
    /// attempt the run lacks, so `roko learn telemetry check` passes.
    #[test]
    fn demo_workspace_telemetry_check_reports_no_coverage_gaps() {
        let workdir = tempfile::tempdir().expect("tempdir");
        let report = seed_demo_workspace(workdir.path(), None).expect("seed");
        let seeded = &report.seeded_groups;
        for group in ["episodes: 7 records", "cost rows: 7 records"] {
            assert!(seeded.contains(&group.to_string()), "{report:?}");
        }

        let layout = RokoLayout::for_project(workdir.path());
        let run = RunRecords::load(&layout.run_dir(DEMO_RUN_ID)).expect("the demo run");
        let legacy = LegacyRows::load(&layout, DEMO_RUN_ID).expect("the legacy logs");
        let checked = check(&run, &legacy);
        assert_eq!(checked.coverage.len(), 3, "{checked:?}");
        for coverage in &checked.coverage {
            assert_eq!(coverage.verdicts, 7, "{coverage:?}");
            assert_eq!(coverage.joined, 7, "{coverage:?}");
            assert!(coverage.missing.is_empty(), "{coverage:?}");
            assert!(coverage.orphans.is_empty(), "{coverage:?}");
        }
        assert!(checked.failures().is_empty(), "{:?}", checked.failures());
    }
}
