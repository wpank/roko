//! Provider failover: a planned model whose provider cannot take the task hands
//! it to the next usable candidate within the same attempt.

use super::helper_calls::SideCall;
use super::*;

// ─── Provider failover ──────────────────────────────────────────────────────

/// `RokoError::Gateway` category for a task left without a usable provider.
///
/// The error is non-retryable, so `TaskExecutorCell` fails the attempt at once
/// instead of re-running a dispatch that would be refused again.
const PROVIDER_EXHAUSTED_CATEGORY: &str = "provider_exhausted";

/// `role` of the cost and efficiency rows of a call failover refused.
const FAILOVER_REFUSED_ROLE: &str = "failover_refused";

/// A model to dispatch: the key sent to the bridge and the config resolving it.
#[derive(Clone)]
struct DispatchCandidate {
    model_key: String,
    /// A clone of the run config with a `[models.*]` entry that serves a
    /// hinted slug on another configured provider; `None` uses the run config.
    config: Option<Arc<RokoConfig>>,
}

/// Why the provider behind a model cannot take this dispatch.
#[derive(Debug, Clone)]
struct ProviderRefusal {
    model_key: String,
    /// Wire slug of the refused model, used to find the same model elsewhere.
    model_slug: String,
    provider_id: String,
    provider_kind: roko_core::agent::ProviderKind,
    /// The provider's own words, or why it cannot be called.
    reason: String,
    /// When the provider is expected to accept work again (unix ms).
    until_ms: Option<i64>,
    /// Calling the provider again cannot help (missing, not dispatchable, no
    /// credentials, out of usage, billing), unlike an open circuit that may
    /// already have recovered.
    definitive: bool,
    /// Why, as a class ([`roko_learn::telemetry::FailoverRefusal::class`]).
    class: &'static str,
    /// Whether a call reached the provider before it refused.
    called: bool,
    /// When the refusal happened (unix ms).
    at_ms: i64,
}

/// The models failover passed over before the one that ran (bug-35379d).
/// Empty when the planned model ran.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct FailoverChain {
    /// Refused model keys, in order: the planned model first.
    pub(super) models: Vec<String>,
    /// Why the planned model did not run.
    pub(super) reason: Option<String>,
    /// Every refusal, with its class and whether a call was made
    /// (bug-220385).
    pub(super) refusals: Vec<roko_learn::telemetry::FailoverRefusal>,
}

impl FailoverChain {
    fn of(refusals: &[ProviderRefusal]) -> Self {
        Self {
            models: refusals
                .iter()
                .map(|refusal| refusal.model_key.clone())
                .collect(),
            reason: refusals.first().map(|refusal| {
                format!(
                    "`{}` on `{}`: {}",
                    refusal.model_key, refusal.provider_id, refusal.reason
                )
            }),
            refusals: refusals
                .iter()
                .map(|refusal| roko_learn::telemetry::FailoverRefusal {
                    model: refusal.model_key.clone(),
                    provider: refusal.provider_id.clone(),
                    class: refusal.class.to_string(),
                    reason: refusal.reason.clone(),
                    called: refusal.called,
                    at: Some(refusal.at_ms),
                    until: refusal.until_ms,
                })
                .collect(),
        }
    }
}

/// Provider kinds that serve the same model family over another transport.
fn same_family_kinds(
    kind: roko_core::agent::ProviderKind,
) -> &'static [roko_core::agent::ProviderKind] {
    use roko_core::agent::ProviderKind;
    match kind {
        ProviderKind::ClaudeCli | ProviderKind::AnthropicApi => {
            &[ProviderKind::ClaudeCli, ProviderKind::AnthropicApi]
        }
        ProviderKind::GeminiCli | ProviderKind::GeminiApi => {
            &[ProviderKind::GeminiCli, ProviderKind::GeminiApi]
        }
        _ => &[],
    }
}

/// CLI and ACP harnesses bring their own tools, so a profile's
/// `supports_tools` only matters for providers driven by roko's tool loop.
fn provider_brings_own_tools(provider: &roko_core::config::schema::ProviderConfig) -> bool {
    matches!(
        provider.transport(),
        roko_core::config::ProviderTransport::Cli { .. }
            | roko_core::config::ProviderTransport::Acp { .. }
    )
}

fn missing_credentials_reason(
    provider: &roko_core::config::schema::ProviderConfig,
    provider_id: &str,
) -> String {
    provider.api_key_env.as_deref().map_or_else(
        || format!("provider `{provider_id}` is not installed or has no credentials"),
        |env| format!("{env} is not set"),
    )
}

fn format_local_ms(ms: i64) -> String {
    use chrono::TimeZone as _;
    chrono::Local.timestamp_millis_opt(ms).single().map_or_else(
        || ms.to_string(),
        |at| at.format("%Y-%m-%d %H:%M %:z").to_string(),
    )
}

impl GraphTaskDispatcher {
    /// Run the provider bridge, treating the planned model as a preference.
    ///
    /// A model whose provider is missing, not dispatchable, without
    /// credentials, disabled, or has an open circuit is skipped before any
    /// call. One that refuses with a usage-window error ("You've hit your
    /// session limit · resets 4pm") is quarantined in the persisted health
    /// registry until its reported reset (else
    /// `routing.exhaustion_cooldown_secs`). Either way the next candidate from
    /// [`Self::failover_candidates`] runs in the same attempt, so no task
    /// retry is burned. An explicit `--model` override is a pin and never
    /// fails over. When nothing usable remains the attempt fails with a
    /// non-retryable error that says how to recover.
    ///
    /// Returns the dispatch with the models failover passed over, which the
    /// attempt's records carry beside the one that ran. A call a provider
    /// refused gets cost and efficiency rows of its own, keyed by
    /// `attempt_key` (role `failover_refused`, bug-220385). Each call starts
    /// on `progress`, so a call the watchdog cancels is recorded against the
    /// model it ran on (bug-aa2044).
    pub(super) async fn run_bridge_with_failover(
        &self,
        spec: &TaskExecutionSpec,
        task_id: &str,
        attempt_key: String,
        mut request: AgentDispatchRequest,
        progress: Option<&super::watchdog::AttemptProgress>,
    ) -> Result<(crate::dispatch_v2::AgentResultDispatch, FailoverChain)> {
        let pinned = self.cli_model_override.is_some();
        let mut candidate = DispatchCandidate {
            model_key: request.model_key.clone(),
            config: None,
        };
        let mut refusals: Vec<ProviderRefusal> = Vec::new();
        loop {
            if !pinned && let Some(refusal) = self.blocked_provider(&candidate) {
                let definitive = refusal.definitive;
                refusals.push(refusal);
                match self.failover_model(spec, task_id, &refusals) {
                    Ok(next) => {
                        candidate = next;
                        continue;
                    }
                    Err(error) if definitive => return Err(error),
                    // An open circuit may already have recovered; with no
                    // usable alternative, dispatch the planned model.
                    Err(_) => {
                        refusals.pop();
                    }
                }
            }

            request.model_key = candidate.model_key.clone();
            if let Some(progress) = progress {
                progress.call_started(
                    self.resolve_candidate(&candidate),
                    FailoverChain::of(&refusals),
                );
            }
            let call_started = Instant::now();
            let dispatch = match &candidate.config {
                Some(config) => {
                    self.factory
                        .run_shared_agent_bridge_with_config(request.clone(), Arc::clone(config))
                        .await
                }
                None => self.factory.run_shared_agent_bridge(request.clone()).await,
            }
            .map_err(|error| RokoError::Agent {
                backend: "graph-task-executor".to_string(),
                message: error.to_string(),
            })?;
            if dispatch.result.success {
                return Ok((dispatch, FailoverChain::of(&refusals)));
            }
            let Some(exhaustion) = dispatch
                .result
                .output
                .body
                .as_text()
                .ok()
                .and_then(roko_agent::provider::error_classify::detect_provider_exhaustion)
            else {
                return Ok((dispatch, FailoverChain::of(&refusals)));
            };

            let cooldown_ms = i64::try_from(
                self.config
                    .routing
                    .exhaustion_cooldown_secs
                    .saturating_mul(1_000),
            )
            .unwrap_or(i64::MAX);
            let until_ms = exhaustion.resets_at_ms.unwrap_or_else(|| {
                chrono::Utc::now()
                    .timestamp_millis()
                    .saturating_add(cooldown_ms)
            });
            let provider_id = dispatch.target.provider_id.clone();
            self.factory
                .health_registry
                .record_exhaustion(&provider_id, until_ms);
            // The caller settles only the result it receives; account the
            // refused call here.
            let refused_cost_usd = f64::from(dispatch.result.usage.cost_usd);
            self.task_spend.record(
                &format!("{}/{task_id}", spec.plan_id),
                &dispatch.result.usage,
            );
            self.budget_ledger
                .settle(&spec.plan_id, 0, refused_cost_usd)?;
            let refused_calls = refusals.iter().filter(|refusal| refusal.called).count();
            self.write_side_call_rows(
                spec,
                task_id,
                &attempt_key,
                &format!("{attempt_key}/refused-{}", refused_calls + 1),
                FAILOVER_REFUSED_ROLE,
                &SideCall::of(
                    &dispatch,
                    u64::try_from(call_started.elapsed().as_millis()).unwrap_or(u64::MAX),
                ),
            )
            .await;
            tracing::warn!(
                plan_id = %spec.plan_id,
                task_id,
                provider = %provider_id,
                model = %candidate.model_key,
                until = %format_local_ms(until_ms),
                reason = %exhaustion.message,
                "provider is out of usage; skipping it until its reset"
            );
            refusals.push(ProviderRefusal {
                model_key: candidate.model_key.clone(),
                model_slug: dispatch.target.model_slug.clone(),
                provider_id,
                provider_kind: dispatch.target.provider_kind,
                reason: exhaustion.message,
                until_ms: Some(until_ms),
                definitive: true,
                class: "provider_exhausted",
                called: true,
                at_ms: chrono::Utc::now().timestamp_millis(),
            });
            if pinned {
                return Err(self.no_usable_provider(&refusals, &[], true));
            }
            candidate = self.failover_model(spec, task_id, &refusals)?;
        }
    }

    fn resolve_candidate(
        &self,
        candidate: &DispatchCandidate,
    ) -> crate::dispatch_v2::ProviderDispatchSpec {
        let config = candidate.config.as_ref().unwrap_or(&self.config);
        crate::dispatch_v2::ProviderDispatchResolver::new(Arc::clone(config))
            .resolve(&candidate.model_key)
    }

    /// The refusal for `candidate` when its provider must not be called now:
    /// missing, not dispatchable, without credentials, statically disabled,
    /// or its circuit is open in the health registry.
    fn blocked_provider(&self, candidate: &DispatchCandidate) -> Option<ProviderRefusal> {
        use crate::dispatch_v2::ProviderRuntime;
        use roko_learn::provider_health::ErrorClass;

        let target = self.resolve_candidate(candidate);
        let provider_id = target.provider_id.clone();
        // Skipped before any call.
        let refusal = |class: &'static str,
                       reason: String,
                       until_ms: Option<i64>,
                       definitive: bool| ProviderRefusal {
            model_key: candidate.model_key.clone(),
            model_slug: target.model_slug.clone(),
            provider_id: provider_id.clone(),
            provider_kind: target.provider_kind,
            reason,
            until_ms,
            definitive,
            class,
            called: false,
            at_ms: chrono::Utc::now().timestamp_millis(),
        };
        let Some(provider) = target.provider_config.as_ref() else {
            return Some(refusal(
                "not_configured",
                format!("provider `{provider_id}` is not configured"),
                None,
                true,
            ));
        };
        if let ProviderRuntime::Unsupported(unsupported) = &target.runtime {
            return Some(refusal(
                "not_dispatchable",
                format!(
                    "provider `{provider_id}` is not dispatchable: {}",
                    unsupported.detail
                ),
                None,
                true,
            ));
        }
        if self
            .config
            .routing
            .disabled_providers
            .contains(&provider_id)
        {
            return Some(refusal(
                "disabled",
                "listed in routing.disabled_providers".to_string(),
                None,
                false,
            ));
        }
        let config = candidate.config.as_ref().unwrap_or(&self.config);
        if !config.provider_available_for_model_key(&candidate.model_key) {
            return Some(refusal(
                "no_credentials",
                missing_credentials_reason(provider, &provider_id),
                None,
                true,
            ));
        }
        let registry = &self.factory.health_registry;
        if registry.is_available(&provider_id) {
            return None;
        }
        let health = registry.get(&provider_id);
        let (class, reason, definitive) = match health.failure_window.back().map(|r| r.error_class)
        {
            Some(ErrorClass::Exhausted) => ("provider_exhausted", "out of usage", true),
            Some(ErrorClass::Billing) => ("billing", "billing failure", true),
            _ => (
                "circuit_open",
                "circuit open after repeated failures",
                false,
            ),
        };
        Some(refusal(
            class,
            reason.to_string(),
            health.cooldown_until,
            definitive,
        ))
    }

    /// Candidates after `refusals`, in order: the first refused model's slug
    /// on another configured provider of its family (claude_cli can run any
    /// Claude slug), `[routing] fallback_models`, `agent.fallback_model`, and
    /// `agent.default_model`.
    fn failover_candidates(&self, refusals: &[ProviderRefusal]) -> Vec<DispatchCandidate> {
        let mut candidates: Vec<DispatchCandidate> = Vec::new();
        let push_run_model = |candidates: &mut Vec<DispatchCandidate>, model_key: &str| {
            if !model_key.trim().is_empty()
                && !candidates
                    .iter()
                    .any(|candidate| candidate.model_key == model_key)
            {
                candidates.push(DispatchCandidate {
                    model_key: model_key.to_string(),
                    config: None,
                });
            }
        };
        if let Some(first) = refusals.first() {
            for (key, profile) in self.config.effective_models() {
                if profile.slug == first.model_slug && key != first.model_key {
                    push_run_model(&mut candidates, &key);
                }
            }
            let family = same_family_kinds(first.provider_kind);
            let base_profile = self
                .resolve_candidate(&DispatchCandidate {
                    model_key: first.model_key.clone(),
                    config: None,
                })
                .model_profile
                .unwrap_or_else(|| roko_core::config::schema::ModelProfile {
                    slug: first.model_slug.clone(),
                    supports_tools: true,
                    ..Default::default()
                });
            for (provider_id, provider) in self.config.effective_providers() {
                if !family.contains(&provider.kind) || provider_id == first.provider_id {
                    continue;
                }
                let model_key = format!("{}@{provider_id}", first.model_slug);
                let mut config = (*self.config).clone();
                config.models.insert(
                    model_key.clone(),
                    roko_core::config::schema::ModelProfile {
                        provider: provider_id,
                        ..base_profile.clone()
                    },
                );
                candidates.push(DispatchCandidate {
                    model_key,
                    config: Some(Arc::new(config)),
                });
            }
        }
        for model_key in self
            .config
            .routing
            .fallback_models
            .iter()
            .chain(self.config.agent.fallback_model.iter())
            .chain(std::iter::once(&self.config.agent.default_model))
        {
            push_run_model(&mut candidates, model_key);
        }
        candidates
    }

    /// The first usable model in [`Self::failover_candidates`], with the
    /// substitution and its reason logged at WARN.
    fn failover_model(
        &self,
        spec: &TaskExecutionSpec,
        task_id: &str,
        refusals: &[ProviderRefusal],
    ) -> Result<DispatchCandidate> {
        let mut skipped = Vec::new();
        for candidate in self.failover_candidates(refusals) {
            match self.failover_candidate(&candidate, refusals) {
                Ok(provider_id) => {
                    if let Some(refused) = refusals.last() {
                        tracing::warn!(
                            plan_id = %spec.plan_id,
                            task_id,
                            from_model = %refused.model_key,
                            from_provider = %refused.provider_id,
                            to_model = %candidate.model_key,
                            to_provider = %provider_id,
                            reason = %refused.reason,
                            "model substitution: `{}` on `{}` is unusable ({}); running `{}` on `{provider_id}` instead",
                            refused.model_key,
                            refused.provider_id,
                            refused.reason,
                            candidate.model_key,
                        );
                    }
                    return Ok(candidate);
                }
                Err(why) => skipped.push(format!("{}: {why}", candidate.model_key)),
            }
        }
        Err(self.no_usable_provider(refusals, &skipped, false))
    }

    /// The provider id of `candidate` when it can take the task now, else why not.
    fn failover_candidate(
        &self,
        candidate: &DispatchCandidate,
        refusals: &[ProviderRefusal],
    ) -> std::result::Result<String, String> {
        let target = self.resolve_candidate(candidate);
        let provider_id = &target.provider_id;
        let (Some(profile), Some(provider)) = (&target.model_profile, &target.provider_config)
        else {
            return Err(format!("provider `{provider_id}` is not configured"));
        };
        if refusals
            .iter()
            .any(|refusal| &refusal.provider_id == provider_id)
        {
            return Err(format!("provider `{provider_id}` is unavailable too"));
        }
        if let crate::dispatch_v2::ProviderRuntime::Unsupported(unsupported) = &target.runtime {
            return Err(format!(
                "provider `{provider_id}` is not dispatchable: {}",
                unsupported.detail
            ));
        }
        if self.config.routing.disabled_providers.contains(provider_id) {
            return Err(format!(
                "provider `{provider_id}` is in routing.disabled_providers"
            ));
        }
        if !profile.supports_tools && !provider_brings_own_tools(provider) {
            return Err("model does not support tools".to_string());
        }
        let config = candidate.config.as_ref().unwrap_or(&self.config);
        if !config.provider_available_for_model_key(&candidate.model_key) {
            return Err(missing_credentials_reason(provider, provider_id));
        }
        let registry = &self.factory.health_registry;
        if !registry.is_available(provider_id) {
            let until = registry
                .get(provider_id)
                .cooldown_until
                .map(|ms| format!(" until {}", format_local_ms(ms)))
                .unwrap_or_default();
            return Err(format!("provider `{provider_id}` is quarantined{until}"));
        }
        Ok(provider_id.clone())
    }

    /// Non-retryable error for a task left without a usable provider, naming
    /// each refusal, each skipped fallback, and how to recover.
    fn no_usable_provider(
        &self,
        refusals: &[ProviderRefusal],
        skipped: &[String],
        pinned: bool,
    ) -> RokoError {
        let refused = refusals
            .iter()
            .map(|refusal| {
                let until = refusal
                    .until_ms
                    .map(|ms| format!(" (skipped until {})", format_local_ms(ms)))
                    .unwrap_or_default();
                format!(
                    "`{}` on `{}`: {}{until}",
                    refusal.model_key, refusal.provider_id, refusal.reason
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        let fallback = if pinned {
            "The --model override pins this model, so no failover was attempted; drop --model to \
             allow it."
                .to_string()
        } else if skipped.is_empty() {
            format!(
                "No fallback model is configured: set [routing] fallback_models in roko.toml{} \
                 and put that provider's API key in ~/.roko/.env (loaded automatically at startup).",
                self.api_key_model_examples()
            )
        } else {
            format!(
                "No fallback model is usable ({}): put the missing API key in ~/.roko/.env \
                 (loaded automatically at startup) or extend [routing] fallback_models.",
                skipped.join("; ")
            )
        };
        let wait = refusals
            .iter()
            .filter_map(|refusal| refusal.until_ms)
            .min()
            .map_or_else(
                || "wait for the provider to recover".to_string(),
                |ms| format!("wait until {}", format_local_ms(ms)),
            );
        RokoError::Gateway {
            category: PROVIDER_EXHAUSTED_CATEGORY,
            retryable: false,
            message: format!(
                "no usable provider for this task: {refused}. {fallback} Otherwise {wait} and re-run."
            ),
        }
    }

    /// " (e.g. kimi-k2-5 needs MOONSHOT_API_KEY, …)" for configured API-key models.
    fn api_key_model_examples(&self) -> String {
        let providers = self.config.effective_providers();
        let mut examples: Vec<String> = self
            .config
            .effective_models()
            .into_iter()
            .filter(|(_, profile)| profile.supports_tools)
            .filter_map(|(key, profile)| {
                let env = providers.get(&profile.provider)?.api_key_env.clone()?;
                Some(format!("{key} needs {env}"))
            })
            .collect();
        examples.sort();
        examples.truncate(3);
        if examples.is_empty() {
            String::new()
        } else {
            format!(" (e.g. {})", examples.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use roko_core::agent::ProviderKind;
    use roko_core::config::schema::{ModelProfile, ProviderConfig};
    use roko_graph::Cell;
    use tempfile::tempdir;

    use super::*;
    use crate::graph_task_dispatch::tests::{
        FIXTURE_HANG_GUARD_SECS, FIXTURE_PROVIDER_TIMEOUT_MS, final_turn, jsonl_rows_where,
        make_spec, make_task_def, recording_feedback, spawn_openai_mock, tool_call_turn,
    };

    // ─── Provider failover on usage exhaustion ──────────────────────────────

    fn write_executable(path: &Path, body: &str) {
        std::fs::write(path, body).expect("write script");
        let mut permissions = std::fs::metadata(path)
            .expect("script metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("make script executable");
    }

    /// A fake `claude` that refuses exactly as the CLI did on 2026-09-26 and
    /// appends one line to `calls` per invocation.
    fn session_limit_claude(path: &Path, calls: &Path) {
        write_executable(
            path,
            &format!(
                r#"#!/bin/sh
cat >/dev/null
echo called >> '{}'
printf '%s\n' '{{"type":"result","subtype":"success","is_error":true,"total_cost_usd":0,"result":"You’ve hit your session limit · resets 4pm (Europe/Berlin)"}}'
exit 1
"#,
                calls.display()
            ),
        );
    }

    fn invocations(calls: &Path) -> usize {
        std::fs::read_to_string(calls).map_or(0, |log| log.lines().count())
    }

    /// `claude_cli` (fake script) plus two OpenAI-compatible fallbacks: one
    /// whose key env var is never set, and one pointed at `api_base_url`.
    fn failover_config(claude: &Path, api_base_url: &str, fallback_models: &[&str]) -> RokoConfig {
        let provider =
            |kind, command: Option<String>, base_url: Option<&str>, key_env: Option<&str>| {
                ProviderConfig {
                    kind,
                    base_url: base_url.map(str::to_string),
                    api_key_env: key_env.map(str::to_string),
                    command,
                    args: None,
                    timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                    ttft_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                    connect_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                    extra_headers: None,
                    max_concurrent: None,
                    limits: None,
                    require_confirmation: false,
                }
            };
        let api_model = |provider: &str, slug: &str| ModelProfile {
            provider: provider.to_string(),
            slug: slug.to_string(),
            context_window: 128_000,
            max_output: Some(1_024),
            // Unknown slugs default to a 3-tool cap, which would drop `bash`.
            max_tools: Some(32),
            supports_tools: true,
            tool_format: "openai_json".to_string(),
            ..ModelProfile::default()
        };
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.agent.default_model = "claude-sonnet".to_string();
        config.agent.bare_mode = false;
        config.providers.insert(
            "claude_cli".to_string(),
            provider(
                ProviderKind::ClaudeCli,
                Some(claude.display().to_string()),
                None,
                None,
            ),
        );
        // `PATH` is always set, standing in for a key loaded from ~/.roko/.env.
        config.providers.insert(
            "mock_api".to_string(),
            provider(
                ProviderKind::OpenAiCompat,
                None,
                Some(api_base_url),
                Some("PATH"),
            ),
        );
        config.providers.insert(
            "keyless_api".to_string(),
            provider(
                ProviderKind::OpenAiCompat,
                None,
                Some("http://127.0.0.1:9/v1"),
                Some("ROKO_TEST_FAILOVER_KEY_NEVER_SET"),
            ),
        );
        config.models.insert(
            "claude-sonnet".to_string(),
            ModelProfile {
                provider: "claude_cli".to_string(),
                slug: "claude-sonnet-4-6".to_string(),
                ..ModelProfile::default()
            },
        );
        config.models.insert(
            "keyless-model".to_string(),
            api_model("keyless_api", "keyless-1"),
        );
        config.models.insert(
            "api-model".to_string(),
            api_model("mock_api", "api-model-1"),
        );
        config.routing.fallback_models = fallback_models.iter().map(|m| m.to_string()).collect();
        // The mock API answers without SSE. The stall watchdog would attach
        // live output, over which the tool loop streams, so keep it off.
        config.conductor.silence_timeout_secs = 0;
        config.conductor.task_stall_secs = 0;
        config
    }

    /// A live cell for an implementer task hinted at `model_hint`.
    fn failover_cell(
        dispatcher: Arc<GraphTaskDispatcher>,
        model_hint: &str,
    ) -> roko_graph::cells::TaskExecutorCell {
        let task = TaskDef {
            id: "T08".to_string(),
            title: "Implement with failover".to_string(),
            description: Some("Edit notes and write hello.txt".to_string()),
            model_hint: Some(model_hint.to_string()),
            timeout_secs: FIXTURE_HANG_GUARD_SECS,
            max_retries: 2,
            ..make_task_def("focused")
        };
        let config = toml::Value::Table(toml::map::Map::from_iter([
            (
                "plan_id".to_string(),
                toml::Value::String("p-failover".to_string()),
            ),
            ("title".to_string(), toml::Value::String(task.title.clone())),
            (
                "timeout_secs".to_string(),
                toml::Value::Integer(FIXTURE_HANG_GUARD_SECS as i64),
            ),
            ("max_retries".to_string(), toml::Value::Integer(2)),
            (
                "task_def_json".to_string(),
                toml::Value::String(serde_json::to_string(&task).expect("serialize task")),
            ),
        ]));
        roko_graph::cells::TaskExecutorCell::live(config, dispatcher)
    }

    #[tokio::test]
    async fn exhausted_claude_cli_fails_over_to_api_model_with_file_and_shell_tools() {
        let temp = tempdir().expect("tempdir");
        let workdir = temp.path().join("work");
        std::fs::create_dir_all(&workdir).expect("workdir");
        std::fs::write(workdir.join("notes.txt"), "draft notes\n").expect("seed notes");
        let calls = temp.path().join("claude-calls.log");
        let claude = temp.path().join("fake-claude.sh");
        session_limit_claude(&claude, &calls);
        let (base_url, requests) = spawn_openai_mock(vec![
            tool_call_turn(
                "call-read",
                "read_file",
                serde_json::json!({ "path": "notes.txt" }),
            ),
            tool_call_turn(
                "call-edit",
                "edit_file",
                serde_json::json!({
                    "path": "notes.txt", "old_string": "draft", "new_string": "final"
                }),
            ),
            tool_call_turn(
                "call-write",
                "write_file",
                serde_json::json!({ "path": "hello.txt", "content": "hi from fallback" }),
            ),
            tool_call_turn(
                "call-shell",
                "bash",
                serde_json::json!({ "command": "cat hello.txt" }),
            ),
            final_turn("fallback finished"),
            final_turn("second task on fallback"),
        ]);
        let config = Arc::new(failover_config(
            &claude,
            &base_url,
            &["keyless-model", "api-model"],
        ));
        let health = Arc::new(
            roko_learn::provider_health::ProviderHealthRegistry::load_or_new(
                &temp.path().join("provider-health.json"),
            ),
        );
        let factory = Arc::new(
            SharedAgentFactory::new(Arc::clone(&config), None, None, None)
                .await
                .with_health_registry(Arc::clone(&health)),
        );
        let dispatcher = Arc::new(GraphTaskDispatcher::new(
            factory,
            Arc::clone(&config),
            workdir.clone(),
        ));
        let cell = failover_cell(dispatcher, "claude-sonnet-4-6");

        let output = cell
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T08".to_string()),
            )
            .await
            .expect("failover completes the task within one attempt");
        assert_eq!(output[0].body.as_text().expect("text"), "fallback finished");
        assert_eq!(invocations(&calls), 1, "the refusal is not retried");
        // Tool results the fallback model saw, in order: read, edit, write, shell.
        let tool_results: Vec<String> = requests.lock()[4]["messages"]
            .as_array()
            .expect("messages")
            .iter()
            .filter(|message| message["role"] == "tool")
            .map(|message| message["content"].to_string())
            .collect();
        assert_eq!(tool_results.len(), 4, "{tool_results:#?}");
        assert_eq!(
            std::fs::read_to_string(workdir.join("notes.txt")).expect("notes"),
            "final notes\n",
            "{tool_results:#?}"
        );
        assert_eq!(
            std::fs::read_to_string(workdir.join("hello.txt")).expect("hello"),
            "hi from fallback",
            "{tool_results:#?}"
        );
        assert!(
            tool_results[3].contains("hi from fallback"),
            "shell output reaches the model: {tool_results:#?}"
        );

        // claude_cli stays quarantined until the parsed "resets 4pm".
        let claude_health = health.get("claude_cli");
        assert_eq!(
            claude_health.state,
            roko_learn::provider_health::CircuitState::Open
        );
        let until = claude_health.cooldown_until.expect("quarantine end");
        let now = chrono::Utc::now().timestamp_millis();
        assert!(until > now && until <= now + 24 * 3_600_000, "{until}");

        // The next dispatch skips the quarantined CLI without calling it.
        let second = cell
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T08-next".to_string()),
            )
            .await
            .expect("next dispatch runs on the fallback");
        assert_eq!(
            second[0].body.as_text().expect("text"),
            "second task on fallback"
        );
        assert_eq!(invocations(&calls), 1);
        let requests = requests.lock();
        assert_eq!(requests.len(), 6);
        assert!(
            requests
                .iter()
                .all(|request| request["model"] == "api-model-1"),
            "every API turn names the fallback model"
        );
    }

    /// One attempt of a task planned for `claude-sonnet-4-6` on a Claude CLI
    /// that refuses with its session limit, which fails over to
    /// `api-model`, answering as `api-model-1`, with every record written
    /// under the returned workdir's `.roko` for the returned run.
    async fn fail_over_from_an_exhausted_cli(temp: &tempfile::TempDir) -> (PathBuf, &'static str) {
        let workdir = temp.path().join("work");
        std::fs::create_dir_all(&workdir).expect("workdir");
        let calls = temp.path().join("claude-calls.log");
        let claude = temp.path().join("fake-claude.sh");
        session_limit_claude(&claude, &calls);
        let mut answer = final_turn("fallback finished");
        answer["model"] = serde_json::json!("api-model-1");
        let (base_url, _requests) = spawn_openai_mock(vec![answer]);
        let config = Arc::new(failover_config(&claude, &base_url, &["api-model"]));
        let health = Arc::new(
            roko_learn::provider_health::ProviderHealthRegistry::load_or_new(
                &temp.path().join("provider-health.json"),
            ),
        );
        let factory = Arc::new(
            SharedAgentFactory::new(Arc::clone(&config), None, None, None)
                .await
                .with_health_registry(health),
        );
        let dispatcher = GraphTaskDispatcher::new(factory, Arc::clone(&config), workdir.clone())
            .with_feedback(recording_feedback(&workdir));
        let task = TaskDef {
            id: "T08".to_string(),
            title: "Implement with failover".to_string(),
            model_hint: Some("claude-sonnet-4-6".to_string()),
            timeout_secs: FIXTURE_HANG_GUARD_SECS,
            ..make_task_def("focused")
        };
        let run = "graph-failover-run";
        dispatcher
            .dispatch(
                &make_spec(&task),
                Vec::new(),
                &CellContext::new().with_run_id(run.to_string()),
            )
            .await
            .expect("the fallback runs the task");
        drop(dispatcher);
        assert_eq!(invocations(&calls), 1);
        (workdir, run)
    }

    /// A call a provider refused for its usage limit is on the attempt's
    /// records (bug-220385): the verdict and episode list the refusal with
    /// its class, and the refused call has its own cost and efficiency rows.
    #[tokio::test]
    async fn an_exhaustion_refusal_during_failover_is_recorded() {
        let temp = tempdir().expect("tempdir");
        let (workdir, run) = fail_over_from_an_exhausted_cli(&temp).await;

        let verdicts = jsonl_rows_where(
            &workdir.join(".roko/runs").join(run).join("attempts.jsonl"),
            1,
            |row| row["schema_version"] == "roko.verdict/1",
        )
        .await;
        let verdict = &verdicts[0];
        let key = verdict["attempt_key"].as_str().expect("attempt key");
        let planned = verdict["executed"]["model_requested"]
            .as_str()
            .expect("planned model");
        let refusals = verdict["executed"]["failover_refusals"]
            .as_array()
            .expect("failover refusals");
        assert_eq!(refusals.len(), 1, "{verdict}");
        let refusal = &refusals[0];
        assert_eq!(refusal["model"], planned);
        assert_eq!(refusal["provider"], "claude_cli");
        assert_eq!(refusal["class"], "provider_exhausted");
        assert_eq!(refusal["called"], true);
        assert!(
            refusal["at"].is_i64() && refusal["until"].is_i64(),
            "{refusal}"
        );
        let reason = refusal["reason"].as_str().unwrap_or_default();
        assert!(reason.contains("session limit"), "{reason}");

        let episodes = roko_learn::episode_logger::EpisodeLogger::read_all(
            &workdir.join(".roko/episodes.jsonl"),
        )
        .await
        .expect("episodes");
        assert_eq!(
            episodes[0].extra["failover_refusals"][0]["class"],
            "provider_exhausted"
        );

        let refused = |row: &serde_json::Value| row["role"] == FAILOVER_REFUSED_ROLE;
        let costs = jsonl_rows_where(&workdir.join(".roko/learn/costs.jsonl"), 1, refused).await;
        assert_eq!(costs.len(), 1);
        assert_eq!(costs[0]["attempt_key"], key);
        assert_eq!(costs[0]["provider"], "claude_cli");
        assert_eq!(costs[0]["success"], false);
        let efficiency =
            jsonl_rows_where(&workdir.join(".roko/learn/efficiency.jsonl"), 1, |row| {
                row["schema"] == roko_learn::efficiency::AGENT_EFFICIENCY_EVENT_SCHEMA
                    && refused(row)
            })
            .await;
        assert_eq!(efficiency[0]["attempt_id"], format!("{key}/refused-1"));
    }

    /// A task planned for an exhausted provider runs on a fallback: the
    /// attempt's verdict, episode, cost and efficiency rows name the planned
    /// model, why it did not run, and the model that did.
    #[tokio::test]
    async fn failover_records_planned_and_substitute_model() {
        let temp = tempdir().expect("tempdir");
        let (workdir, run) = fail_over_from_an_exhausted_cli(&temp).await;

        let verdicts = jsonl_rows_where(
            &workdir.join(".roko/runs").join(run).join("attempts.jsonl"),
            1,
            |row| row["schema_version"] == "roko.verdict/1",
        )
        .await;
        let executed = &verdicts[0]["executed"];
        let planned = executed["model_requested"].as_str().expect("planned model");
        assert_eq!(executed["failover_chain"], serde_json::json!([planned]));
        let reason = executed["failover_reason"]
            .as_str()
            .expect("failover reason");
        assert!(reason.contains("session limit"), "{reason}");
        assert!(reason.contains("claude_cli"), "{reason}");
        assert_eq!(executed["provider"], "mock_api");
        assert_eq!(executed["model_dispatched"], "api-model-1");
        assert_eq!(executed["model_reported"], "api-model-1");
        assert_eq!(executed["model_mismatch"], false);

        let episodes = roko_learn::episode_logger::EpisodeLogger::read_all(
            &workdir.join(".roko/episodes.jsonl"),
        )
        .await
        .expect("episodes");
        assert_eq!(episodes.len(), 1);
        assert_eq!(episodes[0].model, "api-model-1");
        assert_eq!(episodes[0].extra["substituted_from"], planned);
        assert_eq!(episodes[0].extra["failover_reason"], reason);
        // The refused call has rows of its own; these are the call that ran.
        let ran = |row: &serde_json::Value| row["role"] != FAILOVER_REFUSED_ROLE;
        let costs = jsonl_rows_where(&workdir.join(".roko/learn/costs.jsonl"), 1, ran).await;
        assert_eq!(costs[0]["model"], "api-model-1");
        assert_eq!(costs[0]["substituted_from"], planned);
        assert_eq!(costs[0]["substitution_reason"], reason);
        let efficiency =
            jsonl_rows_where(&workdir.join(".roko/learn/efficiency.jsonl"), 1, |row| {
                row["schema"] == roko_learn::efficiency::AGENT_EFFICIENCY_EVENT_SCHEMA && ran(row)
            })
            .await;
        assert_eq!(efficiency[0]["model"], "api-model-1");
        assert_eq!(efficiency[0]["substituted_from"], planned);
    }

    #[tokio::test]
    async fn exhausted_provider_without_usable_fallback_fails_once_with_fix_hint() {
        let temp = tempdir().expect("tempdir");
        let calls = temp.path().join("claude-calls.log");
        let claude = temp.path().join("fake-claude.sh");
        session_limit_claude(&claude, &calls);
        let config = Arc::new(failover_config(
            &claude,
            "http://127.0.0.1:9/v1",
            &["keyless-model"],
        ));
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher = Arc::new(GraphTaskDispatcher::new(
            factory,
            Arc::clone(&config),
            temp.path().to_path_buf(),
        ));
        let error = failover_cell(dispatcher, "claude-sonnet-4-6")
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T08".to_string()),
            )
            .await
            .expect_err("no usable provider fails the task");

        let RokoError::Gateway {
            category,
            retryable,
            message,
        } = &error
        else {
            panic!("expected a non-retryable gateway error, got {error:?}");
        };
        assert_eq!(*category, PROVIDER_EXHAUSTED_CATEGORY);
        assert!(!retryable);
        assert!(
            message.contains("You’ve hit your session limit"),
            "{message}"
        );
        assert!(
            message.contains("keyless-model: ROKO_TEST_FAILOVER_KEY_NEVER_SET is not set"),
            "{message}"
        );
        assert!(message.contains("~/.roko/.env"), "{message}");
        assert_eq!(
            invocations(&calls),
            1,
            "max_retries = 2 must not re-run an exhausted provider"
        );
    }

    /// `roko init` workspaces configure only `claude_cli` and the default
    /// model; a hint naming another model must never fail the task.
    #[tokio::test]
    async fn hint_for_unconfigured_provider_runs_same_slug_then_default_model() {
        let temp = tempdir().expect("tempdir");
        let args_log = temp.path().join("claude-args.log");
        let claude = temp.path().join("fake-claude.sh");
        write_executable(
            &claude,
            &format!(
                r#"#!/bin/sh
cat >/dev/null
echo "$@" >> '{}'
printf '%s\n' '{{"type":"content_block_delta","delta":{{"text":"ran"}}}}'
printf '%s\n' '{{"type":"result","session_id":"s","total_cost_usd":0,"usage":{{"input_tokens":1,"output_tokens":1}}}}'
"#,
                args_log.display()
            ),
        );
        // What `roko init` + `synthesize_claude_cli_config` produce: one CLI
        // provider and a `[models.*]` entry for the default model only.
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.agent.default_model = "claude-sonnet-4-6".to_string();
        config.agent.bare_mode = false;
        config.providers.insert(
            "claude_cli".to_string(),
            ProviderConfig {
                kind: ProviderKind::ClaudeCli,
                base_url: None,
                api_key_env: None,
                command: Some(claude.display().to_string()),
                args: None,
                timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                ttft_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                connect_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                extra_headers: None,
                max_concurrent: None,
                limits: None,
                require_confirmation: false,
            },
        );
        config.models.insert(
            "claude-sonnet-4-6".to_string(),
            ModelProfile {
                provider: "claude_cli".to_string(),
                slug: "claude-sonnet-4-6".to_string(),
                ..ModelProfile::default()
            },
        );
        // An API key in the environment synthesizes a usable standard provider
        // (`OPENAI_API_KEY` would send `gpt-4o` to the real OpenAI API), and
        // gate commands inherit the keys roko loads from `~/.roko/.env`. Shadow
        // each one with a provider whose key is never set.
        for (id, kind) in [
            ("anthropic", ProviderKind::AnthropicApi),
            ("openai", ProviderKind::OpenAiCompat),
            ("gemini", ProviderKind::GeminiApi),
            ("perplexity", ProviderKind::PerplexityApi),
        ] {
            config.providers.insert(
                id.to_string(),
                ProviderConfig {
                    kind,
                    base_url: None,
                    api_key_env: Some("ROKO_TEST_FAILOVER_KEY_NEVER_SET".to_string()),
                    command: None,
                    args: None,
                    timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                    ttft_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                    connect_timeout_ms: Some(FIXTURE_PROVIDER_TIMEOUT_MS),
                    extra_headers: None,
                    max_concurrent: None,
                    limits: None,
                    require_confirmation: false,
                },
            );
        }
        let config = Arc::new(config);
        let factory =
            Arc::new(SharedAgentFactory::new(Arc::clone(&config), None, None, None).await);
        let dispatcher = Arc::new(GraphTaskDispatcher::new(
            factory,
            Arc::clone(&config),
            temp.path().to_path_buf(),
        ));

        // 1. `claude-opus-4-6` resolves to an unconfigured `anthropic_api`;
        //    claude_cli serves the same slug.
        let output = failover_cell(Arc::clone(&dispatcher), "claude-opus-4-6")
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T1".to_string()),
            )
            .await
            .expect("hinted slug runs on claude_cli");
        assert_eq!(output[0].body.as_text().expect("text"), "ran");
        // 2. No configured provider can serve `gpt-4o`: the default model runs.
        failover_cell(dispatcher, "gpt-4o")
            .execute(
                Vec::new(),
                &CellContext::new().with_cell_id("T2".to_string()),
            )
            .await
            .expect("default model runs");

        let calls = std::fs::read_to_string(&args_log).expect("claude invocations");
        let models: Vec<&str> = calls
            .lines()
            .filter_map(|line| line.split("--model ").nth(1)?.split_whitespace().next())
            .collect();
        assert_eq!(models, ["claude-opus-4-6", "claude-sonnet-4-6"], "{calls}");
    }
}
