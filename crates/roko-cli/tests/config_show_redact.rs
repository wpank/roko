//! Regression tests: `roko config show --effective` must not emit literal secrets.
//!
//! These tests exercise the `serialize_effective_redacted` path in
//! `roko_core::config::loader` which is called by `cmd_show_effective`.

use roko_core::config::loader::{redact_secrets_in_toml, serialize_effective_redacted};
use roko_core::config::schema::RokoConfig;

/// Build a `RokoConfig` that contains at least one non-empty value in every
/// known secret field, so we can assert each one is masked in the output.
fn config_with_all_secrets() -> RokoConfig {
    let toml = r#"
schema_version = 2

[serve.auth]
enabled = true
api_key = "roko-secret-api-key"

[server]
auth_token = "bearer-secret-token"

[deploy]
railway_api_token = "railway-live-token"

[chain]
wallet_key = "0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"

[webhooks.github]
secret = "github-webhook-secret"
"#;
    RokoConfig::from_toml(toml).expect("parse test config")
}

#[test]
fn config_show_effective_redacts_secret() {
    let config = config_with_all_secrets();
    let output =
        serialize_effective_redacted(&config).expect("serialize_effective_redacted must not fail");

    // No literal secret values must appear in the output.
    assert!(
        !output.contains("roko-secret-api-key"),
        "serve.auth.api_key must be redacted; got:\n{output}"
    );
    assert!(
        !output.contains("bearer-secret-token"),
        "server.auth_token must be redacted; got:\n{output}"
    );
    assert!(
        !output.contains("railway-live-token"),
        "deploy.railway_api_token must be redacted; got:\n{output}"
    );
    assert!(
        !output.contains("0xdeadbeef"),
        "chain.wallet_key must be redacted; got:\n{output}"
    );
    assert!(
        !output.contains("github-webhook-secret"),
        "webhooks.github.secret must be redacted; got:\n{output}"
    );

    // The mask placeholder must appear for non-empty fields.
    assert!(
        output.contains("****"),
        "redacted fields must show '****'; got:\n{output}"
    );
}

#[test]
fn config_show_effective_preserves_non_secret_fields() {
    let toml = r#"
schema_version = 2

[agent]
default_model = "claude-sonnet-4-6"

[serve.auth]
enabled = true
api_key = "some-secret"
"#;
    let config = RokoConfig::from_toml(toml).expect("parse");
    let output = serialize_effective_redacted(&config).expect("serialize");

    // Non-secret fields must be preserved.
    assert!(
        output.contains("claude-sonnet-4-6"),
        "model slug must not be redacted; got:\n{output}"
    );
    // Secret must be redacted.
    assert!(
        !output.contains("some-secret"),
        "api_key must be redacted; got:\n{output}"
    );
}

#[test]
fn redact_secrets_in_toml_leaves_empty_fields_unchanged() {
    // An empty api_key should not be replaced with **** because it conveys
    // "not configured" rather than a real secret.
    let toml_str = r#"[serve.auth]
enabled = true
api_key = ""
"#;
    let output = redact_secrets_in_toml(toml_str);
    // The empty string should remain empty (or omitted) — NOT become "****".
    assert!(
        !output.contains("****"),
        "empty secret fields must not be masked; got:\n{output}"
    );
}

#[test]
fn redact_secrets_in_toml_does_not_redact_api_key_env() {
    // api_key_env is the *name* of an environment variable, not a secret value.
    // It must never be redacted.
    let toml_str = r#"[providers.anthropic]
kind = "anthropic_api"
api_key_env = "ANTHROPIC_API_KEY"
"#;
    let output = redact_secrets_in_toml(toml_str);
    assert!(
        output.contains("ANTHROPIC_API_KEY"),
        "api_key_env (env var name) must not be redacted; got:\n{output}"
    );
}
