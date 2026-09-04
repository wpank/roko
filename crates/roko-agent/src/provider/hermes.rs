use crate::Agent;
use crate::hermes::{HermesAcpAgent, HermesAcpConfig, HermesConfig, HermesHttpAgent};
use crate::hermes::{HermesFlavor, HermesOneShotAgent, HermesOneShotConfig};
use crate::provider::{
    AgentCreationError, AgentOptions, ProviderAdapter, ProviderError, configured_resource_limits,
};
use roko_core::agent::ProviderKind;
use roko_core::config::schema::{ModelProfile, ProviderConfig, ProviderTransport};
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

        let timeout_ms = options.effective_timeout_ms(provider.timeout_ms);
        let timeout = Duration::from_millis(timeout_ms);

        let working_dir = options
            .working_dir
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let resource_limits = configured_resource_limits(provider)?;

        // Tier selection via typed transport.
        let transport = provider.transport();
        match transport {
            ProviderTransport::Http { .. } => {
                // Tier 1: HTTP via HermesHttpAgent.
                let mut config = HermesConfig::from_provider_config(provider);
                config.timeout = timeout;
                if !model.slug.is_empty() {
                    config.model = Some(model.slug.clone());
                }
                let mut agent = HermesHttpAgent::new(config);
                if let Some(prompt) = &options.system_prompt {
                    agent = agent.with_system_prompt(prompt.clone());
                }
                Ok(Box::new(agent))
            }
            ProviderTransport::Acp { ref command, .. } => {
                // Tier 3: ACP over stdio.
                let binary = command.trim();
                let binary = if binary.is_empty() { "hermes" } else { binary };
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
                    mcp_servers: options.local_tool_mcp_servers.as_ref().map(|servers| {
                        Value::Array(servers.iter().map(|server| server.to_acp_json()).collect())
                    }),
                    resource_limits,
                    system_prompt: options.system_prompt.clone(),
                };
                let agent = HermesAcpAgent::new(config);
                Ok(Box::new(agent))
            }
            ProviderTransport::Cli { .. } | ProviderTransport::Local => {
                // Tier 2: One-shot CLI.
                let binary = match &transport {
                    ProviderTransport::Cli { command, .. } => {
                        let trimmed = command.trim();
                        if trimmed.is_empty() {
                            "hermes"
                        } else {
                            trimmed
                        }
                    }
                    _ => "hermes",
                };
                let config = HermesOneShotConfig {
                    binary: binary.to_string(),
                    flavor: HermesFlavor::ChatQuiet,
                    model_override: if model.slug.is_empty() {
                        None
                    } else {
                        Some(model.slug.clone())
                    },
                    timeout,
                    resource_limits,
                    system_prompt: options.system_prompt.clone(),
                    ..Default::default()
                };
                let agent = HermesOneShotAgent::new(config);
                Ok(Box::new(agent))
            }
        }
    }

    fn supports_per_call_local_mcp(&self, provider: &ProviderConfig) -> bool {
        matches!(provider.transport(), ProviderTransport::Acp { .. })
    }

    fn classify_error(&self, status: u16, body: &Value) -> ProviderError {
        super::error_classify::classify_cli_error(status, body, "Hermes")
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
            limits: None,
            require_confirmation: false,
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
            limits: None,
            require_confirmation: false,
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
            limits: None,
            require_confirmation: false,
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
            limits: None,
            require_confirmation: false,
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
