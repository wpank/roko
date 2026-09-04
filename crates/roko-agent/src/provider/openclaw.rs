use crate::Agent;
use crate::openclaw::{
    OpenClawAcpAgent, OpenClawAcpConfig, OpenClawInferAgent, OpenClawInferConfig,
};
use crate::provider::{
    AgentCreationError, AgentOptions, ProviderAdapter, ProviderError, configured_resource_limits,
};
use roko_core::agent::ProviderKind;
use roko_core::config::schema::{ModelProfile, ProviderConfig, ProviderTransport};
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;

/// Adapter for the OpenClaw harness (CLI infer or ACP).
///
/// Transport tier selection:
///
/// - `args` contains `"acp"` → Tier 3 [`OpenClawAcpAgent`] (ACP over stdio)
/// - Otherwise → Tier 2 [`OpenClawInferAgent`] (`openclaw infer ... --json`)
pub struct OpenClawProviderAdapter;

impl ProviderAdapter for OpenClawProviderAdapter {
    fn kind(&self) -> ProviderKind {
        ProviderKind::OpenClaw
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
        let transport = provider.transport();

        match transport {
            ProviderTransport::Acp { ref command, .. } => {
                // Tier 3: ACP over stdio.
                let binary = command.trim();
                let binary = if binary.is_empty() {
                    "openclaw"
                } else {
                    binary
                };
                let config = OpenClawAcpConfig {
                    binary: binary.to_string(),
                    cwd: working_dir,
                    gateway_url: provider.base_url.clone(),
                    session_key: Some("agent:main:roko".to_string()),
                    timeout,
                    auto_approve_permissions: !provider.require_confirmation,
                    resource_limits,
                    system_prompt: options.system_prompt.clone(),
                };
                let agent = OpenClawAcpAgent::new(config);
                Ok(Box::new(agent))
            }
            _ => {
                // Tier 2: CLI infer.
                let binary = match &transport {
                    ProviderTransport::Cli { command, .. } => {
                        let trimmed = command.trim();
                        if trimmed.is_empty() {
                            "openclaw"
                        } else {
                            trimmed
                        }
                    }
                    _ => "openclaw",
                };
                let mut config = OpenClawInferConfig {
                    binary: binary.into(),
                    timeout,
                    resource_limits,
                    system_prompt: options.system_prompt.clone(),
                    ..Default::default()
                };
                if !model.slug.is_empty() {
                    config.model_override = Some(model.slug.clone());
                }
                let agent = OpenClawInferAgent::new(config)
                    .map_err(|e| AgentCreationError::MissingConfig(e.to_string()))?;
                Ok(Box::new(agent))
            }
        }
    }

    fn classify_error(&self, status: u16, body: &Value) -> ProviderError {
        super::error_classify::classify_cli_error(status, body, "OpenClaw")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openclaw_adapter_kind() {
        let adapter = OpenClawProviderAdapter;
        assert_eq!(adapter.kind(), ProviderKind::OpenClaw);
    }

    #[test]
    fn openclaw_adapter_selects_infer_by_default() {
        let provider = ProviderConfig {
            kind: ProviderKind::OpenClaw,
            base_url: None,
            api_key_env: None,
            command: Some("openclaw".to_string()),
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
            provider: "openclaw".to_string(),
            slug: "openai/gpt-5.5".to_string(),
            ..Default::default()
        };
        let options = AgentOptions {
            name: "openclaw-infer".to_string(),
            ..Default::default()
        };
        let agent = OpenClawProviderAdapter
            .create_agent(&provider, &model, &options)
            .expect("create openclaw infer agent");
        assert_eq!(agent.backend_id(), "openclaw-infer");
    }

    #[test]
    fn openclaw_adapter_selects_acp_when_args_contain_acp() {
        let provider = ProviderConfig {
            kind: ProviderKind::OpenClaw,
            base_url: None,
            api_key_env: None,
            command: Some("openclaw".to_string()),
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
            provider: "openclaw".to_string(),
            slug: "openai/gpt-5.5".to_string(),
            ..Default::default()
        };
        let options = AgentOptions {
            name: "openclaw-acp".to_string(),
            ..Default::default()
        };
        let agent = OpenClawProviderAdapter
            .create_agent(&provider, &model, &options)
            .expect("create openclaw ACP agent");
        assert_eq!(agent.backend_id(), "openclaw-acp");
    }

    #[test]
    fn openclaw_adapter_rejects_wrong_kind() {
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
        let result = OpenClawProviderAdapter.create_agent(&provider, &model, &options);
        assert!(result.is_err());
    }

    #[test]
    fn openclaw_classify_error_rate_limit() {
        let err = OpenClawProviderAdapter.classify_error(429, &Value::Null);
        assert!(matches!(err, ProviderError::RateLimit { .. }));
    }

    #[test]
    fn openclaw_classify_error_auth() {
        let err = OpenClawProviderAdapter.classify_error(401, &Value::Null);
        assert!(matches!(err, ProviderError::AuthFailure));
    }

    #[test]
    fn openclaw_classify_error_stderr_model_not_found() {
        let body = Value::String("model not found: gpt-99".to_string());
        let err = OpenClawProviderAdapter.classify_error(0, &body);
        assert!(matches!(err, ProviderError::ModelNotFound));
    }
}
