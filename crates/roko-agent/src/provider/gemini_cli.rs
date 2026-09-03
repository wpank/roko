//! Adapter for the Google Gemini CLI subprocess provider.
//!
//! This adapter spawns the `gemini` binary (Google Gemini CLI) as a subprocess.
//! Unlike the Gemini API backends, this uses Google OAuth authentication rather
//! than an API key — users authenticate via `gemini /auth` beforehand.
//!
//! The adapter falls through to [`ExecAgent`](crate::ExecAgent) for legacy
//! one-shot callers. Runner-v2 uses Gemini's headless stream-JSON protocol and
//! authenticated per-task MCP configuration through its CLI dispatch adapter.
//!
//! # Configuration
//!
//! ```toml
//! [providers.gemini-cli]
//! kind = "gemini_cli"
//! # No api_key_env — uses Google OAuth via `gemini /auth`
//! # Optional: override the binary path
//! command = "gemini"   # default
//!
//! [models.gemini-3-flash]
//! provider = "gemini-cli"
//! slug = "gemini-3-flash-preview"
//! ```

use crate::Agent;
use crate::exec::ExecAgent;
use crate::provider::pre_flight::binary_on_path;
use crate::provider::{
    AgentCreationError, AgentOptions, ProviderAdapter, ProviderError, configured_resource_limits,
};
use roko_core::agent::ProviderKind;
use roko_core::config::schema::{ModelProfile, ProviderConfig};
use roko_core::defaults::DEFAULT_REQUEST_TIMEOUT_MS;
use serde_json::Value;
use std::path::PathBuf;

/// Default Gemini CLI binary name.
const DEFAULT_GEMINI_COMMAND: &str = "gemini";

/// Gemini headless stream-JSON translation.
pub mod stream;

/// Provider adapter for the Gemini CLI subprocess.
pub struct GeminiCliAdapter;

impl ProviderAdapter for GeminiCliAdapter {
    fn kind(&self) -> ProviderKind {
        ProviderKind::GeminiCli
    }

    fn create_agent(
        &self,
        provider: &ProviderConfig,
        model: &ModelProfile,
        options: &AgentOptions,
    ) -> Result<Box<dyn Agent>, AgentCreationError> {
        if provider.kind != self.kind() {
            return Err(AgentCreationError::InvalidKind(provider.kind));
        }

        let command = provider
            .command
            .as_deref()
            .map(str::trim)
            .filter(|cmd| !cmd.is_empty())
            .unwrap_or(DEFAULT_GEMINI_COMMAND);

        // Pre-flight: the binary must be present on PATH before we attempt to
        // spawn. Report a clear error rather than falling back silently.
        if !binary_on_path(command) {
            return Err(AgentCreationError::BinaryNotFound(format!(
                "{command} (Gemini CLI — install from https://github.com/google-gemini/gemini-cli \
                 or configure providers.*.command in roko.toml)"
            )));
        }

        let working_dir = options
            .working_dir
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let timeout_ms = options
            .timeout_ms
            .or(provider.timeout_ms)
            .unwrap_or(DEFAULT_REQUEST_TIMEOUT_MS);

        // Build the argument list for `gemini -m <slug> -p`.
        // -m selects the model; -p signals non-interactive prompt mode (reads stdin).
        let mut extra_args = vec!["-m".to_string(), model.slug.clone(), "-p".to_string()];

        // Append provider-level args (e.g. `--sandbox`, `--debug`) before option-level args.
        if let Some(provider_args) = &provider.args {
            extra_args.extend(provider_args.iter().cloned());
        }
        extra_args.extend(options.extra_args.iter().cloned());

        let name = if options.name.is_empty() {
            format!("gemini-cli:{}", model.slug)
        } else {
            options.name.clone()
        };

        let mut agent = ExecAgent::new(
            command,
            extra_args,
            crate::safety::SafetyLayer::with_defaults().with_role("implementer"),
        )
        .with_timeout_ms(timeout_ms)
        .with_name(name)
        .with_current_dir(working_dir);

        if let Some(limits) = configured_resource_limits(provider)? {
            agent = agent.with_resource_limits(limits);
        }

        for (key, value) in &options.env {
            agent = agent.with_env_var(key.clone(), value.clone());
        }

        Ok(Box::new(agent))
    }

    fn classify_error(&self, status: u16, body: &Value) -> ProviderError {
        // For a CLI subprocess, body typically carries stderr text.
        let stderr = body
            .as_str()
            .or_else(|| body.pointer("/error").and_then(Value::as_str))
            .or_else(|| body.pointer("/message").and_then(Value::as_str))
            .unwrap_or("");
        let lower = stderr.to_ascii_lowercase();

        if lower.contains("rate limit") || lower.contains("quota") {
            return ProviderError::RateLimit {
                retry_after_ms: None,
            };
        }
        if lower.contains("unauthorized")
            || lower.contains("permission denied")
            || lower.contains("unauthenticated")
            || lower.contains("sign in")
            || lower.contains("not logged in")
        {
            return ProviderError::AuthFailure;
        }
        if lower.contains("timed out") || lower.contains("timeout") {
            return ProviderError::Timeout;
        }
        if lower.contains("context window")
            || lower.contains("context length")
            || lower.contains("token limit")
        {
            return ProviderError::ContextOverflow;
        }
        if lower.contains("model not found") || lower.contains("unknown model") {
            return ProviderError::ModelNotFound;
        }

        match status {
            429 => ProviderError::RateLimit {
                retry_after_ms: None,
            },
            401 | 403 => ProviderError::AuthFailure,
            404 => ProviderError::ModelNotFound,
            408 => ProviderError::Timeout,
            500..=599 => ProviderError::ServerError(status),
            _ => {
                if stderr.is_empty() {
                    ProviderError::Other(format!("gemini CLI exit status {status}"))
                } else {
                    ProviderError::Other(stderr.to_string())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::config::DEFAULT_TTFT_TIMEOUT_MS;

    fn gemini_cli_provider(command: Option<&str>) -> ProviderConfig {
        ProviderConfig {
            kind: ProviderKind::GeminiCli,
            base_url: None,
            api_key_env: None,
            command: command.map(str::to_string),
            args: None,
            timeout_ms: Some(30_000),
            ttft_timeout_ms: Some(DEFAULT_TTFT_TIMEOUT_MS),
            connect_timeout_ms: None,
            extra_headers: None,
            max_concurrent: None,
            limits: None,
        }
    }

    fn gemini_cli_model() -> ModelProfile {
        ModelProfile {
            provider: "gemini-cli".to_string(),
            slug: "gemini-3-flash-preview".to_string(),
            context_window: 1_048_576,
            max_output: Some(8_192),
            supports_tools: false,
            ..Default::default()
        }
    }

    #[test]
    fn gemini_cli_adapter_kind() {
        assert_eq!(GeminiCliAdapter.kind(), ProviderKind::GeminiCli);
    }

    #[test]
    fn gemini_cli_adapter_rejects_wrong_kind() {
        let provider = ProviderConfig {
            kind: ProviderKind::GeminiApi,
            ..gemini_cli_provider(None)
        };
        let result =
            GeminiCliAdapter.create_agent(&provider, &gemini_cli_model(), &AgentOptions::default());
        let Err(err) = result else {
            panic!("expected Err, got Ok");
        };
        assert!(matches!(err, AgentCreationError::InvalidKind(_)));
    }

    #[test]
    fn gemini_cli_adapter_missing_binary_returns_binary_not_found() {
        let provider = gemini_cli_provider(Some("roko-nonexistent-gemini-binary-xyz-090"));
        let result =
            GeminiCliAdapter.create_agent(&provider, &gemini_cli_model(), &AgentOptions::default());
        let Err(err) = result else {
            panic!("expected Err, got Ok");
        };
        if let AgentCreationError::BinaryNotFound(ref msg) = err {
            assert!(
                msg.contains("roko-nonexistent-gemini-binary-xyz-090"),
                "error message should include the missing binary name: {msg}"
            );
        } else {
            panic!("expected BinaryNotFound, got {err:?}");
        }
    }

    #[test]
    fn gemini_cli_adapter_classify_rate_limit() {
        let err = GeminiCliAdapter.classify_error(
            429,
            &serde_json::Value::String("rate limit exceeded".to_string()),
        );
        assert!(matches!(err, ProviderError::RateLimit { .. }));
    }

    #[test]
    fn gemini_cli_adapter_classify_auth_failure_unauthenticated_message() {
        let err = GeminiCliAdapter.classify_error(
            401,
            &serde_json::Value::String("not logged in — run `gemini /auth`".to_string()),
        );
        assert!(matches!(err, ProviderError::AuthFailure));
    }

    #[test]
    fn gemini_cli_adapter_classify_auth_failure_status_code() {
        let err = GeminiCliAdapter.classify_error(403, &serde_json::Value::Null);
        assert!(matches!(err, ProviderError::AuthFailure));
    }

    #[test]
    fn gemini_cli_adapter_classify_model_not_found() {
        let err = GeminiCliAdapter.classify_error(
            404,
            &serde_json::Value::String("model not found".to_string()),
        );
        assert!(matches!(err, ProviderError::ModelNotFound));
    }

    #[test]
    fn gemini_cli_adapter_classify_timeout() {
        let err = GeminiCliAdapter.classify_error(
            408,
            &serde_json::Value::String("request timed out".to_string()),
        );
        assert!(matches!(err, ProviderError::Timeout));
    }

    #[test]
    fn gemini_cli_adapter_classify_server_error() {
        let err = GeminiCliAdapter.classify_error(503, &serde_json::Value::Null);
        assert!(matches!(err, ProviderError::ServerError(503)));
    }
}
