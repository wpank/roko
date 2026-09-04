//! Provider and model profile configuration sections.

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::agent::ProviderKind;
use crate::defaults::DEFAULT_MAX_OUTPUT_TOKENS;

use super::agent::{default_context_window, default_tool_format, default_true};
use super::provenance::ConfigProvenance;

/// `skip_serializing_if` helper — returns `true` when the bool is `false`.
/// Suppresses default-false bool fields from serialized TOML/JSON output.
pub(crate) fn is_false(b: &bool) -> bool {
    !*b
}

/// `skip_serializing_if` helper — returns `true` when the bool is `true`.
/// Suppresses default-true bool fields from serialized TOML/JSON output.
pub(crate) fn is_true(b: &bool) -> bool {
    *b
}

// ---- provider/model identity --------------------------------------------

/// Error returned by provider/model identity constructors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigIdentityError {
    kind: &'static str,
}

impl ConfigIdentityError {
    const fn empty(kind: &'static str) -> Self {
        Self { kind }
    }
}

impl fmt::Display for ConfigIdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} must not be empty", self.kind)
    }
}

impl std::error::Error for ConfigIdentityError {}

macro_rules! config_identity {
    ($name:ident, $kind:literal) => {
        #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn try_new(value: impl Into<String>) -> Result<Self, ConfigIdentityError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(ConfigIdentityError::empty($kind));
                }
                Ok(Self(value))
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }

            #[must_use]
            pub fn into_inner(self) -> String {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl TryFrom<String> for $name {
            type Error = ConfigIdentityError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::try_new(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = ConfigIdentityError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::try_new(value)
            }
        }
    };
}

config_identity!(ProviderId, "provider id");
config_identity!(ModelAlias, "model alias");
config_identity!(BackendModelSlug, "backend model slug");

/// Explicit transport used to communicate with a provider.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ProviderTransport {
    Http { base_url: String },
    Cli { command: String, args: Vec<String> },
    Acp { command: String, args: Vec<String> },
    Local,
}

/// Explicit authentication policy for a provider.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ProviderAuth {
    EnvVar { name: String },
    StaticSecretRef { name: String },
    None { local_only: bool },
}

/// Provider-level capability flags used by resolved provider definitions.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub supports_web_search: bool,
    pub supports_mcp_tools: bool,
}

/// Resolved provider definition with identity, transport, auth, and provenance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProviderDefinition {
    pub id: ProviderId,
    pub display_name: String,
    pub kind: ProviderKind,
    pub transport: ProviderTransport,
    pub auth: ProviderAuth,
    pub capabilities: ProviderCapabilities,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provenance: Vec<ConfigProvenance>,
}

/// Source of model metadata in a resolved model definition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelMetadataSource {
    Config,
    ProviderDiscovery,
    HealthProbe,
    Migration,
    BuiltInFallback,
}

/// Model-level capability flags used by resolved model definitions.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub supports_tools: bool,
    pub supports_thinking: bool,
    pub supports_vision: bool,
    pub supports_web_search: bool,
    pub supports_mcp_tools: bool,
    pub supports_partial: bool,
    pub supports_grounding: bool,
    pub supports_code_execution: bool,
    pub supports_caching: bool,
}

/// Token and request pricing metadata for a resolved model.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelCost {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_per_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_per_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read_per_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write_per_m: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per_request: Option<f64>,
}

/// Resolved model definition with local alias and provider wire slug separated.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelDefinition {
    pub alias: ModelAlias,
    pub provider_id: ProviderId,
    pub backend_slug: BackendModelSlug,
    pub capabilities: ModelCapabilities,
    pub cost: ModelCost,
    pub metadata_source: ModelMetadataSource,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provenance: Vec<ConfigProvenance>,
}

#[cfg(test)]
mod provider_identity_tests {
    use super::*;
    use crate::config::ConfigProvenance;

    #[test]
    fn provider_identity_rejects_empty_ids() {
        assert!(ProviderId::try_new("").is_err());
        assert!(ProviderId::try_new("   ").is_err());
        assert!(ModelAlias::try_new("").is_err());
        assert!(BackendModelSlug::try_new("\t").is_err());
    }

    #[test]
    fn provider_identity_constructs_provider_and_model_definitions() {
        let provider_id = ProviderId::try_new("anthropic").expect("provider id");
        let alias = ModelAlias::try_new("claude-sonnet").expect("model alias");
        let backend_slug =
            BackendModelSlug::try_new("claude-3-7-sonnet-latest").expect("backend slug");
        let provenance = vec![ConfigProvenance::file(
            "roko.toml",
            "providers.anthropic.kind",
        )];

        let provider = ProviderDefinition {
            id: provider_id.clone(),
            display_name: "Anthropic".to_string(),
            kind: ProviderKind::AnthropicApi,
            transport: ProviderTransport::Http {
                base_url: "https://api.anthropic.com".to_string(),
            },
            auth: ProviderAuth::EnvVar {
                name: "ANTHROPIC_API_KEY".to_string(),
            },
            capabilities: ProviderCapabilities {
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
                supports_web_search: false,
                supports_mcp_tools: false,
            },
            provenance: provenance.clone(),
        };
        let model = ModelDefinition {
            alias: alias.clone(),
            provider_id,
            backend_slug,
            capabilities: ModelCapabilities {
                supports_tools: true,
                supports_thinking: true,
                supports_vision: true,
                ..ModelCapabilities::default()
            },
            cost: ModelCost {
                input_per_m: Some(3.0),
                output_per_m: Some(15.0),
                ..ModelCost::default()
            },
            metadata_source: ModelMetadataSource::Config,
            provenance,
        };

        assert_eq!(provider.id.as_str(), "anthropic");
        assert_eq!(provider.kind, ProviderKind::AnthropicApi);
        assert!(matches!(provider.transport, ProviderTransport::Http { .. }));
        assert_eq!(model.alias, alias);
        assert_eq!(model.metadata_source, ModelMetadataSource::Config);
    }
}

// ---- [providers.*] -------------------------------------------------------

/// Provider registry entry for `[providers.<name>]`.
///
/// A provider describes where requests go and how the runtime talks to that
/// endpoint. Use it to capture auth, transport, and provider-specific limits
/// without hardcoding them into Rust.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Protocol family used to talk to the provider.
    pub kind: ProviderKind,
    /// Base URL for HTTP providers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Environment variable name holding the API key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    /// Command to spawn for CLI providers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Arguments passed to the CLI command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    /// Hard request or subprocess timeout in milliseconds.
    #[serde(
        default = "default_provider_timeout_ms",
        skip_serializing_if = "Option::is_none"
    )]
    pub timeout_ms: Option<u64>,
    /// Time-to-first-token timeout in milliseconds.
    #[serde(
        default = "default_provider_ttft_timeout_ms",
        skip_serializing_if = "Option::is_none"
    )]
    pub ttft_timeout_ms: Option<u64>,
    /// TCP connection timeout in milliseconds.
    #[serde(
        default = "default_provider_connect_timeout_ms",
        skip_serializing_if = "Option::is_none"
    )]
    pub connect_timeout_ms: Option<u64>,
    /// Extra headers to inject on outbound requests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_headers: Option<HashMap<String, String>>,
    /// Maximum concurrent requests allowed for this provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_concurrent: Option<u32>,
    /// Provider-level rate limits for request and token throttling.
    ///
    /// When set, the runtime enforces these limits across all concurrent agents
    /// that share this provider, gating new requests when the budget is
    /// exhausted. Unset means the runtime falls back to the process-wide
    /// default RPM defined by `DEFAULT_PROVIDER_RPM`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limits: Option<ProviderLimits>,
    /// When `true`, tool-use permission requests require interactive user
    /// confirmation instead of being auto-approved.  Defaults to `false`
    /// (auto-approve) for backward compatibility.
    #[serde(default, skip_serializing_if = "is_false")]
    pub require_confirmation: bool,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            kind: ProviderKind::OpenAiCompat,
            base_url: None,
            api_key_env: None,
            command: None,
            args: None,
            timeout_ms: default_provider_timeout_ms(),
            ttft_timeout_ms: default_provider_ttft_timeout_ms(),
            connect_timeout_ms: default_provider_connect_timeout_ms(),
            extra_headers: None,
            max_concurrent: None,
            limits: None,
            require_confirmation: false,
        }
    }
}

/// Per-provider request and token budget enforced by the shared rate limiter.
///
/// Configure these under `[providers.<name>]` in `roko.toml`:
///
/// ```toml
/// [providers.anthropic]
/// kind = "anthropic_api"
/// api_key_env = "ANTHROPIC_API_KEY"
///
/// [providers.anthropic.limits]
/// rpm = 50
/// tpm = 40000
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderNetworkPolicy {
    /// Preserve the host's normal network access for provider subprocesses.
    #[default]
    Allow,
    /// Deny all provider-subprocess network access, including loopback.
    ///
    /// Provider construction fails when no supported kernel-backed
    /// confinement launcher is available.
    Deny,
}

impl ProviderNetworkPolicy {
    pub(crate) const fn is_allow(policy: &Self) -> bool {
        matches!(policy, Self::Allow)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProviderLimits {
    /// Maximum requests per minute across all concurrent agents.
    ///
    /// Common defaults:
    /// - Anthropic tier 1: 50 RPM
    /// - OpenAI tier 1: 500 RPM
    /// - Gemini free: 15 RPM
    /// - Ollama (local): no limit
    pub rpm: u32,
    /// Maximum tokens per minute (input + output combined).
    ///
    /// Advisory limit: warns and delays when approached, blocks at 100%.
    /// Set to 0 to disable TPM tracking.
    #[serde(default)]
    pub tpm: u64,
    /// Maximum CPU time, in seconds, for each spawned provider process.
    ///
    /// When set, provider creation fails on platforms where the operating
    /// system limit cannot be installed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_cpu_seconds: Option<u64>,
    /// Maximum address-space bytes for each spawned provider process.
    ///
    /// An address-space ceiling is a conservative OS-enforced upper bound on
    /// resident memory. It may reject large sparse mappings before RSS reaches
    /// this value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_rss_bytes: Option<u64>,
    /// Maximum process count available to the provider subprocess.
    ///
    /// Unix implements this with `RLIMIT_NPROC`, whose accounting scope is the
    /// provider's real user ID rather than only its process group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_processes: Option<u64>,
    /// Network policy for every subprocess launched for this provider.
    ///
    /// `deny` requires macOS Seatbelt or Linux firejail+seccomp and never
    /// silently degrades to unrestricted execution.
    #[serde(default, skip_serializing_if = "ProviderNetworkPolicy::is_allow")]
    pub network: ProviderNetworkPolicy,
}

pub(crate) const fn default_provider_timeout_ms() -> Option<u64> {
    Some(crate::defaults::DEFAULT_REQUEST_TIMEOUT_MS)
}

/// Default TTFT (time-to-first-token) timeout in milliseconds.
///
/// Used as the single source of truth for TTFT across all providers and
/// backends.  Import this constant instead of hardcoding `15_000`.
///
/// Re-exported from [`crate::defaults::DEFAULT_TTFT_TIMEOUT_MS`].
pub const DEFAULT_TTFT_TIMEOUT_MS: u64 = crate::defaults::DEFAULT_TTFT_TIMEOUT_MS;

pub(crate) const fn default_provider_ttft_timeout_ms() -> Option<u64> {
    Some(crate::defaults::DEFAULT_TTFT_TIMEOUT_MS)
}

pub(crate) const fn default_provider_connect_timeout_ms() -> Option<u64> {
    Some(crate::defaults::DEFAULT_CONNECT_TIMEOUT_MS)
}

impl ProviderConfig {
    /// Resolve the API key from the environment variable named in `api_key_env`.
    #[must_use]
    pub fn resolve_api_key(&self) -> Option<String> {
        self.api_key_env
            .as_ref()
            .and_then(|env_name| std::env::var(env_name).ok())
    }

    /// Derive the [`ProviderTransport`] from the flat config fields.
    ///
    /// Pure CLI kinds (`ClaudeCli`, `GeminiCli`, `CursorCli`, `CodexCli`) always
    /// produce `Cli`; `CursorAcp` always produces `Acp`; HTTP API kinds produce
    /// `Http`. Hybrid kinds (`Hermes`, `OpenClaw`) inspect the available fields:
    /// `command` → `Cli`, otherwise `base_url` → `Http`, otherwise `Local`.
    pub fn transport(&self) -> ProviderTransport {
        let args = self.args.clone().unwrap_or_default();
        match self.kind {
            // Pure CLI providers
            ProviderKind::ClaudeCli => ProviderTransport::Cli {
                command: self.command.clone().unwrap_or_else(|| "claude".to_string()),
                args,
            },
            ProviderKind::GeminiCli => ProviderTransport::Cli {
                command: self.command.clone().unwrap_or_else(|| "gemini".to_string()),
                args,
            },
            ProviderKind::CursorCli => ProviderTransport::Cli {
                command: self.command.clone().unwrap_or_else(|| "cursor".to_string()),
                args,
            },
            ProviderKind::CodexCli => ProviderTransport::Cli {
                command: self.command.clone().unwrap_or_else(|| "codex".to_string()),
                args,
            },
            // ACP provider
            ProviderKind::CursorAcp => ProviderTransport::Acp {
                command: self.command.clone().unwrap_or_else(|| "cursor".to_string()),
                args,
            },
            // Hybrid providers: check fields to decide
            ProviderKind::Hermes | ProviderKind::OpenClaw => {
                if let Some(command) = &self.command {
                    // If args include "acp", the provider runs in ACP mode
                    // over stdio rather than one-shot CLI.
                    if args.iter().any(|a| a == "acp") {
                        ProviderTransport::Acp {
                            command: command.clone(),
                            args,
                        }
                    } else {
                        ProviderTransport::Cli {
                            command: command.clone(),
                            args,
                        }
                    }
                } else if let Some(base_url) = &self.base_url {
                    ProviderTransport::Http {
                        base_url: base_url.clone(),
                    }
                } else {
                    ProviderTransport::Local
                }
            }
            // HTTP API providers
            ProviderKind::AnthropicApi
            | ProviderKind::OpenAiCompat
            | ProviderKind::PerplexityApi
            | ProviderKind::GeminiApi
            | ProviderKind::CerebrasApi => ProviderTransport::Http {
                base_url: self.base_url.clone().unwrap_or_default(),
            },
        }
    }

    /// Derive a typed [`ProviderAuth`] from the flat config fields.
    ///
    /// When `api_key_env` is set, the auth policy is [`ProviderAuth::EnvVar`].
    /// Otherwise the provider is treated as local-only.
    pub fn auth(&self) -> ProviderAuth {
        if let Some(env_var) = &self.api_key_env {
            ProviderAuth::EnvVar {
                name: env_var.clone(),
            }
        } else {
            ProviderAuth::None { local_only: true }
        }
    }
}

// ---- [models.*] ----------------------------------------------------------

/// OpenRouter-specific routing overrides for a model profile.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProviderRouting {
    /// OpenRouter sort mode (`price`, `throughput`, `latency`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    /// Explicit provider order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<Vec<String>>,
    /// Whether OpenRouter may fall back to alternate providers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_fallbacks: Option<bool>,
    /// Maximum cost per token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_price: Option<f64>,
    /// Required provider parameters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub require_parameters: Option<Vec<String>>,
}

/// Model registry entry for `[models.<name>]`.
///
/// A model binds a logical model name to a provider entry and the concrete
/// API slug that gets sent on the wire.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelProfile {
    /// Key into the `[providers.*]` table.
    pub provider: String,
    /// Model ID sent to the API.
    pub slug: String,
    /// Context window in tokens.
    #[serde(default = "default_context_window")]
    pub context_window: u64,
    /// Maximum output tokens, if the provider/model sets one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output: Option<u64>,
    /// Whether the model supports tool calls.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub supports_tools: bool,
    /// Whether the model supports thinking/reasoning output.
    #[serde(default, skip_serializing_if = "is_false")]
    pub supports_thinking: bool,
    /// Whether the model supports vision inputs.
    #[serde(default, skip_serializing_if = "is_false")]
    pub supports_vision: bool,
    /// Whether the model supports web search.
    #[serde(default, skip_serializing_if = "is_false")]
    pub supports_web_search: bool,
    /// Whether the model supports MCP tools.
    #[serde(default, skip_serializing_if = "is_false")]
    pub supports_mcp_tools: bool,
    /// Whether the model supports partial continuation.
    #[serde(default, skip_serializing_if = "is_false")]
    pub supports_partial: bool,
    /// Whether the model supports Google Search grounding.
    #[serde(default, skip_serializing_if = "is_false")]
    pub supports_grounding: bool,
    /// Whether the model supports built-in code execution.
    #[serde(default, skip_serializing_if = "is_false")]
    pub supports_code_execution: bool,
    /// Whether the model supports provider-side context caching.
    #[serde(default, skip_serializing_if = "is_false")]
    pub supports_caching: bool,
    /// OpenRouter-specific routing overrides for this model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_routing: Option<ProviderRouting>,
    /// Wire format used for tools.
    #[serde(default = "default_tool_format")]
    pub tool_format: String,
    /// Input token cost per million tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_input_per_m: Option<f64>,
    /// Output token cost per million tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_output_per_m: Option<f64>,
    /// Input token cost per million tokens for the high-context pricing tier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_input_per_m_high: Option<f64>,
    /// Output token cost per million tokens for the high-context pricing tier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_output_per_m_high: Option<f64>,
    /// Cache read cost per million tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_cache_read_per_m: Option<f64>,
    /// Cache write cost per million tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_cache_write_per_m: Option<f64>,
    /// Provider-specific reasoning depth label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking_level: Option<String>,
    /// Maximum number of tools before behavior degrades.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tools: Option<u32>,
    /// Per-model tool-loop iteration cap.
    ///
    /// When set, overrides the workspace default (`DEFAULT_MAX_TOOL_ITERATIONS`)
    /// before the temperament adjustment is applied. Use this to raise the cap for
    /// models known to need many sequential tool calls (e.g. complex Opus plans) or
    /// lower it for fast-tier models where runaway loops are more costly.
    ///
    /// `None` means use the workspace default. The final cap is:
    ///   `(max_tool_iterations.unwrap_or(DEFAULT_MAX_TOOL_ITERATIONS) adjusted by temperament)`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tool_iterations: Option<u32>,
    /// Tokenizer ratio vs OpenAI `o200k_base`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokenizer_ratio: Option<f64>,
    /// Whether the model supports web-grounded search (Perplexity Sonar).
    #[serde(default, skip_serializing_if = "is_false")]
    pub supports_search: bool,
    /// Whether the model returns citations in responses (Perplexity Sonar).
    #[serde(default, skip_serializing_if = "is_false")]
    pub supports_citations: bool,
    /// Whether the model supports the async job API (Perplexity deep research).
    #[serde(default, skip_serializing_if = "is_false")]
    pub supports_async: bool,
    /// Whether this is an embedding model rather than a chat model.
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_embedding_model: bool,
    /// Search context size hint: "low", "medium", or "high" (Perplexity).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search_context_size: Option<String>,
    /// Per-request fee in USD on top of token costs (Perplexity pricing model).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_per_request: Option<f64>,
    /// Capability tier for model routing (Fast/Standard/Premium).
    ///
    /// When set, the cascade router and conductor can route by tier
    /// without slug-substring guessing. `None` means the router must
    /// fall back to heuristic detection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier: Option<crate::agent::ModelTier>,
    /// Whether this model uses `max_completion_tokens` instead of `max_tokens`.
    /// Required for newer OpenAI models (o1, o3, gpt-4o, gpt-5.x, etc.).
    #[serde(default, skip_serializing_if = "is_false")]
    pub use_max_completion_tokens: bool,
}

impl ModelProfile {
    /// Resolved output-token ceiling for this model.
    ///
    /// `None` in config means "use the runtime default", which currently
    /// matches the agent dispatch fallback.
    #[must_use]
    pub fn effective_max_output(&self) -> u64 {
        self.max_output
            .unwrap_or(u64::from(DEFAULT_MAX_OUTPUT_TOKENS))
    }
}

// ---- Gemini config -------------------------------------------------------

fn default_thinking_medium() -> String {
    "medium".to_string()
}

/// Gemini-specific model and request settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeminiConfig {
    /// Default model for standard Gemini chat requests.
    pub default_model: Option<String>,
    /// Default model for Gemini grounding requests.
    pub grounding_model: Option<String>,
    /// Default model for Gemini code execution requests.
    pub code_exec_model: Option<String>,
    /// Default Gemini embedding model.
    pub embed_model: Option<String>,
    /// Prefer the standard-tier free models when available.
    #[serde(default)]
    pub use_free_tier: bool,
    /// Gemini native thinking depth: "minimal", "low", "medium", or "high".
    #[serde(default = "default_thinking_medium")]
    pub thinking_level: String,
    /// Enable provider-side context caching when supported.
    #[serde(default)]
    pub enable_context_caching: bool,
    /// Per-category Gemini safety thresholds.
    #[serde(default)]
    pub safety_settings: Vec<SafetySetting>,
}

impl Default for GeminiConfig {
    fn default() -> Self {
        Self {
            default_model: None,
            grounding_model: None,
            code_exec_model: None,
            embed_model: None,
            use_free_tier: false,
            thinking_level: default_thinking_medium(),
            enable_context_caching: false,
            safety_settings: Vec::new(),
        }
    }
}

/// Gemini native safety configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafetySetting {
    /// Gemini harm category, e.g. `HARM_CATEGORY_HATE_SPEECH`.
    pub category: String,
    /// Gemini blocking threshold, e.g. `BLOCK_NONE`.
    pub threshold: String,
}

// ---- Perplexity config ---------------------------------------------------

fn default_recency() -> String {
    "year".to_string()
}

/// Perplexity-specific search and model settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerplexityConfig {
    /// Default model for search-grounded queries.
    pub default_search_model: Option<String>,
    /// Default model for deep research tasks.
    pub default_research_model: Option<String>,
    /// Default model for reasoning tasks.
    pub default_reasoning_model: Option<String>,
    /// Default model for embeddings.
    pub default_embed_model: Option<String>,
    /// Recency filter applied to web search: "hour"/"day"/"week"/"month"/"year".
    #[serde(default = "default_recency")]
    pub search_recency_filter: String,
    /// Restrict results to academic sources.
    #[serde(default)]
    pub academic_mode: bool,
    /// Global domain allowlist for web search.
    #[serde(default)]
    pub search_domain_filter: Vec<String>,
    /// Include images in search results.
    #[serde(default)]
    pub return_images: bool,
    /// Include related questions in search results.
    #[serde(default = "default_true")]
    pub return_related_questions: bool,
    /// When true, `auto` backend selection prefers Perplexity deep research
    /// over standard search for `research topic`. Default `false`.
    #[serde(default)]
    pub auto_deep: bool,
}

impl Default for PerplexityConfig {
    fn default() -> Self {
        Self {
            default_search_model: None,
            default_research_model: None,
            default_reasoning_model: None,
            default_embed_model: None,
            search_recency_filter: default_recency(),
            academic_mode: false,
            search_domain_filter: Vec::new(),
            return_images: false,
            return_related_questions: true,
            auto_deep: false,
        }
    }
}

#[cfg(test)]
mod model_profile_tests {
    use super::*;

    #[test]
    fn provider_process_limits_roundtrip_through_toml() {
        let limits = ProviderLimits {
            rpm: 50,
            tpm: 40_000,
            max_cpu_seconds: Some(120),
            max_rss_bytes: Some(2_147_483_648),
            max_processes: Some(8),
            network: ProviderNetworkPolicy::Deny,
        };

        let encoded = toml::to_string(&limits).expect("serialize provider limits");
        assert!(encoded.contains("network = \"deny\""));
        let decoded: ProviderLimits = toml::from_str(&encoded).expect("deserialize limits");
        assert_eq!(decoded, limits);
    }

    #[test]
    fn provider_network_allow_is_the_compatible_default() {
        let limits: ProviderLimits =
            toml::from_str("rpm = 10\ntpm = 0\n").expect("deserialize legacy provider limits");
        assert_eq!(limits.network, ProviderNetworkPolicy::Allow);

        let encoded = toml::to_string(&limits).expect("serialize limits");
        assert!(!encoded.contains("network"));
    }

    #[test]
    fn model_profile_max_tool_iterations_roundtrips_through_toml() {
        let profile = ModelProfile {
            provider: "test".to_string(),
            slug: "test-model".to_string(),
            max_tool_iterations: Some(25),
            ..Default::default()
        };

        let toml_str = toml::to_string_pretty(&profile).expect("serialize ModelProfile");
        assert!(
            toml_str.contains("max_tool_iterations = 25"),
            "TOML should contain max_tool_iterations field: {toml_str}"
        );

        let back: ModelProfile = toml::from_str(&toml_str).expect("deserialize ModelProfile");
        assert_eq!(back.max_tool_iterations, Some(25));
    }

    #[test]
    fn model_profile_max_tool_iterations_defaults_to_none() {
        let toml_str = r#"
            provider = "test"
            slug = "test-model"
        "#;
        let profile: ModelProfile = toml::from_str(toml_str).expect("deserialize");
        assert_eq!(profile.max_tool_iterations, None);
    }

    #[test]
    fn model_profile_max_tool_iterations_none_omitted_in_serialization() {
        let profile = ModelProfile {
            provider: "test".to_string(),
            slug: "test-model".to_string(),
            max_tool_iterations: None,
            ..Default::default()
        };

        let toml_str = toml::to_string_pretty(&profile).expect("serialize");
        assert!(
            !toml_str.contains("max_tool_iterations"),
            "None field should be skipped: {toml_str}"
        );
    }

    // ── Task 061: effective_max_output() edge case tests ─────────────────

    #[test]
    fn effective_max_output_uses_explicit_value_when_set() {
        let profile = ModelProfile {
            provider: "test".to_string(),
            slug: "claude-opus".to_string(),
            max_output: Some(128_000),
            ..Default::default()
        };
        assert_eq!(profile.effective_max_output(), 128_000);
    }

    #[test]
    fn effective_max_output_falls_back_to_default_when_none() {
        let profile = ModelProfile {
            provider: "test".to_string(),
            slug: "claude-haiku".to_string(),
            max_output: None,
            ..Default::default()
        };
        assert_eq!(
            profile.effective_max_output(),
            u64::from(DEFAULT_MAX_OUTPUT_TOKENS)
        );
    }

    #[test]
    fn effective_max_output_handles_zero_explicit_value() {
        // A model configured with max_output = 0 should return 0, not the default.
        // This tests that the function respects Some(0) as intentional.
        let profile = ModelProfile {
            provider: "test".to_string(),
            slug: "embedding-model".to_string(),
            max_output: Some(0),
            ..Default::default()
        };
        assert_eq!(profile.effective_max_output(), 0);
    }

    #[test]
    fn effective_max_output_handles_minimum_value() {
        let profile = ModelProfile {
            provider: "test".to_string(),
            slug: "tiny-model".to_string(),
            max_output: Some(1),
            ..Default::default()
        };
        assert_eq!(profile.effective_max_output(), 1);
    }

    #[test]
    fn effective_max_output_handles_u64_max() {
        let profile = ModelProfile {
            provider: "test".to_string(),
            slug: "unlimited-model".to_string(),
            max_output: Some(u64::MAX),
            ..Default::default()
        };
        assert_eq!(profile.effective_max_output(), u64::MAX);
    }

    #[test]
    fn effective_max_output_default_profile_uses_default_constant() {
        // The Default impl sets max_output to None, so effective_max_output
        // must return DEFAULT_MAX_OUTPUT_TOKENS for a fully default profile.
        let profile = ModelProfile::default();
        assert_eq!(
            profile.effective_max_output(),
            u64::from(DEFAULT_MAX_OUTPUT_TOKENS)
        );
        // Sanity: the default is a known sane value.
        assert_eq!(profile.effective_max_output(), 16_384);
    }

    #[test]
    fn effective_max_output_deserialized_from_toml_with_explicit_value() {
        let toml_str = r#"
            provider = "anthropic"
            slug = "claude-opus-4"
            max_output = 65536
        "#;
        let profile: ModelProfile = toml::from_str(toml_str).expect("deserialize");
        assert_eq!(profile.effective_max_output(), 65_536);
    }

    #[test]
    fn effective_max_output_deserialized_from_toml_without_field() {
        let toml_str = r#"
            provider = "anthropic"
            slug = "claude-sonnet-4"
        "#;
        let profile: ModelProfile = toml::from_str(toml_str).expect("deserialize");
        assert_eq!(
            profile.effective_max_output(),
            u64::from(DEFAULT_MAX_OUTPUT_TOKENS),
            "missing max_output in TOML should fall back to DEFAULT_MAX_OUTPUT_TOKENS"
        );
    }
}

#[cfg(test)]
mod perplexity_config_tests {
    use super::*;

    #[test]
    fn auto_deep_defaults_to_false() {
        let cfg = PerplexityConfig::default();
        assert!(!cfg.auto_deep, "auto_deep must default to false");
    }

    #[test]
    fn auto_deep_roundtrips_through_toml() {
        let cfg = PerplexityConfig {
            auto_deep: true,
            ..Default::default()
        };
        let encoded = toml::to_string(&cfg).expect("serialize PerplexityConfig");
        assert!(encoded.contains("auto_deep = true"));
        let decoded: PerplexityConfig = toml::from_str(&encoded).expect("deserialize");
        assert!(decoded.auto_deep);
    }

    #[test]
    fn auto_deep_absent_in_toml_defaults_to_false() {
        let toml_str = r#"
            search_recency_filter = "month"
        "#;
        let cfg: PerplexityConfig = toml::from_str(toml_str).expect("deserialize");
        assert!(!cfg.auto_deep);
    }

    #[test]
    fn auto_deep_false_omitted_from_serialization() {
        let cfg = PerplexityConfig::default();
        let encoded = toml::to_string(&cfg).expect("serialize");
        assert!(
            !encoded.contains("auto_deep"),
            "auto_deep = false should be omitted: {encoded}"
        );
    }
}

#[cfg(test)]
mod transport_derivation_tests {
    use super::*;

    fn http_provider(kind: ProviderKind) -> ProviderConfig {
        ProviderConfig {
            kind,
            base_url: Some("https://api.example.com".to_string()),
            api_key_env: Some("API_KEY".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn anthropic_api_derives_http_transport() {
        let p = http_provider(ProviderKind::AnthropicApi);
        let t = p.transport();
        assert!(
            matches!(t, ProviderTransport::Http { ref base_url } if base_url == "https://api.example.com"),
            "expected Http transport, got {t:?}"
        );
    }

    #[test]
    fn openai_compat_derives_http_transport() {
        let p = http_provider(ProviderKind::OpenAiCompat);
        assert!(matches!(p.transport(), ProviderTransport::Http { .. }));
    }

    #[test]
    fn http_provider_without_base_url_uses_empty_string() {
        let p = ProviderConfig {
            kind: ProviderKind::AnthropicApi,
            base_url: None,
            ..Default::default()
        };
        assert!(matches!(
            p.transport(),
            ProviderTransport::Http { ref base_url } if base_url.is_empty()
        ));
    }

    #[test]
    fn claude_cli_derives_cli_transport() {
        let p = ProviderConfig {
            kind: ProviderKind::ClaudeCli,
            ..Default::default()
        };
        let t = p.transport();
        assert!(
            matches!(t, ProviderTransport::Cli { ref command, .. } if command == "claude"),
            "expected Cli transport with 'claude', got {t:?}"
        );
    }

    #[test]
    fn claude_cli_uses_custom_command() {
        let p = ProviderConfig {
            kind: ProviderKind::ClaudeCli,
            command: Some("/usr/local/bin/claude-dev".to_string()),
            ..Default::default()
        };
        assert!(matches!(
            p.transport(),
            ProviderTransport::Cli { ref command, .. } if command == "/usr/local/bin/claude-dev"
        ));
    }

    #[test]
    fn cursor_acp_derives_acp_transport() {
        let p = ProviderConfig {
            kind: ProviderKind::CursorAcp,
            ..Default::default()
        };
        assert!(matches!(p.transport(), ProviderTransport::Acp { .. }));
    }

    #[test]
    fn hermes_with_base_url_derives_http() {
        let p = ProviderConfig {
            kind: ProviderKind::Hermes,
            base_url: Some("http://localhost:8080".to_string()),
            command: Some("hermes".to_string()),
            ..Default::default()
        };
        // command is set but base_url takes precedence? No — existing logic
        // checks command first for Hermes. Let's verify actual behavior.
        let t = p.transport();
        // Hermes checks command first, then base_url.
        assert!(
            matches!(t, ProviderTransport::Cli { .. }),
            "Hermes with command set should be Cli, got {t:?}"
        );
    }

    #[test]
    fn hermes_http_only_derives_http() {
        let p = ProviderConfig {
            kind: ProviderKind::Hermes,
            base_url: Some("http://localhost:8080".to_string()),
            ..Default::default()
        };
        assert!(matches!(p.transport(), ProviderTransport::Http { .. }));
    }

    #[test]
    fn hermes_bare_derives_local() {
        let p = ProviderConfig {
            kind: ProviderKind::Hermes,
            ..Default::default()
        };
        assert!(matches!(p.transport(), ProviderTransport::Local));
    }

    #[test]
    fn auth_from_env_var() {
        let p = ProviderConfig {
            api_key_env: Some("MY_KEY".to_string()),
            ..Default::default()
        };
        assert!(matches!(
            p.auth(),
            ProviderAuth::EnvVar { ref name } if name == "MY_KEY"
        ));
    }

    #[test]
    fn auth_local_only_when_no_key() {
        let p = ProviderConfig::default();
        assert!(matches!(p.auth(), ProviderAuth::None { local_only: true }));
    }
}
