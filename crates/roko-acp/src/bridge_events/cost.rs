//! Cost accounting, budget tracking, efficiency telemetry, and ACP role helpers.

use std::{
    collections::HashSet,
    path::Path,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::Instant,
};

use roko_agent::ModelCallService;
use roko_core::ContentHash;
use roko_core::DaimonPolicy;
use roko_core::agent::{AgentRole, ResolvedModel, resolve_model};
use roko_core::config::schema::RokoConfig;
use roko_core::defaults::DEFAULT_REQUEST_TIMEOUT_MS;
use roko_core::task::{TaskCategory, TaskComplexityBand};
use roko_learn::{
    cost_table::CostTable,
    efficiency::AgentEfficiencyEvent,
    episode_logger::{Episode, EpisodeLogger, Usage as EpUsage},
    model_router::RoutingContext,
};
use roko_neuro::KnowledgeTier;
use tokio::task;
use tracing::{debug, error, info, warn};

use crate::session::AcpSession;
use crate::types::{ClientCapabilities, PermissionAction, StopReason};
use roko_core::tool::ToolPermission;

use super::StreamResult;
use super::experiments::AcpCascadeSelection;

/// Maximum assistant response bytes stored in one history turn.
pub(crate) const MAX_HISTORY_ASSISTANT_BYTES: usize = 10_240;

pub(crate) fn pricing_table() -> &'static CostTable {
    static TABLE: OnceLock<CostTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        CostTable {
            models: std::collections::HashMap::new(),
        }
        .with_defaults()
    })
}

/// Calculate model cost from token counts.
///
/// Returns `None` when the model slug has no pricing row. Unknown pricing
/// stays unknown instead of collapsing to zero.
pub fn calculate_cost_for_model_slug(
    model_slug: &str,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
) -> Option<f64> {
    let pricing = pricing_table().models.get(model_slug)?;
    Some(
        (input_tokens as f64 * pricing.input_per_m / 1_000_000.0)
            + (output_tokens as f64 * pricing.output_per_m / 1_000_000.0)
            + (cache_read_tokens as f64 * pricing.cache_read_per_m / 1_000_000.0),
    )
}

pub(crate) fn calculate_cost_without_cache_for_model_slug(
    model_slug: &str,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
) -> Option<f64> {
    let pricing = pricing_table().models.get(model_slug)?;
    Some(
        (input_tokens as f64 * pricing.input_per_m / 1_000_000.0)
            + (output_tokens as f64 * pricing.output_per_m / 1_000_000.0)
            + (cache_read_tokens as f64 * pricing.input_per_m / 1_000_000.0),
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn append_acp_episode(
    roko_config: &RokoConfig,
    workdir: &Path,
    session: &AcpSession,
    model_key: &str,
    prompt_text: &str,
    workflow_config: &str,
    is_pipeline_dispatch: bool,
    dispatch_started: Instant,
    stream_result: Option<&StreamResult>,
    task_error: Option<&str>,
    stream_error: Option<&str>,
    // When provided, overrides the pricing-table cost calculation with the
    // actual cost reported by the provider (e.g. from `WorkflowRunReport.cost`).
    cost_override: Option<f64>,
    // When cascade routing was used, retain both the selected config key and
    // maturity stage so the decision is inspectable via `roko learn episodes`.
    cascade_selection: Option<&AcpCascadeSelection>,
) {
    let resolved = resolve_model(roko_config, model_key);
    let elapsed = dispatch_started.elapsed();
    let input_hash = ContentHash::of(prompt_text.as_bytes()).to_hex();
    let output_source = stream_result
        .map(|sr| sr.assistant_text.as_str())
        .filter(|text| !text.is_empty())
        .or(task_error)
        .or(stream_error)
        .unwrap_or("");
    let output_hash = ContentHash::of(output_source.as_bytes()).to_hex();
    let mode = session.config_state.agent_mode.clone();
    let mut episode = Episode::new(mode.clone(), session.session_id.clone());

    episode.kind = if is_pipeline_dispatch {
        format!("acp-pipeline-{workflow_config}")
    } else {
        "acp-dispatch".to_string()
    };
    episode.agent_template = mode.clone();
    episode.model = resolved.slug.clone();
    episode.backend = resolved.provider_kind.label().to_string();
    episode.trigger_kind = if is_pipeline_dispatch {
        "acp_pipeline".to_string()
    } else {
        "acp_dispatch".to_string()
    };
    episode.trigger_signal_hash = input_hash.clone();
    episode.input_signal_hash = input_hash;
    episode.output_signal_hash = output_hash;
    episode.episode_id = episode.id.clone();
    episode.duration_secs = elapsed.as_secs_f64();
    let stream_usage = stream_result.and_then(|sr| sr.usage.as_ref());
    let mut usage = EpUsage {
        wall_ms: elapsed.as_millis() as u64,
        ..EpUsage::default()
    };
    if let Some(provider_usage) = stream_usage {
        let input_tokens = provider_usage.input_tokens;
        let output_tokens = provider_usage.output_tokens;
        let cached_read_tokens = provider_usage.cached_read_tokens.unwrap_or(0);
        usage.input_tokens = input_tokens;
        usage.output_tokens = output_tokens;
        usage.cache_read_tokens = cached_read_tokens;
        usage.cache_write_tokens = provider_usage.cached_write_tokens.unwrap_or(0);
        usage.cost_usd = cost_override.unwrap_or_else(|| {
            calculate_cost_for_model_slug(
                &resolved.slug,
                input_tokens,
                output_tokens,
                cached_read_tokens,
            )
            .unwrap_or(0.0)
        });
        usage.cost_usd_without_cache = cost_override.unwrap_or_else(|| {
            calculate_cost_without_cache_for_model_slug(
                &resolved.slug,
                input_tokens,
                output_tokens,
                cached_read_tokens,
            )
            .unwrap_or(usage.cost_usd)
        });
    }
    episode.usage = usage;
    episode.tokens_used = stream_usage.map(|usage| usage.total_tokens).unwrap_or(0);
    episode
        .extra
        .insert("entry_point".to_string(), serde_json::json!("acp"));
    episode
        .extra
        .insert("model".to_string(), serde_json::json!(resolved.slug));
    episode
        .extra
        .insert("mode".to_string(), serde_json::json!(mode));
    episode.extra.insert(
        "session_id".to_string(),
        serde_json::json!(session.session_id.clone()),
    );
    episode
        .extra
        .insert("workflow".to_string(), serde_json::json!(workflow_config));
    episode.extra.insert(
        "provider_kind".to_string(),
        serde_json::json!(resolved.provider_kind.label()),
    );
    if let Some(selection) = cascade_selection {
        episode.extra.insert(
            "cascade_selected_model".to_string(),
            serde_json::json!(selection.model_key),
        );
        episode.extra.insert(
            "cascade_stage".to_string(),
            serde_json::json!(selection.stage),
        );
    }

    let success = acp_dispatch_succeeded(stream_result, task_error, stream_error);
    episode.success = success;

    if !success {
        let failure_reason = task_error
            .or(stream_error)
            .map(str::to_string)
            .or_else(|| {
                stream_result.map(|sr| match sr.prompt_result.stop_reason {
                    StopReason::Cancelled => "cancelled".to_string(),
                    StopReason::MaxTokens => "max_tokens".to_string(),
                    StopReason::MaxTurnRequests => "max_turn_requests".to_string(),
                    StopReason::Refusal => "refusal".to_string(),
                    StopReason::EndTurn => "unknown failure".to_string(),
                })
            })
            .unwrap_or_else(|| "unknown failure".to_string());
        episode.failure_reason = Some(failure_reason);
    }

    let episodes_path = workdir.join(".roko").join("episodes.jsonl");
    if let Some(parent) = episodes_path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    let logger = EpisodeLogger::new(&episodes_path);
    if let Err(err) = logger.append(&episode).await {
        error!(
            session_id = %session.session_id,
            error = %err,
            "failed to append ACP episode"
        );
    }

    // Spawn background distillation so the knowledge store learns from each ACP interaction.
    let distill_model = roko_config.agent.default_model.clone();
    let distill_caller: Arc<dyn roko_core::foundation::ModelCaller> = Arc::new(
        ModelCallService::new(distill_model)
            .with_config(roko_config.clone())
            .with_working_dir(workdir)
            .with_immune_root(workdir),
    );
    spawn_acp_distillation(workdir, episode, distill_caller);

    // Auto-dream consolidation (opt-in): after enough episodes accumulate,
    // spawn a background dream cycle so patterns are extracted into
    // `.roko/dreams/`.
    maybe_spawn_dream_consolidation(workdir, roko_config);
}

/// Return the number of episodes recorded since the last dream report when an
/// ACP turn should start a dream consolidation, or `None` when it should not.
///
/// ACP dreams are opt-in. With `learning.dreams.trigger_on_acp_episodes` off
/// (the default) this returns `None` without reading the episode log. With it
/// on, a dream is due once `learning.dreams.acp_episode_threshold` episodes
/// have accumulated since the last dream report.
pub(crate) fn acp_dream_due(workdir: &Path, config: &RokoConfig) -> Option<usize> {
    let dreams = &config.learning.dreams;
    if !dreams.trigger_on_acp_episodes {
        return None;
    }

    let episodes_path = workdir.join(".roko").join("episodes.jsonl");
    let dream_dir = workdir.join(".roko").join("dreams");

    // Count episodes since the last dream report.  Both helpers are
    // cheap (file I/O only, no LLM calls) so running them on the
    // current thread is acceptable.
    let last_dream_ts = roko_dreams::runner::load_latest_dream_report(&dream_dir)
        .ok()
        .flatten()
        .map(|r| r.completed_at);

    let text = match std::fs::read_to_string(&episodes_path) {
        Ok(t) => t,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return None,
        Err(err) => {
            debug!(?err, "skipping dream check: could not read episode log");
            return None;
        }
    };

    let episodes_since_dream = match last_dream_ts {
        Some(ts) => text
            .lines()
            .filter_map(|line| serde_json::from_str::<Episode>(line).ok())
            .filter(|ep| ep.timestamp > ts)
            .count(),
        None => text.lines().filter(|line| !line.trim().is_empty()).count(),
    };

    (episodes_since_dream >= dreams.effective_acp_episode_threshold())
        .then_some(episodes_since_dream)
}

/// The dream consolidations ACP sessions run in this process.
static ACP_DREAM_SLOTS: DreamSlots = DreamSlots::new();

/// Counts running dream consolidations against
/// `learning.dreams.max_concurrent`.
///
/// A dream is due on every ACP turn until it writes its report, so without
/// this count each turn would start another one while the first runs.
#[derive(Debug, Default)]
pub(crate) struct DreamSlots {
    running: AtomicUsize,
}

impl DreamSlots {
    pub(crate) const fn new() -> Self {
        Self {
            running: AtomicUsize::new(0),
        }
    }

    /// Take a slot when fewer than `max_concurrent` dreams are running.
    pub(crate) fn try_acquire(&self, max_concurrent: usize) -> Option<DreamSlot<'_>> {
        self.running
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |running| {
                (running < max_concurrent).then_some(running + 1)
            })
            .ok()
            .map(|_| DreamSlot { slots: self })
    }
}

/// A running dream's place in [`DreamSlots`], given back when dropped.
#[derive(Debug)]
pub(crate) struct DreamSlot<'a> {
    slots: &'a DreamSlots,
}

impl Drop for DreamSlot<'_> {
    fn drop(&mut self) {
        self.slots.running.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Claim a dream for an ACP turn: [`acp_dream_due`] reports one is due and
/// `slots` has room under `learning.dreams.max_concurrent`. Returns the
/// episode count and the slot, which the dream holds until it ends.
pub(crate) fn claim_acp_dream<'a>(
    slots: &'a DreamSlots,
    workdir: &Path,
    config: &RokoConfig,
) -> Option<(usize, DreamSlot<'a>)> {
    let episodes_since_dream = acp_dream_due(workdir, config)?;
    let max_concurrent = config.learning.dreams.effective_max_concurrent();
    let Some(slot) = slots.try_acquire(max_concurrent) else {
        debug!(
            episodes_since_dream,
            max_concurrent, "skipping dream consolidation: the running dreams fill max_concurrent"
        );
        return None;
    };
    Some((episodes_since_dream, slot))
}

/// Spawn a background dream consolidation when [`claim_acp_dream`] gets one.
/// This is fire-and-forget: failures are logged but never block the caller.
pub(crate) fn maybe_spawn_dream_consolidation(workdir: &Path, config: &RokoConfig) {
    let Some((episodes_since_dream, slot)) = claim_acp_dream(&ACP_DREAM_SLOTS, workdir, config)
    else {
        return;
    };
    let workdir = workdir.to_path_buf();

    info!(
        episodes_since_dream,
        threshold = config.learning.dreams.effective_acp_episode_threshold(),
        "triggering background dream consolidation"
    );

    let dream_config = roko_dreams::DreamLoopConfig {
        auto_dream: true,
        idle_threshold_mins: 0,
        min_episodes_for_dream: 1,
        schedule: roko_dreams::DreamSchedulePolicy::default(),
        agent: roko_dreams::DreamAgentConfig {
            command: config
                .agent
                .command
                .clone()
                .unwrap_or_else(|| "claude".into()),
            args: Vec::new(),
            model: Some(config.agent.default_model.clone()),
            bare_mode: true,
            effort: "medium".to_string(),
            fallback_model: None,
            timeout_ms: DEFAULT_REQUEST_TIMEOUT_MS,
            env: Vec::new(),
        },
    };

    // Use `consolidate_async` with `tokio::spawn` rather than
    // `consolidate_now` with `spawn_blocking`.  `consolidate_now` calls
    // `block_in_place` internally, which panics when invoked from a
    // `spawn_blocking` thread because blocking threads are not async workers
    // and carry no reactor context to yield from.
    let _handle = tokio::spawn(async move {
        let mut runner = roko_dreams::DreamRunner::new(workdir, dream_config);
        if let Err(err) = runner.consolidate_async().await {
            warn!(?err, "background dream consolidation failed");
        }
        // The dream is over, failed or not: free its slot.
        drop(slot);
    });
}

/// Build the canonical efficiency event used for both learning telemetry and
/// persisted ACP session spend accounting.
pub(crate) fn acp_efficiency_event(
    session_id: &str,
    resolved: &ResolvedModel,
    dispatch_started: Instant,
    stream_result: Option<&StreamResult>,
    succeeded: bool,
    cost_override: Option<f64>,
) -> AgentEfficiencyEvent {
    let elapsed_ms = dispatch_started.elapsed().as_millis() as u64;
    let usage = stream_result.and_then(|sr| sr.usage.as_ref());

    let input_tokens = usage.map_or(0, |u| u.input_tokens);
    let output_tokens = usage.map_or(0, |u| u.output_tokens);
    let cached_read = usage.and_then(|u| u.cached_read_tokens).unwrap_or(0);
    let cached_write = usage.and_then(|u| u.cached_write_tokens).unwrap_or(0);

    let cost_usd = cost_override.unwrap_or_else(|| {
        calculate_cost_for_model_slug(&resolved.slug, input_tokens, output_tokens, cached_read)
            .unwrap_or(0.0)
    });
    let cost_usd_without_cache = cost_override.unwrap_or_else(|| {
        calculate_cost_without_cache_for_model_slug(
            &resolved.slug,
            input_tokens,
            output_tokens,
            cached_read,
        )
        .unwrap_or(cost_usd)
    });

    let outcome = if succeeded { "success" } else { "failure" }.to_string();

    AgentEfficiencyEvent {
        agent_id: session_id.to_string(),
        backend: resolved.provider_kind.label().to_string(),
        model: resolved.slug.clone(),
        model_used: resolved.slug.clone(),
        input_tokens,
        output_tokens,
        cache_read_tokens: cached_read,
        cache_write_tokens: cached_write,
        cost_usd,
        cost_usd_without_cache,
        wall_time_ms: elapsed_ms,
        duration_ms: elapsed_ms,
        outcome,
        timestamp: chrono::Utc::now().to_rfc3339(),
        ..AgentEfficiencyEvent::default()
    }
}

/// Emit an [`AgentEfficiencyEvent`] to `.roko/learn/efficiency.jsonl`.
///
/// This is fire-and-forget: the write is spawned on a blocking thread so it
/// never delays the response stream, and failures are logged but swallowed.
pub(crate) fn emit_acp_efficiency_event(workdir: &Path, event: AgentEfficiencyEvent) {
    let path = workdir.join(".roko").join("learn").join("efficiency.jsonl");

    task::spawn_blocking(move || {
        let line = match serde_json::to_string(&event) {
            Ok(json) => json,
            Err(err) => {
                tracing::warn!(error = %err, "failed to serialize efficiency event");
                return;
            }
        };
        if let Err(err) = roko_core::io::append_jsonl_line(&path, &line) {
            tracing::warn!(error = %err, "failed to write efficiency event");
        }
    });
}

pub(crate) fn acp_role_for_mode(mode: &str) -> AgentRole {
    match mode {
        "plan" => AgentRole::Strategist,
        "research" => AgentRole::Researcher,
        _ => AgentRole::Implementer,
    }
}

/// Intersect the ACP client's session declarations with the selected role's
/// permission ceiling. Interactive allow/always-allow decisions remain a
/// separate per-call gate in `AcpBuiltinToolHandler`.
pub(crate) fn derive_acp_tool_capabilities(
    mode: &str,
    client: &ClientCapabilities,
    has_session_mcp: bool,
    trusted_actions: &HashSet<PermissionAction>,
) -> ToolPermission {
    let role = acp_role_for_mode(mode).tool_permissions();
    let fs = client.fs.as_ref();
    let mcp = client.mcp_servers == Some(true) && has_session_mcp;
    let write = fs.map_or_else(
        || {
            trusted_actions.contains(&PermissionAction::FileCreate)
                || trusted_actions.contains(&PermissionAction::FileEdit)
        },
        |caps| caps.write_text_file,
    );
    let exec = client
        .terminal
        .unwrap_or_else(|| trusted_actions.contains(&PermissionAction::TerminalCommand));
    ToolPermission {
        read: role.read && (fs.is_some_and(|caps| caps.read_text_file) || mcp),
        write: role.write && write,
        exec: role.exec && exec,
        git: role.git
            && client
                .terminal
                .unwrap_or_else(|| trusted_actions.contains(&PermissionAction::GitOperation)),
        network: role.network && mcp,
    }
}

pub(crate) fn acp_routing_context(
    mode: &str,
    prompt: &str,
    effort: &str,
    workdir: &Path,
) -> RoutingContext {
    let _prompt_len = prompt.len();
    let task_category = if mode == "research" {
        TaskCategory::Research
    } else {
        TaskCategory::Implementation
    };

    let role = acp_role_for_mode(mode);

    // T4: Load DaimonState from disk so affect-based routing actually works.
    // Canonical path is .roko/daimon/affect.json; fall back to legacy
    // .roko/state/daimon.json for old workspaces that haven't migrated yet.
    // We read-only — the orchestrator is the sole writer of DaimonState.
    let daimon_policy = {
        let canonical = workdir.join(".roko").join("daimon").join("affect.json");
        let daimon_path = if canonical.exists() {
            canonical
        } else {
            workdir.join(".roko").join("state").join("daimon.json")
        };

        if daimon_path.exists() {
            std::fs::read_to_string(&daimon_path)
                .ok()
                .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
                .and_then(|v| {
                    let confidence = v.get("state")?.get("confidence")?.as_f64()?;
                    let behavioral_state_str = v.get("state")?.get("behavioral_state")?.as_str()?;
                    use roko_core::BehavioralState;
                    let behavioral_state = match behavioral_state_str {
                        "struggling" => BehavioralState::Struggling,
                        "coasting" => BehavioralState::Coasting,
                        "exploring" => BehavioralState::Exploring,
                        "focused" => BehavioralState::Focused,
                        "resting" => BehavioralState::Resting,
                        _ => BehavioralState::Engaged,
                    };
                    Some(DaimonPolicy::new(confidence, behavioral_state))
                })
                .unwrap_or_default()
        } else {
            DaimonPolicy::default()
        }
    };

    RoutingContext {
        task_category,
        complexity: TaskComplexityBand::Standard,
        iteration: 0,
        role,
        crate_familiarity: 0.5,
        has_prior_failure: false,
        conductor_load: 0.0,
        active_agents: 0,
        ready_queue_depth: 0,
        max_queue_wait_hours: 0.0,
        daimon_policy,
        thinking_level: Some(effort.to_owned()).filter(|value| !value.trim().is_empty()),
        temperament: None,
        previous_model: None,
        plan_context_tokens: None,
        tier_thresholds: None,
        cfactor: None,
    }
}

pub(crate) fn acp_dispatch_succeeded(
    stream_result: Option<&StreamResult>,
    task_error: Option<&str>,
    stream_error: Option<&str>,
) -> bool {
    task_error.is_none()
        && stream_error.is_none()
        && stream_result
            .map(|sr| matches!(sr.prompt_result.stop_reason, StopReason::EndTurn))
            .unwrap_or(false)
}

pub(crate) fn truncate_to_title(text: &str, max_len: usize) -> String {
    let trimmed = text.trim();
    // Take first line only.
    let first_line = trimmed.lines().next().unwrap_or(trimmed);
    if first_line.len() <= max_len {
        return first_line.to_owned();
    }
    let mut end = max_len;
    // Back up to last word boundary.
    while end > 0 && !first_line.is_char_boundary(end) {
        end -= 1;
    }
    // Try to find a space to break at a word boundary.
    if let Some(space_pos) = first_line[..end].rfind(' ') {
        format!("{}...", &first_line[..space_pos])
    } else {
        format!("{}...", &first_line[..end])
    }
}

pub(crate) fn truncate_assistant_history(text: &str) -> String {
    if text.len() <= MAX_HISTORY_ASSISTANT_BYTES {
        return text.to_owned();
    }

    let mut end = MAX_HISTORY_ASSISTANT_BYTES;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }

    let mut truncated = String::with_capacity(end + "...[truncated]".len());
    truncated.push_str(&text[..end]);
    truncated.push_str("...[truncated]");
    truncated
}

/// Maps a knowledge tier to its human-readable label.
pub(crate) fn knowledge_tier_label(tier: KnowledgeTier) -> &'static str {
    match tier {
        KnowledgeTier::Transient => "transient",
        KnowledgeTier::Working => "working",
        KnowledgeTier::Consolidated => "consolidated",
        KnowledgeTier::Persistent => "persistent",
    }
}

pub(crate) fn score_to_confidence(score: f64) -> f64 {
    let score = score.max(0.0);
    score / (1.0 + score)
}

/// Distil a logged ACP `episode` into durable knowledge in the background,
/// recording what the distillation call costs (bug-aad63e): the bare
/// `ModelCallService` ACP distils through records nothing itself.
pub(crate) fn spawn_acp_distillation(
    workdir: &Path,
    episode: Episode,
    caller: Arc<dyn roko_core::foundation::ModelCaller>,
) {
    roko_neuro::spawn_recorded_episode_distillation(workdir.to_path_buf(), episode, caller);
}

#[cfg(test)]
mod distillation_tests {
    use async_trait::async_trait;
    use roko_core::foundation::{ModelCallRequest, ModelCallResponse, ModelCaller, TokenUsage};

    use super::*;

    /// A distillation model that reports a known cost and distils nothing.
    struct FakeDistiller;

    #[async_trait]
    impl ModelCaller for FakeDistiller {
        async fn call(&self, _req: ModelCallRequest) -> roko_core::Result<ModelCallResponse> {
            Ok(ModelCallResponse {
                content: r#"{"entries": []}"#.to_string(),
                model: "fake-distiller".to_string(),
                usage: TokenUsage {
                    input_tokens: 700,
                    output_tokens: 30,
                    total_tokens: 730,
                    cost_usd: 0.125,
                },
                stop_reason: Some("end_turn".to_string()),
                request_id: None,
            })
        }
    }

    /// bug-aad63e: the distillation call that follows each ACP episode
    /// records its spend once, under the distiller's role and the episode's
    /// task.
    #[tokio::test]
    async fn acp_distillation_records_spend() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let episode = Episode::new("acp", "acp-session-1");

        spawn_acp_distillation(tmp.path(), episode, Arc::new(FakeDistiller));

        // The distillation runs detached; the efficiency row is written last.
        let learn = tmp.path().join(".roko").join("learn");
        let deadline = Instant::now() + std::time::Duration::from_secs(10);
        while !learn.join("efficiency.jsonl").exists() && Instant::now() < deadline {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let costs = std::fs::read_to_string(learn.join("costs.jsonl")).expect("cost log");
        let rows: Vec<serde_json::Value> = costs
            .lines()
            .map(|line| serde_json::from_str(line).expect("JSONL row"))
            .collect();
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0]["role"], roko_neuro::DISTILLATION_ROLE);
        assert_eq!(rows[0]["cost_usd"], 0.125);
        assert_eq!(rows[0]["task_id"], "acp-session-1");
    }
}
