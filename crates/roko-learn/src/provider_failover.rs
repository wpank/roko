//! Provider failover, the policy every dispatch path shares (gap-28ceb9).
//!
//! A planned model is a preference. When its provider cannot take a call,
//! because it is disabled, has no credentials, or the health registry holds it
//! out (an open circuit, or a usage-window quarantine), or when it refuses a
//! call with a usage exhaustion ("You've hit your session limit · resets
//! 4pm"), the call moves to the next usable candidate:
//!
//! 1. the refused model's slug on another configured provider of its family
//!    (`claude_cli` can run any Claude slug the Anthropic API serves);
//! 2. `[routing] fallback_models`;
//! 3. `agent.fallback_model`;
//! 4. `agent.default_model`.
//!
//! An exhaustion quarantines its provider in the registry until the reported
//! reset, else for `routing.exhaustion_cooldown_secs`. A pinned call (an
//! explicit model override) never moves. Each substitution is logged at WARN,
//! and [`Failover::refusals`] keeps the models passed over for the call's
//! record (bug-35379d).
//!
//! Graph plan runs apply the policy in `GraphTaskDispatcher`'s
//! `run_bridge_with_failover`, through [`failover_candidates`]; serve's
//! one-shot dispatch and ACP prompts apply it through [`Failover`]. A Graph
//! task the model ladder routed takes step 1 ([`same_model_candidates`]),
//! then the runnable rungs above its own, and never a cheaper model
//! (decision 1119, backlog 1120).

use std::sync::Arc;

use roko_core::agent::{ProviderKind, resolve_model};
use roko_core::config::schema::{ModelProfile, ProviderConfig, RokoConfig};

use crate::provider_health::{ErrorClass, ProviderHealthRegistry};

/// Provider kinds that serve the same model family over another transport.
#[must_use]
pub fn same_family_kinds(kind: ProviderKind) -> &'static [ProviderKind] {
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
#[must_use]
pub fn provider_brings_own_tools(provider: &ProviderConfig) -> bool {
    matches!(
        provider.transport(),
        roko_core::config::ProviderTransport::Cli { .. }
            | roko_core::config::ProviderTransport::Acp { .. }
    )
}

/// Why `provider_id` cannot be called for lack of credentials.
#[must_use]
pub fn missing_credentials_reason(provider: &ProviderConfig, provider_id: &str) -> String {
    provider.api_key_env.as_deref().map_or_else(
        || format!("provider `{provider_id}` is not installed or has no credentials"),
        |env| format!("{env} is not set"),
    )
}

/// `ms` (unix milliseconds) as a local date and time, for operators.
#[must_use]
pub fn format_local_ms(ms: i64) -> String {
    use chrono::TimeZone as _;
    chrono::Local.timestamp_millis_opt(ms).single().map_or_else(
        || ms.to_string(),
        |at| at.format("%Y-%m-%d %H:%M %:z").to_string(),
    )
}

/// A model to dispatch: its key, and the config that resolves it.
#[derive(Debug, Clone)]
pub struct FailoverCandidate {
    /// The model key the dispatch path resolves.
    pub model_key: String,
    /// A clone of the caller's config with a `[models.*]` entry that serves a
    /// refused model's slug on another provider of its family; `None` keeps
    /// the caller's config.
    pub config: Option<Arc<RokoConfig>>,
}

/// The refused model whose slug [`failover_candidates`] looks for elsewhere.
#[derive(Debug, Clone)]
pub struct RefusedModel {
    /// Its model key.
    pub model_key: String,
    /// The slug it sends on the wire.
    pub model_slug: String,
    /// The provider that refused it.
    pub provider_id: String,
    /// That provider's kind, which names the model's family.
    pub provider_kind: ProviderKind,
    /// Its profile, copied onto the slug's other providers; `None` takes a
    /// tool-capable default.
    pub profile: Option<ModelProfile>,
}

/// The candidates after `first_refused` (the call's first refused model), in
/// the order the module docs give. A candidate whose provider already refused
/// or cannot take the call is the caller's to skip.
#[must_use]
pub fn failover_candidates(
    config: &Arc<RokoConfig>,
    first_refused: Option<&RefusedModel>,
) -> Vec<FailoverCandidate> {
    let mut candidates = first_refused
        .map(|first| same_model_candidates(config, first))
        .unwrap_or_default();
    for model_key in config
        .routing
        .fallback_models
        .iter()
        .chain(config.agent.fallback_model.iter())
        .chain(std::iter::once(&config.agent.default_model))
    {
        push_run_model(&mut candidates, model_key);
    }
    candidates
}

/// The first group of [`failover_candidates`]: `first`'s slug under its
/// other `[models.*]` keys, then on each other configured provider of its
/// family. A Graph task the model ladder routed fails over to these, then up
/// its rungs (backlog 1120).
#[must_use]
pub fn same_model_candidates(
    config: &Arc<RokoConfig>,
    first: &RefusedModel,
) -> Vec<FailoverCandidate> {
    let mut candidates: Vec<FailoverCandidate> = Vec::new();
    for (key, profile) in config.effective_models() {
        if profile.slug == first.model_slug && key != first.model_key {
            push_run_model(&mut candidates, &key);
        }
    }
    let family = same_family_kinds(first.provider_kind);
    let base_profile = first.profile.clone().unwrap_or_else(|| ModelProfile {
        slug: first.model_slug.clone(),
        supports_tools: true,
        ..Default::default()
    });
    for (provider_id, provider) in config.effective_providers() {
        if !family.contains(&provider.kind) || provider_id == first.provider_id {
            continue;
        }
        let model_key = format!("{}@{provider_id}", first.model_slug);
        let mut synthesized = (**config).clone();
        synthesized.models.insert(
            model_key.clone(),
            ModelProfile {
                provider: provider_id,
                ..base_profile.clone()
            },
        );
        candidates.push(FailoverCandidate {
            model_key,
            config: Some(Arc::new(synthesized)),
        });
    }
    candidates
}

/// Append `model_key`, under the caller's config, unless it is blank or
/// already a candidate.
fn push_run_model(candidates: &mut Vec<FailoverCandidate>, model_key: &str) {
    if !model_key.trim().is_empty()
        && !candidates
            .iter()
            .any(|candidate| candidate.model_key == model_key)
    {
        candidates.push(FailoverCandidate {
            model_key: model_key.to_string(),
            config: None,
        });
    }
}

/// A usage exhaustion a provider refused a call with, as recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedExhaustion {
    /// The provider's own words.
    pub message: String,
    /// Until when the provider is quarantined (unix ms).
    pub until_ms: i64,
}

/// When `text`, a refused call's output or error, is a usage exhaustion
/// (`roko_agent::provider::error_classify::detect_provider_exhaustion`),
/// quarantine `provider_id` in `registry` until its reported reset, else for
/// `routing.exhaustion_cooldown_secs`, and return what was recorded.
pub fn record_exhaustion(
    config: &RokoConfig,
    registry: &ProviderHealthRegistry,
    provider_id: &str,
    text: &str,
) -> Option<RecordedExhaustion> {
    let exhaustion = roko_agent::provider::error_classify::detect_provider_exhaustion(text)?;
    let cooldown_ms = i64::try_from(
        config
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
    registry.record_exhaustion(provider_id, until_ms);
    Some(RecordedExhaustion {
        message: exhaustion.message,
        until_ms,
    })
}

/// A model a call's failover passed over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The model's key.
    pub model_key: String,
    /// Its provider.
    pub provider_id: String,
    /// Why, as a class: `disabled`, `no_credentials`, `provider_exhausted`,
    /// `billing` or `circuit_open`.
    pub class: &'static str,
    /// Why, for the operator.
    pub reason: String,
    /// When the provider should take work again (unix ms), when known.
    pub until_ms: Option<i64>,
    /// Whether a call reached the provider before it refused.
    pub called: bool,
}

impl Refusal {
    /// "`model` on `provider`: reason", with the time it is skipped until.
    #[must_use]
    pub fn describe(&self) -> String {
        let until = self
            .until_ms
            .map(|ms| format!(" (skipped until {})", format_local_ms(ms)))
            .unwrap_or_default();
        format!(
            "`{}` on `{}`: {}{until}",
            self.model_key, self.provider_id, self.reason
        )
    }
}

/// What to do about provider `provider_id`, of `kind`, rejecting roko's
/// credentials, and how to use it again before its skip ends (backlog
/// 1115): the hint `roko run` and ACP both give.
#[must_use]
pub fn credentials_hint(provider_id: &str, kind: ProviderKind, key_env: Option<&str>) -> String {
    format!(
        "`{provider_id}` rejected its credentials: {}, then run `roko config providers \
         reset-health {provider_id}` to use it before its skip ends.",
        credentials_fix(kind, key_env)
    )
}

/// How to restore credentials a provider of `kind` rejected (backlog 1115): a
/// CLI agent's login, which needs USER and HOME in its environment, or a valid
/// key in the variable its config names.
#[must_use]
pub fn credentials_fix(kind: ProviderKind, key_env: Option<&str>) -> String {
    let login = match kind {
        ProviderKind::ClaudeCli => Some("claude /login"),
        ProviderKind::CodexCli => Some("codex login"),
        ProviderKind::GeminiCli => Some("gemini /auth"),
        ProviderKind::CursorCli | ProviderKind::CursorAcp => Some("cursor-agent login"),
        _ => None,
    };
    match (login, key_env) {
        (Some(login), _) => format!("run `{login}`; under `env -i` also pass USER and HOME"),
        (None, Some(env)) => {
            format!("put a valid key in {env} (~/.roko/.env is loaded automatically at startup)")
        }
        (None, None) => "log its CLI in or give it valid credentials".to_string(),
    }
}

/// What a model key resolves to under a config.
struct Target {
    provider_id: String,
    provider_kind: ProviderKind,
    model_slug: String,
    profile: Option<ModelProfile>,
    provider: Option<ProviderConfig>,
}

impl Target {
    fn of(config: &RokoConfig, model_key: &str) -> Self {
        let resolved = resolve_model(config, model_key);
        let provider_id = resolved
            .profile
            .as_ref()
            .map(|profile| profile.provider.trim().to_string())
            .filter(|provider| !provider.is_empty())
            .unwrap_or_else(|| resolved.provider_kind.label().to_string());
        Self {
            provider_id,
            provider_kind: resolved.provider_kind,
            model_slug: resolved.slug,
            profile: resolved.profile,
            provider: resolved.provider_config,
        }
    }
}

/// Why a provider must not be called now.
struct Hold {
    class: &'static str,
    reason: String,
    until_ms: Option<i64>,
    /// Calling it again cannot help soon, unlike an open circuit that may
    /// already have recovered.
    definitive: bool,
}

/// The registry's hold on `provider_id`, if it holds it out of routing.
fn health_hold(registry: &ProviderHealthRegistry, provider_id: &str) -> Option<Hold> {
    if registry.is_available(provider_id) {
        return None;
    }
    let health = registry.get(provider_id);
    let (class, reason, definitive) = match health
        .failure_window
        .back()
        .map(|record| record.error_class)
    {
        Some(ErrorClass::Exhausted) => ("provider_exhausted", "out of usage", true),
        Some(ErrorClass::Billing) => ("billing", "billing failure", true),
        // A login does not fix itself within a run (bug-52c48f).
        Some(ErrorClass::AuthFailure) => ("auth_failure", "not logged in or key rejected", true),
        _ => (
            "circuit_open",
            "circuit open after repeated failures",
            false,
        ),
    };
    Some(Hold {
        class,
        reason: reason.to_string(),
        until_ms: health.cooldown_until,
        definitive,
    })
}

/// One call's failover: the model it runs on, and the models it passed over
/// and why, which its record carries beside the one that ran.
#[derive(Debug, Clone)]
pub struct Failover {
    config: Arc<RokoConfig>,
    pinned: bool,
    needs_tools: bool,
    first_refused: Option<RefusedModel>,
    refusals: Vec<Refusal>,
}

impl Failover {
    /// The failover of a call planned under `config`. A `pinned` call never
    /// moves; `needs_tools` passes over candidates that cannot run roko's tool
    /// loop.
    #[must_use]
    pub fn new(config: Arc<RokoConfig>, pinned: bool, needs_tools: bool) -> Self {
        Self {
            config,
            pinned,
            needs_tools,
            first_refused: None,
            refusals: Vec::new(),
        }
    }

    /// The models passed over so far, the planned model first.
    #[must_use]
    pub fn refusals(&self) -> &[Refusal] {
        &self.refusals
    }

    /// What to do about each provider among the refusals that rejected
    /// roko's credentials ([`credentials_hint`]), once per provider: a
    /// login, which waiting does not bring (backlog 1115, gap-ade918).
    #[must_use]
    pub fn credentials_hints(&self) -> Vec<String> {
        let providers = self.config.effective_providers();
        let mut hinted: Vec<&str> = Vec::new();
        let mut hints = Vec::new();
        for refusal in &self.refusals {
            if refusal.class != "auth_failure" || hinted.contains(&refusal.provider_id.as_str()) {
                continue;
            }
            let Some(provider) = providers.get(&refusal.provider_id) else {
                continue;
            };
            hinted.push(&refusal.provider_id);
            let key_env = provider.api_key_env.as_deref();
            hints.push(credentials_hint(
                &refusal.provider_id,
                provider.kind,
                key_env,
            ));
        }
        hints
    }

    /// The candidate a call starts on: `planned`, unless its provider is
    /// disabled, has no credentials, or is held out by `registry`; then the
    /// first usable candidate. A planned provider held out only by an open
    /// circuit or `routing.disabled_providers` still runs when nothing else
    /// can, and a pinned call always starts on `planned`.
    ///
    /// # Errors
    ///
    /// Neither the planned model nor any candidate can run; the message names
    /// each refusal and each candidate passed over.
    pub fn start(
        &mut self,
        registry: &ProviderHealthRegistry,
        planned: &str,
    ) -> Result<FailoverCandidate, String> {
        let planned_candidate = FailoverCandidate {
            model_key: planned.to_string(),
            config: None,
        };
        if self.pinned {
            return Ok(planned_candidate);
        }
        let target = Target::of(&self.config, planned);
        let Some(hold) = self.planned_hold(registry, &target, planned) else {
            return Ok(planned_candidate);
        };
        let definitive = hold.definitive;
        self.refuse(&target, planned, hold, false);
        match self.next(registry) {
            Ok(next) => Ok(next),
            Err(_) if !definitive => {
                self.refusals.pop();
                self.first_refused = None;
                Ok(planned_candidate)
            }
            Err(error) => Err(error),
        }
    }

    /// After `candidate`'s provider refused a call with `text`, its output or
    /// error: when `text` is a usage exhaustion, quarantine the provider in
    /// `registry` ([`record_exhaustion`]) and return the next usable
    /// candidate. `Ok(None)` when `text` is no exhaustion: the refusal stands.
    ///
    /// # Errors
    ///
    /// An exhaustion with nowhere to go: the call is pinned, or no candidate
    /// can run. The provider is quarantined either way.
    pub fn after_refusal(
        &mut self,
        registry: &ProviderHealthRegistry,
        candidate: &FailoverCandidate,
        text: &str,
    ) -> Result<Option<FailoverCandidate>, String> {
        let config = candidate.config.as_deref().unwrap_or(&self.config);
        let target = Target::of(config, &candidate.model_key);
        let Some(exhaustion) = record_exhaustion(&self.config, registry, &target.provider_id, text)
        else {
            return Ok(None);
        };
        tracing::warn!(
            provider = %target.provider_id,
            model = %candidate.model_key,
            until = %format_local_ms(exhaustion.until_ms),
            reason = %exhaustion.message,
            "provider is out of usage; skipping it until its reset"
        );
        let hold = Hold {
            class: "provider_exhausted",
            reason: exhaustion.message,
            until_ms: Some(exhaustion.until_ms),
            definitive: true,
        };
        self.refuse(&target, &candidate.model_key, hold, true);
        if self.pinned {
            return Err(self.no_usable_model(&[]));
        }
        self.next(registry).map(Some)
    }

    /// Why the planned model's provider must not be called now. A model that
    /// resolves to no configured provider is left to the dispatch path, which
    /// says why it cannot run it.
    fn planned_hold(
        &self,
        registry: &ProviderHealthRegistry,
        target: &Target,
        model_key: &str,
    ) -> Option<Hold> {
        if self
            .config
            .routing
            .disabled_providers
            .contains(&target.provider_id)
        {
            return Some(Hold {
                class: "disabled",
                reason: "listed in routing.disabled_providers".to_string(),
                until_ms: None,
                definitive: false,
            });
        }
        if let Some(provider) = target.provider.as_ref()
            && !self.config.provider_available_for_model_key(model_key)
        {
            return Some(Hold {
                class: "no_credentials",
                reason: missing_credentials_reason(provider, &target.provider_id),
                until_ms: None,
                definitive: true,
            });
        }
        health_hold(registry, &target.provider_id)
    }

    fn refuse(&mut self, target: &Target, model_key: &str, hold: Hold, called: bool) {
        if self.first_refused.is_none() {
            self.first_refused = Some(RefusedModel {
                model_key: model_key.to_string(),
                model_slug: target.model_slug.clone(),
                provider_id: target.provider_id.clone(),
                provider_kind: target.provider_kind,
                profile: target.profile.clone(),
            });
        }
        self.refusals.push(Refusal {
            model_key: model_key.to_string(),
            provider_id: target.provider_id.clone(),
            class: hold.class,
            reason: hold.reason,
            until_ms: hold.until_ms,
            called,
        });
    }

    /// The first usable candidate after the refusals so far, with the
    /// substitution logged at WARN.
    fn next(&self, registry: &ProviderHealthRegistry) -> Result<FailoverCandidate, String> {
        let mut skipped = Vec::new();
        for candidate in failover_candidates(&self.config, self.first_refused.as_ref()) {
            match self.usable(registry, &candidate) {
                Ok(provider_id) => {
                    if let Some(refused) = self.refusals.last() {
                        tracing::warn!(
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
        Err(self.no_usable_model(&skipped))
    }

    /// The provider of `candidate` when it can take the call now, else why not.
    fn usable(
        &self,
        registry: &ProviderHealthRegistry,
        candidate: &FailoverCandidate,
    ) -> Result<String, String> {
        let config = candidate.config.as_deref().unwrap_or(&self.config);
        let target = Target::of(config, &candidate.model_key);
        let provider_id = &target.provider_id;
        let (Some(profile), Some(provider)) = (&target.profile, &target.provider) else {
            return Err(format!("provider `{provider_id}` is not configured"));
        };
        if self
            .refusals
            .iter()
            .any(|refusal| &refusal.provider_id == provider_id)
        {
            return Err(format!("provider `{provider_id}` is unavailable too"));
        }
        if self.config.routing.disabled_providers.contains(provider_id) {
            return Err(format!(
                "provider `{provider_id}` is in routing.disabled_providers"
            ));
        }
        if self.needs_tools && !profile.supports_tools && !provider_brings_own_tools(provider) {
            return Err("model does not support tools".to_string());
        }
        if !config.provider_available_for_model_key(&candidate.model_key) {
            return Err(missing_credentials_reason(provider, provider_id));
        }
        if let Some(hold) = health_hold(registry, provider_id) {
            let until = hold
                .until_ms
                .map(|ms| format!(" until {}", format_local_ms(ms)))
                .unwrap_or_default();
            return Err(format!("provider `{provider_id}` is quarantined{until}"));
        }
        Ok(provider_id.clone())
    }

    /// Why the call has nowhere to go: each refusal, each candidate passed
    /// over, and how to recover.
    fn no_usable_model(&self, skipped: &[String]) -> String {
        let refused = self
            .refusals
            .iter()
            .map(Refusal::describe)
            .collect::<Vec<_>>()
            .join("; ");
        if self.pinned {
            return format!(
                "no usable provider: {refused}. The model is pinned, so no failover was attempted."
            );
        }
        let fallbacks = if skipped.is_empty() {
            "no fallback model is configured; add `[routing] fallback_models` to roko.toml"
                .to_string()
        } else {
            format!("no fallback model can run: {}", skipped.join("; "))
        };
        format!("no usable provider: {refused}; {fallbacks}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// bug-52c48f: a provider whose last failure was an auth failure is held
    /// out as such, definitively, like an exhausted or a billing one, and not
    /// as a circuit that may recover on its own.
    #[test]
    fn health_hold_reports_auth_failure_as_definitive() {
        let registry = ProviderHealthRegistry::new();
        registry.record_failure("claude_cli", ErrorClass::AuthFailure);
        let hold = health_hold(&registry, "claude_cli").expect("the provider is held");
        assert_eq!(hold.class, "auth_failure");
        assert!(hold.definitive);
        assert!(hold.until_ms.is_some());

        for _ in 0..3 {
            registry.record_failure("zai", ErrorClass::ServerError);
        }
        let hold = health_hold(&registry, "zai").expect("three failures hold it");
        assert_eq!(hold.class, "circuit_open");
        assert!(!hold.definitive);
    }

    /// A config with a refusing `claude_cli` provider, an Anthropic API
    /// provider of the same family, and two OpenAI-compatible providers: one
    /// whose key env var is set (`PATH`) and one whose is never set. The
    /// Anthropic provider is named like the one `ANTHROPIC_API_KEY`
    /// synthesizes, so a key in the environment cannot add another.
    fn config() -> RokoConfig {
        let provider = |kind, command: Option<&str>, key_env: Option<&str>| ProviderConfig {
            kind,
            command: command.map(str::to_string),
            api_key_env: key_env.map(str::to_string),
            base_url: Some("http://127.0.0.1:9/v1".to_string()),
            ..ProviderConfig::default()
        };
        let model = |provider: &str, slug: &str| ModelProfile {
            provider: provider.to_string(),
            slug: slug.to_string(),
            supports_tools: true,
            ..ModelProfile::default()
        };
        let mut config = RokoConfig::default();
        config.providers.clear();
        config.models.clear();
        config.providers.insert(
            "claude_cli".to_string(),
            provider(ProviderKind::ClaudeCli, Some("/bin/sh"), None),
        );
        config.providers.insert(
            "anthropic".to_string(),
            provider(ProviderKind::AnthropicApi, None, Some("PATH")),
        );
        config.providers.insert(
            "api".to_string(),
            provider(ProviderKind::OpenAiCompat, None, Some("PATH")),
        );
        config.providers.insert(
            "keyless".to_string(),
            provider(
                ProviderKind::OpenAiCompat,
                None,
                Some("ROKO_TEST_FAILOVER_KEY_NEVER_SET"),
            ),
        );
        config.models.insert(
            "sonnet".to_string(),
            model("claude_cli", "claude-sonnet-4-6"),
        );
        config
            .models
            .insert("keyless-model".to_string(), model("keyless", "keyless-1"));
        config
            .models
            .insert("api-model".to_string(), model("api", "api-model-1"));
        config.agent.default_model = "sonnet".to_string();
        config.routing.fallback_models = vec!["keyless-model".to_string(), "api-model".to_string()];
        // The OpenAI-compatible providers synthesized from keys in the
        // environment never take part.
        config.routing.disabled_providers = vec![
            "openai".to_string(),
            "gemini".to_string(),
            "perplexity".to_string(),
        ];
        config
    }

    const SESSION_LIMIT: &str = "You’ve hit your session limit · resets 4pm (Europe/Berlin)";

    /// The candidates follow the Graph path's order: the refused slug on its
    /// family's other providers, `[routing] fallback_models`, then
    /// `agent.default_model`.
    #[test]
    fn failover_candidates_keep_the_graph_order() {
        let config = Arc::new(config());
        let refused = RefusedModel {
            model_key: "sonnet".to_string(),
            model_slug: "claude-sonnet-4-6".to_string(),
            provider_id: "claude_cli".to_string(),
            provider_kind: ProviderKind::ClaudeCli,
            profile: None,
        };

        let candidates = failover_candidates(&config, Some(&refused));

        let keys: Vec<&str> = candidates
            .iter()
            .map(|candidate| candidate.model_key.as_str())
            .collect();
        assert_eq!(
            keys,
            [
                "claude-sonnet-4-6@anthropic",
                "keyless-model",
                "api-model",
                "sonnet"
            ]
        );
        let synthesized = candidates[0].config.as_ref().expect("its own config");
        assert_eq!(
            synthesized.models["claude-sonnet-4-6@anthropic"].provider,
            "anthropic"
        );
        assert!(
            candidates[1..]
                .iter()
                .all(|candidate| candidate.config.is_none())
        );
    }

    /// gap-28ceb9: a session-limit refusal quarantines the provider until its
    /// reset and moves the call to the first usable candidate. Neither the
    /// refused provider nor one without a key is chosen, and the refusal is
    /// kept for the call's record.
    #[test]
    fn an_exhausted_provider_is_quarantined_and_the_call_moves_on() {
        let mut config = config();
        config
            .routing
            .disabled_providers
            .push("anthropic".to_string());
        let registry = ProviderHealthRegistry::new();
        let mut failover = Failover::new(Arc::new(config), false, true);

        let planned = failover.start(&registry, "sonnet").expect("planned runs");
        assert_eq!(planned.model_key, "sonnet");
        let next = failover
            .after_refusal(&registry, &planned, SESSION_LIMIT)
            .expect("a candidate runs")
            .expect("an exhaustion");

        assert_eq!(next.model_key, "api-model");
        assert!(!registry.is_available("claude_cli"));
        let [refusal] = failover.refusals() else {
            panic!("one refusal: {:?}", failover.refusals());
        };
        assert_eq!(
            (refusal.model_key.as_str(), refusal.class, refusal.called),
            ("sonnet", "provider_exhausted", true)
        );
        assert!(refusal.reason.contains("session limit"), "{refusal:?}");
        // A failure that is no exhaustion leaves the provider alone.
        let other = failover
            .after_refusal(&registry, &next, "connection reset by peer")
            .expect("not an exhaustion");
        assert!(other.is_none());
        assert!(registry.is_available("api"));
    }

    /// A planned model whose provider is quarantined is passed over before any
    /// call; a pinned call keeps it, and its exhaustion has nowhere to go.
    #[test]
    fn a_quarantined_planned_model_is_skipped_unless_pinned() {
        let mut config = config();
        config
            .routing
            .disabled_providers
            .push("anthropic".to_string());
        let config = Arc::new(config);
        let registry = ProviderHealthRegistry::new();
        let in_an_hour = chrono::Utc::now().timestamp_millis() + 3_600_000;
        registry.record_exhaustion("claude_cli", in_an_hour);

        let mut failover = Failover::new(Arc::clone(&config), false, false);
        let first = failover.start(&registry, "sonnet").expect("a candidate");
        assert_eq!(first.model_key, "api-model");
        assert_eq!(failover.refusals()[0].class, "provider_exhausted");
        assert!(!failover.refusals()[0].called);

        let mut pinned = Failover::new(config, true, false);
        let first = pinned.start(&registry, "sonnet").expect("pinned runs");
        assert_eq!(first.model_key, "sonnet");
        let error = pinned
            .after_refusal(&registry, &first, SESSION_LIMIT)
            .expect_err("a pinned call does not move");
        assert!(error.contains("pinned"), "{error}");
    }

    /// With no usable candidate the error names the refusal and every
    /// candidate passed over.
    #[test]
    fn no_usable_candidate_names_every_refusal() {
        let mut config = config();
        config
            .routing
            .disabled_providers
            .push("anthropic".to_string());
        config.routing.fallback_models = vec!["keyless-model".to_string()];
        let registry = ProviderHealthRegistry::new();
        let mut failover = Failover::new(Arc::new(config), false, false);
        let planned = failover.start(&registry, "sonnet").expect("planned runs");

        let error = failover
            .after_refusal(&registry, &planned, SESSION_LIMIT)
            .expect_err("nothing can run");

        assert!(error.contains("`sonnet` on `claude_cli`"), "{error}");
        assert!(
            error.contains("keyless-model: ROKO_TEST_FAILOVER_KEY_NEVER_SET is not set"),
            "{error}"
        );
        assert!(
            error.contains("sonnet: provider `claude_cli` is unavailable too"),
            "{error}"
        );
    }
}
