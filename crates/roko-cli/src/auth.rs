//! CLI-side auth helpers for communicating with `roko-serve`.
//!
//! Provides a single source of truth for API key resolution and header
//! construction so that every CLI path (chat, doctor, TUI) uses the same
//! logic.

use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};

/// Environment variable consulted when resolving the API key.
pub const ROKO_API_KEY_ENV: &str = "ROKO_API_KEY";

/// Which source the resolved API key came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiKeySource {
    /// Supplied via an explicit CLI flag (e.g. `--api-key`).
    CliFlag,
    /// Read from the `ROKO_API_KEY` environment variable.
    EnvVar,
    /// Read from `[serve.auth] api_key` in `roko.toml`.
    Config,
    /// Read from `~/.roko/credentials.json` (stored by `roko login`).
    StoredCredential,
}

impl ApiKeySource {
    /// Human-readable label for diagnostics output.
    #[allow(dead_code)]
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::CliFlag => "CLI flag (--api-key)",
            Self::EnvVar => "ROKO_API_KEY env var",
            Self::Config => "roko.toml [serve.auth]",
            Self::StoredCredential => "~/.roko/credentials.json (roko login)",
        }
    }
}

/// Result of [`resolve_api_key`]: the key value and where it came from.
#[derive(Debug, Clone)]
pub struct ResolvedApiKey {
    /// The API key value.
    pub key: String,
    /// Where the key was resolved from.
    #[allow(dead_code)]
    pub source: ApiKeySource,
    /// HTTP authentication method associated with the credential.
    pub method: AuthMethod,
}

/// How a resolved credential must be sent to `roko-serve`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMethod {
    /// Static API keys use the historical `X-Api-Key` header.
    ApiKey,
    /// Privy access tokens are JWT bearer credentials.
    Bearer,
}

impl ResolvedApiKey {
    /// Build the request headers appropriate for this credential.
    #[must_use]
    pub fn headers(&self) -> HeaderMap {
        auth_headers_with_method(&self.key, self.method)
    }
}

/// Resolve an API key using the standard precedence chain:
///
/// 1. Explicit CLI flag (`cli_override`)
/// 2. `ROKO_API_KEY` environment variable
/// 3. `config.serve.auth.api_key` from `roko.toml`
/// 4. Stored credential from `~/.roko/credentials.json` (`roko login`)
///
/// Returns `None` when no key is available from any source.
#[must_use]
pub fn resolve_api_key(
    config: &roko_core::config::ServeAuthConfig,
    cli_override: Option<&str>,
) -> Option<ResolvedApiKey> {
    let env_value = std::env::var(ROKO_API_KEY_ENV).ok();
    let stored_credential = crate::credentials::load_credential().ok().flatten();
    resolve_api_key_inner(
        config,
        cli_override,
        env_value.as_deref(),
        stored_credential
            .as_ref()
            .map(|credential| (credential.token.as_str(), credential.method.as_str())),
    )
}

/// Inner implementation that accepts the env-var value and stored credential
/// as parameters so tests can exercise the precedence chain without mutating
/// process state or touching the filesystem.
fn resolve_api_key_inner(
    config: &roko_core::config::ServeAuthConfig,
    cli_override: Option<&str>,
    env_value: Option<&str>,
    stored_credential: Option<(&str, &str)>,
) -> Option<ResolvedApiKey> {
    // 1. CLI flag takes highest precedence.
    if let Some(key) = cli_override {
        let key = key.trim();
        if !key.is_empty() {
            return Some(ResolvedApiKey {
                key: key.to_string(),
                source: ApiKeySource::CliFlag,
                method: AuthMethod::ApiKey,
            });
        }
    }

    // 2. Environment variable.
    if let Some(key) = env_value {
        let key = key.trim();
        if !key.is_empty() {
            return Some(ResolvedApiKey {
                key: key.to_string(),
                source: ApiKeySource::EnvVar,
                method: AuthMethod::ApiKey,
            });
        }
    }

    // 3. Config file.
    let key = config.api_key.trim();
    if !key.is_empty() {
        return Some(ResolvedApiKey {
            key: key.to_string(),
            source: ApiKeySource::Config,
            method: AuthMethod::ApiKey,
        });
    }

    // 4. Stored credential from `roko login`.
    if let Some((key, method)) = stored_credential {
        let key = key.trim();
        if !key.is_empty() {
            return Some(ResolvedApiKey {
                key: key.to_string(),
                source: ApiKeySource::StoredCredential,
                method: if method.eq_ignore_ascii_case("privy") {
                    AuthMethod::Bearer
                } else {
                    AuthMethod::ApiKey
                },
            });
        }
    }

    None
}

/// Build a [`HeaderMap`] containing the `X-Api-Key` header for a
/// `roko-serve` request.
///
/// Returns an empty map when `api_key` is empty so callers can always
/// merge the result into their request builder.
#[must_use]
pub fn auth_headers(api_key: &str) -> HeaderMap {
    auth_headers_with_method(api_key, AuthMethod::ApiKey)
}

/// Build authentication headers for a credential and its stored method.
#[must_use]
pub fn auth_headers_with_method(credential: &str, method: AuthMethod) -> HeaderMap {
    let mut headers = HeaderMap::new();
    if !credential.is_empty() {
        let value = match method {
            AuthMethod::ApiKey => HeaderValue::from_str(credential),
            AuthMethod::Bearer => HeaderValue::from_str(&format!("Bearer {credential}")),
        };
        if let Ok(value) = value {
            match method {
                AuthMethod::ApiKey => {
                    headers.insert("X-Api-Key", value);
                }
                AuthMethod::Bearer => {
                    headers.insert(AUTHORIZATION, value);
                }
            }
        }
    }
    headers
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::config::ServeAuthConfig;

    fn cfg(api_key: &str) -> ServeAuthConfig {
        ServeAuthConfig {
            enabled: true,
            api_key: api_key.into(),
            ..ServeAuthConfig::default()
        }
    }

    #[test]
    fn cli_flag_takes_precedence_over_env_and_config() {
        let resolved = resolve_api_key_inner(
            &cfg("from-config"),
            Some("from-cli"),
            Some("from-env"),
            Some(("from-stored", "privy")),
        )
        .expect("should resolve");
        assert_eq!(resolved.key, "from-cli");
        assert_eq!(resolved.source, ApiKeySource::CliFlag);
    }

    #[test]
    fn env_var_takes_precedence_over_config() {
        let resolved = resolve_api_key_inner(
            &cfg("from-config"),
            None,
            Some("from-env"),
            Some(("from-stored", "privy")),
        )
        .expect("should resolve");
        assert_eq!(resolved.key, "from-env");
        assert_eq!(resolved.source, ApiKeySource::EnvVar);
    }

    #[test]
    fn config_key_used_when_no_override() {
        let resolved = resolve_api_key_inner(
            &cfg("from-config"),
            None,
            None,
            Some(("from-stored", "privy")),
        )
        .expect("should resolve");
        assert_eq!(resolved.key, "from-config");
        assert_eq!(resolved.source, ApiKeySource::Config);
    }

    #[test]
    fn stored_credential_used_as_last_resort() {
        let resolved = resolve_api_key_inner(&cfg(""), None, None, Some(("from-stored", "privy")))
            .expect("should resolve");
        assert_eq!(resolved.key, "from-stored");
        assert_eq!(resolved.source, ApiKeySource::StoredCredential);
        assert_eq!(resolved.method, AuthMethod::Bearer);
    }

    #[test]
    fn returns_none_when_no_key_available() {
        assert!(resolve_api_key_inner(&cfg(""), None, None, None).is_none());
    }

    #[test]
    fn empty_cli_flag_falls_through_to_config() {
        let resolved = resolve_api_key_inner(&cfg("from-config"), Some("  "), None, None)
            .expect("should resolve");
        assert_eq!(resolved.key, "from-config");
        assert_eq!(resolved.source, ApiKeySource::Config);
    }

    #[test]
    fn whitespace_only_env_falls_through_to_config() {
        let resolved = resolve_api_key_inner(&cfg("from-config"), None, Some("  "), None)
            .expect("should resolve");
        assert_eq!(resolved.key, "from-config");
        assert_eq!(resolved.source, ApiKeySource::Config);
    }

    #[test]
    fn whitespace_stored_credential_falls_through_to_none() {
        assert!(resolve_api_key_inner(&cfg(""), None, None, Some(("  ", "privy"))).is_none());
    }

    #[test]
    fn auth_headers_builds_x_api_key() {
        let headers = auth_headers("test-key");
        assert_eq!(
            headers.get("X-Api-Key").unwrap().to_str().unwrap(),
            "test-key"
        );
    }

    #[test]
    fn auth_headers_empty_for_empty_key() {
        let headers = auth_headers("");
        assert!(headers.is_empty());
    }

    #[test]
    fn stored_privy_credential_builds_bearer_header() {
        let resolved = resolve_api_key_inner(&cfg(""), None, None, Some(("jwt-token", "privy")))
            .expect("should resolve");
        let headers = resolved.headers();
        assert_eq!(
            headers.get(AUTHORIZATION).unwrap().to_str().unwrap(),
            "Bearer jwt-token"
        );
        assert!(!headers.contains_key("X-Api-Key"));
    }

    #[test]
    fn stored_api_key_credential_builds_x_api_key_header() {
        let resolved = resolve_api_key_inner(&cfg(""), None, None, Some(("stored-key", "api_key")))
            .expect("should resolve");
        let headers = resolved.headers();
        assert_eq!(
            headers.get("X-Api-Key").unwrap().to_str().unwrap(),
            "stored-key"
        );
        assert!(!headers.contains_key(AUTHORIZATION));
    }

    #[test]
    fn source_labels_are_descriptive() {
        assert!(!ApiKeySource::CliFlag.label().is_empty());
        assert!(!ApiKeySource::EnvVar.label().is_empty());
        assert!(!ApiKeySource::Config.label().is_empty());
    }
}
