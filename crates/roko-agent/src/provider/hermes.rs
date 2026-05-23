use crate::Agent;
use crate::hermes::{HermesAcpAgent, HermesAcpConfig, HermesConfig, HermesHttpAgent};
use crate::hermes::{HermesFlavor, HermesOneShotAgent, HermesOneShotConfig};
use crate::provider::{AgentCreationError, AgentOptions, ProviderAdapter, ProviderError};
use roko_core::agent::ProviderKind;
use roko_core::config::schema::{ModelProfile, ProviderConfig};
use roko_core::defaults::DEFAULT_REQUEST_TIMEOUT_MS;
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;

/// Adapter for the Hermes harness (HTTP, one-shot CLI, or ACP).
///
/// Transport tier selection:
///
/// - `base_url` present → Tier 1 [`HermesHttpAgent`] (OpenAI-compatible HTTP)
/// - `args` contains `"acp"` → Tier 3 [`HermesAcpAgent`] (ACP over stdio)
/// - Otherwise → Tier 2 [`HermesOneShotAgent`] (one-shot CLI)
pub struct HermesProviderAdapter;

impl ProviderAdapter for HermesProviderAdapter {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Hermes
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

        let timeout_ms = options
            .timeout_ms
            .or(provider.timeout_ms)
            .unwrap_or(DEFAULT_REQUEST_TIMEOUT_MS);
        let timeout = Duration::from_millis(timeout_ms);

        let working_dir = options
            .working_dir
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        // Tier selection: base_url → HTTP, args contain "acp" → ACP, else → oneshot.
        let is_acp = provider
            .args
            .as_ref()
            .is_some_and(|args| args.iter().any(|a| a == "acp"));

        if provider.base_url.is_some() {
            // Tier 1: HTTP via HermesHttpAgent.
            let mut config = HermesConfig::from_provider_config(provider);
            config.timeout = timeout;
            if model.slug.is_empty() {
                // No model override — use config default.
            } else {
                config.model = Some(model.slug.clone());
            }
            let agent = HermesHttpAgent::new(config);
            Ok(Box::new(agent))
        } else if is_acp {
            // Tier 3: ACP over stdio.
            let binary = provider
                .command
                .as_deref()
                .map(str::trim)
                .filter(|c| !c.is_empty())
                .unwrap_or("hermes");
            let config = HermesAcpConfig {
                binary: binary.to_string(),
                cwd: working_dir,
                session_key: None,
                model_hint: if model.slug.is_empty() {
                    None
                } else {
                    Some(model.slug.clone())
                },
                timeout,
                mcp_servers: None,
            };
            let agent = HermesAcpAgent::new(config);
            Ok(Box::new(agent))
        } else {
            // Tier 2: One-shot CLI.
            let binary = provider
                .command
                .as_deref()
                .map(str::trim)
                .filter(|c| !c.is_empty())
                .unwrap_or("hermes");
            let config = HermesOneShotConfig {
                binary: binary.to_string(),
                flavor: HermesFlavor::ChatQuiet,
                model_override: if model.slug.is_empty() {
                    None
                } else {
                    Some(model.slug.clone())
                },
                timeout,
                ..Default::default()
            };
            let agent = HermesOneShotAgent::new(config);
            Ok(Box::new(agent))
        }
    }

    fn classify_error(&self, status: u16, body: &Value) -> ProviderError {
        let stderr = body
            .as_str()
            .or_else(|| body.pointer("/error").and_then(Value::as_str))
            .or_else(|| body.pointer("/message").and_then(Value::as_str))
            .unwrap_or("");
        let lower = stderr.to_ascii_lowercase();

        if lower.contains("rate limit") {
            return ProviderError::RateLimit {
                retry_after_ms: None,
            };
        }
        if lower.contains("unauthorized") || lower.contains("permission denied") {
            return ProviderError::AuthFailure;
        }
        if lower.contains("timed out") || lower.contains("timeout") {
            return ProviderError::Timeout;
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
                    ProviderError::Other(format!("Hermes exit status {status}"))
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

    #[test]
    fn hermes_adapter_kind() {
        let adapter = HermesProviderAdapter;
        assert_eq!(adapter.kind(), ProviderKind::Hermes);
    }

    #[test]
    fn hermes_adapter_selects_http_when_base_url_present() {
        let provider = ProviderConfig {
            kind: ProviderKind::Hermes,
            base_url: Some("http://localhost:8642".to_string()),
            api_key_env: None,
            command: None,
            args: None,
            timeout_ms: Some(5_000),
            ttft_timeout_ms: None,
            connect_timeout_ms: None,
            extra_headers: None,
            max_concurrent: None,
        };
        let model = ModelProfile {
            provider: "hermes".to_string(),
            slug: "hermes-3-llama-70b".to_string(),
            ..Default::default()
        };
        let options = AgentOptions {
            name: "hermes-http".to_string(),
            ..Default::default()
        };
        let agent = HermesProviderAdapter
            .create_agent(&provider, &model, &options)
            .expect("create hermes HTTP agent");
        assert_eq!(agent.backend_id(), "hermes-http");
    }

    #[test]
    fn hermes_adapter_selects_oneshot_by_default() {
        let provider = ProviderConfig {
            kind: ProviderKind::Hermes,
            base_url: None,
            api_key_env: None,
            command: Some("hermes".to_string()),
            args: None,
            timeout_ms: Some(5_000),
            ttft_timeout_ms: None,
            connect_timeout_ms: None,
            extra_headers: None,
            max_concurrent: None,
        };
        let model = ModelProfile {
            provider: "hermes".to_string(),
            slug: "hermes-3-llama-70b".to_string(),
            ..Default::default()
        };
        let options = AgentOptions {
            name: "hermes-oneshot".to_string(),
            ..Default::default()
        };
        let agent = HermesProviderAdapter
            .create_agent(&provider, &model, &options)
            .expect("create hermes oneshot agent");
        assert_eq!(agent.backend_id(), "hermes-oneshot");
    }

    #[test]
    fn hermes_adapter_selects_acp_when_args_contain_acp() {
        let provider = ProviderConfig {
            kind: ProviderKind::Hermes,
            base_url: None,
            api_key_env: None,
            command: Some("hermes".to_string()),
            args: Some(vec!["acp".to_string()]),
            timeout_ms: Some(5_000),
            ttft_timeout_ms: None,
            connect_timeout_ms: None,
            extra_headers: None,
            max_concurrent: None,
        };
        let model = ModelProfile {
            provider: "hermes".to_string(),
            slug: "hermes-3-llama-70b".to_string(),
            ..Default::default()
        };
        let options = AgentOptions {
            name: "hermes-acp".to_string(),
            ..Default::default()
        };
        let agent = HermesProviderAdapter
            .create_agent(&provider, &model, &options)
            .expect("create hermes ACP agent");
        assert_eq!(agent.backend_id(), "hermes-acp");
    }

    #[test]
    fn hermes_adapter_rejects_wrong_kind() {
        let provider = ProviderConfig {
            kind: ProviderKind::OpenAiCompat,
            base_url: None,
            api_key_env: None,
            command: None,
            args: None,
            timeout_ms: None,
            ttft_timeout_ms: None,
            connect_timeout_ms: None,
            extra_headers: None,
            max_concurrent: None,
        };
        let model = ModelProfile::default();
        let options = AgentOptions::default();
        let result = HermesProviderAdapter.create_agent(&provider, &model, &options);
        assert!(result.is_err());
    }

    #[test]
    fn hermes_classify_error_rate_limit() {
        let err = HermesProviderAdapter.classify_error(429, &Value::Null);
        assert!(matches!(err, ProviderError::RateLimit { .. }));
    }

    #[test]
    fn hermes_classify_error_auth() {
        let err = HermesProviderAdapter.classify_error(401, &Value::Null);
        assert!(matches!(err, ProviderError::AuthFailure));
    }

    #[test]
    fn hermes_classify_error_stderr_timeout() {
        let body = Value::String("request timed out after 90s".to_string());
        let err = HermesProviderAdapter.classify_error(0, &body);
        assert!(matches!(err, ProviderError::Timeout));
    }
}
