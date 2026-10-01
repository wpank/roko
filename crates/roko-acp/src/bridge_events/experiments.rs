//! Experiment assignment, settlement, and cascade model routing for ACP.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use roko_agent::rate_limit::{ProviderRateLimitSnapshot, ProviderRateLimiter};
use roko_core::agent::resolve_model;
use roko_core::config::schema::RokoConfig;
use roko_learn::{
    model_call_feedback::{ModelCallJournal, load_recovered_router},
    model_router::RoutingContext,
    prompt_experiment::{
        AssignmentSettlement, ExperimentStatus, ExperimentStore, PromptAttemptKey,
    },
    provider_health::ProviderHealthRegistry,
};
use std::collections::HashSet;
use std::sync::Mutex;
use tokio::task;
use tracing::{debug, info, warn};

use roko_core::agent::ResolvedModel;

use super::cost::acp_role_for_mode;
use super::cost::acp_routing_context;
use super::protocol::{CASCADE_ROUTER_IO_LOCK, EXPERIMENT_STORE_IO_LOCK};

#[derive(Clone)]
pub(crate) struct AcpExperimentAssignment {
    pub(crate) experiment_id: String,
    pub(crate) variant_id: String,
    pub(crate) section_name: String,
    pub(crate) content: String,
    pub(crate) model_slug: Option<String>,
    /// P1-21: Durable receipt key for the canonical experiment lifecycle.
    /// Populated when `prepare_attempt_assignments` succeeds.
    pub(crate) attempt_key: Option<PromptAttemptKey>,
    /// Assignment IDs returned by `prepare_attempt_assignments`, needed by
    /// `mark_attempt_dispatched` to record the exact included subset.
    pub(crate) prepared_assignment_ids: Vec<String>,
}

pub(crate) fn experiment_store_lock() -> std::sync::MutexGuard<'static, ()> {
    EXPERIMENT_STORE_IO_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Select one running experiment deterministically for this ACP role.
///
/// The persisted map is intentionally sorted before selection so HashMap
/// iteration order cannot change which experiment receives an ACP turn.
pub(crate) fn assign_acp_experiment(
    path: &Path,
    mode: &str,
    session_id: &str,
) -> Option<AcpExperimentAssignment> {
    let _guard = experiment_store_lock();
    let store = ExperimentStore::load_or_new(path);
    let role = acp_role_for_mode(mode).label();
    let mut experiments = store
        .experiments()
        .values()
        .filter(|experiment| experiment.status == ExperimentStatus::Running)
        .filter(|experiment| {
            experiment.role.as_deref().is_none_or(|configured| {
                configured.eq_ignore_ascii_case(mode) || configured.eq_ignore_ascii_case(role)
            })
        })
        .collect::<Vec<_>>();
    experiments.sort_by(|left, right| left.experiment_id.cmp(&right.experiment_id));
    let experiment = experiments.first()?;
    let variant = experiment.assign_variant()?;

    // P1-21: Prepare a durable receipt key so ACP dispatches participate in
    // the canonical experiment lifecycle. Use session_id as run_id, "acp" as
    // plan_id, and the mode as task_id.
    let attempt_key = PromptAttemptKey::new(session_id, "acp", mode, 1);
    let prepare_result = ExperimentStore::prepare_attempt_assignments(
        path,
        &attempt_key,
        Some(role),
        &[experiment.section_name.as_str()],
    );
    let (attempt_key, prepared_assignment_ids) = match prepare_result {
        Ok(assignments) => {
            let ids = assignments
                .iter()
                .map(|a| a.assignment_id.clone())
                .collect::<Vec<_>>();
            (Some(attempt_key), ids)
        }
        Err(err) => {
            tracing::debug!(
                error = %err,
                "P1-21: ACP experiment receipt preparation failed (non-fatal)"
            );
            (None, Vec::new())
        }
    };

    Some(AcpExperimentAssignment {
        experiment_id: experiment.experiment_id.clone(),
        variant_id: variant.id.clone(),
        section_name: experiment.section_name.clone(),
        content: variant.content.clone(),
        model_slug: variant.slug.clone().filter(|slug| !slug.trim().is_empty()),
        attempt_key,
        prepared_assignment_ids,
    })
}

pub(crate) fn experiment_model_key(
    config: &RokoConfig,
    assignment: &AcpExperimentAssignment,
) -> Option<String> {
    let requested = assignment.model_slug.as_deref()?.trim();
    let models = config.effective_models();
    if models.contains_key(requested) {
        return Some(requested.to_string());
    }
    let mut matching = models
        .iter()
        .filter(|(_, profile)| profile.slug.trim() == requested)
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    matching.sort();
    matching.into_iter().next()
}

pub(crate) fn applicable_acp_experiment(
    config: &RokoConfig,
    current_model_key: &str,
    model_selection_explicit: bool,
    assignment: Option<AcpExperimentAssignment>,
) -> (Option<AcpExperimentAssignment>, Option<String>) {
    let Some(assignment) = assignment else {
        return (None, None);
    };
    if assignment.model_slug.is_none() {
        return (Some(assignment), None);
    }

    let Some(candidate) = experiment_model_key(config, &assignment) else {
        warn!(
            experiment_id = %assignment.experiment_id,
            variant_id = %assignment.variant_id,
            model_slug = ?assignment.model_slug,
            "skipping ACP experiment variant with unresolved model"
        );
        return (None, None);
    };
    if model_selection_explicit && resolve_model(config, current_model_key).model_key != candidate {
        debug!(
            experiment_id = %assignment.experiment_id,
            variant_id = %assignment.variant_id,
            experiment_model = %candidate,
            selected_model = current_model_key,
            "skipping ACP model experiment because the session model was explicitly selected"
        );
        return (None, None);
    }

    let model_override = (!model_selection_explicit).then_some(candidate);
    (Some(assignment), model_override)
}

pub(crate) fn render_experiment_context(assignment: &AcpExperimentAssignment) -> String {
    format!(
        "ACP experiment `{}` variant `{}` for section `{}`:\n{}",
        assignment.experiment_id,
        assignment.variant_id,
        assignment.section_name,
        assignment.content.trim()
    )
}

/// Replace a named section's content in a flat system prompt string.
///
/// Looks up the canonical heading for `assignment.section_name` (e.g.
/// `"conventions"` -> `"## Project Conventions"`) and replaces everything
/// between that heading and the next `## ` heading (or end of string) with
/// the experiment variant content.
///
/// For heading-less sections (like `role_identity`), replaces the content
/// from the start of the prompt up to the first `## ` heading.
///
/// Falls back to `append_context` if the heading is not found in the
/// prompt, so the experiment content is never silently lost.
pub(crate) fn replace_experiment_section(
    system_prompt: &str,
    assignment: &AcpExperimentAssignment,
) -> String {
    let section_name = assignment.section_name.as_str();
    let variant_content = assignment.content.trim();

    match roko_compose::section_heading_for_name(section_name) {
        Some(heading) => {
            // Section has a heading like "## Project Conventions".
            // Find it in the prompt and replace everything between it and the
            // next heading.
            if let Some(heading_start) = system_prompt.find(heading) {
                let content_start = heading_start + heading.len();
                // Find the next "## " heading after this one, or end of string.
                let rest = &system_prompt[content_start..];
                let next_heading_offset = find_next_heading(rest);
                let content_end = content_start + next_heading_offset;

                let mut result = String::with_capacity(system_prompt.len());
                result.push_str(&system_prompt[..content_start]);
                result.push_str("\n\n");
                result.push_str(variant_content);
                result.push_str("\n\n");
                result.push_str(system_prompt[content_end..].trim_start());
                result
            } else {
                // Heading not present in the prompt -- fall back to append.
                debug!(
                    section = section_name,
                    heading, "experiment section heading not found in system prompt; appending"
                );
                crate::knowledge::append_context(
                    system_prompt,
                    &render_experiment_context(assignment),
                )
            }
        }
        None => {
            // Heading-less sections (e.g. `role_identity`) occupy the content
            // before the first `## ` heading.
            let first_heading = find_next_heading(system_prompt);
            if first_heading > 0 {
                let mut result = String::with_capacity(system_prompt.len());
                result.push_str(variant_content);
                result.push_str("\n\n");
                result.push_str(system_prompt[first_heading..].trim_start());
                result
            } else {
                // No heading found at all -- replace the entire prompt.
                variant_content.to_string()
            }
        }
    }
}

/// Find the byte offset of the next markdown `## ` heading in `text`.
///
/// Returns `text.len()` if no heading is found (i.e. content runs to
/// the end of the string).
fn find_next_heading(text: &str) -> usize {
    // Look for "\n## " which marks a heading at the start of a line.
    // Also check if the text itself starts with "## ".
    let trimmed = text.trim_start_matches('\n');
    let skip = text.len() - trimmed.len();

    if trimmed.starts_with("## ") {
        return skip;
    }

    text[skip..]
        .find("\n## ")
        .map(|pos| skip + pos + 1) // +1 to skip past the newline, pointing at "## "
        .unwrap_or(text.len())
}

pub(crate) fn record_acp_experiment_outcome(
    path: &Path,
    assignment: &AcpExperimentAssignment,
    success: bool,
) -> std::io::Result<()> {
    // P2-ACP-1: Experiment receipt parity with the runner.
    //
    // The graph engine runner uses `settle_attempt` as the sole stats writer:
    // `settle_attempt_unlocked` calls `record_outcome_for_experiment` internally
    // for every Dispatched assignment in the bucket.  Calling
    // `record_outcome_for_experiment` here *and* then calling `settle_attempt`
    // would double-count every trial when the canonical receipt protocol is
    // active (i.e. `attempt_key` is Some).
    //
    // Decision matrix:
    //
    // | attempt_key | mark_dispatched? | settlement action       |
    // |-------------|-----------------|-------------------------|
    // | None        | n/a             | direct legacy fallback  |
    // | Some        | succeeded       | settle_attempt (stats)  |
    // | Some        | failed/skipped  | settle_attempt (Abandoned, no stats) |
    //
    // The last row intentionally matches the runner: if the experiment content
    // was never durably dispatched to the provider, the result is discarded.
    if assignment.attempt_key.is_none() {
        // Legacy fallback: the prepare phase failed; directly update stats so
        // outcomes are never silently lost when no receipt bucket exists.
        let _guard = experiment_store_lock();
        ExperimentStore::transaction(path, |store| {
            if !store.record_outcome_for_experiment(
                &assignment.experiment_id,
                &assignment.variant_id,
                success,
            ) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!(
                        "experiment '{}' variant '{}' disappeared before outcome recording",
                        assignment.experiment_id, assignment.variant_id
                    ),
                ));
            }
            store.record_metric(
                &assignment.experiment_id,
                &assignment.variant_id,
                if success { 1.0 } else { 0.0 },
            );
            Ok(())
        })?;
        return Ok(());
    }

    // Canonical receipt protocol: settle_attempt is the sole stats writer,
    // matching the graph engine runner path exactly.
    // attempt_key.is_none() was handled by the early-return above; this
    // branch is only reached when attempt_key is Some.
    let Some(attempt_key) = assignment.attempt_key.as_ref() else {
        // Defensive: the early-return above should have caught this, but
        // avoid a panic under any future refactor that changes the control flow.
        tracing::warn!(
            experiment_id = %assignment.experiment_id,
            variant_id = %assignment.variant_id,
            "record_acp_experiment_outcome: attempt_key unexpectedly None after is_none check (non-fatal)"
        );
        return Ok(());
    };
    let settlement = AssignmentSettlement::Observed { success };
    if let Err(err) = ExperimentStore::settle_attempt(path, attempt_key, settlement) {
        tracing::debug!(
            error = %err,
            "P2-ACP-1: ACP experiment receipt settlement failed (non-fatal)"
        );
    }

    Ok(())
}

pub(crate) fn cascade_router_model_slugs(
    roko_config: &RokoConfig,
    resolved_slug: &str,
) -> Vec<String> {
    let mut model_slugs = roko_config.models.keys().cloned().collect::<Vec<_>>();
    if model_slugs.is_empty() {
        model_slugs.push(resolved_slug.to_owned());
    }
    model_slugs.sort();
    model_slugs
}

pub(crate) fn acp_model_providers(
    roko_config: &RokoConfig,
    model_keys: &[String],
) -> HashMap<String, String> {
    let models = roko_config.effective_models();
    model_keys
        .iter()
        .filter_map(|key| {
            models
                .get(key)
                .or_else(|| models.values().find(|profile| profile.slug == *key))
                .map(|profile| (key.clone(), profile.provider.clone()))
        })
        .collect()
}

pub(crate) fn provider_near_rate_limit(snapshot: &ProviderRateLimitSnapshot) -> bool {
    let rpm_pressured = snapshot.rpm_limit > 0
        && snapshot.rpm_used.saturating_mul(5) >= u64::from(snapshot.rpm_limit).saturating_mul(4);
    let tpm_pressured = snapshot.tpm_limit > 0
        && snapshot.tpm_used.saturating_mul(5) >= snapshot.tpm_limit.saturating_mul(4);
    rpm_pressured || tpm_pressured
}

pub(crate) fn rate_aware_model_candidates(
    model_keys: Vec<String>,
    model_providers: &HashMap<String, String>,
    snapshots: &[ProviderRateLimitSnapshot],
) -> (Vec<String>, Vec<String>) {
    let pressured = snapshots
        .iter()
        .filter(|snapshot| provider_near_rate_limit(snapshot))
        .map(|snapshot| snapshot.provider_id.clone())
        .collect::<HashSet<_>>();
    let preferred = model_keys
        .iter()
        .filter(|key| {
            model_providers
                .get(key.as_str())
                .is_none_or(|provider| !pressured.contains(provider))
        })
        .cloned()
        .collect::<Vec<_>>();
    if preferred.is_empty() {
        (model_keys, pressured.into_iter().collect())
    } else {
        (preferred, pressured.into_iter().collect())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AcpCascadeSelection {
    pub(crate) model_key: String,
    pub(crate) stage: String,
}

pub(crate) struct AcpCascadeRequest<'a> {
    pub(crate) workdir: &'a Path,
    pub(crate) roko_config: &'a RokoConfig,
    pub(crate) mode: &'a str,
    pub(crate) prompt: &'a str,
    pub(crate) effort: &'a str,
    pub(crate) resolved_slug: &'a str,
    pub(crate) model_selection_explicit: bool,
    pub(crate) provider_health: &'a ProviderHealthRegistry,
    pub(crate) rate_limiter: &'a ProviderRateLimiter,
}

pub(crate) fn acp_cascade_selection_enabled() -> bool {
    std::env::var_os("ROKO_ACP_CASCADE_SELECT").is_some_and(|value| value == "1")
}

/// Select a model for an ACP session using the cascade router.
///
/// Returns a model key and routing stage when:
/// 1. `ROKO_ACP_CASCADE_SELECT=1` exactly.
/// 2. The cascade router state file exists at `workdir/.roko/learn/cascade-router.json`.
///
/// Returns `None` (leaving model selection to the caller) when the env var is
/// absent, disabled, or the router file does not yet exist (cold start).
pub(crate) fn cascade_select_model(request: AcpCascadeRequest<'_>) -> Option<AcpCascadeSelection> {
    let AcpCascadeRequest {
        workdir,
        roko_config,
        mode,
        prompt,
        effort,
        resolved_slug,
        model_selection_explicit,
        provider_health,
        rate_limiter,
    } = request;
    if model_selection_explicit || !acp_cascade_selection_enabled() {
        return None;
    }

    let router_path = workdir
        .join(".roko")
        .join("learn")
        .join("cascade-router.json");

    if !router_path.exists() {
        return None;
    }

    let model_slugs = cascade_router_model_slugs(roko_config, resolved_slug);
    let initial_candidate_count = model_slugs.len();
    let model_providers = acp_model_providers(roko_config, &model_slugs);
    let (model_slugs, mut pressured_providers) =
        rate_aware_model_candidates(model_slugs, &model_providers, &rate_limiter.snapshot());
    let rate_candidates_filtered = model_slugs.len() < initial_candidate_count;
    pressured_providers.sort();
    let candidate_providers = model_slugs
        .iter()
        .filter_map(|model| model_providers.get(model))
        .cloned()
        .collect::<HashSet<_>>();
    let mut degraded_providers = candidate_providers
        .iter()
        .filter(|provider| !provider_health.is_healthy(provider))
        .cloned()
        .collect::<Vec<_>>();
    degraded_providers.sort();
    let has_healthy_provider = candidate_providers
        .iter()
        .any(|provider| provider_health.is_healthy(provider));
    // The snapshot first takes what a crashed writer journaled and never
    // saved (bug-8a78e1).
    let router = load_recovered_router(&router_path, model_slugs);
    let ctx = acp_routing_context(mode, prompt, effort, workdir);
    let cascade_model =
        router.route_with_health_scored(&ctx, provider_health, &model_providers, None, None);
    if rate_candidates_filtered {
        info!(
            selected_model = %cascade_model.primary.slug,
            providers = ?pressured_providers,
            reason = "RPM/TPM utilization at or above 80%",
            "ACP adaptive routing deprioritized rate-pressured providers"
        );
    } else if !pressured_providers.is_empty() {
        warn!(
            selected_model = %cascade_model.primary.slug,
            providers = ?pressured_providers,
            reason = "all ACP candidates are near RPM/TPM limits",
            "ACP adaptive routing retained least-bad rate-pressured candidates"
        );
    }
    if !degraded_providers.is_empty() && has_healthy_provider {
        info!(
            selected_model = %cascade_model.primary.slug,
            providers = ?degraded_providers,
            reason = "canonical provider circuit health",
            "ACP adaptive routing deprioritized degraded providers"
        );
    } else if !degraded_providers.is_empty() {
        warn!(
            selected_model = %cascade_model.primary.slug,
            providers = ?degraded_providers,
            reason = "all ACP candidates have open provider circuits",
            "ACP adaptive routing retained least-bad degraded candidates"
        );
    }
    Some(AcpCascadeSelection {
        model_key: cascade_model.primary.slug,
        stage: cascade_model.stage.label().to_owned(),
    })
}

pub(crate) fn resolve_acp_dispatch_model(
    roko_config: &RokoConfig,
    requested_model_key: &str,
    cascade_selection: Option<AcpCascadeSelection>,
) -> (ResolvedModel, String, Option<AcpCascadeSelection>) {
    let requested = resolve_model(roko_config, requested_model_key);
    let requested_dispatch_key = requested.model_key.clone();
    let Some(selection) = cascade_selection else {
        return (requested, requested_dispatch_key, None);
    };

    let selected = resolve_model(roko_config, &selection.model_key);
    if selected.profile.is_none() {
        warn!(
            requested_model = requested_model_key,
            selected_model = %selection.model_key,
            stage = %selection.stage,
            "cascade router selected an unconfigured ACP model; retaining requested model"
        );
        return (requested, requested_dispatch_key, None);
    }

    let dispatch_model_key = selected.model_key.clone();
    (selected, dispatch_model_key, Some(selection))
}

pub(crate) fn compute_acp_reward(success: bool, wall_ms: u64, output_tokens: Option<u64>) -> f64 {
    if !success {
        return 0.0;
    }

    let latency_bonus = if wall_ms < 5_000 {
        0.15
    } else if wall_ms < 15_000 {
        0.05
    } else {
        0.0
    };
    let token_bonus = match output_tokens {
        Some(tokens) if tokens < 2_000 => 0.05,
        Some(tokens) if tokens < 5_000 => 0.02,
        _ => 0.0,
    };

    let score: f64 = 0.8 + latency_bonus + token_bonus;
    score.min(1.0)
}

pub(crate) fn record_cascade_observation(
    router_path: PathBuf,
    model_slug: String,
    routing_ctx: RoutingContext,
    success: bool,
    wall_ms: u64,
    output_tokens: Option<u64>,
    model_slugs: Vec<String>,
) -> task::JoinHandle<()> {
    task::spawn_blocking(move || {
        let _guard = CASCADE_ROUTER_IO_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        let router = load_recovered_router(&router_path, model_slugs);

        if router.model_index_for_slug(&model_slug).is_none() {
            debug!(
                model = %model_slug,
                "skipping cascade observation: model not in router arms"
            );
            return;
        }

        let context_vec = routing_ctx.to_features();
        let reward = compute_acp_reward(success, wall_ms, output_tokens);
        // A failed dispatch is a trial without a success (bug-8da8ba). The
        // journal makes the observation durable before it is applied, and
        // `save` drops it from the journal once the snapshot holds it
        // (find-0dc1d5).
        let journal = ModelCallJournal::for_snapshot(&router_path);
        journal.observe(&router, &model_slug, context_vec, reward, success);

        if let Err(error) = journal.save(&router) {
            warn!(
                path = %router_path.display(),
                error = %error,
                "failed to persist cascade router after ACP observation"
            );
        }
    })
}

/// Mark an ACP experiment attempt as dispatched with the final prompt hash.
///
/// This completes the `Prepared -> Dispatched` transition in the durable
/// experiment lifecycle, matching what the Graph engine does for runner
/// dispatches. Non-fatal: logs and returns on failure.
pub(crate) fn mark_acp_experiment_dispatched(
    experiment_path: &Path,
    assignment: &AcpExperimentAssignment,
    prompt_hash: &str,
) {
    let Some(attempt_key) = assignment.attempt_key.as_ref() else {
        return;
    };
    if assignment.prepared_assignment_ids.is_empty() {
        return;
    }

    let included_ids: Vec<&str> = assignment
        .prepared_assignment_ids
        .iter()
        .map(String::as_str)
        .collect();

    let _guard = experiment_store_lock();
    if let Err(err) = ExperimentStore::mark_attempt_dispatched(
        experiment_path,
        attempt_key,
        prompt_hash,
        &included_ids,
    ) {
        debug!(
            experiment_id = %assignment.experiment_id,
            variant_id = %assignment.variant_id,
            error = %err,
            "ACP experiment mark_attempt_dispatched failed (non-fatal)"
        );
    }
}
