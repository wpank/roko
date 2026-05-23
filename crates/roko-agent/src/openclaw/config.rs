//! Configuration types for OpenClaw adapters.
//!
//! Parsed from roko.toml `[providers.openclaw]` blocks.

use std::ffi::OsString;
use std::time::Duration;

/// Top-level OpenClaw configuration, dispatched by `transport` field.
#[derive(Clone, Debug)]
pub enum OpenClawConfig {
    Infer(OpenClawInferConfig),
    Acp(super::acp_agent::OpenClawAcpConfig),
}

/// Configuration for `OpenClawInferAgent` (Tier 2: `openclaw infer ... --json`).
///
/// # TOML mapping
///
/// ```toml
/// [providers.openclaw]
/// kind = "openclaw"
/// transport = "one-shot"
/// binary = "openclaw"
/// timeout_ms = 90000
///
/// [providers.openclaw.infer]
/// model_override = "openai/gpt-5.5"
/// provider_override = "openai"
/// thinking = "medium"
/// transport_hint = "local"
/// extra_args = ["--json"]
/// ```
#[derive(Clone, Debug)]
pub struct OpenClawInferConfig {
    /// Path or name of the `openclaw` binary. Default: `"openclaw"`.
    pub binary: OsString,

    /// Override model for every invocation (e.g. `"openai/gpt-5.5"`).
    /// Maps to `--model <value>`.
    pub model_override: Option<String>,

    /// Override provider for every invocation (e.g. `"openai"`).
    /// Maps to `--provider <value>`.
    pub provider_override: Option<String>,

    /// Thinking level: `"off"`, `"minimal"`, `"low"`, `"medium"`,
    /// `"high"`, `"adaptive"`, `"xhigh"`, `"max"`.
    /// Maps to `--thinking <value>`.
    pub thinking: Option<String>,

    /// Whether to force local or gateway routing.
    pub transport_hint: TransportHint,

    /// Extra CLI arguments appended to every invocation.
    /// `--json` should always be included.
    pub extra_args: Vec<String>,

    /// Timeout for the entire `openclaw infer` subprocess.
    /// After this duration, SIGTERM is sent, then SIGKILL after a
    /// 5-second grace period.
    pub timeout: Duration,
}

impl Default for OpenClawInferConfig {
    fn default() -> Self {
        Self {
            binary: "openclaw".into(),
            model_override: None,
            provider_override: None,
            thinking: None,
            transport_hint: TransportHint::Auto,
            extra_args: vec!["--json".to_string()],
            timeout: Duration::from_millis(90_000),
        }
    }
}

/// Routing hint for the `openclaw infer` subprocess.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TransportHint {
    /// `--local` -- never route through the OpenClaw gateway.
    Local,
    /// `--gateway` -- force gateway routing.
    Gateway,
    /// No flag -- let OpenClaw decide.
    #[default]
    Auto,
}

impl TransportHint {
    /// Returns the CLI flag for this hint, if any.
    pub fn as_flag(&self) -> Option<&'static str> {
        match self {
            Self::Local => Some("--local"),
            Self::Gateway => Some("--gateway"),
            Self::Auto => None,
        }
    }
}

/// Errors from OpenClaw config parsing.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("missing required field: {0}")]
    MissingField(&'static str),

    #[error("invalid transport value: {0} (expected \"one-shot\" or \"acp\")")]
    InvalidTransport(String),

    #[error("invalid thinking level: {0}")]
    InvalidThinking(String),

    #[error("invalid transport_hint: {0} (expected \"local\", \"gateway\", or \"auto\")")]
    InvalidTransportHint(String),

    #[error("invalid timeout_ms: {0}")]
    InvalidTimeout(String),
}

impl OpenClawInferConfig {
    /// Build argv for `openclaw infer model run`.
    ///
    /// Returns the arguments to pass after the binary name. The binary
    /// itself is not included -- `ChildProcessRunner` holds it.
    ///
    /// # Arguments
    ///
    /// * `prompt` -- the prompt text to send.
    ///
    /// # Returns
    ///
    /// A `Vec<String>` of CLI arguments in the correct order.
    pub fn build_argv(&self, prompt: &str) -> Vec<String> {
        let mut args = vec![
            "infer".to_string(),
            "model".to_string(),
            "run".to_string(),
            "--prompt".to_string(),
            prompt.to_string(),
        ];

        // --json is typically in extra_args, but ensure it's always present
        if !self.extra_args.contains(&"--json".to_string()) {
            args.push("--json".to_string());
        }

        if let Some(model) = &self.model_override {
            args.push("--model".to_string());
            args.push(model.clone());
        }

        if let Some(provider) = &self.provider_override {
            args.push("--provider".to_string());
            args.push(provider.clone());
        }

        if let Some(thinking) = &self.thinking {
            args.push("--thinking".to_string());
            args.push(thinking.clone());
        }

        if let Some(flag) = self.transport_hint.as_flag() {
            args.push(flag.to_string());
        }

        // Append extra_args (which may include --json and other flags)
        for arg in &self.extra_args {
            if !args.contains(arg) {
                args.push(arg.clone());
            }
        }

        args
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_json_flag() {
        let config = OpenClawInferConfig::default();
        assert!(config.extra_args.contains(&"--json".to_string()));
    }

    #[test]
    fn build_argv_basic() {
        let config = OpenClawInferConfig::default();
        let argv = config.build_argv("What is 2+2?");
        assert!(argv.contains(&"infer".to_string()));
        assert!(argv.contains(&"model".to_string()));
        assert!(argv.contains(&"run".to_string()));
        assert!(argv.contains(&"--prompt".to_string()));
        assert!(argv.contains(&"What is 2+2?".to_string()));
        assert!(argv.contains(&"--json".to_string()));
    }

    #[test]
    fn build_argv_with_overrides() {
        let config = OpenClawInferConfig {
            model_override: Some("openai/gpt-5.5".to_string()),
            provider_override: Some("openai".to_string()),
            thinking: Some("high".to_string()),
            transport_hint: TransportHint::Local,
            ..Default::default()
        };
        let argv = config.build_argv("hello");
        assert!(argv.contains(&"--model".to_string()));
        assert!(argv.contains(&"openai/gpt-5.5".to_string()));
        assert!(argv.contains(&"--provider".to_string()));
        assert!(argv.contains(&"openai".to_string()));
        assert!(argv.contains(&"--thinking".to_string()));
        assert!(argv.contains(&"high".to_string()));
        assert!(argv.contains(&"--local".to_string()));
    }

    #[test]
    fn transport_hint_flags() {
        assert_eq!(TransportHint::Local.as_flag(), Some("--local"));
        assert_eq!(TransportHint::Gateway.as_flag(), Some("--gateway"));
        assert_eq!(TransportHint::Auto.as_flag(), None);
    }

    #[test]
    fn default_timeout_is_90s() {
        let config = OpenClawInferConfig::default();
        assert_eq!(config.timeout, Duration::from_millis(90_000));
    }
}
