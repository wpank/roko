use crate::Agent;
use crate::openclaw::{
    OpenClawAcpAgent, OpenClawAcpConfig, OpenClawInferAgent, OpenClawInferConfig,
};
use crate::provider::{AgentCreationError, AgentOptions, ProviderAdapter, ProviderError};
use roko_core::agent::ProviderKind;
use roko_core::config::schema::{ModelProfile, ProviderConfig};
use roko_core::defaults::DEFAULT_REQUEST_TIMEOUT_MS;
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

        let timeout_ms = options
            .timeout_ms
            .or(provider.timeout_ms)
            .unwrap_or(DEFAULT_REQUEST_TIMEOUT_MS);
        let timeout = Duration::from_millis(timeout_ms);

        let working_dir = options
            .working_dir
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let binary = provider
            .command
            .as_deref()
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .unwrap_or("openclaw");

        let is_acp = provider
            .args
            .as_ref()
            .is_some_and(|args| args.iter().any(|a| a == "acp"));

        if is_acp {
            // Tier 3: ACP over stdio.
            let config = OpenClawAcpConfig {
                binary: binary.to_string(),
                cwd: working_dir,
                gateway_url: provider.base_url.clone(),
                session_key: Some("agent:main:roko".to_string()),
                timeout,
                auto_approve_permissions: true,
            };
            let agent = OpenClawAcpAgent::new(config);
            Ok(Box::new(agent))
        } else {
            // Tier 2: CLI infer.
            let mut config = OpenClawInferConfig {
                binary: binary.into(),
                timeout,
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
                    ProviderError::Other(format!("OpenClaw exit status {status}"))
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
