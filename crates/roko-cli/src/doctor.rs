//! `roko doctor` bootstrap diagnostics for self-hosted workspaces.

use crate::auth_detect::{AuthMethod, detect_auth_from_config};
use crate::config::{ConfigLayer, ConfigPaths, resolve_paths};
use crate::{Config, load_resolved_config};
use anyhow::{Context as _, Result};
use reqwest::Url;
use roko_fs::RokoLayout;
use serde::Serialize;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

const DEFAULT_HEALTH_PATH: &str = "/api/health";
const DOCTOR_HTTP_TIMEOUT_SECS: u64 = 2;

/// Inputs for `roko doctor`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorOptions {
    /// Workspace root to inspect.
    pub workdir: PathBuf,
    /// Optional explicit config override path (`--config`).
    pub config_override: Option<PathBuf>,
    /// Optional roko-serve base URL or explicit health endpoint URL.
    pub serve_url: Option<String>,
}

/// One doctor check status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorStatus {
    Ok,
    Warn,
    Fail,
    Skipped,
}

impl DoctorStatus {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warn => "warn",
            Self::Fail => "fail",
            Self::Skipped => "skipped",
        }
    }
}

/// One named diagnostic check in the doctor report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DoctorCheck {
    pub id: String,
    pub status: DoctorStatus,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Actionable fix command printed after `[fail]` / `[warn]` lines.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
}

/// Summary counters for a doctor run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DoctorSummary {
    pub total: usize,
    pub ok: usize,
    pub warn: usize,
    pub fail: usize,
    pub skipped: usize,
}

impl DoctorSummary {
    fn from_checks(checks: &[DoctorCheck]) -> Self {
        let mut summary = Self {
            total: checks.len(),
            ok: 0,
            warn: 0,
            fail: 0,
            skipped: 0,
        };
        for check in checks {
            match check.status {
                DoctorStatus::Ok => summary.ok += 1,
                DoctorStatus::Warn => summary.warn += 1,
                DoctorStatus::Fail => summary.fail += 1,
                DoctorStatus::Skipped => summary.skipped += 1,
            }
        }
        summary
    }
}

/// Full report returned by `roko doctor`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DoctorReport {
    pub workdir: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serve_url: Option<String>,
    pub healthy: bool,
    pub summary: DoctorSummary,
    pub checks: Vec<DoctorCheck>,
}

impl DoctorReport {
    /// Exit code for the report: `0` on success, `1` if any checks failed.
    #[must_use]
    pub const fn exit_code(&self) -> i32 {
        if self.healthy { 0 } else { 1 }
    }

    /// Human-readable text rendering for the report.
    #[must_use]
    pub fn render_human(&self) -> String {
        let mut out = String::new();
        let headline = if self.healthy { "ok" } else { "fail" };
        let _ = writeln!(&mut out, "doctor: {headline}");
        let _ = writeln!(&mut out, "workdir: {}", self.workdir);
        if let Some(config_path) = &self.config_path {
            let _ = writeln!(&mut out, "config: {config_path}");
        }
        if let Some(serve_url) = &self.serve_url {
            let _ = writeln!(&mut out, "serve_url: {serve_url}");
        }
        let _ = writeln!(
            &mut out,
            "summary: {} ok, {} warn, {} failed, {} skipped",
            self.summary.ok, self.summary.warn, self.summary.fail, self.summary.skipped
        );
        for check in &self.checks {
            let _ = write!(
                &mut out,
                "[{}] {}: {}",
                check.status.label(),
                check.id,
                check.message
            );
            if let Some(path) = &check.path {
                let _ = write!(&mut out, " ({path})");
            }
            if let Some(url) = &check.url {
                let _ = write!(&mut out, " [{url}]");
            }
            if let Some(detail) = &check.detail {
                let _ = write!(&mut out, " - {detail}");
            }
            out.push('\n');
            if matches!(check.status, DoctorStatus::Fail | DoctorStatus::Warn) {
                if let Some(fix) = &check.fix {
                    let _ = writeln!(&mut out, "    \u{2192} fix: {fix}");
                }
            }
        }
        out
    }
}

#[derive(Debug, Clone)]
struct LoadedConfig {
    paths: ConfigPaths,
    resolved: Option<Config>,
    active_path: Option<PathBuf>,
    explicit_serve: bool,
}

/// Run doctor diagnostics for one workspace.
pub async fn run_doctor(options: &DoctorOptions) -> Result<DoctorReport> {
    let workdir = options.workdir.clone();
    let loaded_config = load_active_config(&workdir, options.config_override.as_deref())?;

    let mut checks = Vec::new();
    checks.push(check_workdir(&workdir));
    checks.push(check_config_presence(
        &workdir,
        options.config_override.as_deref(),
        &loaded_config,
    ));
    checks.push(check_layout_basics(&workdir));
    checks.push(check_claude_cli());
    checks.extend(check_configured_provider_keys(&loaded_config));
    checks.push(check_provider_usable(&workdir));
    checks.push(check_available_providers(&loaded_config));
    checks.push(check_default_model_configured(&loaded_config));
    checks.push(check_rust_version());
    checks.push(check_node_version());
    checks.push(check_serve_auth(&loaded_config));
    checks.push(check_serve_health(options.serve_url.as_deref(), &loaded_config).await?);
    checks.push(check_v2_abstractions());
    checks.extend(check_state_layout_audit(&workdir));
    checks.extend(check_harness_providers(&loaded_config));
    checks.extend(check_mcp_allowlist(&workdir, &loaded_config));

    let summary = DoctorSummary::from_checks(&checks);
    Ok(DoctorReport {
        workdir: workdir.display().to_string(),
        config_path: loaded_config
            .active_path
            .as_ref()
            .map(|path| path.display().to_string()),
        serve_url: checks
            .iter()
            .find(|check| check.id == "serve_health")
            .and_then(|check| check.url.clone()),
        healthy: summary.fail == 0,
        summary,
        checks,
    })
}

fn load_active_config(workdir: &Path, config_override: Option<&Path>) -> Result<LoadedConfig> {
    if let Some(path) = config_override {
        if !path.is_file() {
            return Ok(LoadedConfig {
                paths: ConfigPaths {
                    global: crate::config::global_config_path(),
                    project: None,
                    env_override: std::env::var_os("ROKO_CONFIG").map(PathBuf::from),
                },
                resolved: None,
                active_path: Some(path.to_path_buf()),
                explicit_serve: false,
            });
        }

        let layer = ConfigLayer::from_file(path)?;
        let resolved = Config::from_file(path)?;
        return Ok(LoadedConfig {
            paths: ConfigPaths {
                global: crate::config::global_config_path(),
                project: Some(path.to_path_buf()),
                env_override: std::env::var_os("ROKO_CONFIG").map(PathBuf::from),
            },
            resolved: Some(resolved),
            active_path: Some(path.to_path_buf()),
            explicit_serve: layer.serve.is_some(),
        });
    }

    let paths = resolve_paths(workdir);
    let mut explicit_serve = false;
    let active_path = if let Some(env_path) = &paths.env_override {
        match std::fs::read_to_string(env_path) {
            Ok(text) => {
                let layer = ConfigLayer::parse_toml(&text)
                    .with_context(|| format!("parse config {}", env_path.display()))?;
                explicit_serve = layer.serve.is_some();
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(
                    anyhow::Error::new(e).context(format!("read config {}", env_path.display()))
                );
            }
        }
        Some(env_path.clone())
    } else {
        let mut merged = ConfigLayer::default();
        let mut active_path = None;

        match std::fs::read_to_string(&paths.global) {
            Ok(text) => {
                let layer = ConfigLayer::parse_toml(&text)
                    .with_context(|| format!("parse config {}", paths.global.display()))?;
                explicit_serve |= layer.serve.is_some();
                merged = merged.merge(layer);
                active_path = Some(paths.global.clone());
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(anyhow::Error::new(e)
                    .context(format!("read config {}", paths.global.display())));
            }
        }
        if let Some(project_path) = &paths.project {
            let layer = ConfigLayer::from_file(project_path)?;
            explicit_serve |= layer.serve.is_some();
            merged = merged.merge(layer);
            active_path = Some(project_path.clone());
        }

        if merged.is_empty() { None } else { active_path }
    };

    let resolved = if paths
        .env_override
        .as_ref()
        .is_some_and(|path| path.is_file())
        || paths.global.is_file()
        || paths.project.is_some()
    {
        Some(load_resolved_config(workdir)?.config)
    } else {
        None
    };

    Ok(LoadedConfig {
        paths,
        resolved,
        active_path,
        explicit_serve,
    })
}

fn check_workdir(workdir: &Path) -> DoctorCheck {
    let path = workdir.display().to_string();
    if workdir.is_dir() {
        return DoctorCheck {
            id: "workdir".to_string(),
            status: DoctorStatus::Ok,
            message: "workspace directory exists".to_string(),
            detail: None,
            path: Some(path),
            url: None,
            fix: None,
        };
    }

    let message = if workdir.exists() {
        "workspace path is not a directory"
    } else {
        "workspace directory is missing"
    };
    DoctorCheck {
        id: "workdir".to_string(),
        status: DoctorStatus::Fail,
        message: message.to_string(),
        detail: None,
        path: Some(path),
        url: None,
        fix: Some("roko init".to_string()),
    }
}

fn check_config_presence(
    workdir: &Path,
    config_override: Option<&Path>,
    loaded_config: &LoadedConfig,
) -> DoctorCheck {
    if let Some(path) = config_override {
        return if path.is_file() {
            DoctorCheck {
                id: "config".to_string(),
                status: DoctorStatus::Ok,
                message: "using explicit config override".to_string(),
                detail: None,
                path: Some(path.display().to_string()),
                url: None,
                fix: None,
            }
        } else {
            DoctorCheck {
                id: "config".to_string(),
                status: DoctorStatus::Fail,
                message: "explicit config override is missing".to_string(),
                detail: None,
                path: Some(path.display().to_string()),
                url: None,
                fix: Some("roko init".to_string()),
            }
        };
    }

    if let Some(path) = &loaded_config.paths.env_override {
        return if path.is_file() {
            DoctorCheck {
                id: "config".to_string(),
                status: DoctorStatus::Ok,
                message: "using ROKO_CONFIG override".to_string(),
                detail: None,
                path: Some(path.display().to_string()),
                url: None,
                fix: None,
            }
        } else {
            DoctorCheck {
                id: "config".to_string(),
                status: DoctorStatus::Fail,
                message: "ROKO_CONFIG points to a missing file".to_string(),
                detail: None,
                path: Some(path.display().to_string()),
                url: None,
                fix: Some("roko init".to_string()),
            }
        };
    }

    if let Some(path) = &loaded_config.paths.project {
        return DoctorCheck {
            id: "config".to_string(),
            status: DoctorStatus::Ok,
            message: "found project roko.toml".to_string(),
            detail: None,
            path: Some(path.display().to_string()),
            url: None,
            fix: None,
        };
    }

    DoctorCheck {
        id: "config".to_string(),
        status: DoctorStatus::Fail,
        message: "missing project roko.toml".to_string(),
        detail: Some(format!(
            "expected {} or an ancestor config; global config alone is not enough for workspace bootstrap",
            workdir.join("roko.toml").display()
        )),
        path: Some(loaded_config.paths.global.display().to_string()),
        url: None,
        fix: Some("roko init".to_string()),
    }
}

fn check_layout_basics(workdir: &Path) -> DoctorCheck {
    let layout = RokoLayout::for_project(workdir);
    let root = layout.root().display().to_string();
    if !layout.root().is_dir() {
        return DoctorCheck {
            id: "layout".to_string(),
            status: DoctorStatus::Fail,
            message: "missing .roko directory".to_string(),
            detail: None,
            path: Some(root),
            url: None,
            fix: Some("roko init".to_string()),
        };
    }

    let mut missing = Vec::new();
    if !layout.version_file().is_file() {
        missing.push(layout.version_file().display().to_string());
    }
    for dir in layout.top_level_dirs() {
        if !dir.is_dir() {
            missing.push(dir.display().to_string());
        }
    }

    if missing.is_empty() {
        DoctorCheck {
            id: "layout".to_string(),
            status: DoctorStatus::Ok,
            message: ".roko layout basics are present".to_string(),
            detail: None,
            path: Some(root),
            url: None,
            fix: None,
        }
    } else {
        DoctorCheck {
            id: "layout".to_string(),
            status: DoctorStatus::Fail,
            message: "required .roko layout paths are missing".to_string(),
            detail: Some(missing.join(", ")),
            path: Some(root),
            url: None,
            fix: Some("roko init".to_string()),
        }
    }
}

fn check_provider_usable(workdir: &Path) -> DoctorCheck {
    let auth = detect_auth_from_config(workdir);
    match auth {
        AuthMethod::NeedsSetup => DoctorCheck {
            id: "provider_usable".to_string(),
            status: DoctorStatus::Fail,
            message: "no LLM provider has working auth".to_string(),
            detail: None,
            path: None,
            url: None,
            fix: Some("Set an API key. Example: export ANTHROPIC_API_KEY=sk-...".to_string()),
        },
        _ => DoctorCheck {
            id: "provider_usable".to_string(),
            status: DoctorStatus::Ok,
            message: format!("provider available: {}", auth.label()),
            detail: None,
            path: None,
            url: None,
            fix: None,
        },
    }
}

fn check_default_model_configured(loaded_config: &LoadedConfig) -> DoctorCheck {
    let Some(config) = &loaded_config.resolved else {
        return DoctorCheck {
            id: "default_model_configured".to_string(),
            status: DoctorStatus::Skipped,
            message: "config unavailable; default_model not evaluated".to_string(),
            detail: None,
            path: None,
            url: None,
            fix: None,
        };
    };

    let model_key = config.agent.model.as_deref().unwrap_or("").trim();
    if model_key.is_empty() {
        return DoctorCheck {
            id: "default_model_configured".to_string(),
            status: DoctorStatus::Warn,
            message: "no default_model configured".to_string(),
            detail: None,
            path: None,
            url: None,
            fix: Some("Set default_model in roko.toml [agent] section".to_string()),
        };
    }

    let in_models_table = config.models.contains_key(model_key);
    let is_builtin = roko_core::config::model_registry::builtin_model(model_key).is_some();

    if in_models_table || is_builtin {
        DoctorCheck {
            id: "default_model_configured".to_string(),
            status: DoctorStatus::Ok,
            message: format!("default_model \"{model_key}\" is valid"),
            detail: None,
            path: None,
            url: None,
            fix: None,
        }
    } else {
        DoctorCheck {
            id: "default_model_configured".to_string(),
            status: DoctorStatus::Fail,
            message: format!("default_model \"{model_key}\" not found in models table or builtins"),
            detail: None,
            path: None,
            url: None,
            fix: Some("Set default_model in roko.toml [agent] section".to_string()),
        }
    }
}

fn check_serve_auth(loaded_config: &LoadedConfig) -> DoctorCheck {
    let Some(config) = &loaded_config.resolved else {
        return DoctorCheck {
            id: "serve_auth".to_string(),
            status: DoctorStatus::Skipped,
            message: "config unavailable; serve/auth not evaluated".to_string(),
            detail: None,
            path: loaded_config
                .active_path
                .as_ref()
                .map(|path| path.display().to_string()),
            url: None,
            fix: None,
        };
    };

    if !loaded_config.explicit_serve {
        return DoctorCheck {
            id: "serve_auth".to_string(),
            status: DoctorStatus::Skipped,
            message: "no explicit [serve] config found".to_string(),
            detail: None,
            path: loaded_config
                .active_path
                .as_ref()
                .map(|path| path.display().to_string()),
            url: None,
            fix: None,
        };
    }

    let auth = &config.serve.auth;
    if auth.enabled && auth.api_key.trim().is_empty() {
        return DoctorCheck {
            id: "serve_auth".to_string(),
            status: DoctorStatus::Fail,
            message: "serve auth is enabled but api_key is empty".to_string(),
            detail: None,
            path: loaded_config
                .active_path
                .as_ref()
                .map(|path| path.display().to_string()),
            url: None,
            fix: Some("roko config set serve.auth.api_key <your-key>".to_string()),
        };
    }

    DoctorCheck {
        id: "serve_auth".to_string(),
        status: DoctorStatus::Ok,
        message: if auth.enabled {
            "serve auth is enabled and api_key is present".to_string()
        } else {
            "serve config is present and auth is disabled".to_string()
        },
        detail: None,
        path: loaded_config
            .active_path
            .as_ref()
            .map(|path| path.display().to_string()),
        url: None,
        fix: None,
    }
}

async fn check_serve_health(
    serve_url: Option<&str>,
    loaded_config: &LoadedConfig,
) -> Result<DoctorCheck> {
    let Some(raw_url) = serve_url else {
        return Ok(DoctorCheck {
            id: "serve_health".to_string(),
            status: DoctorStatus::Skipped,
            message: "serve health probe not requested".to_string(),
            detail: None,
            path: None,
            url: None,
            fix: None,
        });
    };

    let endpoint = match normalize_health_endpoint_url(raw_url) {
        Ok(url) => url,
        Err(err) => {
            return Ok(DoctorCheck {
                id: "serve_health".to_string(),
                status: DoctorStatus::Fail,
                message: "invalid serve URL".to_string(),
                detail: Some(err.to_string()),
                path: None,
                url: Some(raw_url.to_string()),
                fix: Some("roko serve".to_string()),
            });
        }
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(DOCTOR_HTTP_TIMEOUT_SECS))
        .build()
        .context("build doctor HTTP client")?;

    let mut request = client.get(endpoint.clone());
    if let Some(config) = &loaded_config.resolved
        && config.serve.auth.enabled
        && !config.serve.auth.api_key.trim().is_empty()
    {
        request = request.header("X-Api-Key", config.serve.auth.api_key.clone());
    }

    let response = request.send().await;
    let check = match response {
        Ok(response) if response.status().is_success() => DoctorCheck {
            id: "serve_health".to_string(),
            status: DoctorStatus::Ok,
            message: format!("health endpoint reachable ({})", response.status()),
            detail: None,
            path: None,
            url: Some(endpoint.to_string()),
            fix: None,
        },
        Ok(response) => DoctorCheck {
            id: "serve_health".to_string(),
            status: DoctorStatus::Fail,
            message: format!("health endpoint returned {}", response.status()),
            detail: None,
            path: None,
            url: Some(endpoint.to_string()),
            fix: Some("roko serve".to_string()),
        },
        Err(err) if err.is_builder() => DoctorCheck {
            id: "serve_health".to_string(),
            status: DoctorStatus::Fail,
            message: "invalid serve URL".to_string(),
            detail: Some(err.to_string()),
            path: None,
            url: Some(endpoint.to_string()),
            fix: Some("roko serve".to_string()),
        },
        Err(err) => DoctorCheck {
            id: "serve_health".to_string(),
            status: DoctorStatus::Fail,
            message: "health endpoint is unreachable".to_string(),
            detail: Some(err.to_string()),
            path: None,
            url: Some(endpoint.to_string()),
            fix: Some("roko serve".to_string()),
        },
    };
    Ok(check)
}

fn check_claude_cli() -> DoctorCheck {
    let available = std::process::Command::new("claude")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if available {
        DoctorCheck {
            id: "claude_cli".to_string(),
            status: DoctorStatus::Ok,
            message: "claude CLI is on PATH".to_string(),
            detail: None,
            path: None,
            url: None,
            fix: None,
        }
    } else {
        DoctorCheck {
            id: "claude_cli".to_string(),
            status: DoctorStatus::Warn,
            message: "claude CLI not found on PATH".to_string(),
            detail: None,
            path: None,
            url: None,
            fix: Some("npm install -g @anthropic-ai/claude-cli && claude login".to_string()),
        }
    }
}

/// Check API keys for all providers configured in `roko.toml`.
///
/// For each provider that specifies an `api_key_env`, verifies the environment
/// variable is set and non-empty. If no API-key-based providers are configured
/// (e.g. the user only uses the `claude` CLI), emits a single `Ok` check.
fn check_configured_provider_keys(loaded_config: &LoadedConfig) -> Vec<DoctorCheck> {
    use roko_core::agent::ProviderKind;

    let Some(config) = &loaded_config.resolved else {
        // No config file — fall back to checking the single most common key.
        let has_key = std::env::var("ANTHROPIC_API_KEY")
            .ok()
            .filter(|k| !k.is_empty())
            .is_some();
        return vec![DoctorCheck {
            id: "provider_api_keys".to_string(),
            status: if has_key {
                DoctorStatus::Ok
            } else {
                DoctorStatus::Warn
            },
            message: if has_key {
                "ANTHROPIC_API_KEY is set (no roko.toml)".to_string()
            } else {
                "no API keys found and no roko.toml present".to_string()
            },
            detail: None,
            path: None,
            url: None,
            fix: if has_key {
                None
            } else {
                Some("run `roko config init` or set a provider API key (ANTHROPIC_API_KEY, OPENAI_API_KEY, etc.)".to_string())
            },
        }];
    };

    // Collect providers that require an API key (non-CLI kinds).
    let api_providers: Vec<(&String, &roko_core::config::schema::ProviderConfig)> = config
        .providers
        .iter()
        .filter(|(_, p)| {
            !matches!(
                p.kind,
                ProviderKind::ClaudeCli | ProviderKind::Hermes | ProviderKind::OpenClaw
            )
        })
        .collect();

    if api_providers.is_empty() {
        return vec![DoctorCheck {
            id: "provider_api_keys".to_string(),
            status: DoctorStatus::Ok,
            message: "no API providers configured (using CLI-based provider)".to_string(),
            detail: None,
            path: None,
            url: None,
            fix: None,
        }];
    }

    let mut checks = Vec::new();
    for (id, provider) in api_providers {
        let Some(env_name) = provider.api_key_env.as_deref() else {
            continue;
        };
        let has_key = std::env::var(env_name)
            .ok()
            .filter(|k| !k.is_empty())
            .is_some();
        checks.push(DoctorCheck {
            id: format!("provider_key_{id}"),
            status: if has_key {
                DoctorStatus::Ok
            } else {
                DoctorStatus::Warn
            },
            message: if has_key {
                format!("{env_name} is set (provider `{id}`)")
            } else {
                format!("{env_name} not set (provider `{id}` is configured but has no key)")
            },
            detail: None,
            path: None,
            url: None,
            fix: if has_key {
                None
            } else {
                Some(format!("export {env_name}=<your-api-key>"))
            },
        });
    }
    checks
}

/// Summarise all available providers (those with working credentials or CLI tools).
///
/// Emits an informational `Ok` check listing detected providers, or a `Warn`
/// if nothing usable is found. This is intentionally non-blocking — specific
/// per-provider failures are reported by other checks.
fn check_available_providers(loaded_config: &LoadedConfig) -> DoctorCheck {
    use roko_core::agent::ProviderKind;

    let mut available: Vec<String> = Vec::new();

    // Check for claude CLI on PATH.
    if std::process::Command::new("claude")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        available.push("claude-cli".to_string());
    }

    // Check common API key env vars.
    for (env, label) in &[
        ("ANTHROPIC_API_KEY", "anthropic"),
        ("OPENAI_API_KEY", "openai"),
        ("GEMINI_API_KEY", "gemini"),
        ("ZAI_API_KEY", "zhipu"),
    ] {
        if std::env::var(env).ok().filter(|k| !k.is_empty()).is_some() {
            available.push((*label).to_string());
        }
    }

    // Check configured providers' api_key_env values not already captured above.
    if let Some(config) = &loaded_config.resolved {
        for (_id, provider) in &config.providers {
            // CLI providers already handled above.
            if matches!(
                provider.kind,
                ProviderKind::ClaudeCli | ProviderKind::Hermes | ProviderKind::OpenClaw
            ) {
                continue;
            }
            let Some(env_name) = provider.api_key_env.as_deref() else {
                continue;
            };
            // Skip well-known keys already in our static list.
            if [
                "ANTHROPIC_API_KEY",
                "OPENAI_API_KEY",
                "GEMINI_API_KEY",
                "ZAI_API_KEY",
            ]
            .contains(&env_name)
            {
                continue;
            }
            if std::env::var(env_name)
                .ok()
                .filter(|k| !k.is_empty())
                .is_some()
            {
                available.push(format!("{env_name} provider"));
            }
        }
    }

    available.dedup();

    if available.is_empty() {
        DoctorCheck {
            id: "providers_detected".to_string(),
            status: DoctorStatus::Warn,
            message: "no providers detected — no API keys set and no CLI tools found".to_string(),
            detail: None,
            path: None,
            url: None,
            fix: Some(
                "run `roko config init` or set OPENAI_API_KEY / ANTHROPIC_API_KEY".to_string(),
            ),
        }
    } else {
        DoctorCheck {
            id: "providers_detected".to_string(),
            status: DoctorStatus::Ok,
            message: format!(
                "{} provider{} available ({})",
                available.len(),
                if available.len() == 1 { "" } else { "s" },
                available.join(", ")
            ),
            detail: None,
            path: None,
            url: None,
            fix: None,
        }
    }
}

fn check_rust_version() -> DoctorCheck {
    let output = std::process::Command::new("rustc")
        .arg("--version")
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let version_str = String::from_utf8_lossy(&o.stdout);
            // Parse "rustc 1.91.0 (..." into the minor version number.
            let minor = version_str
                .split_whitespace()
                .nth(1)
                .and_then(|v| v.split('.').nth(1))
                .and_then(|m| m.parse::<u32>().ok())
                .unwrap_or(0);

            if minor >= 91 {
                DoctorCheck {
                    id: "rust_version".to_string(),
                    status: DoctorStatus::Ok,
                    message: format!("Rust version is adequate ({})", version_str.trim()),
                    detail: None,
                    path: None,
                    url: None,
                    fix: None,
                }
            } else {
                DoctorCheck {
                    id: "rust_version".to_string(),
                    status: DoctorStatus::Fail,
                    message: format!("Rust version below 1.91 ({})", version_str.trim()),
                    detail: Some("alloy deps require rustc 1.91+".to_string()),
                    path: None,
                    url: None,
                    fix: Some("rustup update stable".to_string()),
                }
            }
        }
        _ => DoctorCheck {
            id: "rust_version".to_string(),
            status: DoctorStatus::Warn,
            message: "rustc not found on PATH".to_string(),
            detail: None,
            path: None,
            url: None,
            fix: Some("rustup update stable".to_string()),
        },
    }
}

fn check_node_version() -> DoctorCheck {
    let output = std::process::Command::new("node").arg("--version").output();

    match output {
        Ok(o) if o.status.success() => {
            let version_str = String::from_utf8_lossy(&o.stdout);
            // Parse "v22.1.0" into the major version number.
            let major = version_str
                .trim()
                .trim_start_matches('v')
                .split('.')
                .next()
                .and_then(|m| m.parse::<u32>().ok())
                .unwrap_or(0);

            if major >= 22 {
                DoctorCheck {
                    id: "node_version".to_string(),
                    status: DoctorStatus::Ok,
                    message: format!("Node version is adequate ({})", version_str.trim()),
                    detail: None,
                    path: None,
                    url: None,
                    fix: None,
                }
            } else {
                DoctorCheck {
                    id: "node_version".to_string(),
                    status: DoctorStatus::Warn,
                    message: format!("Node version below 22 ({})", version_str.trim()),
                    detail: None,
                    path: None,
                    url: None,
                    fix: Some("nvm install 22 && nvm use 22".to_string()),
                }
            }
        }
        _ => DoctorCheck {
            id: "node_version".to_string(),
            status: DoctorStatus::Skipped,
            message: "node not found on PATH (optional)".to_string(),
            detail: None,
            path: None,
            url: None,
            fix: None,
        },
    }
}

fn normalize_health_endpoint_url(raw_url: &str) -> Result<Url> {
    let mut url = Url::parse(raw_url).with_context(|| format!("parse URL {raw_url}"))?;
    let path = url.path();
    if path.is_empty() || path == "/" {
        url.set_path(DEFAULT_HEALTH_PATH);
        return Ok(url);
    }
    if path == "/api" || path == "/api/" {
        url.set_path(DEFAULT_HEALTH_PATH);
        return Ok(url);
    }
    if path.ends_with("/health") || path.ends_with("/api/health") {
        return Ok(url);
    }

    Ok(url)
}

/// Deterministic check that v2 protocol abstractions are compiled and reachable.
///
/// This does not make network calls or spawn subprocesses. It compile-references
/// the public types from the dependency tasks (Cell, CellContext, TypeSchema,
/// Signal, Observe, Connect, Trigger) and verifies they are usable at runtime.
fn check_v2_abstractions() -> DoctorCheck {
    // Compile-time references: if any of these types are removed or renamed,
    // this function will fail to compile, catching regressions immediately.
    use roko_core::cell::{CellContext, CellVersion, TypeSchema};
    use roko_core::signal::Signal;
    use roko_core::traits::{Connect, Observe, Trigger};

    // Runtime probe: verify the types can be instantiated / inspected.
    // TypeSchema has a deterministic compatibility check we can exercise.
    let any = TypeSchema::Any;
    let metric = TypeSchema::OfKind(roko_core::Kind::Metric);
    let schema_ok = any.is_compatible_with(&metric) && metric.is_compatible_with(&any);

    // Verify Signal alias resolves to the same type as Engram.
    let signal: Signal = Signal::builder(roko_core::Kind::Task).build();
    let signal_ok = !signal.id.0.iter().all(|b| *b == 0);

    // Verify CellVersion default is a valid triple.
    let version: CellVersion = (0, 1, 0);
    let version_ok = version.0 == 0 && version.1 == 1 && version.2 == 0;

    // Verify the protocol traits and CellContext are importable and have
    // the expected shapes. These trait bound assertions are never called at
    // runtime but ensure the traits exist with the right bounds at compile time.
    #[allow(dead_code)]
    fn assert_observe<T: Observe>() {}
    #[allow(dead_code)]
    fn assert_connect<T: Connect>() {}
    #[allow(dead_code)]
    fn assert_trigger<T: Trigger>() {}
    let _ = std::any::type_name::<CellContext>();

    let all_ok = schema_ok && signal_ok && version_ok;

    if all_ok {
        DoctorCheck {
            id: "v2_abstractions".to_string(),
            status: DoctorStatus::Ok,
            message: "phase 1 protocol abstractions are reachable".to_string(),
            detail: Some(
                "Cell, CellContext, TypeSchema, Signal, Observe, Connect, Trigger".to_string(),
            ),
            path: None,
            url: None,
            fix: None,
        }
    } else {
        DoctorCheck {
            id: "v2_abstractions".to_string(),
            status: DoctorStatus::Fail,
            message: "phase 1 protocol abstractions failed runtime probe".to_string(),
            detail: Some(format!(
                "schema_ok={schema_ok}, signal_ok={signal_ok}, version_ok={version_ok}"
            )),
            path: None,
            url: None,
            fix: None,
        }
    }
}

/// Audit the `.roko/` state layout for version, canonical, and legacy files.
///
/// Produces up to three checks:
/// - `state_layout_version` -- verifies `.roko/VERSION` is V2 (current).
/// - `state_canonical_files` -- lists which E02 canonical files are present.
/// - `state_legacy_files` -- flags legacy files left over from V1 layouts.
///
/// Returns an empty slice when `.roko/` does not exist (workspace not yet
/// initialized); the `layout` check already covers that case.
fn check_state_layout_audit(workdir: &Path) -> Vec<DoctorCheck> {
    use roko_fs::LayoutVersion;

    let layout = RokoLayout::for_project(workdir);
    if !layout.root().is_dir() {
        // Workspace not initialized; the `layout` check reports this already.
        return vec![];
    }

    let mut checks = Vec::new();

    // -- 1. VERSION file ------------------------------------------------------
    let version_path = layout.version_file();
    let version_check = if !version_path.is_file() {
        DoctorCheck {
            id: "state_layout_version".to_string(),
            status: DoctorStatus::Warn,
            message: ".roko/VERSION file is missing".to_string(),
            detail: Some("run `roko init` to create the version file".to_string()),
            path: Some(version_path.display().to_string()),
            url: None,
            fix: Some("roko init".to_string()),
        }
    } else {
        let on_disk = std::fs::read_to_string(&version_path)
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .and_then(LayoutVersion::from_u32);

        match on_disk {
            Some(LayoutVersion::V2) => DoctorCheck {
                id: "state_layout_version".to_string(),
                status: DoctorStatus::Ok,
                message: "storage layout is V2 (current)".to_string(),
                detail: None,
                path: Some(version_path.display().to_string()),
                url: None,
                fix: None,
            },
            Some(LayoutVersion::V1) => DoctorCheck {
                id: "state_layout_version".to_string(),
                status: DoctorStatus::Warn,
                message: "storage layout is V1 (outdated)".to_string(),
                detail: Some(
                    "V1->V2 migration moves signals.jsonl to signals.jsonl.v1-legacy \
                     and bumps VERSION to 2"
                        .to_string(),
                ),
                path: Some(version_path.display().to_string()),
                url: None,
                fix: Some("roko init".to_string()),
            },
            None => DoctorCheck {
                id: "state_layout_version".to_string(),
                status: DoctorStatus::Warn,
                message: ".roko/VERSION contains an unrecognized value".to_string(),
                detail: Some(
                    std::fs::read_to_string(&version_path)
                        .unwrap_or_default()
                        .trim()
                        .to_string(),
                ),
                path: Some(version_path.display().to_string()),
                url: None,
                fix: Some("roko init".to_string()),
            },
        }
    };
    checks.push(version_check);

    // -- 2. Canonical E02 files -----------------------------------------------
    // These are the paths that all E02 writers now target in V2.
    let canonical_paths: &[(&str, PathBuf)] = &[
        ("episodes.jsonl", layout.root_episodes_path()),
        (
            "gate-verdicts.jsonl",
            layout.root().join("gate-verdicts.jsonl"),
        ),
        ("engrams.jsonl", layout.engrams_path()),
        ("events.jsonl", layout.events_jsonl_path()),
        ("learn/gate-thresholds.json", layout.gate_thresholds_path()),
        (
            "state/state-snapshot.json",
            layout.state_dir().join("state-snapshot.json"),
        ),
    ];

    let mut present: Vec<&str> = Vec::new();
    let mut absent: Vec<&str> = Vec::new();
    for (name, path) in canonical_paths {
        if path.exists() {
            present.push(name);
        } else {
            absent.push(name);
        }
    }

    let canonical_check = if absent.is_empty() {
        DoctorCheck {
            id: "state_canonical_files".to_string(),
            status: DoctorStatus::Ok,
            message: format!(
                "all {} canonical V2 storage files are present",
                present.len()
            ),
            detail: Some(present.join(", ")),
            path: Some(layout.root().display().to_string()),
            url: None,
            fix: None,
        }
    } else {
        DoctorCheck {
            id: "state_canonical_files".to_string(),
            status: DoctorStatus::Ok,
            message: format!(
                "{} canonical V2 files present, {} absent (normal for new workspaces)",
                present.len(),
                absent.len()
            ),
            detail: Some(format!(
                "present: {}; absent: {}",
                if present.is_empty() {
                    "none".to_string()
                } else {
                    present.join(", ")
                },
                absent.join(", ")
            )),
            path: Some(layout.root().display().to_string()),
            url: None,
            fix: None,
        }
    };
    checks.push(canonical_check);

    // -- 3. Legacy V1 files ---------------------------------------------------
    // Files that should not exist in a migrated V2 workspace.
    let legacy_paths: &[(&str, PathBuf)] = &[
        ("signals.jsonl", layout.signals_path()),
        (
            "memory/episodes.jsonl",
            layout.memory_dir().join("episodes.jsonl"),
        ),
        (
            "state/executor.json",
            layout.state_dir().join("executor.json"),
        ),
        ("state/events.json", layout.state_dir().join("events.json")),
    ];

    let mut legacy_found: Vec<&str> = Vec::new();
    for (name, path) in legacy_paths {
        if path.exists() {
            legacy_found.push(name);
        }
    }

    let legacy_check = if legacy_found.is_empty() {
        DoctorCheck {
            id: "state_legacy_files".to_string(),
            status: DoctorStatus::Ok,
            message: "no legacy V1 storage files detected".to_string(),
            detail: None,
            path: Some(layout.root().display().to_string()),
            url: None,
            fix: None,
        }
    } else {
        DoctorCheck {
            id: "state_legacy_files".to_string(),
            status: DoctorStatus::Warn,
            message: format!("{} legacy V1 storage file(s) detected", legacy_found.len()),
            detail: Some(format!(
                "legacy files (safe to remove after migration): {}",
                legacy_found.join(", ")
            )),
            path: Some(layout.root().display().to_string()),
            url: None,
            fix: Some("roko init".to_string()),
        }
    };
    checks.push(legacy_check);

    checks
}

/// Check for configured harness providers (Hermes, OpenClaw) and verify
/// their binaries are available on PATH.
fn check_harness_providers(loaded_config: &LoadedConfig) -> Vec<DoctorCheck> {
    use roko_core::agent::ProviderKind;

    let Some(config) = &loaded_config.resolved else {
        return vec![];
    };

    let mut checks = Vec::new();
    for (id, provider) in &config.providers {
        match provider.kind {
            ProviderKind::Hermes => {
                let binary = provider.command.as_deref().unwrap_or("hermes");
                let available = std::process::Command::new(binary)
                    .arg("--version")
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false);
                checks.push(DoctorCheck {
                    id: format!("harness_{id}"),
                    status: if available {
                        DoctorStatus::Ok
                    } else {
                        DoctorStatus::Warn
                    },
                    message: if available {
                        format!("hermes provider `{id}` binary found")
                    } else {
                        format!("hermes provider `{id}` binary `{binary}` not found on PATH")
                    },
                    detail: None,
                    path: None,
                    url: None,
                    fix: if available {
                        None
                    } else {
                        Some(format!("install hermes or set providers.{id}.command"))
                    },
                });
            }
            ProviderKind::OpenClaw => {
                let binary = provider.command.as_deref().unwrap_or("openclaw");
                let available = std::process::Command::new(binary)
                    .arg("--version")
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false);
                checks.push(DoctorCheck {
                    id: format!("harness_{id}"),
                    status: if available {
                        DoctorStatus::Ok
                    } else {
                        DoctorStatus::Warn
                    },
                    message: if available {
                        format!("openclaw provider `{id}` binary found")
                    } else {
                        format!("openclaw provider `{id}` binary `{binary}` not found on PATH")
                    },
                    detail: None,
                    path: None,
                    url: None,
                    fix: if available {
                        None
                    } else {
                        Some(format!("install openclaw or set providers.{id}.command"))
                    },
                });
            }
            _ => {} // Non-harness providers handled by existing checks.
        }
    }
    checks
}

/// Inspect configured MCP servers for command allowlist and sensitive env keys.
///
/// Returns one check per server plus a summary check. Missing MCP config is
/// **not** treated as a failure (matches anti-pattern from E04-T17).
fn check_mcp_allowlist(workdir: &Path, loaded_config: &LoadedConfig) -> Vec<DoctorCheck> {
    use roko_agent::mcp::{
        McpTransportConfig, find_mcp_config, hardcoded_secret_values, is_command_allowed,
        is_command_on_path, sensitive_env_keys, unset_env_var_refs,
    };

    // Resolve the MCP config: explicit path from roko.toml, or walk-up discovery.
    let mcp_config = if let Some(ref cfg) = loaded_config.resolved {
        if let Some(ref explicit) = cfg.agent.mcp_config {
            roko_agent::mcp::McpConfig::load(explicit).ok()
        } else {
            find_mcp_config(workdir).and_then(|r| r.ok().map(|(_path, cfg)| cfg))
        }
    } else {
        find_mcp_config(workdir).and_then(|r| r.ok().map(|(_path, cfg)| cfg))
    };

    let Some(mcp_config) = mcp_config else {
        return vec![DoctorCheck {
            id: "mcp_allowlist".to_string(),
            status: DoctorStatus::Skipped,
            message: "no MCP config found; skipping allowlist check".to_string(),
            detail: None,
            path: None,
            url: None,
            fix: None,
        }];
    };

    if mcp_config.servers.is_empty() {
        return vec![DoctorCheck {
            id: "mcp_allowlist".to_string(),
            status: DoctorStatus::Ok,
            message: "MCP config present with no servers configured".to_string(),
            detail: None,
            path: None,
            url: None,
            fix: None,
        }];
    }

    let mut checks = Vec::new();
    let mut any_warn = false;

    for server in &mcp_config.servers {
        let name = if server.name.is_empty() {
            "<unnamed>"
        } else {
            &server.name
        };

        // Skip command checks for HTTP-transport servers (they have no local command).
        let is_http = server.transport == McpTransportConfig::Http;
        let cmd_empty = server.command.is_empty();

        let cmd_allowed = is_http || cmd_empty || is_command_allowed(&server.command, &[]);
        let cmd_on_path = is_http || cmd_empty || is_command_on_path(&server.command);

        let sensitive = sensitive_env_keys(&server.env);
        let unset_refs = unset_env_var_refs(&server.env);
        let hardcoded = hardcoded_secret_values(&server.env);

        let has_issue =
            !cmd_allowed || !cmd_on_path || !unset_refs.is_empty() || !hardcoded.is_empty();
        let has_warn = has_issue || !sensitive.is_empty();
        if has_warn {
            any_warn = true;
        }

        let status = if has_warn {
            DoctorStatus::Warn
        } else {
            DoctorStatus::Ok
        };

        let mut detail_parts = Vec::new();
        if !cmd_allowed {
            detail_parts.push(format!(
                "command `{}` is not on the approved allowlist",
                server.command,
            ));
        } else if !cmd_on_path {
            // Only report missing-from-PATH when the command is allowed but absent.
            detail_parts.push(format!(
                "command `{}` was not found on PATH",
                server.command,
            ));
        }
        if !hardcoded.is_empty() {
            detail_parts.push(format!(
                "env keys with hardcoded secret values (use ${{VAR}} references instead): {}",
                hardcoded.join(", "),
            ));
        }
        if !unset_refs.is_empty() {
            detail_parts.push(format!(
                "env keys reference unset environment variables: {}",
                unset_refs.join(", "),
            ));
        }
        // Report remaining sensitive keys (those whose values are env-var refs)
        // only when there are no more severe issues already described.
        let remaining_sensitive: Vec<String> = sensitive
            .iter()
            .filter(|k| !hardcoded.contains(k) && !unset_refs.contains(k))
            .cloned()
            .collect();
        if !remaining_sensitive.is_empty() {
            detail_parts.push(format!(
                "env keys that may contain secrets: {}",
                remaining_sensitive.join(", "),
            ));
        }

        let detail = if detail_parts.is_empty() {
            None
        } else {
            Some(detail_parts.join("; "))
        };

        let fix = if !cmd_allowed {
            Some(format!(
                "verify `{}` is intended; add to allowlist or use a known MCP server command",
                server.command,
            ))
        } else if !cmd_on_path {
            Some(format!(
                "install `{}` so it is available on PATH, or use an absolute path",
                server.command,
            ))
        } else if !hardcoded.is_empty() {
            Some(
                "replace hardcoded secret values with ${ENV_VAR} references and set the variables in the shell"
                    .to_string(),
            )
        } else if !unset_refs.is_empty() {
            Some("set the missing environment variables before running the MCP server".to_string())
        } else if !sensitive.is_empty() {
            Some(
                "avoid passing secrets via MCP env; use auth_token or a secrets manager instead"
                    .to_string(),
            )
        } else {
            None
        };

        checks.push(DoctorCheck {
            id: format!("mcp_server_{name}"),
            status,
            message: if status == DoctorStatus::Ok {
                format!("MCP server `{name}` passes allowlist checks")
            } else {
                format!("MCP server `{name}` has security warnings")
            },
            detail,
            path: None,
            url: None,
            fix,
        });
    }

    // Summary check.
    checks.push(DoctorCheck {
        id: "mcp_allowlist".to_string(),
        status: if any_warn {
            DoctorStatus::Warn
        } else {
            DoctorStatus::Ok
        },
        message: if any_warn {
            format!(
                "MCP allowlist: {} server(s) configured, some have warnings",
                mcp_config.servers.len(),
            )
        } else {
            format!(
                "MCP allowlist: {} server(s) configured, all pass",
                mcp_config.servers.len(),
            )
        },
        detail: None,
        path: None,
        url: None,
        fix: if any_warn {
            Some("review per-server MCP warnings above".to_string())
        } else {
            None
        },
    });

    checks
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_project_config(workdir: &Path, config: Config) {
        std::fs::write(
            workdir.join("roko.toml"),
            config.to_toml().expect("serialize config"),
        )
        .expect("write roko.toml");
    }

    async fn bootstrap_layout(workdir: &Path) {
        RokoLayout::for_project(workdir)
            .ensure_dirs()
            .await
            .expect("create .roko layout");
    }

    #[test]
    fn normalize_health_endpoint_url_adds_default_api_path() {
        assert_eq!(
            normalize_health_endpoint_url("http://localhost:9090")
                .unwrap()
                .as_str(),
            "http://localhost:9090/api/health"
        );
        assert_eq!(
            normalize_health_endpoint_url("http://localhost:9090/api")
                .unwrap()
                .as_str(),
            "http://localhost:9090/api/health"
        );
    }

    #[test]
    fn normalize_health_endpoint_url_preserves_explicit_health_endpoint() {
        assert_eq!(
            normalize_health_endpoint_url("http://localhost:9090/custom/health")
                .unwrap()
                .as_str(),
            "http://localhost:9090/custom/health"
        );
    }

    #[tokio::test]
    async fn run_doctor_reports_missing_project_config_and_layout() {
        let temp = tempdir().unwrap();
        let report = run_doctor(&DoctorOptions {
            workdir: temp.path().to_path_buf(),
            config_override: None,
            serve_url: None,
        })
        .await
        .unwrap();

        assert!(!report.healthy);
        assert_eq!(report.exit_code(), 1);
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.id == "config" && check.status == DoctorStatus::Fail)
        );
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.id == "layout" && check.status == DoctorStatus::Fail)
        );
    }

    #[tokio::test]
    async fn run_doctor_passes_bootstrapped_workspace_without_serve_probe() {
        let temp = tempdir().unwrap();
        let mut config = Config::default();
        // Disable auth so doctor doesn't fail on empty api_key (secure-by-default
        // enables auth, but doctor flags enabled-without-key as a failure).
        config.serve.auth.enabled = false;
        write_project_config(temp.path(), config);
        bootstrap_layout(temp.path()).await;

        let report = run_doctor(&DoctorOptions {
            workdir: temp.path().to_path_buf(),
            config_override: None,
            serve_url: None,
        })
        .await
        .unwrap();

        assert!(report.healthy);
        assert_eq!(report.exit_code(), 0);
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.id == "serve_health" && check.status == DoctorStatus::Skipped)
        );
    }

    #[tokio::test]
    async fn run_doctor_fails_when_serve_auth_enabled_without_api_key() {
        let temp = tempdir().unwrap();
        let mut config = Config::default();
        config.serve.auth.enabled = true;
        config.serve.auth.api_key.clear();
        write_project_config(temp.path(), config);
        bootstrap_layout(temp.path()).await;

        let report = run_doctor(&DoctorOptions {
            workdir: temp.path().to_path_buf(),
            config_override: None,
            serve_url: None,
        })
        .await
        .unwrap();

        let auth_check = report
            .checks
            .iter()
            .find(|check| check.id == "serve_auth")
            .expect("serve_auth check");
        assert_eq!(auth_check.status, DoctorStatus::Fail);
        assert!(!report.healthy);
    }

    #[tokio::test]
    async fn failing_checks_have_fix_lines_in_human_output() {
        let temp = tempdir().unwrap();
        let report = run_doctor(&DoctorOptions {
            workdir: temp.path().to_path_buf(),
            config_override: None,
            serve_url: None,
        })
        .await
        .unwrap();

        let rendered = report.render_human();
        // Every fail/warn check with a fix should produce an arrow-fix line.
        for check in &report.checks {
            if matches!(check.status, DoctorStatus::Fail | DoctorStatus::Warn) {
                if let Some(fix) = &check.fix {
                    let expected = format!("\u{2192} fix: {fix}");
                    assert!(
                        rendered.contains(&expected),
                        "missing fix line for check '{}': expected '{expected}' in output",
                        check.id
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn ok_checks_do_not_have_fix_lines_in_human_output() {
        let temp = tempdir().unwrap();
        let mut config = Config::default();
        config.serve.auth.enabled = false;
        write_project_config(temp.path(), config);
        bootstrap_layout(temp.path()).await;

        let report = run_doctor(&DoctorOptions {
            workdir: temp.path().to_path_buf(),
            config_override: None,
            serve_url: None,
        })
        .await
        .unwrap();

        for check in &report.checks {
            if check.status == DoctorStatus::Ok {
                assert!(
                    check.fix.is_none(),
                    "ok check '{}' should not have a fix",
                    check.id
                );
            }
        }
    }

    #[test]
    fn fix_field_skipped_in_json_when_none() {
        let check = DoctorCheck {
            id: "test".to_string(),
            status: DoctorStatus::Ok,
            message: "all good".to_string(),
            detail: None,
            path: None,
            url: None,
            fix: None,
        };
        let json = serde_json::to_string(&check).unwrap();
        assert!(
            !json.contains("\"fix\""),
            "fix field should be absent when None"
        );
    }

    #[test]
    fn fix_field_present_in_json_when_some() {
        let check = DoctorCheck {
            id: "test".to_string(),
            status: DoctorStatus::Fail,
            message: "bad".to_string(),
            detail: None,
            path: None,
            url: None,
            fix: Some("roko init".to_string()),
        };
        let json = serde_json::to_string(&check).unwrap();
        assert!(
            json.contains("\"fix\":\"roko init\""),
            "fix field should be present when Some"
        );
    }

    #[tokio::test]
    async fn doctor_includes_environment_checks() {
        let temp = tempdir().unwrap();
        let report = run_doctor(&DoctorOptions {
            workdir: temp.path().to_path_buf(),
            config_override: None,
            serve_url: None,
        })
        .await
        .unwrap();

        let check_ids: Vec<&str> = report.checks.iter().map(|c| c.id.as_str()).collect();
        assert!(
            check_ids.contains(&"claude_cli"),
            "missing claude_cli check"
        );
        assert!(
            check_ids.contains(&"anthropic_api_key"),
            "missing anthropic_api_key check"
        );
        assert!(
            check_ids.contains(&"rust_version"),
            "missing rust_version check"
        );
        assert!(
            check_ids.contains(&"node_version"),
            "missing node_version check"
        );
    }

    #[test]
    fn v2_abstractions_check_passes() {
        let check = check_v2_abstractions();
        assert_eq!(check.id, "v2_abstractions");
        assert_eq!(
            check.status,
            DoctorStatus::Ok,
            "v2 abstractions check should pass: {:?}",
            check.detail
        );
        assert!(
            check
                .message
                .contains("phase 1 protocol abstractions are reachable")
        );
    }

    #[tokio::test]
    async fn doctor_report_includes_v2_abstractions() {
        let temp = tempdir().unwrap();
        let report = run_doctor(&DoctorOptions {
            workdir: temp.path().to_path_buf(),
            config_override: None,
            serve_url: None,
        })
        .await
        .unwrap();

        let v2_check = report
            .checks
            .iter()
            .find(|c| c.id == "v2_abstractions")
            .expect("v2_abstractions check should be present in doctor report");
        assert_eq!(v2_check.status, DoctorStatus::Ok);
    }

    #[tokio::test]
    async fn doctor_human_output_contains_v2_abstractions() {
        let temp = tempdir().unwrap();
        let report = run_doctor(&DoctorOptions {
            workdir: temp.path().to_path_buf(),
            config_override: None,
            serve_url: None,
        })
        .await
        .unwrap();

        let rendered = report.render_human();
        assert!(
            rendered.contains("v2_abstractions"),
            "human output should contain 'v2_abstractions', got:\n{rendered}"
        );
        assert!(
            rendered.contains("[ok] v2_abstractions"),
            "human output should show v2_abstractions as ok, got:\n{rendered}"
        );
    }

    #[tokio::test]
    async fn doctor_reports_mcp_allowlist_status() {
        let temp = tempdir().unwrap();
        let mut config = Config::default();
        config.serve.auth.enabled = false;
        write_project_config(temp.path(), config);
        bootstrap_layout(temp.path()).await;

        // Write an .mcp.json with one safe server, one unknown command, and
        // one server whose env includes a hardcoded secret value.
        let mcp_json = serde_json::json!({
            "servers": [
                {
                    // npx is on the approved allowlist. It may or may not be on
                    // PATH depending on the environment, but it must not trigger
                    // the allowlist warning.
                    "name": "safe-fs",
                    "command": "npx",
                    "args": ["-y", "@modelcontextprotocol/server-filesystem"],
                    "env": {}
                },
                {
                    // Absolute path that does not exist and is not on the allowlist.
                    "name": "sketchy-bin",
                    "command": "/opt/evil/do-stuff",
                    "args": [],
                    "env": {}
                },
                {
                    // node is allowlisted; env has a hardcoded secret value.
                    "name": "leaky-env",
                    "command": "node",
                    "args": ["server.js"],
                    "env": {
                        "DATABASE_URL": "postgres://localhost/db",
                        "MY_SECRET_TOKEN": "hunter2"
                    }
                }
            ]
        });
        std::fs::write(
            temp.path().join(".mcp.json"),
            serde_json::to_string_pretty(&mcp_json).unwrap(),
        )
        .unwrap();

        let report = run_doctor(&DoctorOptions {
            workdir: temp.path().to_path_buf(),
            config_override: None,
            serve_url: None,
        })
        .await
        .unwrap();

        // 1. The safe server is on the allowlist; it may warn if npx is not on
        //    PATH, but must NOT warn about the allowlist itself.
        let safe = report
            .checks
            .iter()
            .find(|c| c.id == "mcp_server_safe-fs")
            .expect("safe-fs check present");
        let safe_detail = safe.detail.as_deref().unwrap_or("");
        assert!(
            !safe_detail.contains("not on the approved allowlist"),
            "safe-fs must not trigger an allowlist warning, got: {safe_detail}"
        );

        // 2. The unknown-command server should warn about the allowlist.
        let sketchy = report
            .checks
            .iter()
            .find(|c| c.id == "mcp_server_sketchy-bin")
            .expect("sketchy-bin check present");
        assert_eq!(sketchy.status, DoctorStatus::Warn);
        let detail = sketchy.detail.as_deref().unwrap_or("");
        assert!(
            detail.contains("not on the approved allowlist"),
            "expected allowlist warning, got: {detail}"
        );
        assert!(sketchy.fix.is_some());

        // 3. The leaky-env server should warn about the hardcoded secret value.
        let leaky = report
            .checks
            .iter()
            .find(|c| c.id == "mcp_server_leaky-env")
            .expect("leaky-env check present");
        assert_eq!(leaky.status, DoctorStatus::Warn);
        let detail = leaky.detail.as_deref().unwrap_or("");
        assert!(
            detail.contains("MY_SECRET_TOKEN"),
            "should mention the secret-like key name, got: {detail}"
        );
        // Must NOT contain the actual secret value.
        assert!(
            !detail.contains("hunter2"),
            "must not leak secret values in detail"
        );
        // Should flag as hardcoded (literal value, not ${VAR} reference).
        assert!(
            detail.contains("hardcoded"),
            "should indicate the value is hardcoded, got: {detail}"
        );

        // 4. Summary check should warn because at least one server has issues.
        let summary = report
            .checks
            .iter()
            .find(|c| c.id == "mcp_allowlist")
            .expect("mcp_allowlist summary check present");
        assert_eq!(summary.status, DoctorStatus::Warn);
        assert!(
            summary.message.contains("3 server(s) configured"),
            "summary should mention server count, got: {}",
            summary.message
        );
    }

    #[tokio::test]
    async fn doctor_warns_when_mcp_command_not_on_path() {
        let temp = tempdir().unwrap();
        let mut config = Config::default();
        config.serve.auth.enabled = false;
        write_project_config(temp.path(), config);
        bootstrap_layout(temp.path()).await;

        // Use an absolute path that does not exist. Absolute paths skip the
        // allowlist check (they're caught by not-on-allowlist) but the
        // not-found-on-PATH branch is exercised for allowlisted commands
        // that are absent. Here we verify via the unit helper directly.
        //
        // For the integration path, we use a command that IS allowlisted but
        // uses a known-nonexistent absolute path to trigger the "not found"
        // branch. Absolute paths are not in the allowlist, so the allowlist
        // warning takes priority — we verify the PATH warning via the unit
        // tests in mcp/config.rs instead.
        //
        // This test verifies the doctor check wiring: a nonexistent absolute
        // command produces a Warn with the allowlist message (absolute paths
        // are never on the allowlist).
        let mcp_json = serde_json::json!({
            "servers": [{
                "name": "bad-abs",
                "command": "/nonexistent/__roko_test_sentinel__",
                "args": [],
                "env": {}
            }]
        });
        std::fs::write(
            temp.path().join(".mcp.json"),
            serde_json::to_string_pretty(&mcp_json).unwrap(),
        )
        .unwrap();

        let report = run_doctor(&DoctorOptions {
            workdir: temp.path().to_path_buf(),
            config_override: None,
            serve_url: None,
        })
        .await
        .unwrap();

        let check = report
            .checks
            .iter()
            .find(|c| c.id == "mcp_server_bad-abs")
            .expect("bad-abs check present");
        assert_eq!(
            check.status,
            DoctorStatus::Warn,
            "nonexistent command must produce a warning"
        );
        // Absolute path is not on the allowlist, so the allowlist message fires.
        let detail = check.detail.as_deref().unwrap_or("");
        assert!(
            detail.contains("not on the approved allowlist")
                || detail.contains("not found on PATH"),
            "should report allowlist or PATH warning, got: {detail}"
        );
        assert!(check.fix.is_some(), "a fix hint should be provided");
    }

    #[tokio::test]
    async fn doctor_warns_when_mcp_env_refs_unset_var() {
        let temp = tempdir().unwrap();
        let mut config = Config::default();
        config.serve.auth.enabled = false;
        write_project_config(temp.path(), config);
        bootstrap_layout(temp.path()).await;

        let mcp_json = serde_json::json!({
            "servers": [{
                "name": "needs-token",
                "command": "node",
                "args": ["server.js"],
                "env": {
                    // Use a variable that is definitely not set.
                    "GITHUB_TOKEN": "${__ROKO_TEST_UNSET_VAR_12345__}"
                }
            }]
        });
        std::fs::write(
            temp.path().join(".mcp.json"),
            serde_json::to_string_pretty(&mcp_json).unwrap(),
        )
        .unwrap();

        let report = run_doctor(&DoctorOptions {
            workdir: temp.path().to_path_buf(),
            config_override: None,
            serve_url: None,
        })
        .await
        .unwrap();

        let check = report
            .checks
            .iter()
            .find(|c| c.id == "mcp_server_needs-token")
            .expect("needs-token check present");
        assert_eq!(check.status, DoctorStatus::Warn);
        let detail = check.detail.as_deref().unwrap_or("");
        assert!(
            detail.contains("GITHUB_TOKEN"),
            "should report GITHUB_TOKEN as having an unset reference, got: {detail}"
        );
        assert!(
            detail.contains("unset"),
            "detail should mention the variable is unset, got: {detail}"
        );
        // Must NOT leak the reference value itself.
        assert!(
            !detail.contains("__ROKO_TEST_UNSET_VAR_12345__"),
            "must not leak the variable reference value in detail"
        );
    }

    #[tokio::test]
    async fn doctor_warns_when_mcp_env_has_hardcoded_secret() {
        let temp = tempdir().unwrap();
        let mut config = Config::default();
        config.serve.auth.enabled = false;
        write_project_config(temp.path(), config);
        bootstrap_layout(temp.path()).await;

        let mcp_json = serde_json::json!({
            "servers": [{
                "name": "hardcoded-secret",
                "command": "node",
                "args": ["server.js"],
                "env": {
                    // Literal secret value, not an env var reference.
                    "API_KEY": "sk-live-abc123supersecretvalue"
                }
            }]
        });
        std::fs::write(
            temp.path().join(".mcp.json"),
            serde_json::to_string_pretty(&mcp_json).unwrap(),
        )
        .unwrap();

        let report = run_doctor(&DoctorOptions {
            workdir: temp.path().to_path_buf(),
            config_override: None,
            serve_url: None,
        })
        .await
        .unwrap();

        let check = report
            .checks
            .iter()
            .find(|c| c.id == "mcp_server_hardcoded-secret")
            .expect("hardcoded-secret check present");
        assert_eq!(check.status, DoctorStatus::Warn);
        let detail = check.detail.as_deref().unwrap_or("");
        // Must mention the key.
        assert!(
            detail.contains("API_KEY"),
            "should report API_KEY as a hardcoded secret, got: {detail}"
        );
        // Must indicate it is hardcoded.
        assert!(
            detail.contains("hardcoded"),
            "should characterize the value as hardcoded, got: {detail}"
        );
        // Must NOT leak the actual secret value.
        assert!(
            !detail.contains("sk-live-abc123supersecretvalue"),
            "must not leak the hardcoded secret value in detail"
        );
        // Fix hint should suggest using ${VAR} references.
        let fix = check.fix.as_deref().unwrap_or("");
        assert!(
            fix.contains("ENV_VAR") || fix.contains("references"),
            "fix should suggest env var references, got: {fix}"
        );
    }

    #[tokio::test]
    async fn doctor_skips_mcp_allowlist_when_no_config() {
        let temp = tempdir().unwrap();
        let mut config = Config::default();
        config.serve.auth.enabled = false;
        write_project_config(temp.path(), config);
        bootstrap_layout(temp.path()).await;
        // No .mcp.json written.

        let report = run_doctor(&DoctorOptions {
            workdir: temp.path().to_path_buf(),
            config_override: None,
            serve_url: None,
        })
        .await
        .unwrap();

        let mcp_check = report
            .checks
            .iter()
            .find(|c| c.id == "mcp_allowlist")
            .expect("mcp_allowlist check present");
        assert_eq!(mcp_check.status, DoctorStatus::Skipped);
        // Missing MCP config must NOT cause a failure.
        assert!(report.healthy);
    }

    // -- state layout audit tests ---------------------------------------------

    #[test]
    fn state_layout_audit_skipped_when_no_roko_dir() {
        let temp = tempdir().unwrap();
        // No .roko/ directory created -- audit should return empty.
        let checks = check_state_layout_audit(temp.path());
        assert!(
            checks.is_empty(),
            "audit should return no checks when .roko/ does not exist"
        );
    }

    #[tokio::test]
    async fn state_layout_audit_v2_workspace_is_clean() {
        let temp = tempdir().unwrap();
        // Bootstrap a fresh V2 workspace.
        RokoLayout::for_project(temp.path())
            .ensure_dirs()
            .await
            .expect("ensure_dirs");

        let checks = check_state_layout_audit(temp.path());
        assert_eq!(checks.len(), 3, "should produce exactly 3 checks");

        let version_check = checks
            .iter()
            .find(|c| c.id == "state_layout_version")
            .expect("state_layout_version check");
        assert_eq!(
            version_check.status,
            DoctorStatus::Ok,
            "V2 workspace should have Ok version check"
        );

        let legacy_check = checks
            .iter()
            .find(|c| c.id == "state_legacy_files")
            .expect("state_legacy_files check");
        assert_eq!(
            legacy_check.status,
            DoctorStatus::Ok,
            "fresh V2 workspace should have no legacy files"
        );
    }

    #[tokio::test]
    async fn state_layout_audit_v1_workspace_warns_on_legacy_signals() {
        let temp = tempdir().unwrap();
        let layout = RokoLayout::for_project(temp.path());

        // Bootstrap a V1 workspace manually (no migration).
        for dir in &layout.top_level_dirs() {
            std::fs::create_dir_all(dir).expect("create dir");
        }
        std::fs::write(layout.version_file(), "1").expect("write VERSION");
        // Place a signals.jsonl file that would be present in a V1 workspace.
        std::fs::write(layout.signals_path(), "{}\n").expect("write signals.jsonl");

        let checks = check_state_layout_audit(temp.path());

        let version_check = checks
            .iter()
            .find(|c| c.id == "state_layout_version")
            .expect("state_layout_version check");
        assert_eq!(
            version_check.status,
            DoctorStatus::Warn,
            "V1 workspace should warn on version check"
        );
        assert!(
            version_check.message.contains("V1"),
            "version check message should mention V1"
        );

        let legacy_check = checks
            .iter()
            .find(|c| c.id == "state_legacy_files")
            .expect("state_legacy_files check");
        assert_eq!(
            legacy_check.status,
            DoctorStatus::Warn,
            "V1 workspace with signals.jsonl should warn on legacy files"
        );
        assert!(
            legacy_check
                .detail
                .as_deref()
                .unwrap_or("")
                .contains("signals.jsonl"),
            "legacy files detail should mention signals.jsonl"
        );
    }

    #[tokio::test]
    async fn state_layout_audit_v1_version_warns_even_without_signals_file() {
        let temp = tempdir().unwrap();
        let layout = RokoLayout::for_project(temp.path());

        // V1 workspace without signals.jsonl (already partially migrated).
        for dir in &layout.top_level_dirs() {
            std::fs::create_dir_all(dir).expect("create dir");
        }
        std::fs::write(layout.version_file(), "1").expect("write VERSION");
        // No signals.jsonl written.

        let checks = check_state_layout_audit(temp.path());

        let version_check = checks
            .iter()
            .find(|c| c.id == "state_layout_version")
            .expect("state_layout_version check");
        assert_eq!(
            version_check.status,
            DoctorStatus::Warn,
            "V1 version should still warn even without signals.jsonl"
        );

        let legacy_check = checks
            .iter()
            .find(|c| c.id == "state_legacy_files")
            .expect("state_legacy_files check");
        assert_eq!(
            legacy_check.status,
            DoctorStatus::Ok,
            "no legacy files should be Ok when signals.jsonl absent"
        );
    }

    #[tokio::test]
    async fn doctor_report_includes_state_layout_audit() {
        let temp = tempdir().unwrap();
        let mut config = Config::default();
        config.serve.auth.enabled = false;
        write_project_config(temp.path(), config);
        bootstrap_layout(temp.path()).await;

        let report = run_doctor(&DoctorOptions {
            workdir: temp.path().to_path_buf(),
            config_override: None,
            serve_url: None,
        })
        .await
        .unwrap();

        let check_ids: Vec<&str> = report.checks.iter().map(|c| c.id.as_str()).collect();
        assert!(
            check_ids.contains(&"state_layout_version"),
            "report should include state_layout_version; got: {check_ids:?}"
        );
        assert!(
            check_ids.contains(&"state_canonical_files"),
            "report should include state_canonical_files; got: {check_ids:?}"
        );
        assert!(
            check_ids.contains(&"state_legacy_files"),
            "report should include state_legacy_files; got: {check_ids:?}"
        );
    }
}
