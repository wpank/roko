//! `roko.toml` -> Hermes adapter config.
//!
//! Parses the `[providers.hermes]` block from roko configuration into
//! typed config structs.

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Deserializer, Serialize};

/// Configuration for the Hermes harness adapter.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HermesConfig {
    /// Hermes binary path (default: "hermes").
    #[serde(default = "default_binary")]
    pub binary: String,

    /// Gateway HTTP endpoint (default: "http://localhost:8642").
    #[serde(default = "default_endpoint")]
    pub endpoint: String,

    /// Default model for one-shot/ACP modes.
    #[serde(default)]
    pub model: Option<String>,

    /// API key env var name (reads from env at runtime).
    #[serde(default)]
    pub api_key_env: Option<String>,

    /// State directory for crash recovery, session persistence.
    #[serde(default)]
    pub state_dir: Option<PathBuf>,

    /// Request timeout.
    #[serde(
        default = "default_timeout_ms",
        serialize_with = "serialize_duration_ms",
        deserialize_with = "deserialize_duration_ms"
    )]
    pub timeout: Duration,

    /// Gateway port (default: 8642).
    #[serde(default = "default_gateway_port")]
    pub gateway_port: u16,

    /// Whether to auto-start the gateway service.
    #[serde(default)]
    pub auto_start_gateway: bool,

    /// Crash recovery settings.
    #[serde(default)]
    pub crash_recovery: CrashRecoveryConfig,
}

/// Crash recovery configuration for the Hermes gateway.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrashRecoveryConfig {
    /// Enable automatic crash recovery (restart + retry).
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Maximum restarts before giving up.
    #[serde(default = "default_max_restarts")]
    pub max_restarts: u32,

    /// Time to wait for gateway to become healthy after restart.
    #[serde(
        default = "default_health_timeout_ms",
        serialize_with = "serialize_duration_ms",
        deserialize_with = "deserialize_duration_ms"
    )]
    pub health_timeout: Duration,
}

// ---- Default functions --------------------------------------------------------

fn default_binary() -> String {
    "hermes".to_string()
}

fn default_endpoint() -> String {
    "http://localhost:8642".to_string()
}

fn default_timeout_ms() -> Duration {
    Duration::from_millis(90_000)
}

fn default_gateway_port() -> u16 {
    8642
}

fn default_true() -> bool {
    true
}

fn default_max_restarts() -> u32 {
    3
}

fn default_health_timeout_ms() -> Duration {
    Duration::from_millis(30_000)
}

// ---- Serde helpers ------------------------------------------------------------

/// Serialize a `Duration` as a millisecond integer.
fn serialize_duration_ms<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u64(duration.as_millis() as u64)
}

/// Deserialize a `Duration` from a millisecond integer.
fn deserialize_duration_ms<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let ms = u64::deserialize(deserializer)?;
    Ok(Duration::from_millis(ms))
}

// ---- Default impls ------------------------------------------------------------

impl Default for HermesConfig {
    fn default() -> Self {
        Self {
            binary: default_binary(),
            endpoint: default_endpoint(),
            model: None,
            api_key_env: None,
            state_dir: None,
            timeout: default_timeout_ms(),
            gateway_port: default_gateway_port(),
            auto_start_gateway: false,
            crash_recovery: CrashRecoveryConfig::default(),
        }
    }
}

impl Default for CrashRecoveryConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            max_restarts: default_max_restarts(),
            health_timeout: default_health_timeout_ms(),
        }
    }
}

impl HermesConfig {
    /// Construct a `HermesConfig` from a [`roko_core::config::ProviderConfig`].
    ///
    /// Maps `ProviderConfig` fields to the Hermes-specific config:
    /// - `base_url` -> `endpoint`
    /// - `command` -> `binary`
    /// - `api_key_env` -> `api_key_env`
    /// - `timeout_ms` -> `timeout`
    pub fn from_provider_config(provider: &roko_core::config::ProviderConfig) -> Self {
        let mut cfg = Self::default();

        if let Some(ref url) = provider.base_url {
            cfg.endpoint = url.clone();
        }
        if let Some(ref cmd) = provider.command {
            cfg.binary = cmd.clone();
        }
        if let Some(ref key_env) = provider.api_key_env {
            cfg.api_key_env = Some(key_env.clone());
        }
        if let Some(timeout_ms) = provider.timeout_ms {
            cfg.timeout = Duration::from_millis(timeout_ms);
        }

        cfg
    }

    /// Resolve the API key from the environment.
    ///
    /// Returns `None` if no `api_key_env` is configured or the env var is
    /// unset.
    #[must_use]
    pub fn resolve_api_key(&self) -> Option<String> {
        self.api_key_env
            .as_ref()
            .and_then(|env_var| std::env::var(env_var).ok())
    }

    /// Return the effective state directory, falling back to
    /// `.roko/state/hermes/`.
    #[must_use]
    pub fn effective_state_dir(&self) -> PathBuf {
        self.state_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from(".roko").join("state").join("hermes"))
    }

    /// Build the full gateway URL (endpoint + port).
    #[must_use]
    pub fn gateway_url(&self) -> String {
        // If the endpoint already contains a port, return as-is.
        // Otherwise, append the configured port.
        if self.endpoint.contains(&format!(":{}", self.gateway_port)) {
            self.endpoint.clone()
        } else {
            format!(
                "{}:{}",
                self.endpoint.trim_end_matches('/'),
                self.gateway_port
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_values() {
        let cfg = HermesConfig::default();
        assert_eq!(cfg.binary, "hermes");
        assert_eq!(cfg.endpoint, "http://localhost:8642");
        assert_eq!(cfg.model, None);
        assert_eq!(cfg.api_key_env, None);
        assert_eq!(cfg.state_dir, None);
        assert_eq!(cfg.timeout, Duration::from_millis(90_000));
        assert_eq!(cfg.gateway_port, 8642);
        assert!(!cfg.auto_start_gateway);
    }

    #[test]
    fn default_crash_recovery_config() {
        let cfg = CrashRecoveryConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.max_restarts, 3);
        assert_eq!(cfg.health_timeout, Duration::from_millis(30_000));
    }

    #[test]
    fn serde_roundtrip() {
        let cfg = HermesConfig {
            binary: "custom-hermes".to_string(),
            endpoint: "http://10.0.0.1:9999".to_string(),
            model: Some("gpt-4".to_string()),
            api_key_env: Some("MY_KEY".to_string()),
            state_dir: Some(PathBuf::from("/tmp/hermes-state")),
            timeout: Duration::from_millis(60_000),
            gateway_port: 9999,
            auto_start_gateway: true,
            crash_recovery: CrashRecoveryConfig {
                enabled: false,
                max_restarts: 5,
                health_timeout: Duration::from_millis(10_000),
            },
        };

        let json = serde_json::to_string(&cfg).expect("serialize");
        let back: HermesConfig = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(back.binary, "custom-hermes");
        assert_eq!(back.endpoint, "http://10.0.0.1:9999");
        assert_eq!(back.model, Some("gpt-4".to_string()));
        assert_eq!(back.api_key_env, Some("MY_KEY".to_string()));
        assert_eq!(back.gateway_port, 9999);
        assert!(back.auto_start_gateway);
        assert!(!back.crash_recovery.enabled);
        assert_eq!(back.crash_recovery.max_restarts, 5);
    }

    /// Helper to build a minimal `ProviderConfig` for tests.
    fn test_provider_config(
        base_url: Option<String>,
        api_key_env: Option<String>,
        command: Option<String>,
        timeout_ms: Option<u64>,
    ) -> roko_core::config::ProviderConfig {
        roko_core::config::ProviderConfig {
            kind: roko_core::agent::ProviderKind::Hermes,
            base_url,
            api_key_env,
            command,
            args: None,
            timeout_ms,
            ttft_timeout_ms: None,
            connect_timeout_ms: None,
            extra_headers: None,
            max_concurrent: None,
        }
    }

    #[test]
    fn from_provider_config_maps_fields() {
        let provider = test_provider_config(
            Some("http://hermes.local:8642".to_string()),
            Some("HERMES_API_KEY".to_string()),
            Some("/usr/local/bin/hermes".to_string()),
            Some(120_000),
        );

        let cfg = HermesConfig::from_provider_config(&provider);
        assert_eq!(cfg.endpoint, "http://hermes.local:8642");
        assert_eq!(cfg.binary, "/usr/local/bin/hermes");
        assert_eq!(cfg.api_key_env, Some("HERMES_API_KEY".to_string()));
        assert_eq!(cfg.timeout, Duration::from_millis(120_000));
    }

    #[test]
    fn from_provider_config_uses_defaults_for_missing_fields() {
        let provider = test_provider_config(None, None, None, None);

        let cfg = HermesConfig::from_provider_config(&provider);
        assert_eq!(cfg.binary, "hermes");
        assert_eq!(cfg.endpoint, "http://localhost:8642");
        assert_eq!(cfg.api_key_env, None);
        assert_eq!(cfg.timeout, Duration::from_millis(90_000));
    }

    #[test]
    fn resolve_api_key_returns_none_when_no_env_var_configured() {
        let cfg = HermesConfig::default();
        assert!(cfg.resolve_api_key().is_none());
    }

    #[test]
    fn resolve_api_key_returns_none_for_unset_env_var() {
        let cfg = HermesConfig {
            api_key_env: Some("ROKO_TEST_HERMES_NONEXISTENT_KEY_12345".to_string()),
            ..Default::default()
        };
        assert!(cfg.resolve_api_key().is_none());
    }

    #[test]
    fn effective_state_dir_fallback() {
        let cfg = HermesConfig::default();
        let dir = cfg.effective_state_dir();
        assert!(dir.ends_with("state/hermes"));
    }

    #[test]
    fn effective_state_dir_override() {
        let cfg = HermesConfig {
            state_dir: Some(PathBuf::from("/custom/state")),
            ..Default::default()
        };
        assert_eq!(cfg.effective_state_dir(), PathBuf::from("/custom/state"));
    }

    #[test]
    fn gateway_url_with_default_port() {
        let cfg = HermesConfig::default();
        // Default endpoint already contains :8642
        let url = cfg.gateway_url();
        assert!(url.contains("8642"));
    }
}
