//! HTTP serving, server, deploy, scheduler, and webhook configuration sections.

use serde::{Deserialize, Serialize};

use super::agent::default_true;

// ---- [statehub] ----------------------------------------------------------

/// StateHub projection persistence and retention settings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateHubConfig {
    /// Maximum age of retained projection versions (`ms`, `s`, `m`, `h`, or `d`).
    #[serde(default = "default_projection_history_retention")]
    pub history_retention: String,
}

impl StateHubConfig {
    /// Parse the configured projection-history age window.
    pub fn history_retention_duration(&self) -> Result<std::time::Duration, String> {
        crate::duration::parse_duration(&self.history_retention).map_err(|e| e.to_string())
    }
}

impl Default for StateHubConfig {
    fn default() -> Self {
        Self {
            history_retention: default_projection_history_retention(),
        }
    }
}

fn default_projection_history_retention() -> String {
    "7d".to_string()
}

// ---- [serve] -------------------------------------------------------------

/// How much agent output streams live before the immune boundary screens it.
///
/// Configurable via `serve.live_agent_output` in `roko.toml`.
///
/// **`"tool_steps"` (default):** each tool invocation streams as it happens —
/// its name and target only (for example `Write apps/x.ts` or
/// `` Bash `cargo test` ``).  No arguments beyond the target, and no tool
/// results.  This is always safe to emit regardless of bind address.
///
/// **`"trusted"`:** in addition to tool steps, the full text, reasoning, tool
/// calls, and tool results stream before the immune boundary screens them.
/// This setting takes effect **only** when the server is loopback-only.  A
/// non-loopback bind ignores `"trusted"` and behaves as `"tool_steps"`,
/// logging a warning at startup.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveAgentOutput {
    /// Stream tool-call names and targets only (always safe to emit).
    #[default]
    ToolSteps,
    /// Stream full text, reasoning, and tool results before immune-boundary
    /// screening.  Requires a loopback-only bind; silently reverts to
    /// `ToolSteps` on a non-loopback host.
    Trusted,
}

impl LiveAgentOutput {
    /// Resolve the configured mode against the current bind address.
    ///
    /// Returns [`LiveAgentOutput::Trusted`] only when `self` is `Trusted`
    /// **and** `loopback_only` is `true`.  All other combinations return
    /// [`LiveAgentOutput::ToolSteps`] — trusted output must never be enabled
    /// on a publicly-reachable host.
    #[must_use]
    pub fn effective(self, loopback_only: bool) -> LiveAgentOutput {
        if self == LiveAgentOutput::Trusted && loopback_only {
            LiveAgentOutput::Trusted
        } else {
            LiveAgentOutput::ToolSteps
        }
    }
}

/// The groups of routes `roko serve` can mount without authentication (G1): `/health`,
/// `/ready`, `/metrics`, the GitHub and Slack webhook receivers, webhook triggers, the share
/// readers (`/api/shared/{token}`, `/runs/{id}`), and the 403 answers of a disabled terminal.
pub const PUBLIC_ROUTE_GROUPS: [&str; 7] = [
    "health", "ready", "metrics", "webhooks", "triggers", "shared", "terminal",
];

/// API serving options.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServeConfig {
    /// Port override for `roko serve`. Falls back to `server.port` (default 6677).
    #[serde(default)]
    pub port: Option<u16>,
    /// Shared transcript retention period in days.
    ///
    /// Newly created shares expire after this many days unless they are
    /// created with `--no-expire`.
    #[serde(default = "default_share_ttl_days")]
    pub share_ttl_days: u64,
    /// Whether to expose the PTY terminal routes.
    ///
    /// Disabled by default because the terminal is shell access.
    #[serde(default)]
    pub terminal_enabled: bool,
    /// Command lines `POST /api/terminal/sessions` may run in place of the
    /// login shell, matched exactly once runs of whitespace are collapsed.
    ///
    /// Empty by default: a session request that names a `command` is refused
    /// and every session runs the login shell. Listing a command is the
    /// explicit opt-in for running it directly.
    #[serde(default)]
    pub terminal_commands: Vec<String>,
    /// Most PTY sessions that may be open at once; creating one more is
    /// refused. `0` removes the cap.
    #[serde(default = "default_terminal_max_sessions")]
    pub terminal_max_sessions: usize,
    /// Seconds a PTY session may live, attached or not, before the server
    /// closes it. `0` lets sessions live until they exit or are deleted.
    #[serde(default = "default_terminal_session_ttl_secs")]
    pub terminal_session_ttl_secs: u64,
    /// Automatically orchestrate follow-up work when publish events arrive.
    #[serde(default = "default_true")]
    pub auto_orchestrate: bool,
    /// Times `POST /api/plans/{id}/revise` asks the planning agent again,
    /// with the validation diagnostics, after it proposes a revision that
    /// fails validation. `0` makes one attempt only.
    #[serde(default = "default_revision_max_retries")]
    pub revision_max_retries: u32,
    /// Authentication settings for `/api/*`.
    #[serde(default)]
    pub auth: ServeAuthConfig,
    /// Cloud deployment settings.
    #[serde(default)]
    pub deploy: ServeDeployConfig,
    /// Whether `roko` with no subcommand should auto-start the HTTP server.
    ///
    /// Disabled by default so the control plane is opt-in.
    #[serde(default)]
    pub auto_start: bool,
    /// Set to `true` to acknowledge the risk of a public bind without auth.
    ///
    /// Required when binding to a non-loopback address with `auth.enabled = false`.
    #[serde(default)]
    pub acknowledge_public_risk: bool,
    /// Additional remote IPs or CIDR ranges allowed to POST RuntimeEvent ingest without API auth.
    ///
    /// Loopback callers are always allowed. Public callers should normally use
    /// `serve.auth.enabled = true` instead of this allowlist.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub event_ingest_allowlist: Vec<String>,
    /// How much agent output streams live before the immune boundary screens it.
    ///
    /// - `"tool_steps"` (default): each tool invocation streams its name and
    ///   target as it happens.  No arguments beyond the target, no tool results.
    /// - `"trusted"`: full text, reasoning, tool calls, and tool results stream
    ///   before immune-boundary screening.  Takes effect **only** when the server
    ///   is loopback-only; a non-loopback bind silently reverts to `"tool_steps"`
    ///   and logs a warning at startup.
    #[serde(default)]
    pub live_agent_output: LiveAgentOutput,
    /// Optional OTLP tracing export. Disabled when `otlp_endpoint` is absent.
    #[serde(default)]
    pub tracing: TracingConfig,
    /// Limits on runs a chat host starts over `POST /mcp` (9116).
    #[serde(default)]
    pub mcp: ServeMcpConfig,
    /// The unauthenticated route groups to mount, from [`PUBLIC_ROUTE_GROUPS`] (G1, 9327).
    ///
    /// The default is all of them. A showcase deploy lists only `health` and `ready`; in showcase
    /// mode the socket, relay, MCP and terminal routes are not mounted either way.
    #[serde(default = "default_public_routes")]
    pub public_routes: Vec<String>,
}

/// `[serve.mcp]`: limits on the runs a chat host starts over `POST /mcp`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServeMcpConfig {
    /// The most a run started over `/mcp` may spend, in USD. A `run_prompt`
    /// or `plan_run` call must name a cap (`max_usd`) above 0 and at most
    /// this, and the cap becomes the run's budget ceiling. Defaults to 5.00;
    /// runs started any other way are not affected.
    #[serde(default = "default_mcp_max_run_usd")]
    pub max_run_usd: f64,
    /// Let a chat host's run start without the data-model boundary
    /// (`[agent.data_llm]`), which screens what the run reads (9117).
    /// Defaults to false: such a run is refused.
    #[serde(default)]
    pub allow_without_data_llm: bool,
}

impl Default for ServeMcpConfig {
    fn default() -> Self {
        Self {
            max_run_usd: default_mcp_max_run_usd(),
            allow_without_data_llm: false,
        }
    }
}

fn default_mcp_max_run_usd() -> f64 {
    5.0
}

// ---- runs a chat host starts (9117, gap-cf427e) --------------------------

/// What stops a run a chat host starts over `/mcp` before anything is queued
/// (9117, gap-cf427e). A host reads [`Self::code`] as the `refusal` of the
/// tool error's data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatRunBlocker {
    /// No data-model boundary: `[agent.data_llm]`, which screens what such a
    /// run reads, is unset while `[serve.mcp] allow_without_data_llm` is off;
    /// or it names a model roko cannot call over an API with its key set,
    /// which fails every agent roko's own tool loop runs.
    NoDataBoundary,
    /// No model a task can be put on runs in roko's own tool loop with its
    /// credentials set. Such a run holds its tool calls that act on the
    /// outside world for approval (`stage`, decision 9107), which a CLI agent
    /// cannot do, so every task would fail at dispatch.
    NoToolLoopModel,
    /// No model such a run can be put on has a price. The run is held to its
    /// `max_usd`, a cap can count only priced calls, and dispatch refuses an
    /// unpriced model under a cap (gap-8548e3), so every task would fail.
    NoPrice,
}

impl ChatRunBlocker {
    /// The machine-readable code: `no_data_boundary`, `no_tool_loop_model` or
    /// `no_price`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::NoDataBoundary => "no_data_boundary",
            Self::NoToolLoopModel => "no_tool_loop_model",
            Self::NoPrice => "no_price",
        }
    }
}

/// Why a run a chat host starts may not start ([`super::schema::RokoConfig::chat_run_refusal`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatRunRefusal {
    /// Every blocker that applies, [`ChatRunBlocker::NoDataBoundary`] first.
    pub blockers: Vec<ChatRunBlocker>,
    /// What the host shows its person: each blocker's reason and fix, in order.
    pub message: String,
}

impl ChatRunRefusal {
    /// The first blocker's code, the one a host matches.
    #[must_use]
    pub fn code(&self) -> &'static str {
        self.blockers.first().map_or("", |blocker| blocker.code())
    }

    /// Every blocker's code, in order.
    #[must_use]
    pub fn codes(&self) -> Vec<&'static str> {
        self.blockers.iter().map(|blocker| blocker.code()).collect()
    }
}

/// The words of [`ChatRunBlocker::NoDataBoundary`] when `[agent.data_llm]` is
/// unset. Chat hosts' readiness probes match "data-model boundary", so the
/// wording stays.
const NO_DATA_BOUNDARY: &str = "a run from a chat host needs the data-model boundary: set \
                                [agent.data_llm] in roko.toml, or [serve.mcp] \
                                allow_without_data_llm = true to run without it";

impl super::schema::RokoConfig {
    /// Why a run a chat host starts may not start, or `None` when it may
    /// (9117, gap-cf427e). Its request text is untrusted data, and the
    /// data-model boundary (`[agent.data_llm]`) screens what such a run
    /// reads, so without the boundary the run is refused unless `[serve.mcp]
    /// allow_without_data_llm` is set. Such a run stages its tool calls that
    /// act on the outside world (decision 9107), which only roko's own tool
    /// loop can do, so it is refused too when no model it can be routed to
    /// runs there with its credentials set ([`Self::tool_loop_models`]). The
    /// run is held to its `max_usd`, so it is refused as well when none of
    /// those models has a price in `prices`, the workspace's price snapshot,
    /// its `[models.*]` rates or the built-in table (gap-8548e3).
    #[must_use]
    pub fn chat_run_refusal(
        &self,
        prices: Option<&crate::pricing_snapshot::PriceSnapshot>,
    ) -> Option<ChatRunRefusal> {
        self.chat_run_refusal_with_env(prices, &|name| std::env::var(name).ok())
    }

    /// [`Self::chat_run_refusal`] with the environment `env` holds.
    pub(crate) fn chat_run_refusal_with_env(
        &self,
        prices: Option<&crate::pricing_snapshot::PriceSnapshot>,
        env: &dyn Fn(&str) -> Option<String>,
    ) -> Option<ChatRunRefusal> {
        let mut blockers = Vec::new();
        let mut reasons = Vec::new();
        match &self.agent.data_llm {
            None if !self.serve.mcp.allow_without_data_llm => {
                blockers.push(ChatRunBlocker::NoDataBoundary);
                reasons.push(NO_DATA_BOUNDARY.to_string());
            }
            None => {}
            Some(data_llm) => {
                let model = data_llm.model.trim();
                if let Err(why) = self.data_llm_model_usable_with_env(model, env) {
                    blockers.push(ChatRunBlocker::NoDataBoundary);
                    reasons.push(format!(
                        "a run from a chat host needs a working data-model boundary, and \
                         [agent.data_llm] model `{model}` {why}: name a model roko calls over an \
                         API, with its key set"
                    ));
                }
            }
        }
        if !self.chat_run_has_tool_loop_model_with_env(env) {
            blockers.push(ChatRunBlocker::NoToolLoopModel);
            reasons.push(self.no_tool_loop_model_reason_with_env(env));
        } else if let Some(reason) = self.no_price_reason_with_env(prices, env) {
            blockers.push(ChatRunBlocker::NoPrice);
            reasons.push(reason);
        }
        (!blockers.is_empty()).then(|| ChatRunRefusal {
            blockers,
            message: reasons.join("; and "),
        })
    }

    /// Why `model` cannot serve as `[agent.data_llm]`'s model, as the end of
    /// a sentence about it, or `None` when it can: roko calls the data model
    /// over an API (`roko_agent::provider::data_llm_boundary`), so it must
    /// resolve to a configured API provider with its key set (gap-cf427e).
    #[must_use]
    pub fn data_llm_model_problem(&self, model: &str) -> Option<String> {
        self.data_llm_model_usable_with_env(model.trim(), &|name| std::env::var(name).ok())
            .err()
    }

    /// The `[models.*]` keys (or names routing gives) whose agents roko's own
    /// tool loop runs with their credentials set (gap-cf427e): models that
    /// can call tools, on a provider that runs there
    /// ([`super::provider::ProviderConfig::runs_in_own_tool_loop`]), is not
    /// in `[routing] disabled_providers` and has its key. Only these can run a
    /// task that stages its tool calls, as a chat host's run does. The
    /// candidates are the `[models.*]` entries and the models routing names:
    /// the ladder's rungs, `[routing] fallback_models`,
    /// `agent.fallback_model` and `agent.default_model`. The ladder's rungs
    /// come first, cheapest first, then the rest by their configured price.
    #[must_use]
    pub fn tool_loop_models(&self) -> Vec<String> {
        self.tool_loop_models_with_env(&|name| std::env::var(name).ok())
    }

    /// Why no model roko's own tool loop runs is set up here, and how to fix
    /// it: the reason a chat host's run is refused with `no_tool_loop_model`,
    /// which a chat host's plan writing gives too (bug-a0432b).
    #[must_use]
    pub fn no_tool_loop_model_reason(&self) -> String {
        self.no_tool_loop_model_reason_with_env(&|name| std::env::var(name).ok())
    }

    /// [`Self::tool_loop_models`] with the environment `env` holds.
    pub(crate) fn tool_loop_models_with_env(
        &self,
        env: &dyn Fn(&str) -> Option<String>,
    ) -> Vec<String> {
        let providers = self.effective_providers_with_env(env);
        let mut models: Vec<(usize, f64, String)> = Vec::new();
        for (rank, key) in self.routing_candidate_models().into_iter().enumerate() {
            let Ok((profile, provider_id, provider)) = self.tool_model(&key, &providers) else {
                continue;
            };
            if self
                .provider_runs_task_tools(&provider_id, provider, env)
                .is_ok()
            {
                let price = profile.cost_input_per_m.unwrap_or(f64::MAX)
                    + profile.cost_output_per_m.unwrap_or(f64::MAX);
                models.push((rank, price, key));
            }
        }
        // Rungs keep the ladder's order (cheapest first); the rest follow by price.
        let rungs = self.ladder_rung_models().len();
        models.sort_by(|(a_rank, a_price, a_key), (b_rank, b_price, b_key)| {
            let (a_rung, b_rung) = (*a_rank < rungs, *b_rank < rungs);
            b_rung
                .cmp(&a_rung)
                .then_with(|| {
                    if a_rung && b_rung {
                        a_rank.cmp(b_rank)
                    } else {
                        a_price.total_cmp(b_price)
                    }
                })
                .then_with(|| a_key.cmp(b_key))
        });
        models.into_iter().map(|(_, _, key)| key).collect()
    }

    /// Whether a task of a chat host's run can be put on a model roko's own
    /// tool loop runs: one of [`Self::tool_loop_models`], or a model routing
    /// names whose slug failover can run on another provider of its family
    /// that qualifies (`claude_cli`'s Claude slug on the Anthropic API).
    fn chat_run_has_tool_loop_model_with_env(&self, env: &dyn Fn(&str) -> Option<String>) -> bool {
        if !self.tool_loop_models_with_env(env).is_empty() {
            return true;
        }
        let providers = self.effective_providers_with_env(env);
        self.routing_candidate_models().iter().any(|key| {
            let Ok((_, provider_id, provider)) = self.tool_model(key, &providers) else {
                return false;
            };
            providers.iter().any(|(other_id, other)| {
                other_id != &provider_id
                    && provider.kind.same_family().contains(&other.kind)
                    && self.provider_runs_task_tools(other_id, other, env).is_ok()
            })
        })
    }

    /// The models routing can put a task on: the ladder's rung models, then
    /// `[routing] fallback_models`, `agent.fallback_model`,
    /// `agent.default_model` and the `[models.*]` keys, each once, by its
    /// `[models.*]` key when an entry has it as key or slug.
    fn routing_candidate_models(&self) -> Vec<String> {
        let models = self.effective_models();
        let named = self
            .ladder_rung_models()
            .into_iter()
            .chain(self.routing.fallback_models.iter().cloned())
            .chain(self.agent.fallback_model.iter().cloned())
            .chain(std::iter::once(self.agent.default_model.clone()))
            .chain(models.keys().cloned());
        let mut seen = std::collections::HashSet::new();
        named
            .filter(|model| !model.trim().is_empty())
            .map(|model| model_key_of(&models, &model))
            .filter(|model| seen.insert(model.clone()))
            .collect()
    }

    /// The `[models.*]` keys of the ladder's rungs, its roles' own included,
    /// cheapest first, each once; none while the ladder is off. A rung runs a
    /// model a `[models.*]` entry names by key or slug, or a built-in model
    /// (bug-4e336b), so the others are left out.
    fn ladder_rung_models(&self) -> Vec<String> {
        if !self.routing.ladder.enabled {
            return Vec::new();
        }
        let models = self.effective_models();
        let mut seen = std::collections::HashSet::new();
        self.routing
            .ladder
            .all_rungs()
            .filter(|rung| rung_model_known(&models, &rung.model))
            .map(|rung| model_key_of(&models, &rung.model))
            .filter(|model| seen.insert(model.clone()))
            .collect()
    }

    /// `model`'s profile, with the id and config of the provider it runs on:
    /// the one its profile names, else (a builtin model, which names its
    /// provider kind's label) the first of its kind, as
    /// `roko_agent::provider::data_llm_boundary` finds it. Otherwise why
    /// not, as the end of a sentence about it.
    fn model_provider<'a>(
        &self,
        model: &str,
        providers: &'a indexmap::IndexMap<String, super::provider::ProviderConfig>,
    ) -> Result<
        (
            super::provider::ModelProfile,
            String,
            &'a super::provider::ProviderConfig,
        ),
        String,
    > {
        let resolved = crate::agent::resolve_model(self, model);
        let Some(profile) = resolved.profile else {
            return Err("is not a configured model".to_string());
        };
        let found = providers.get_key_value(&profile.provider).or_else(|| {
            providers
                .iter()
                .find(|(_, provider)| provider.kind == resolved.provider_kind)
        });
        let Some((provider_id, provider)) = found else {
            return Err(format!(
                "uses provider `{}`, which is not configured",
                profile.provider
            ));
        };
        Ok((profile, provider_id.clone(), provider))
    }

    /// [`Self::model_provider`] of a model that can call tools, as a task's
    /// agent must; otherwise why not.
    fn tool_model<'a>(
        &self,
        model: &str,
        providers: &'a indexmap::IndexMap<String, super::provider::ProviderConfig>,
    ) -> Result<
        (
            super::provider::ModelProfile,
            String,
            &'a super::provider::ProviderConfig,
        ),
        String,
    > {
        let found = self.model_provider(model, providers)?;
        if !found.0.supports_tools || found.0.is_embedding_model {
            return Err("cannot call tools".to_string());
        }
        Ok(found)
    }

    /// Whether provider `id` runs a task's agent in roko's own tool loop now
    /// (it runs there, is not disabled and has its credentials set), or why
    /// not, as the end of a sentence about a model on it.
    fn provider_runs_task_tools(
        &self,
        id: &str,
        provider: &super::provider::ProviderConfig,
        env: &dyn Fn(&str) -> Option<String>,
    ) -> Result<(), String> {
        if !provider.runs_in_own_tool_loop() {
            let kind = provider.kind.to_string();
            let named = if kind == id {
                id.to_string()
            } else {
                format!("{id} ({kind})")
            };
            return Err(format!(
                "runs on {named}, an agent that runs its own tools, so it cannot hold a tool \
                 call for approval"
            ));
        }
        if self
            .routing
            .disabled_providers
            .iter()
            .any(|disabled| disabled == id)
        {
            return Err(format!(
                "runs on {id}, which [routing] disabled_providers turns off"
            ));
        }
        if !self.is_provider_available_with_env(provider, env) {
            return Err(match api_key_name(provider) {
                Some(key) => format!("runs on {id}, whose key {key} is not set"),
                None => format!("runs on {id}, which names no api_key_env"),
            });
        }
        Ok(())
    }

    /// Why `[agent.data_llm]` model `model` cannot serve as the data model,
    /// as the end of a sentence about it, or `Ok` when it can
    /// ([`Self::data_llm_model_problem`]).
    fn data_llm_model_usable_with_env(
        &self,
        model: &str,
        env: &dyn Fn(&str) -> Option<String>,
    ) -> Result<(), String> {
        let providers = self.effective_providers_with_env(env);
        let (_, provider_id, provider) = self.model_provider(model, &providers)?;
        if !provider.runs_in_own_tool_loop() {
            return Err(format!(
                "runs on `{provider_id}` ({}), which roko cannot call over an API",
                provider.kind
            ));
        }
        if !self.is_provider_available_with_env(provider, env) {
            let key = api_key_name(provider).unwrap_or("its API key");
            return Err(format!(
                "runs on `{provider_id}`, whose key ({key}) is not set"
            ));
        }
        Ok(())
    }

    /// Why each model routing can put a task on cannot run a task of a chat
    /// host's run, as `<model> <why>`: the ladder's rungs first, in its
    /// order and named with their rung, a rung that names no model roko knows
    /// (which the ladder skips) included; then the other models
    /// [`Self::routing_candidate_models`] lists.
    fn tool_loop_misses_with_env(&self, env: &dyn Fn(&str) -> Option<String>) -> Vec<String> {
        let providers = self.effective_providers_with_env(env);
        let models = self.effective_models();
        let miss = |model: &str| match self.tool_model(model, &providers) {
            Ok((_, provider_id, provider)) => self
                .provider_runs_task_tools(&provider_id, provider, env)
                .err(),
            Err(why) => Some(why),
        };
        let mut seen = std::collections::HashSet::new();
        let mut misses = Vec::new();
        let ladder = &self.routing.ladder;
        for rung in ladder.all_rungs().filter(|_| ladder.enabled) {
            let model = rung.model.trim();
            if model.is_empty() || !seen.insert(model_key_of(&models, model)) {
                continue;
            }
            let why = if rung_model_known(&models, model) {
                miss(model)
            } else {
                Some(
                    "has no [models.*] entry, and roko has no built-in model of this name"
                        .to_string(),
                )
            };
            if let Some(why) = why {
                misses.push(format!("rung `{}` ({model}) {why}", rung.name));
            }
        }
        for model in self.routing_candidate_models() {
            if !seen.insert(model.clone()) {
                continue;
            }
            if let Some(why) = miss(&model) {
                misses.push(format!("{model} {why}"));
            }
        }
        misses
    }

    /// The words of [`ChatRunBlocker::NoPrice`] when none of
    /// [`Self::tool_loop_models`] has a price in `prices`, its `[models.*]`
    /// rates or the built-in table, and `None` when one has.
    fn no_price_reason_with_env(
        &self,
        prices: Option<&crate::pricing_snapshot::PriceSnapshot>,
        env: &dyn Fn(&str) -> Option<String>,
    ) -> Option<String> {
        const SHOWN: usize = 6;
        let models = self.tool_loop_models_with_env(env);
        let priced = |model: &String| {
            let resolved = crate::agent::resolve_model(self, model);
            super::model_registry::model_has_price(
                prices,
                resolved.profile.as_ref(),
                &resolved.slug,
            )
        };
        if models.is_empty() || models.iter().any(priced) {
            return None;
        }
        let mut named = models[..models.len().min(SHOWN)].join(", ");
        if models.len() > SHOWN {
            named.push_str(&format!(" and {} more", models.len() - SHOWN));
        }
        Some(format!(
            "a run from a chat host is held to its max_usd, and a cap can count only the calls of \
             a model with a price; the models it can run on have none ({named}). To fix it, set \
             cost_input_per_m and cost_output_per_m on one's [models.*] entry in roko.toml (0 for \
             a free local model)"
        ))
    }

    /// The words of [`ChatRunBlocker::NoToolLoopModel`]: what keeps each
    /// model routing can pick from running such a run (no `[models.*]`
    /// entry, no key, an agent that runs its own tools, ...) and the fix.
    /// They never say "not found" nor start with "no ", which older hosts
    /// read as a missing run or plan.
    fn no_tool_loop_model_reason_with_env(&self, env: &dyn Fn(&str) -> Option<String>) -> String {
        const SHOWN: usize = 6;
        let providers = self.effective_providers_with_env(env);
        let mut keys = std::collections::BTreeSet::new();
        for model in self.routing_candidate_models() {
            let Ok((_, _, provider)) = self.tool_model(&model, &providers) else {
                continue;
            };
            if provider.runs_in_own_tool_loop()
                && !self.is_provider_available_with_env(provider, env)
                && let Some(key) = api_key_name(provider)
            {
                keys.insert(key.to_string());
            }
        }
        let mut reason = "a run from a chat host needs a model that runs in roko's own tool \
                          loop with its key set, and this workspace has none"
            .to_string();
        let misses = self.tool_loop_misses_with_env(env);
        if !misses.is_empty() {
            reason.push_str(": ");
            reason.push_str(&misses[..misses.len().min(SHOWN)].join("; "));
            if misses.len() > SHOWN {
                reason.push_str(&format!("; and {} more", misses.len() - SHOWN));
            }
        }
        if keys.is_empty() {
            reason.push_str(
                ". To fix it, add a [models.*] entry on an API provider (such as openai_compat) \
                 to roko.toml, with its API key in ~/.roko/.env (roko serve reads that file when \
                 it starts)",
            );
        } else {
            let keys: Vec<&str> = keys.iter().map(String::as_str).take(4).collect();
            let named = match keys.as_slice() {
                [key] => (*key).to_string(),
                _ => format!("one of {}", keys.join(", ")),
            };
            reason.push_str(&format!(
                ". To fix it, put {named} in ~/.roko/.env and restart roko serve, which reads \
                 that file when it starts"
            ));
        }
        reason
    }
}

/// Whether a ladder rung's `model` names a model roko knows, as the ladder
/// needs to run it: a `[models.*]` entry by key or slug, or a built-in model
/// (bug-4e336b).
fn rung_model_known(
    models: &indexmap::IndexMap<String, super::provider::ModelProfile>,
    model: &str,
) -> bool {
    let model = model.trim();
    models.contains_key(model)
        || models.values().any(|profile| profile.slug.trim() == model)
        || super::model_registry::builtin_model(model).is_some()
}

/// `model` as the key of the `[models.*]` entry that has it as key or slug,
/// else as given.
fn model_key_of(
    models: &indexmap::IndexMap<String, super::provider::ModelProfile>,
    model: &str,
) -> String {
    let model = model.trim();
    if models.contains_key(model) {
        return model.to_string();
    }
    models
        .iter()
        .find(|(_, profile)| profile.slug.trim() == model)
        .map_or_else(|| model.to_string(), |(key, _)| key.clone())
}

/// The name of the variable that holds `provider`'s API key, when it names
/// one.
fn api_key_name(provider: &super::provider::ProviderConfig) -> Option<&str> {
    provider
        .api_key_env
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self {
            port: None,
            share_ttl_days: default_share_ttl_days(),
            terminal_enabled: false,
            terminal_commands: Vec::new(),
            terminal_max_sessions: default_terminal_max_sessions(),
            terminal_session_ttl_secs: default_terminal_session_ttl_secs(),
            auto_orchestrate: true,
            revision_max_retries: default_revision_max_retries(),
            auth: ServeAuthConfig::default(),
            deploy: ServeDeployConfig::default(),
            auto_start: false,
            acknowledge_public_risk: false,
            event_ingest_allowlist: Vec::new(),
            live_agent_output: LiveAgentOutput::default(),
            tracing: TracingConfig::default(),
            mcp: ServeMcpConfig::default(),
            public_routes: default_public_routes(),
        }
    }
}

fn default_public_routes() -> Vec<String> {
    Vec::from(PUBLIC_ROUTE_GROUPS.map(String::from))
}

fn default_share_ttl_days() -> u64 {
    7
}

fn default_terminal_max_sessions() -> usize {
    8
}

fn default_terminal_session_ttl_secs() -> u64 {
    8 * 60 * 60
}

fn default_revision_max_retries() -> u32 {
    1
}

/// Enforcement behaviour for scope-based permission checks.
///
/// Controls whether the auth middleware blocks requests that fail scope checks
/// or merely logs the violation and allows the request through.
///
/// Configurable via `serve.auth.enforcement_mode` in `roko.toml`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnforcementMode {
    /// Block requests that fail scope checks (return 403). This is the default.
    #[default]
    Enforce,
    /// Log the violation but allow the request through (audit-only mode).
    Audit,
    /// Skip scope checks entirely — no logging, no blocking.
    Disabled,
}

/// A JSON Web Key Set endpoint and the issuer bound to its keys.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JwksProvider {
    /// Provider JWKS endpoint.
    pub url: String,
    /// Exact JWT `iss` value accepted for keys returned by this endpoint.
    pub expected_issuer: String,
}

impl JwksProvider {
    /// Construct a provider definition.
    pub fn new(url: impl Into<String>, expected_issuer: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            expected_issuer: expected_issuer.into(),
        }
    }
}

/// Authentication settings for the HTTP API.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServeAuthConfig {
    /// Whether `/api/*` routes require an `X-Api-Key` header. On by default
    /// ([`ServeAuthConfig::default`]).
    ///
    /// A `[serve.auth]` table that leaves the key out (one holding only
    /// `api_key`, say) keeps auth on too: only `enabled = false` turns it off.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Shared API key expected in `X-Api-Key` (legacy single-key mode).
    #[serde(default)]
    pub api_key: String,
    /// Named API keys with scoped permissions (hashes stored in `.roko/api-keys.json`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub api_keys: Vec<ApiKeyEntry>,
    /// Privy application ID whose JWTs this server validates.
    ///
    /// Unset by default, which turns Privy JWT auth off. Setting it is not
    /// enough on its own: anyone can sign in to a Privy app, so without
    /// `privy_workspace_id` or `privy_allowed_roles` every Privy JWT is
    /// rejected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privy_app_id: Option<String>,
    /// Issuer-bound JWKS endpoints. An empty list uses Privy's per-app
    /// endpoint for `privy_app_id`; a non-empty list replaces it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub jwks_providers: Vec<JwksProvider>,
    /// Privy workspace / org ID that the JWT `org_id` claim must match.
    ///
    /// When set, tokens whose `org_id` claim is missing or different are
    /// rejected, and members get admin scope unless `privy_allowed_roles`
    /// narrows it. When `None`, membership is not checked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privy_workspace_id: Option<String>,
    /// Allowed Privy roles (matched against the JWT `role` claim).
    ///
    /// When non-empty, only tokens whose `role` claim is in this list receive
    /// admin scope; others are downgraded to `"read"`. When empty, roles are
    /// not checked, and Privy JWTs are rejected unless `privy_workspace_id`
    /// is set.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub privy_allowed_roles: Vec<String>,
    /// Enforcement mode for scope-based permission checks.
    ///
    /// - `enforce` (default): block requests that fail scope checks (403).
    /// - `audit`: log the violation but allow the request through.
    /// - `disabled`: skip scope checks entirely.
    #[serde(default)]
    pub enforcement_mode: EnforcementMode,
    /// Lifetime of a workspace invitation before it is rejected and cleaned up.
    #[serde(default = "default_invite_expiry_days")]
    pub invite_expiry_days: u64,
}

impl Default for ServeAuthConfig {
    fn default() -> Self {
        Self {
            // Secure-by-default: `/api/*` requires an `X-Api-Key`. `roko init`
            // writes `enabled = true` with an empty key; `roko serve` mints a
            // one-time launch token so new workspaces work without manual key
            // configuration. Local users can opt out via
            // `serve.auth.enabled = false` in `roko.toml`.
            enabled: true,
            api_key: String::new(),
            api_keys: Vec::new(),
            privy_app_id: None,
            jwks_providers: Vec::new(),
            privy_workspace_id: None,
            privy_allowed_roles: Vec::new(),
            enforcement_mode: EnforcementMode::default(),
            invite_expiry_days: default_invite_expiry_days(),
        }
    }
}

fn default_invite_expiry_days() -> u64 {
    7
}

/// A named API key entry with scoped permissions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiKeyEntry {
    /// Human-readable name (e.g. "github-actions", "cli-default").
    pub name: String,
    /// SHA-256 hash of the plaintext key (hex-encoded).
    pub key_hash: String,
    /// Permission scope: "admin", "agent:write", "read", etc.
    #[serde(default = "default_api_key_scope")]
    pub scope: String,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
    /// Optional ISO 8601 expiry timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    /// ISO 8601 timestamp of the last successful use of this key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<String>,
    /// Previous key hashes retained for a 5-minute grace period after rotation.
    /// Each entry is (hash, grace_expires_at_rfc3339).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub previous_key_hashes: Vec<(String, String)>,
}

fn default_api_key_scope() -> String {
    "admin".into()
}

/// Cloud deployment settings attached to the API server configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServeDeployConfig {
    /// Deployment provider, e.g. `"railway"` or `"fly"`.
    #[serde(default = "default_serve_deploy_provider")]
    pub provider: String,
    /// Environment variables that must be present for deployment.
    #[serde(default = "default_serve_deploy_environment")]
    pub environment: Vec<String>,
    /// Webhooks that should be registered after deploy.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub webhooks: Vec<ServeDeployWebhookConfig>,
}

fn default_serve_deploy_provider() -> String {
    "railway".into()
}

fn default_serve_deploy_environment() -> Vec<String> {
    vec![
        "GITHUB_TOKEN".into(),
        "GITHUB_WEBHOOK_SECRET".into(),
        "SLACK_BOT_TOKEN".into(),
        "SLACK_SIGNING_SECRET".into(),
    ]
}

impl Default for ServeDeployConfig {
    fn default() -> Self {
        Self {
            provider: default_serve_deploy_provider(),
            environment: default_serve_deploy_environment(),
            webhooks: Vec::new(),
        }
    }
}

/// A webhook registration entry to run after deployment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServeDeployWebhookConfig {
    /// Webhook provider.
    #[serde(default = "default_serve_deploy_webhook_provider")]
    pub provider: String,
    /// Repository owner.
    #[serde(default)]
    pub owner: String,
    /// Repository name.
    #[serde(default)]
    pub repo: String,
}

fn default_serve_deploy_webhook_provider() -> String {
    "github".into()
}

impl Default for ServeDeployWebhookConfig {
    fn default() -> Self {
        Self {
            provider: default_serve_deploy_webhook_provider(),
            owner: String::new(),
            repo: String::new(),
        }
    }
}

// ---- [serve.tracing] -----------------------------------------------------

/// Optional distributed tracing export configuration.
///
/// Parsed from `[serve.tracing]` in `roko.toml`. When `otlp_endpoint` is absent,
/// tracing export is disabled and no OTLP dependencies are loaded at runtime.
///
/// ```toml
/// [serve.tracing]
/// otlp_endpoint = "http://localhost:4317"
/// service_name = "roko-serve-dev"
/// sample_rate = 0.1
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TracingConfig {
    /// OTLP gRPC endpoint for trace export (e.g. `"http://localhost:4317"`).
    /// When absent, tracing export is disabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub otlp_endpoint: Option<String>,
    /// Service name reported in OTLP spans. Defaults to `"roko-serve"`.
    #[serde(default = "default_tracing_service_name")]
    pub service_name: String,
    /// Sample rate 0.0--1.0. Default 1.0 (trace everything).
    #[serde(default = "default_tracing_sample_rate")]
    pub sample_rate: f64,
}

fn default_tracing_service_name() -> String {
    "roko-serve".to_string()
}

fn default_tracing_sample_rate() -> f64 {
    1.0
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            otlp_endpoint: None,
            service_name: default_tracing_service_name(),
            sample_rate: default_tracing_sample_rate(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L13: a `[deploy]` table that leaves out `worker_image` deploys the
    /// same worker image as a config with no `[deploy]` table.
    #[test]
    fn deploy_table_without_worker_image_keeps_the_default_image() {
        let partial: DeployConfig =
            toml::from_str("backend = \"railway-api\"\n").expect("parse deploy config");
        assert_eq!(partial.worker_image, DeployConfig::default().worker_image);
        assert!(partial.worker_image.is_some());
    }

    /// 9341 (decision 9305): the registry and owner the default `worker_image` names must be the
    /// one `docker-publish.yml`'s `IMAGE_PREFIX` actually publishes to, or `roko deploy railway
    /// --workers` would pull a stale or foreign image.
    #[test]
    fn worker_image_default_matches_the_published_prefix() {
        let workflow = include_str!("../../../../.github/workflows/docker-publish.yml");
        let prefix = workflow
            .lines()
            .find_map(|line| line.trim().strip_prefix("IMAGE_PREFIX:"))
            .map(str::trim)
            .expect("docker-publish.yml sets IMAGE_PREFIX");
        let default_image = default_worker_image().expect("worker_image has a default");
        assert!(
            default_image.starts_with(prefix),
            "default worker_image {default_image:?} must start with published prefix {prefix:?}"
        );
    }

    #[test]
    fn default_share_ttl_days_is_seven() {
        assert_eq!(ServeConfig::default().share_ttl_days, 7);
    }

    #[test]
    fn terminal_defaults_refuse_commands_and_bound_sessions() {
        let cfg = ServeConfig::default();
        assert!(!cfg.terminal_enabled);
        assert!(cfg.terminal_commands.is_empty());
        assert_eq!(cfg.terminal_max_sessions, 8);
        assert_eq!(cfg.terminal_session_ttl_secs, 8 * 60 * 60);

        let cfg: ServeConfig = toml::from_str(concat!(
            "terminal_commands = [\"htop\"]\n",
            "terminal_max_sessions = 0\n",
            "terminal_session_ttl_secs = 0\n",
        ))
        .expect("parse terminal settings");
        assert_eq!(cfg.terminal_commands, vec!["htop".to_string()]);
        assert_eq!(cfg.terminal_max_sessions, 0);
        assert_eq!(cfg.terminal_session_ttl_secs, 0);
    }

    /// gap-b3e513: a rejected plan revision is retried once by default, and
    /// `[serve] revision_max_retries` changes that.
    #[test]
    fn revision_retries_default_to_one_and_are_configurable() {
        assert_eq!(ServeConfig::default().revision_max_retries, 1);
        let cfg: ServeConfig =
            toml::from_str("revision_max_retries = 3\n").expect("parse revision retries");
        assert_eq!(cfg.revision_max_retries, 3);
    }

    #[test]
    fn default_acknowledge_public_risk_is_false() {
        assert!(!ServeConfig::default().acknowledge_public_risk);
    }

    #[test]
    fn default_auto_start_is_false() {
        assert!(!ServeConfig::default().auto_start);
    }

    #[test]
    fn default_auth_is_enabled() {
        assert!(ServeAuthConfig::default().enabled);
    }

    #[test]
    fn invite_expiry_defaults_to_seven_days_and_is_configurable() {
        assert_eq!(ServeAuthConfig::default().invite_expiry_days, 7);

        let cfg: ServeConfig =
            toml::from_str("[auth]\ninvite_expiry_days = 14\n").expect("parse serve auth config");
        assert_eq!(cfg.auth.invite_expiry_days, 14);
    }

    #[test]
    fn jwks_providers_parse_from_auth_config() {
        let cfg: ServeConfig = toml::from_str(
            r#"
[auth]
[[auth.jwks_providers]]
url = "https://identity.example/.well-known/jwks.json"
expected_issuer = "https://identity.example"
"#,
        )
        .expect("parse JWKS provider config");
        assert_eq!(cfg.auth.jwks_providers.len(), 1);
        assert_eq!(
            cfg.auth.jwks_providers[0],
            JwksProvider::new(
                "https://identity.example/.well-known/jwks.json",
                "https://identity.example"
            )
        );
    }

    #[test]
    fn tracing_config_defaults_are_disabled() {
        let tc = TracingConfig::default();
        assert!(tc.otlp_endpoint.is_none());
        assert_eq!(tc.service_name, "roko-serve");
        assert!((tc.sample_rate - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn serve_config_tracing_absent_uses_defaults() {
        let toml_text = "port = 8080\n";
        let cfg: ServeConfig = toml::from_str(toml_text).expect("parse serve config");
        assert!(cfg.tracing.otlp_endpoint.is_none());
        assert_eq!(cfg.tracing.service_name, "roko-serve");
        assert!((cfg.tracing.sample_rate - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn serve_config_tracing_parses_full_block() {
        let toml_text = r#"
port = 6677

[tracing]
otlp_endpoint = "http://localhost:4317"
service_name = "roko-serve-dev"
sample_rate = 0.1
"#;
        let cfg: ServeConfig = toml::from_str(toml_text).expect("parse serve config");
        assert_eq!(
            cfg.tracing.otlp_endpoint.as_deref(),
            Some("http://localhost:4317")
        );
        assert_eq!(cfg.tracing.service_name, "roko-serve-dev");
        assert!((cfg.tracing.sample_rate - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn serve_config_tracing_sample_rate_defaults_to_one() {
        let toml_text = r#"
[tracing]
otlp_endpoint = "http://otel:4317"
"#;
        let cfg: ServeConfig = toml::from_str(toml_text).expect("parse serve config");
        assert!((cfg.tracing.sample_rate - 1.0).abs() < f64::EPSILON);
        assert_eq!(cfg.tracing.service_name, "roko-serve");
    }

    // ---- LiveAgentOutput tests -------------------------------------------

    #[test]
    fn live_agent_output_default_is_tool_steps() {
        assert_eq!(
            ServeConfig::default().live_agent_output,
            LiveAgentOutput::ToolSteps
        );
    }

    #[test]
    fn live_agent_output_tool_steps_round_trips_through_toml() {
        let cfg: ServeConfig =
            toml::from_str(r#"live_agent_output = "tool_steps""#).expect("parse tool_steps");
        assert_eq!(cfg.live_agent_output, LiveAgentOutput::ToolSteps);

        let serialized = toml::to_string(&cfg).expect("serialize");
        assert!(
            serialized.contains("tool_steps"),
            "serialized form must contain 'tool_steps', got: {serialized}"
        );
        let rt: ServeConfig = toml::from_str(&serialized).expect("re-parse");
        assert_eq!(rt.live_agent_output, LiveAgentOutput::ToolSteps);
    }

    #[test]
    fn live_agent_output_trusted_round_trips_through_toml() {
        let cfg: ServeConfig =
            toml::from_str(r#"live_agent_output = "trusted""#).expect("parse trusted");
        assert_eq!(cfg.live_agent_output, LiveAgentOutput::Trusted);

        let serialized = toml::to_string(&cfg).expect("serialize");
        assert!(
            serialized.contains("trusted"),
            "serialized form must contain 'trusted', got: {serialized}"
        );
        let rt: ServeConfig = toml::from_str(&serialized).expect("re-parse");
        assert_eq!(rt.live_agent_output, LiveAgentOutput::Trusted);
    }

    #[test]
    fn live_agent_output_effective_all_four_combinations() {
        // ToolSteps + loopback-only  → ToolSteps (tool steps never escalate)
        assert_eq!(
            LiveAgentOutput::ToolSteps.effective(true),
            LiveAgentOutput::ToolSteps
        );
        // ToolSteps + non-loopback   → ToolSteps
        assert_eq!(
            LiveAgentOutput::ToolSteps.effective(false),
            LiveAgentOutput::ToolSteps
        );
        // Trusted   + loopback-only  → Trusted (the only case that succeeds)
        assert_eq!(
            LiveAgentOutput::Trusted.effective(true),
            LiveAgentOutput::Trusted
        );
        // Trusted   + non-loopback   → ToolSteps (safety downgrade)
        assert_eq!(
            LiveAgentOutput::Trusted.effective(false),
            LiveAgentOutput::ToolSteps
        );
    }
}

// ---- [server] ------------------------------------------------------------

/// HTTP server / gateway settings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    /// Address to bind to.
    #[serde(default = "default_bind")]
    pub bind: String,
    /// Port number.
    #[serde(default = "default_port")]
    pub port: u16,
    /// Allowed CORS origins. Empty = local-only.
    #[serde(default)]
    pub cors_origins: Vec<String>,
    /// Optional bearer token for API authentication.
    #[serde(default)]
    pub auth_token: Option<String>,
    /// Allow all origins when `cors_origins` is empty.
    #[serde(default)]
    pub unsafe_public_cors: bool,
    /// Workspace GC interval in seconds. Defaults to 300 (5 minutes).
    #[serde(default = "default_workspace_gc_interval_secs")]
    pub workspace_gc_interval_secs: u64,
    /// Global rate limit in requests per second. Defaults to 100.
    #[serde(default = "default_rate_limit_per_sec")]
    pub rate_limit_per_sec: u32,
    /// Per-key rate limit in requests per second per API key or IP. Defaults to 30.
    #[serde(default = "default_rate_limit_per_key_per_sec")]
    pub rate_limit_per_key_per_sec: u32,
}

fn default_bind() -> String {
    "127.0.0.1".into()
}

fn default_workspace_gc_interval_secs() -> u64 {
    crate::defaults::DEFAULT_WORKSPACE_GC_INTERVAL_SECS
}

const fn default_port() -> u16 {
    6677
}

const fn default_rate_limit_per_sec() -> u32 {
    100
}

const fn default_rate_limit_per_key_per_sec() -> u32 {
    30
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            port: default_port(),
            cors_origins: Vec::new(),
            auth_token: None,
            unsafe_public_cors: false,
            workspace_gc_interval_secs: default_workspace_gc_interval_secs(),
            rate_limit_per_sec: default_rate_limit_per_sec(),
            rate_limit_per_key_per_sec: default_rate_limit_per_key_per_sec(),
        }
    }
}

// ---- [deploy] ------------------------------------------------------------

/// Cloud deployment configuration.
///
/// ```toml
/// [deploy]
/// backend = "railway-api"
/// railway_api_token = "..."
/// project_id = "..."
/// environment_id = "..."
/// worker_image = "ghcr.io/example/roko-worker:latest"
/// default_region = "us-west1"
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeployConfig {
    /// Which deploy backend to use: `"railway-api"`, `"railway-cli"`, `"manual"`.
    #[serde(default = "default_deploy_backend")]
    pub backend: String,

    /// Railway API token (for the `railway-api` backend).
    #[serde(default)]
    pub railway_api_token: Option<String>,

    /// Railway project ID.
    #[serde(default)]
    pub project_id: Option<String>,

    /// Railway environment ID.
    #[serde(default)]
    pub environment_id: Option<String>,

    /// Docker image for worker containers. Defaults to the published
    /// `ghcr.io/wpank/roko-worker:latest`, whether `[deploy]` is
    /// absent or leaves the key out.
    #[serde(default = "default_worker_image")]
    pub worker_image: Option<String>,

    /// Default region for deployments.
    #[serde(default)]
    pub default_region: Option<String>,
}

fn default_deploy_backend() -> String {
    "manual".into()
}

fn default_worker_image() -> Option<String> {
    Some("ghcr.io/wpank/roko-worker:latest".into())
}

impl Default for DeployConfig {
    fn default() -> Self {
        Self {
            backend: default_deploy_backend(),
            railway_api_token: None,
            project_id: None,
            environment_id: None,
            worker_image: default_worker_image(),
            default_region: None,
        }
    }
}

// ---- [scheduler] ---------------------------------------------------------

/// Cron scheduler configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchedulerConfig {
    /// Cron jobs configured at startup.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cron: Vec<SchedulerCronConfig>,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self { cron: Vec::new() }
    }
}

impl SchedulerConfig {
    /// Returns `true` when no cron jobs are configured.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cron.is_empty()
    }
}

/// One cron job configuration entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchedulerCronConfig {
    /// Human-readable schedule name.
    pub name: String,
    /// Standard cron expression.
    pub expression: String,
    /// Signal kind emitted when the schedule fires.
    pub signal_kind: String,
    /// Extra structured metadata included in the emitted signal body.
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl Default for SchedulerCronConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            expression: String::new(),
            signal_kind: String::new(),
            metadata: serde_json::Value::Null,
        }
    }
}

// ---- [webhooks] ----------------------------------------------------------

/// Webhook ingress configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebhooksConfig {
    /// GitHub webhook configuration.
    #[serde(default)]
    pub github: GithubWebhookConfig,
}

impl Default for WebhooksConfig {
    fn default() -> Self {
        Self {
            github: GithubWebhookConfig::default(),
        }
    }
}

/// GitHub webhook configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GithubWebhookConfig {
    /// Shared secret used to verify `X-Hub-Signature-256`.
    #[serde(default)]
    pub secret: String,
}

impl Default for GithubWebhookConfig {
    fn default() -> Self {
        Self {
            secret: String::new(),
        }
    }
}

// ---- [github] ------------------------------------------------------------

/// Merge strategy when auto-merging plan PRs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeMethod {
    /// Standard merge commit.
    Merge,
    /// Squash all commits into one.
    #[default]
    Squash,
    /// Rebase commits onto the base branch.
    Rebase,
}

impl MergeMethod {
    /// Return the string form expected by the GitHub API.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Merge => "merge",
            Self::Squash => "squash",
            Self::Rebase => "rebase",
        }
    }
}

impl std::fmt::Display for MergeMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// GitHub repository identity and workflow preferences.
///
/// This section covers the repo the runner will operate against (branch
/// creation, PRs, issues).  Webhook secrets remain under `[webhooks.github]`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitHubConfig {
    /// GitHub organisation or user that owns the target repository.
    /// Optional — callers must check at use-site.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// Repository name (without the owner prefix).
    /// Optional — callers must check at use-site.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    /// Base branch for plan PRs.
    #[serde(default = "default_github_default_branch")]
    pub default_branch: String,
    /// When true, the runner automatically opens a draft PR when a plan starts.
    #[serde(default)]
    pub auto_pr: bool,
    /// Strategy used when auto-merging a plan PR.
    #[serde(default)]
    pub merge_method: MergeMethod,
    /// Label prefix applied to roko-managed GitHub labels and issues.
    #[serde(default = "default_github_label_prefix")]
    pub label_prefix: String,
    /// When true, the runner posts a comment and updates labels on the
    /// associated PR whenever a gate passes or fails.  Requires `owner`
    /// and `repo` to be set and `GITHUB_TOKEN` to be available.
    #[serde(default)]
    pub auto_update_prs: bool,
    /// Interval (in hours) between automatic state/progress syncs to GitHub.
    /// Syncs push the current branch and update the PR description with
    /// progress. Set to 0 to disable scheduled syncs. Default: 4 hours.
    #[serde(default = "default_github_sync_interval_hours")]
    pub sync_interval_hours: u32,
    /// When true, the runner deletes roko-managed branches (both local and
    /// remote) after their associated PR has been merged.  Runs as a
    /// post-plan-completion step.
    #[serde(default)]
    pub cleanup_merged_branches: bool,
}

fn default_github_default_branch() -> String {
    "main".to_owned()
}

fn default_github_label_prefix() -> String {
    "roko/".to_owned()
}

const fn default_github_sync_interval_hours() -> u32 {
    4
}

impl Default for GitHubConfig {
    fn default() -> Self {
        Self {
            owner: None,
            repo: None,
            default_branch: default_github_default_branch(),
            auto_pr: false,
            merge_method: MergeMethod::default(),
            label_prefix: default_github_label_prefix(),
            auto_update_prs: false,
            sync_interval_hours: default_github_sync_interval_hours(),
            cleanup_merged_branches: false,
        }
    }
}

#[cfg(test)]
mod statehub_tests {
    use super::*;

    #[test]
    fn statehub_retention_defaults_to_seven_days_and_parses_units() {
        assert_eq!(
            StateHubConfig::default()
                .history_retention_duration()
                .unwrap(),
            std::time::Duration::from_secs(7 * 24 * 60 * 60)
        );
        for (configured, expected_ms) in [
            ("250ms", 250),
            ("60s", 60_000),
            ("2m", 120_000),
            ("3h", 10_800_000),
            ("2d", 172_800_000),
        ] {
            let config = StateHubConfig {
                history_retention: configured.to_string(),
            };
            assert_eq!(
                config.history_retention_duration().unwrap(),
                std::time::Duration::from_millis(expected_ms)
            );
        }
    }

    #[test]
    fn statehub_retention_rejects_zero_missing_units_and_overflow() {
        for configured in ["0s", "60", "1fortnight", "18446744073709551615d"] {
            let config = StateHubConfig {
                history_retention: configured.to_string(),
            };
            assert!(config.history_retention_duration().is_err(), "{configured}");
        }
    }
}

#[cfg(test)]
mod chat_run_tests {
    use super::super::provider::{ModelProfile, ProviderConfig};
    use super::super::schema::RokoConfig;
    use super::*;
    use crate::agent::ProviderKind;

    /// The environment of these tests: only `KEY_SET` holds a value.
    fn env(name: &str) -> Option<String> {
        (name == "KEY_SET").then(|| "sk-test".to_string())
    }

    fn provider(kind: ProviderKind, key: Option<&str>) -> ProviderConfig {
        ProviderConfig {
            kind,
            api_key_env: key.map(str::to_string),
            base_url: (kind == ProviderKind::OpenAiCompat).then(|| "http://127.0.0.1:9".into()),
            command: (kind == ProviderKind::ClaudeCli).then(|| "claude".into()),
            ..ProviderConfig::default()
        }
    }

    fn model(provider: &str, slug: &str, price: Option<f64>) -> ModelProfile {
        ModelProfile {
            provider: provider.to_string(),
            slug: slug.to_string(),
            supports_tools: true,
            cost_input_per_m: price,
            cost_output_per_m: price,
            ..ModelProfile::default()
        }
    }

    /// A workspace whose only provider is the Claude CLI, routed to it
    /// everywhere: the ladder is off and every model routing names is the
    /// CLI's.
    fn cli_only() -> RokoConfig {
        let mut config = RokoConfig::default();
        config.routing.ladder.enabled = false;
        config.routing.fallback_models.clear();
        config.agent.fallback_model = None;
        config.agent.default_model = "cli-sonnet".to_string();
        config.providers.clear();
        config.models.clear();
        config
            .providers
            .insert("claude_cli".into(), provider(ProviderKind::ClaudeCli, None));
        config.models.insert(
            "cli-sonnet".into(),
            model("claude_cli", "claude-sonnet-4-6", None),
        );
        config
    }

    /// gap-cf427e: without the boundary and without a model roko's tool loop
    /// runs, both blockers apply, the boundary first, in words older hosts
    /// still match ("data-model boundary") and never read as a missing plan.
    #[test]
    fn chat_run_refusal_names_each_blocker_in_order() {
        let refusal = cli_only()
            .chat_run_refusal_with_env(None, &env)
            .expect("the run is refused");
        assert_eq!(refusal.codes(), ["no_data_boundary", "no_tool_loop_model"]);
        assert_eq!(refusal.code(), "no_data_boundary");
        assert!(refusal.message.starts_with(NO_DATA_BOUNDARY), "{refusal:?}");
        assert!(
            refusal
                .message
                .contains("cli-sonnet runs on claude_cli, an agent that runs its own tools"),
            "{refusal:?}"
        );
        let lower = refusal.message.to_lowercase();
        assert!(
            !lower.contains("not found") && !lower.starts_with("no "),
            "{lower}"
        );

        let mut allowed = cli_only();
        allowed.serve.mcp.allow_without_data_llm = true;
        let refusal = allowed
            .chat_run_refusal_with_env(None, &env)
            .expect("still refused");
        assert_eq!(refusal.codes(), ["no_tool_loop_model"]);
        assert!(!refusal.message.to_lowercase().starts_with("no "));
        assert!(
            !refusal.message.contains("data-model boundary"),
            "{refusal:?}"
        );
    }

    /// An API model whose key is set lifts `no_tool_loop_model`; one whose key
    /// is missing does not, and the refusal names the key to set.
    #[test]
    fn an_api_model_with_its_key_set_can_run_a_chat_run() {
        let mut config = cli_only();
        config.serve.mcp.allow_without_data_llm = true;
        config.providers.insert(
            "api".into(),
            provider(ProviderKind::OpenAiCompat, Some("KEY_UNSET")),
        );
        config
            .models
            .insert("api-mini".into(), model("api", "api-mini-1", Some(0.5)));
        let refusal = config
            .chat_run_refusal_with_env(None, &env)
            .expect("the key is missing");
        assert_eq!(refusal.codes(), ["no_tool_loop_model"]);
        assert!(
            refusal
                .message
                .contains("api-mini runs on api, whose key KEY_UNSET is not set"),
            "{refusal:?}"
        );
        assert!(
            refusal.message.contains("put KEY_UNSET in ~/.roko/.env"),
            "{refusal:?}"
        );

        config.providers.get_mut("api").expect("api").api_key_env = Some("KEY_SET".into());
        assert_eq!(config.chat_run_refusal_with_env(None, &env), None);
        assert_eq!(config.tool_loop_models_with_env(&env), ["api-mini"]);

        // A disabled provider runs nothing.
        config.routing.disabled_providers = vec!["api".into()];
        assert!(config.chat_run_refusal_with_env(None, &env).is_some());
    }

    /// The Claude CLI's model counts once the Anthropic API, which failover
    /// moves its slug to, has its key.
    #[test]
    fn the_same_model_on_an_api_provider_of_its_family_counts() {
        let mut config = cli_only();
        config.serve.mcp.allow_without_data_llm = true;
        config.providers.insert(
            "anthropic".into(),
            provider(ProviderKind::AnthropicApi, Some("KEY_SET")),
        );
        assert_eq!(config.chat_run_refusal_with_env(None, &env), None);
        assert!(config.tool_loop_models_with_env(&env).is_empty());
    }

    /// A data model roko cannot call over an API, or whose key is missing,
    /// fails every agent of roko's tool loop, so it is no boundary either.
    #[test]
    fn a_data_model_roko_cannot_call_is_no_boundary() {
        let mut config = cli_only();
        config.providers.insert(
            "api".into(),
            provider(ProviderKind::OpenAiCompat, Some("KEY_SET")),
        );
        config
            .models
            .insert("api-mini".into(), model("api", "api-mini-1", Some(0.5)));
        config.agent.data_llm = Some(data_llm("cli-sonnet"));
        let refusal = config
            .chat_run_refusal_with_env(None, &env)
            .expect("the CLI cannot be the data model");
        assert_eq!(refusal.codes(), ["no_data_boundary"]);
        assert!(
            refusal.message.contains("data-model boundary"),
            "{refusal:?}"
        );
        assert!(
            refusal.message.contains("cannot call over an API"),
            "{refusal:?}"
        );

        config.agent.data_llm = Some(data_llm("api-mini"));
        assert_eq!(config.chat_run_refusal_with_env(None, &env), None);

        config.providers.get_mut("api").expect("api").api_key_env = Some("KEY_UNSET".into());
        let refusal = config
            .chat_run_refusal_with_env(None, &env)
            .expect("the data model's key is missing");
        assert!(refusal.message.contains("KEY_UNSET"), "{refusal:?}");
    }

    /// The ladder's rungs come first, in the ladder's order, then the other
    /// models by their configured price; CLI rungs never qualify.
    #[test]
    fn tool_loop_models_list_rungs_first_then_by_price() {
        let mut config = cli_only();
        config.providers.insert(
            "api".into(),
            provider(ProviderKind::OpenAiCompat, Some("KEY_SET")),
        );
        for (key, price) in [
            ("dear", 9.0),
            ("cheap", 0.1),
            ("rung-low", 3.0),
            ("rung-high", 1.0),
        ] {
            config.models.insert(
                key.into(),
                model("api", &format!("{key}-slug"), Some(price)),
            );
        }
        config.routing.ladder.enabled = true;
        config.routing.ladder.roles.clear();
        config.routing.ladder.rungs = [
            ("low", "rung-low"),
            ("mid", "rung-high"),
            ("top", "cli-sonnet"),
        ]
        .into_iter()
        .map(|(name, model)| crate::config::routing::LadderRung {
            name: name.into(),
            model: model.into(),
        })
        .collect();
        assert_eq!(
            config.tool_loop_models_with_env(&env),
            ["rung-low", "rung-high", "cheap", "dear"]
        );
    }

    /// The refusal says what keeps each rung, and each other model routing
    /// names, from a chat host's run: no `[models.*]` entry, a key that is not
    /// set, or an agent that runs its own tools; a rung named by its model's
    /// slug counts once.
    #[test]
    fn the_refusal_names_why_each_rung_is_skipped() {
        let mut config = cli_only();
        config.serve.mcp.allow_without_data_llm = true;
        config.providers.insert(
            "zai".into(),
            provider(ProviderKind::OpenAiCompat, Some("ZAI_KEY_UNSET")),
        );
        config
            .models
            .insert("glm".into(), model("zai", "glm-4.7", Some(1.0)));
        config.routing.ladder.enabled = true;
        config.routing.ladder.roles.clear();
        config.routing.ladder.rungs = [
            ("cheap", "gpt-oss-120b"),
            ("mid", "glm-4.7"),
            ("top", "claude-sonnet-4-6"),
        ]
        .into_iter()
        .map(|(name, model)| crate::config::routing::LadderRung {
            name: name.into(),
            model: model.into(),
        })
        .collect();
        let refusal = config
            .chat_run_refusal_with_env(None, &env)
            .expect("no model qualifies");
        assert_eq!(refusal.codes(), ["no_tool_loop_model"]);
        let message = &refusal.message;
        for cause in [
            "rung `cheap` (gpt-oss-120b) has no [models.*] entry, and roko has no built-in model",
            "rung `mid` (glm-4.7) runs on zai, whose key ZAI_KEY_UNSET is not set",
            "rung `top` (claude-sonnet-4-6) runs on claude_cli, an agent that runs its own tools",
            "put ZAI_KEY_UNSET in ~/.roko/.env and restart roko serve",
        ] {
            assert!(message.contains(cause), "{cause}: {message}");
        }
        assert_eq!(
            message.matches("runs on claude_cli").count(),
            1,
            "{message}"
        );
        assert_eq!(message.matches("runs on zai").count(), 1, "{message}");
        let lower = message.to_lowercase();
        assert!(
            !lower.contains("not found") && !lower.starts_with("no "),
            "{lower}"
        );

        // With the key set, the rung runs; routing names it by its key, once.
        config.providers.get_mut("zai").expect("zai").api_key_env = Some("KEY_SET".into());
        config.routing.fallback_models = vec!["glm-4.7".into()];
        assert_eq!(config.chat_run_refusal_with_env(None, &env), None);
        assert_eq!(config.tool_loop_models_with_env(&env), ["glm"]);
    }

    /// A rung that names a built-in model counts once a provider of its kind
    /// has the key, as the ladder runs such a rung (bug-4e336b).
    #[test]
    fn a_rung_naming_a_builtin_model_counts() {
        let mut config = cli_only();
        config.serve.mcp.allow_without_data_llm = true;
        config.providers.insert(
            "openai".into(),
            provider(ProviderKind::OpenAiCompat, Some("KEY_UNSET")),
        );
        config.routing.ladder.enabled = true;
        config.routing.ladder.roles.clear();
        config.routing.ladder.rungs = [("strong", "gpt-5.4-mini"), ("top", "cli-sonnet")]
            .into_iter()
            .map(|(name, model)| crate::config::routing::LadderRung {
                name: name.into(),
                model: model.into(),
            })
            .collect();
        let refusal = config
            .chat_run_refusal_with_env(None, &env)
            .expect("its key is missing");
        assert!(
            refusal
                .message
                .contains("rung `strong` (gpt-5.4-mini) runs on openai, whose key KEY_UNSET"),
            "{refusal:?}"
        );

        config
            .providers
            .get_mut("openai")
            .expect("openai")
            .api_key_env = Some("KEY_SET".into());
        assert_eq!(config.chat_run_refusal_with_env(None, &env), None);
        assert_eq!(config.tool_loop_models_with_env(&env), ["gpt-5.4-mini"]);
    }

    /// gap-8548e3: a chat host's run is held to its max_usd, so it is refused
    /// while no model it can run on has a price: in the price snapshot it is
    /// given, its `[models.*]` rates or the built-in table. One priced model,
    /// at a rate of 0 too, lifts the refusal.
    #[test]
    fn a_chat_run_needs_a_model_with_a_price() {
        let mut config = cli_only();
        config.serve.mcp.allow_without_data_llm = true;
        config.providers.insert(
            "api".into(),
            provider(ProviderKind::OpenAiCompat, Some("KEY_SET")),
        );
        config
            .models
            .insert("oss".into(), model("api", "gpt-oss-120b", None));
        let refusal = config
            .chat_run_refusal_with_env(None, &env)
            .expect("no model has a price");
        assert_eq!(refusal.codes(), ["no_price"]);
        assert!(refusal.message.contains("(oss)"), "{refusal:?}");
        assert!(refusal.message.contains("cost_input_per_m"), "{refusal:?}");
        let lower = refusal.message.to_lowercase();
        assert!(
            !lower.contains("not found") && !lower.starts_with("no "),
            "{lower}"
        );

        // The price snapshot has a row for gpt-oss-120b.
        let snapshot = crate::pricing_snapshot::PriceSnapshot::builtin().expect("built-in");
        assert_eq!(
            config.chat_run_refusal_with_env(Some(&snapshot), &env),
            None
        );

        config
            .models
            .insert("free".into(), model("api", "free-local-1", Some(0.0)));
        assert_eq!(config.chat_run_refusal_with_env(None, &env), None);
    }

    /// `[agent.data_llm]` naming `model`, with the defaults otherwise.
    fn data_llm(model: &str) -> super::super::agent::DataLlmConfig {
        toml::from_str(&format!("model = \"{model}\"")).expect("a data_llm table")
    }
}
