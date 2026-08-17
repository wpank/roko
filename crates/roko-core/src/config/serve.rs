//! HTTP serving, server, deploy, scheduler, and webhook configuration sections.

use serde::{Deserialize, Serialize};

use super::agent::default_true;

// ---- [statehub] ----------------------------------------------------------

/// StateHub projection persistence and retention settings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateHubConfig {
    /// Maximum age of retained projection versions (`ms`, `s`, `m`, `h`, or `d`).
    #[serde(default = "default_projection_history_retention")]
    pub history_retention: String,
}

impl StateHubConfig {
    /// Parse the configured projection-history age window.
    pub fn history_retention_duration(&self) -> Result<std::time::Duration, String> {
        parse_duration(&self.history_retention)
    }
}

impl Default for StateHubConfig {
    fn default() -> Self {
        Self {
            history_retention: default_projection_history_retention(),
        }
    }
}

fn default_projection_history_retention() -> String {
    "7d".to_string()
}

fn parse_duration(value: &str) -> Result<std::time::Duration, String> {
    let value = value.trim();
    let split = value
        .find(|character: char| !character.is_ascii_digit())
        .ok_or_else(|| "duration requires one of: ms, s, m, h, d".to_string())?;
    let (amount, unit) = value.split_at(split);
    let amount = amount
        .parse::<u64>()
        .map_err(|error| format!("duration must start with a positive integer: {error}"))?;
    if amount == 0 {
        return Err("duration must be greater than zero".to_string());
    }
    let multiplier = match unit {
        "ms" => 1,
        "s" => 1_000,
        "m" => 60_000,
        "h" => 3_600_000,
        "d" => 86_400_000,
        _ => return Err("duration unit must be one of: ms, s, m, h, d".to_string()),
    };
    amount
        .checked_mul(multiplier)
        .map(std::time::Duration::from_millis)
        .ok_or_else(|| "duration is too large".to_string())
}

// ---- [serve] -------------------------------------------------------------

/// API serving options.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServeConfig {
    /// Port override for `roko serve`. Falls back to `server.port` (default 6677).
    #[serde(default)]
    pub port: Option<u16>,
    /// Shared transcript retention period in days.
    ///
    /// Newly created shares expire after this many days unless they are
    /// created with `--no-expire`.
    #[serde(default = "default_share_ttl_days")]
    pub share_ttl_days: u64,
    /// Whether to expose the PTY terminal routes.
    ///
    /// Disabled by default because the terminal is shell access.
    #[serde(default)]
    pub terminal_enabled: bool,
    /// Automatically orchestrate follow-up work when publish events arrive.
    #[serde(default = "default_true")]
    pub auto_orchestrate: bool,
    /// Authentication settings for `/api/*`.
    #[serde(default)]
    pub auth: ServeAuthConfig,
    /// Cloud deployment settings.
    #[serde(default)]
    pub deploy: ServeDeployConfig,
    /// Whether `roko` with no subcommand should auto-start the HTTP server.
    ///
    /// Disabled by default so the control plane is opt-in.
    #[serde(default)]
    pub auto_start: bool,
    /// Set to `true` to acknowledge the risk of a public bind without auth.
    ///
    /// Required when binding to a non-loopback address with `auth.enabled = false`.
    #[serde(default)]
    pub acknowledge_public_risk: bool,
    /// Additional remote IPs or CIDR ranges allowed to POST RuntimeEvent ingest without API auth.
    ///
    /// Loopback callers are always allowed. Public callers should normally use
    /// `serve.auth.enabled = true` instead of this allowlist.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub event_ingest_allowlist: Vec<String>,
    /// Optional OTLP tracing export. Disabled when `otlp_endpoint` is absent.
    #[serde(default)]
    pub tracing: TracingConfig,
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self {
            port: None,
            share_ttl_days: default_share_ttl_days(),
            terminal_enabled: false,
            auto_orchestrate: true,
            auth: ServeAuthConfig::default(),
            deploy: ServeDeployConfig::default(),
            auto_start: false,
            acknowledge_public_risk: false,
            event_ingest_allowlist: Vec::new(),
            tracing: TracingConfig::default(),
        }
    }
}

fn default_share_ttl_days() -> u64 {
    7
}

/// Enforcement behaviour for scope-based permission checks.
///
/// Controls whether the auth middleware blocks requests that fail scope checks
/// or merely logs the violation and allows the request through.
///
/// Configurable via `serve.auth.enforcement_mode` in `roko.toml`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnforcementMode {
    /// Block requests that fail scope checks (return 403). This is the default.
    #[default]
    Enforce,
    /// Log the violation but allow the request through (audit-only mode).
    Audit,
    /// Skip scope checks entirely — no logging, no blocking.
    Disabled,
}

/// A JSON Web Key Set endpoint and the issuer bound to its keys.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JwksProvider {
    /// Provider JWKS endpoint.
    pub url: String,
    /// Exact JWT `iss` value accepted for keys returned by this endpoint.
    pub expected_issuer: String,
}

impl JwksProvider {
    /// Construct a provider definition.
    pub fn new(url: impl Into<String>, expected_issuer: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            expected_issuer: expected_issuer.into(),
        }
    }
}

/// Authentication settings for the HTTP API.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServeAuthConfig {
    /// Whether `/api/*` routes require an `X-Api-Key` header.
    #[serde(default)]
    pub enabled: bool,
    /// Shared API key expected in `X-Api-Key` (legacy single-key mode).
    #[serde(default)]
    pub api_key: String,
    /// Named API keys with scoped permissions (hashes stored in `.roko/api-keys.json`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub api_keys: Vec<ApiKeyEntry>,
    /// Privy application ID for JWT validation (Phase 1b -- stub only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privy_app_id: Option<String>,
    /// Additional issuer-bound JWKS endpoints. An empty list uses Privy's
    /// built-in endpoint for backwards compatibility.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub jwks_providers: Vec<JwksProvider>,
    /// Privy workspace / org ID that the JWT `org_id` claim must match.
    ///
    /// When set, only tokens whose `org_id` claim equals this value are
    /// granted admin scope. When `None`, membership checks are skipped and
    /// a valid signature + app-id is sufficient (legacy behaviour).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privy_workspace_id: Option<String>,
    /// Allowed Privy roles (matched against the JWT `role` claim).
    ///
    /// When non-empty, only tokens whose `role` claim is in this list receive
    /// admin scope; others are downgraded to `"read"`. An empty list disables
    /// role filtering.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub privy_allowed_roles: Vec<String>,
    /// Enforcement mode for scope-based permission checks.
    ///
    /// - `enforce` (default): block requests that fail scope checks (403).
    /// - `audit`: log the violation but allow the request through.
    /// - `disabled`: skip scope checks entirely.
    #[serde(default)]
    pub enforcement_mode: EnforcementMode,
    /// Lifetime of a workspace invitation before it is rejected and cleaned up.
    #[serde(default = "default_invite_expiry_days")]
    pub invite_expiry_days: u64,
}

impl Default for ServeAuthConfig {
    fn default() -> Self {
        Self {
            // Secure-by-default: `/api/*` requires an `X-Api-Key`. Local users
            // can opt back out via `serve.auth.enabled = false` in `roko.toml`,
            // which is what `roko init` writes for new workspaces.
            enabled: true,
            api_key: String::new(),
            api_keys: Vec::new(),
            privy_app_id: None,
            jwks_providers: Vec::new(),
            privy_workspace_id: None,
            privy_allowed_roles: Vec::new(),
            enforcement_mode: EnforcementMode::default(),
            invite_expiry_days: default_invite_expiry_days(),
        }
    }
}

fn default_invite_expiry_days() -> u64 {
    7
}

/// A named API key entry with scoped permissions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiKeyEntry {
    /// Human-readable name (e.g. "github-actions", "cli-default").
    pub name: String,
    /// SHA-256 hash of the plaintext key (hex-encoded).
    pub key_hash: String,
    /// Permission scope: "admin", "agent:write", "read", etc.
    #[serde(default = "default_api_key_scope")]
    pub scope: String,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
    /// Optional ISO 8601 expiry timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    /// ISO 8601 timestamp of the last successful use of this key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<String>,
    /// Previous key hashes retained for a 5-minute grace period after rotation.
    /// Each entry is (hash, grace_expires_at_rfc3339).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub previous_key_hashes: Vec<(String, String)>,
}

fn default_api_key_scope() -> String {
    "admin".into()
}

/// Cloud deployment settings attached to the API server configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServeDeployConfig {
    /// Deployment provider, e.g. `"railway"` or `"fly"`.
    #[serde(default = "default_serve_deploy_provider")]
    pub provider: String,
    /// Environment variables that must be present for deployment.
    #[serde(default = "default_serve_deploy_environment")]
    pub environment: Vec<String>,
    /// Webhooks that should be registered after deploy.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub webhooks: Vec<ServeDeployWebhookConfig>,
}

fn default_serve_deploy_provider() -> String {
    "railway".into()
}

fn default_serve_deploy_environment() -> Vec<String> {
    vec![
        "GITHUB_TOKEN".into(),
        "GITHUB_WEBHOOK_SECRET".into(),
        "SLACK_BOT_TOKEN".into(),
        "SLACK_SIGNING_SECRET".into(),
    ]
}

impl Default for ServeDeployConfig {
    fn default() -> Self {
        Self {
            provider: default_serve_deploy_provider(),
            environment: default_serve_deploy_environment(),
            webhooks: Vec::new(),
        }
    }
}

/// A webhook registration entry to run after deployment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServeDeployWebhookConfig {
    /// Webhook provider.
    #[serde(default = "default_serve_deploy_webhook_provider")]
    pub provider: String,
    /// Repository owner.
    #[serde(default)]
    pub owner: String,
    /// Repository name.
    #[serde(default)]
    pub repo: String,
}

fn default_serve_deploy_webhook_provider() -> String {
    "github".into()
}

impl Default for ServeDeployWebhookConfig {
    fn default() -> Self {
        Self {
            provider: default_serve_deploy_webhook_provider(),
            owner: String::new(),
            repo: String::new(),
        }
    }
}

// ---- [serve.tracing] -----------------------------------------------------

/// Optional distributed tracing export configuration.
///
/// Parsed from `[serve.tracing]` in `roko.toml`. When `otlp_endpoint` is absent,
/// tracing export is disabled and no OTLP dependencies are loaded at runtime.
///
/// ```toml
/// [serve.tracing]
/// otlp_endpoint = "http://localhost:4317"
/// service_name = "roko-serve-dev"
/// sample_rate = 0.1
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TracingConfig {
    /// OTLP gRPC endpoint for trace export (e.g. `"http://localhost:4317"`).
    /// When absent, tracing export is disabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub otlp_endpoint: Option<String>,
    /// Service name reported in OTLP spans. Defaults to `"roko-serve"`.
    #[serde(default = "default_tracing_service_name")]
    pub service_name: String,
    /// Sample rate 0.0--1.0. Default 1.0 (trace everything).
    #[serde(default = "default_tracing_sample_rate")]
    pub sample_rate: f64,
}

fn default_tracing_service_name() -> String {
    "roko-serve".to_string()
}

fn default_tracing_sample_rate() -> f64 {
    1.0
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            otlp_endpoint: None,
            service_name: default_tracing_service_name(),
            sample_rate: default_tracing_sample_rate(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_share_ttl_days_is_seven() {
        assert_eq!(ServeConfig::default().share_ttl_days, 7);
    }

    #[test]
    fn default_acknowledge_public_risk_is_false() {
        assert!(!ServeConfig::default().acknowledge_public_risk);
    }

    #[test]
    fn default_auto_start_is_false() {
        assert!(!ServeConfig::default().auto_start);
    }

    #[test]
    fn default_auth_is_enabled() {
        assert!(ServeAuthConfig::default().enabled);
    }

    #[test]
    fn invite_expiry_defaults_to_seven_days_and_is_configurable() {
        assert_eq!(ServeAuthConfig::default().invite_expiry_days, 7);

        let cfg: ServeConfig =
            toml::from_str("[auth]\ninvite_expiry_days = 14\n").expect("parse serve auth config");
        assert_eq!(cfg.auth.invite_expiry_days, 14);
    }

    #[test]
    fn jwks_providers_parse_from_auth_config() {
        let cfg: ServeConfig = toml::from_str(
            r#"
[auth]
[[auth.jwks_providers]]
url = "https://identity.example/.well-known/jwks.json"
expected_issuer = "https://identity.example"
"#,
        )
        .expect("parse JWKS provider config");
        assert_eq!(cfg.auth.jwks_providers.len(), 1);
        assert_eq!(
            cfg.auth.jwks_providers[0],
            JwksProvider::new(
                "https://identity.example/.well-known/jwks.json",
                "https://identity.example"
            )
        );
    }

    #[test]
    fn tracing_config_defaults_are_disabled() {
        let tc = TracingConfig::default();
        assert!(tc.otlp_endpoint.is_none());
        assert_eq!(tc.service_name, "roko-serve");
        assert!((tc.sample_rate - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn serve_config_tracing_absent_uses_defaults() {
        let toml_text = "port = 8080\n";
        let cfg: ServeConfig = toml::from_str(toml_text).expect("parse serve config");
        assert!(cfg.tracing.otlp_endpoint.is_none());
        assert_eq!(cfg.tracing.service_name, "roko-serve");
        assert!((cfg.tracing.sample_rate - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn serve_config_tracing_parses_full_block() {
        let toml_text = r#"
port = 6677

[tracing]
otlp_endpoint = "http://localhost:4317"
service_name = "roko-serve-dev"
sample_rate = 0.1
"#;
        let cfg: ServeConfig = toml::from_str(toml_text).expect("parse serve config");
        assert_eq!(
            cfg.tracing.otlp_endpoint.as_deref(),
            Some("http://localhost:4317")
        );
        assert_eq!(cfg.tracing.service_name, "roko-serve-dev");
        assert!((cfg.tracing.sample_rate - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn serve_config_tracing_sample_rate_defaults_to_one() {
        let toml_text = r#"
[tracing]
otlp_endpoint = "http://otel:4317"
"#;
        let cfg: ServeConfig = toml::from_str(toml_text).expect("parse serve config");
        assert!((cfg.tracing.sample_rate - 1.0).abs() < f64::EPSILON);
        assert_eq!(cfg.tracing.service_name, "roko-serve");
    }
}

// ---- [server] ------------------------------------------------------------

/// HTTP server / gateway settings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Address to bind to.
    #[serde(default = "default_bind")]
    pub bind: String,
    /// Port number.
    #[serde(default = "default_port")]
    pub port: u16,
    /// Allowed CORS origins. Empty = local-only.
    #[serde(default)]
    pub cors_origins: Vec<String>,
    /// Optional bearer token for API authentication.
    #[serde(default)]
    pub auth_token: Option<String>,
    /// Allow all origins when `cors_origins` is empty.
    #[serde(default)]
    pub unsafe_public_cors: bool,
    /// Workspace GC interval in seconds. Defaults to 300 (5 minutes).
    #[serde(default = "default_workspace_gc_interval_secs")]
    pub workspace_gc_interval_secs: u64,
}

fn default_bind() -> String {
    "127.0.0.1".into()
}

fn default_workspace_gc_interval_secs() -> u64 {
    crate::defaults::DEFAULT_WORKSPACE_GC_INTERVAL_SECS
}

const fn default_port() -> u16 {
    6677
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            port: default_port(),
            cors_origins: Vec::new(),
            auth_token: None,
            unsafe_public_cors: false,
            workspace_gc_interval_secs: default_workspace_gc_interval_secs(),
        }
    }
}

// ---- [deploy] ------------------------------------------------------------

/// Cloud deployment configuration.
///
/// ```toml
/// [deploy]
/// backend = "railway-api"
/// railway_api_token = "..."
/// project_id = "..."
/// environment_id = "..."
/// worker_image = "ghcr.io/example/roko-worker:latest"
/// default_region = "us-west1"
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeployConfig {
    /// Which deploy backend to use: `"railway-api"`, `"railway-cli"`, `"manual"`.
    #[serde(default = "default_deploy_backend")]
    pub backend: String,

    /// Railway API token (for the `railway-api` backend).
    #[serde(default)]
    pub railway_api_token: Option<String>,

    /// Railway project ID.
    #[serde(default)]
    pub project_id: Option<String>,

    /// Railway environment ID.
    #[serde(default)]
    pub environment_id: Option<String>,

    /// Docker image for worker containers.
    #[serde(default)]
    pub worker_image: Option<String>,

    /// Default region for deployments.
    #[serde(default)]
    pub default_region: Option<String>,
}

fn default_deploy_backend() -> String {
    "manual".into()
}

impl Default for DeployConfig {
    fn default() -> Self {
        Self {
            backend: default_deploy_backend(),
            railway_api_token: None,
            project_id: None,
            environment_id: None,
            worker_image: Some("ghcr.io/nunchi-trade/roko-worker:latest".into()),
            default_region: None,
        }
    }
}

// ---- [scheduler] ---------------------------------------------------------

/// Cron scheduler configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// Cron jobs configured at startup.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cron: Vec<SchedulerCronConfig>,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self { cron: Vec::new() }
    }
}

impl SchedulerConfig {
    /// Returns `true` when no cron jobs are configured.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cron.is_empty()
    }
}

/// One cron job configuration entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerCronConfig {
    /// Human-readable schedule name.
    pub name: String,
    /// Standard cron expression.
    pub expression: String,
    /// Signal kind emitted when the schedule fires.
    pub signal_kind: String,
    /// Extra structured metadata included in the emitted signal body.
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl Default for SchedulerCronConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            expression: String::new(),
            signal_kind: String::new(),
            metadata: serde_json::Value::Null,
        }
    }
}

// ---- [webhooks] ----------------------------------------------------------

/// Webhook ingress configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhooksConfig {
    /// GitHub webhook configuration.
    #[serde(default)]
    pub github: GithubWebhookConfig,
}

impl Default for WebhooksConfig {
    fn default() -> Self {
        Self {
            github: GithubWebhookConfig::default(),
        }
    }
}

/// GitHub webhook configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubWebhookConfig {
    /// Shared secret used to verify `X-Hub-Signature-256`.
    #[serde(default)]
    pub secret: String,
}

impl Default for GithubWebhookConfig {
    fn default() -> Self {
        Self {
            secret: String::new(),
        }
    }
}

// ---- [github] ------------------------------------------------------------

/// Merge strategy when auto-merging plan PRs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeMethod {
    /// Standard merge commit.
    Merge,
    /// Squash all commits into one.
    #[default]
    Squash,
    /// Rebase commits onto the base branch.
    Rebase,
}

impl MergeMethod {
    /// Return the string form expected by the GitHub API.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Merge => "merge",
            Self::Squash => "squash",
            Self::Rebase => "rebase",
        }
    }
}

impl std::fmt::Display for MergeMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// GitHub repository identity and workflow preferences.
///
/// This section covers the repo the runner will operate against (branch
/// creation, PRs, issues).  Webhook secrets remain under `[webhooks.github]`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubConfig {
    /// GitHub organisation or user that owns the target repository.
    /// Optional — callers must check at use-site.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// Repository name (without the owner prefix).
    /// Optional — callers must check at use-site.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    /// Base branch for plan PRs.
    #[serde(default = "default_github_default_branch")]
    pub default_branch: String,
    /// When true, the runner automatically opens a draft PR when a plan starts.
    #[serde(default)]
    pub auto_pr: bool,
    /// Strategy used when auto-merging a plan PR.
    #[serde(default)]
    pub merge_method: MergeMethod,
    /// Label prefix applied to roko-managed GitHub labels and issues.
    #[serde(default = "default_github_label_prefix")]
    pub label_prefix: String,
    /// When true, the runner posts a comment and updates labels on the
    /// associated PR whenever a gate passes or fails.  Requires `owner`
    /// and `repo` to be set and `GITHUB_TOKEN` to be available.
    #[serde(default)]
    pub auto_update_prs: bool,
    /// Interval (in hours) between automatic state/progress syncs to GitHub.
    /// Syncs push the current branch and update the PR description with
    /// progress. Set to 0 to disable scheduled syncs. Default: 4 hours.
    #[serde(default = "default_github_sync_interval_hours")]
    pub sync_interval_hours: u32,
    /// When true, the runner deletes roko-managed branches (both local and
    /// remote) after their associated PR has been merged.  Runs as a
    /// post-plan-completion step.
    #[serde(default)]
    pub cleanup_merged_branches: bool,
}

fn default_github_default_branch() -> String {
    "main".to_owned()
}

fn default_github_label_prefix() -> String {
    "roko/".to_owned()
}

const fn default_github_sync_interval_hours() -> u32 {
    4
}

impl Default for GitHubConfig {
    fn default() -> Self {
        Self {
            owner: None,
            repo: None,
            default_branch: default_github_default_branch(),
            auto_pr: false,
            merge_method: MergeMethod::default(),
            label_prefix: default_github_label_prefix(),
            auto_update_prs: false,
            sync_interval_hours: default_github_sync_interval_hours(),
            cleanup_merged_branches: false,
        }
    }
}

#[cfg(test)]
mod statehub_tests {
    use super::*;

    #[test]
    fn statehub_retention_defaults_to_seven_days_and_parses_units() {
        assert_eq!(
            StateHubConfig::default()
                .history_retention_duration()
                .unwrap(),
            std::time::Duration::from_secs(7 * 24 * 60 * 60)
        );
        for (configured, expected_ms) in [
            ("250ms", 250),
            ("60s", 60_000),
            ("2m", 120_000),
            ("3h", 10_800_000),
            ("2d", 172_800_000),
        ] {
            let config = StateHubConfig {
                history_retention: configured.to_string(),
            };
            assert_eq!(
                config.history_retention_duration().unwrap(),
                std::time::Duration::from_millis(expected_ms)
            );
        }
    }

    #[test]
    fn statehub_retention_rejects_zero_missing_units_and_overflow() {
        for configured in ["0s", "60", "1fortnight", "18446744073709551615d"] {
            let config = StateHubConfig {
                history_retention: configured.to_string(),
            };
            assert!(config.history_retention_duration().is_err(), "{configured}");
        }
    }
}
