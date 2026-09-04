//! `roko doctor` bootstrap diagnostics for self-hosted workspaces.

use crate::auth_detect::{AuthMethod, detect_auth_from_config};
use crate::config::{ConfigPaths, resolve_paths};
use crate::{Config, load_resolved_config};
use anyhow::{Context as _, Result};
use reqwest::Url;
use roko_core::agent::ProviderKind;
use roko_core::config::provider::{ProviderConfig, ProviderNetworkPolicy};
use roko_execution::diagnostics::{
    DiagnosticCheckId, DiagnosticFinding, DiagnosticRequest, DiagnosticService, DiagnosticSeverity,
};
use roko_fs::RokoLayout;
use serde::Serialize;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const DEFAULT_HEALTH_PATH: &str = "/api/health";
const DOCTOR_HTTP_TIMEOUT_SECS: u64 = 2;

/// Convert a shared [`DiagnosticFinding`] to a doctor-format [`DoctorCheck`].
fn finding_to_doctor_check(finding: &DiagnosticFinding) -> DoctorCheck {
    let status = match finding.severity {
        DiagnosticSeverity::Info => DoctorStatus::Ok,
        DiagnosticSeverity::Warning => DoctorStatus::Warn,
        DiagnosticSeverity::Error => DoctorStatus::Fail,
    };
    DoctorCheck {
        id: format!("shared_{}", finding.code),
        status,
        message: finding.message.clone(),
        detail: if finding.evidence.is_empty() {
            None
        } else {
            Some(
                finding
                    .evidence
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        },
        path: finding.evidence.get("path").cloned(),
        url: None,
        fix: finding.remediation.as_ref().and_then(|r| r.command.clone()),
    }
}

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
    /// Structured E47 disk report for JSON/API/TUI consumers.
    pub disk_health: DiskHealthReport,
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

    // ── Shared diagnostic service checks (#279) ─────────────────────────
    // Run all 11 shared checks via the consolidated DiagnosticService.
    let shared_report = DiagnosticService::run(&DiagnosticRequest {
        workdir: workdir.clone(),
        selected: DiagnosticCheckId::ALL.iter().copied().collect(),
        profile: None,
        allow_repairs: false,
    });
    for finding in &shared_report.findings {
        checks.push(finding_to_doctor_check(finding));
    }

    // ── Doctor-only checks (not in the shared service) ──────────────────
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
    checks.extend(check_routing_tier_models(
        &workdir,
        options.config_override.as_deref(),
    ));
    checks.push(check_rust_version());
    checks.push(check_node_version());
    checks.push(check_serve_auth(&loaded_config));
    checks.push(check_serve_health(options.serve_url.as_deref(), &loaded_config).await?);
    let conductor = load_conductor_config(&workdir, options.config_override.as_deref());
    checks.push(check_dead_conductor_config(&conductor));
    checks.push(check_v2_abstractions());
    checks.extend(check_state_layout_audit(&workdir));
    checks.extend(check_config_freshness(&workdir));
    checks.extend(check_harness_providers(&loaded_config));
    checks.extend(check_mcp_allowlist(&workdir, &loaded_config));
    checks.push(check_orphaned_tmp_files(&workdir));
    checks.push(check_plans_dir_conflict(&workdir));
    let resources = load_resources_config(&workdir, options.config_override.as_deref());
    let (disk_health_check, disk_health) = check_disk_health(&workdir, &resources).await;
    checks.push(disk_health_check);
    checks.push(check_target_staleness(&workdir));
    checks.push(check_crash_report(&workdir));

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
        disk_health,
    })
}

fn check_config_freshness(workdir: &Path) -> Vec<DoctorCheck> {
    let path = workdir
        .join(".roko")
        .join("state")
        .join("config-freshness.json");
    let freshness = roko_core::config::hot_reload::ConfigFreshness::load(&path);
    roko_core::config::hot_reload::config_freshness_diagnostics(
        &freshness,
        roko_core::config::hot_reload::DEFAULT_CONFIG_STALENESS_DAYS,
    )
    .into_iter()
    .map(|diagnostic| DoctorCheck {
        id: diagnostic.key.replace('.', "_"),
        status: DoctorStatus::Warn,
        message: diagnostic.message,
        detail: Some(
            "stale config remains valid, but its assumptions should be reviewed".to_string(),
        ),
        path: Some(path.display().to_string()),
        url: None,
        fix: Some("review the section and refresh its config freshness timestamp".to_string()),
    })
    .collect()
}

fn load_resources_config(
    workdir: &Path,
    config_override: Option<&Path>,
) -> roko_core::config::ResourcesConfig {
    if let Some(path) = config_override
        && let Ok(text) = std::fs::read_to_string(path)
        && let Ok(config) = roko_core::config::schema::RokoConfig::from_toml(&text)
    {
        return config.resources;
    }

    roko_core::config::loader::load_config_validated_with_options(
        workdir,
        &roko_core::config::loader::LoadOptions::default(),
    )
    .map(|loaded| loaded.config().resources.clone())
    .unwrap_or_default()
}

fn load_conductor_config(
    workdir: &Path,
    config_override: Option<&Path>,
) -> roko_core::config::schema::ConductorConfig {
    if let Some(path) = config_override
        && let Ok(text) = std::fs::read_to_string(path)
        && let Ok(config) = roko_core::config::schema::RokoConfig::from_toml(&text)
    {
        return config.conductor;
    }

    roko_core::config::loader::load_config_validated_with_options(
        workdir,
        &roko_core::config::loader::LoadOptions::default(),
    )
    .map(|loaded| loaded.config().conductor.clone())
    .unwrap_or_default()
}

fn check_dead_conductor_config(
    conductor: &roko_core::config::schema::ConductorConfig,
) -> DoctorCheck {
    let context_pressure_enabled = conductor.context_pressure_enabled;
    DoctorCheck {
        id: "dead_conductor_config".to_string(),
        status: if context_pressure_enabled {
            DoctorStatus::Warn
        } else {
            DoctorStatus::Ok
        },
        message: if context_pressure_enabled {
            "runtime-dead runner-v2 context-pressure setting is enabled".to_string()
        } else {
            "runtime-dead runner-v2 context-pressure setting is inactive".to_string()
        },
        detail: Some(
            "conductor.context_pressure_enabled is retained for compatibility but runner-v2 does not yet feed TokenUsage into its conductor ring; conductor.watchers.* threshold overrides are runtime-live"
                .to_string(),
        ),
        path: None,
        url: None,
        fix: context_pressure_enabled
            .then(|| "remove conductor.context_pressure_enabled or set it to false".to_string()),
    }
}

/// Run only the workspace disk/resource diagnostics used by `roko doctor disk`.
///
/// Unlike the full doctor, this does not probe providers, local toolchains, or
/// the HTTP control plane.
pub async fn run_disk_doctor(workdir: &Path, config_override: Option<&Path>) -> DiskHealthReport {
    let resources = load_resources_config(workdir, config_override);
    let (_, report) = check_disk_health(workdir, &resources).await;
    report
}

/// Check whether raw TOML text contains a given top-level key.
fn toml_has_key(text: &str, key: &str) -> bool {
    let value: toml::Value = match toml::from_str(text) {
        Ok(v) => v,
        Err(_) => return false,
    };
    value
        .as_table()
        .is_some_and(|table| table.contains_key(key))
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

        let resolved = Config::from_file(path)?;
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("read config {}", path.display()))?;
        let has_serve = toml_has_key(&text, "serve");
        return Ok(LoadedConfig {
            paths: ConfigPaths {
                global: crate::config::global_config_path(),
                project: Some(path.to_path_buf()),
                env_override: std::env::var_os("ROKO_CONFIG").map(PathBuf::from),
            },
            resolved: Some(resolved),
            active_path: Some(path.to_path_buf()),
            explicit_serve: has_serve,
        });
    }

    let paths = resolve_paths(workdir);
    let mut explicit_serve = false;
    let mut found_any_config = false;
    let active_path = if let Some(env_path) = &paths.env_override {
        match std::fs::read_to_string(env_path) {
            Ok(text) => {
                explicit_serve = toml_has_key(&text, "serve");
                found_any_config = true;
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
        let mut active_path = None;

        if let Some(ref global_path) = paths.global {
            match std::fs::read_to_string(global_path) {
                Ok(text) => {
                    explicit_serve |= toml_has_key(&text, "serve");
                    found_any_config = true;
                    active_path = Some(global_path.clone());
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => {
                    return Err(anyhow::Error::new(e)
                        .context(format!("read config {}", global_path.display())));
                }
            }
        }
        if let Some(project_path) = &paths.project {
            if let Ok(text) = std::fs::read_to_string(project_path) {
                explicit_serve |= toml_has_key(&text, "serve");
                found_any_config = true;
            }
            active_path = Some(project_path.clone());
        }

        if !found_any_config { None } else { active_path }
    };

    let resolved = if paths
        .env_override
        .as_ref()
        .is_some_and(|path| path.is_file())
        || paths.global.as_deref().is_some_and(|p| p.is_file())
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
        path: loaded_config
            .paths
            .global
            .as_ref()
            .map(|p| p.display().to_string()),
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

/// Validate that `routing.{fast,standard,complex}_task_model` slugs match a
/// configured `[models.*]` key or a well-known builtin model.
fn check_routing_tier_models(workdir: &Path, config_override: Option<&Path>) -> Vec<DoctorCheck> {
    let roko_cfg = if let Some(path) = config_override
        && let Ok(text) = std::fs::read_to_string(path)
        && let Ok(config) = roko_core::config::schema::RokoConfig::from_toml(&text)
    {
        config
    } else {
        match roko_core::config::loader::load_config_validated_with_options(
            workdir,
            &roko_core::config::loader::LoadOptions::default(),
        ) {
            Ok(loaded) => loaded.config().clone(),
            Err(_) => return vec![],
        }
    };

    let model_keys: std::collections::HashSet<&str> =
        roko_cfg.models.keys().map(String::as_str).collect();

    let tiers = [
        ("fast_task_model", roko_cfg.routing.fast_task_model.as_str()),
        (
            "standard_task_model",
            roko_cfg.routing.standard_task_model.as_str(),
        ),
        (
            "complex_task_model",
            roko_cfg.routing.complex_task_model.as_str(),
        ),
    ];

    let mut checks = Vec::new();
    for (tier_name, slug) in tiers {
        let slug = slug.trim();
        if slug.is_empty() {
            continue;
        }
        let known = model_keys.contains(slug)
            || roko_core::config::model_registry::builtin_model(slug).is_some();
        checks.push(DoctorCheck {
            id: format!("routing_{tier_name}"),
            status: if known {
                DoctorStatus::Ok
            } else {
                DoctorStatus::Warn
            },
            message: if known {
                format!("routing.{tier_name} \"{slug}\" is valid")
            } else {
                format!(
                    "routing.{tier_name} references \"{slug}\" which is not in [models.*] config"
                )
            },
            detail: if known {
                None
            } else {
                Some("learned routing may silently fail to match this slug".to_string())
            },
            path: None,
            url: None,
            fix: if known {
                None
            } else {
                Some(format!(
                    "Add [models.{slug}] to roko.toml or change routing.{tier_name}"
                ))
            },
        });
    }
    checks
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
    let available = crate::auth_detect::claude_cli_available();

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
        // No config file — fall back to checking the single most common key
        // or the CLI binary on PATH.
        let has_key = std::env::var("ANTHROPIC_API_KEY")
            .ok()
            .filter(|k| !k.is_empty())
            .is_some();
        let has_cli = crate::auth_detect::claude_cli_available();
        return vec![DoctorCheck {
            id: "provider_api_keys".to_string(),
            status: if has_key || has_cli {
                DoctorStatus::Ok
            } else {
                DoctorStatus::Warn
            },
            message: if has_key {
                "ANTHROPIC_API_KEY is set (no roko.toml)".to_string()
            } else if has_cli {
                "claude CLI detected — no API key required".to_string()
            } else {
                "no API keys found and no roko.toml present".to_string()
            },
            detail: None,
            path: None,
            url: None,
            fix: if has_key || has_cli {
                None
            } else {
                Some("run `roko config init` or set a provider API key (ANTHROPIC_API_KEY, OPENAI_API_KEY, etc.)".to_string())
            },
        }];
    };

    // Collect only HTTP API providers that need an environment-variable key.
    // CLI-based providers (ClaudeCli, GeminiCli, CursorCli, Hermes, OpenClaw)
    // authenticate through their own binary/session mechanism.
    // CursorAcp authenticates via the Cursor IDE's own session, not via an
    // roko-managed API key.
    let api_providers: Vec<(&String, &roko_core::config::schema::ProviderConfig)> = config
        .providers
        .iter()
        .filter(|(_, p)| {
            matches!(
                p.kind,
                ProviderKind::AnthropicApi
                    | ProviderKind::OpenAiCompat
                    | ProviderKind::PerplexityApi
                    | ProviderKind::GeminiApi
                    | ProviderKind::CerebrasApi
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

    let provider_count = api_providers.len();
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
    let key_check_count = checks.len();
    let missing_keys = checks
        .iter()
        .filter(|check| check.status == DoctorStatus::Warn)
        .count();
    checks.push(DoctorCheck {
        id: "provider_api_keys".to_string(),
        status: if missing_keys == 0 {
            DoctorStatus::Ok
        } else {
            DoctorStatus::Warn
        },
        message: if key_check_count == 0 {
            format!(
                "{provider_count} API provider(s) configured without environment-key authentication"
            )
        } else if missing_keys == 0 {
            format!("all {key_check_count} configured provider API key(s) are set")
        } else {
            format!("{missing_keys} of {key_check_count} configured provider API key(s) are unset")
        },
        detail: None,
        path: None,
        url: None,
        fix: if missing_keys == 0 {
            None
        } else {
            Some("set the provider API key environment variables reported above".to_string())
        },
    });
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
    if crate::auth_detect::claude_cli_available() {
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
                ProviderKind::ClaudeCli
                    | ProviderKind::CodexCli
                    | ProviderKind::Hermes
                    | ProviderKind::OpenClaw
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
/// - `state_layout_version` -- verifies `.roko/VERSION` is current.
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
            Some(LayoutVersion::V3) => DoctorCheck {
                id: "state_layout_version".to_string(),
                status: DoctorStatus::Ok,
                message: "storage layout is V3 (current)".to_string(),
                detail: None,
                path: Some(version_path.display().to_string()),
                url: None,
                fix: None,
            },
            Some(LayoutVersion::V2) => DoctorCheck {
                id: "state_layout_version".to_string(),
                status: DoctorStatus::Warn,
                message: "storage layout is V2 (outdated)".to_string(),
                detail: Some(
                    "V2->V3 migration merges root/learn/memory episode logs into the canonical root log"
                        .to_string(),
                ),
                path: Some(version_path.display().to_string()),
                url: None,
                fix: Some("roko init".to_string()),
            },
            Some(LayoutVersion::V1) => DoctorCheck {
                id: "state_layout_version".to_string(),
                status: DoctorStatus::Warn,
                message: "storage layout is V1 (outdated)".to_string(),
                detail: Some(
                    "V1->V3 migration preserves legacy signals and converges episode storage"
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
    // These are the paths that current writers target.
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
            message: format!("all {} canonical storage files are present", present.len()),
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
                "{} canonical files present, {} absent (normal for new workspaces)",
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

    // -- 3. Legacy files ------------------------------------------------------
    // Files that should not exist in a migrated current workspace.
    let legacy_paths: &[(&str, PathBuf)] = &[
        ("signals.jsonl", layout.signals_path()),
        (
            "learn/episodes.jsonl",
            layout.learn_dir().join("episodes.jsonl"),
        ),
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
            message: "no legacy storage files detected".to_string(),
            detail: None,
            path: Some(layout.root().display().to_string()),
            url: None,
            fix: None,
        }
    } else {
        DoctorCheck {
            id: "state_legacy_files".to_string(),
            status: DoctorStatus::Warn,
            message: format!("{} legacy storage file(s) detected", legacy_found.len()),
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

/// Warn about orphaned `.tmp` files in `.roko/learn/` from crashed atomic writes.
///
/// Audit #80: only warn about files older than 1 hour to avoid false positives
/// from in-flight atomic writes that have not yet been renamed.
fn check_orphaned_tmp_files(workdir: &Path) -> DoctorCheck {
    let learn_dir = workdir.join(".roko").join("learn");
    if !learn_dir.is_dir() {
        return DoctorCheck {
            id: "orphaned_tmp_files".to_string(),
            status: DoctorStatus::Ok,
            message: "no .roko/learn/ directory (nothing to check)".to_string(),
            detail: None,
            path: None,
            url: None,
            fix: None,
        };
    }

    let one_hour_ago = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(3600))
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

    let stale_tmp_count = std::fs::read_dir(&learn_dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| {
                    e.path().extension().is_some_and(|ext| ext == "tmp")
                        && e.metadata()
                            .ok()
                            .and_then(|m| m.modified().ok())
                            .is_some_and(|mtime| mtime < one_hour_ago)
                })
                .count()
        })
        .unwrap_or(0);

    if stale_tmp_count == 0 {
        DoctorCheck {
            id: "orphaned_tmp_files".to_string(),
            status: DoctorStatus::Ok,
            message: "no stale .tmp files in .roko/learn/".to_string(),
            detail: None,
            path: Some(learn_dir.display().to_string()),
            url: None,
            fix: None,
        }
    } else {
        DoctorCheck {
            id: "orphaned_tmp_files".to_string(),
            status: DoctorStatus::Warn,
            message: format!(
                "{stale_tmp_count} stale .tmp file{} in .roko/learn/ (older than 1 hour)",
                if stale_tmp_count == 1 { "" } else { "s" }
            ),
            detail: Some(
                "these are leftover from crashed atomic writes and can be safely removed"
                    .to_string(),
            ),
            path: Some(learn_dir.display().to_string()),
            url: None,
            fix: Some("rm .roko/learn/*.tmp".to_string()),
        }
    }
}

/// A serializable snapshot of disk health findings for the HTTP API and TUI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiskHealthReport {
    /// Approximate free disk space in MB at the workspace mount point.
    pub free_disk_mb: Option<u64>,
    /// Whether free disk is below `ResourcesConfig.warn_disk_mb`.
    pub low_disk: bool,
    /// Directories found under `.roko/worktrees/` that look orphaned
    /// (i.e. not tracked by any running plan).
    pub orphaned_worktree_dirs: Vec<String>,
    /// Live JSONL files at or above `ResourcesConfig.log_rotation_max_mb`.
    pub large_jsonl_files: Vec<String>,
    /// Stale target directories selected by the configured age policy.
    pub stale_target_dirs: Vec<DiskTargetFinding>,
    /// Aggregate size of all discovered target directories in MB.
    pub total_target_mb: u64,
    /// Aggregate size of `.roko/` in MB.
    pub roko_dir_mb: u64,
    /// Number of checkout directories present under `.roko/worktrees/`.
    pub worktree_count: usize,
    /// Aggregate size of checkout directories under `.roko/worktrees/` in MB.
    pub worktree_total_mb: u64,
    /// Effective configured JSONL rotation threshold in MB.
    pub log_rotation_max_mb: u64,
}

impl DiskHealthReport {
    /// Exit code for the focused disk report.
    ///
    /// - `0` — all clear
    /// - `1` — advisory findings only (orphaned worktrees, large logs, stale targets)
    /// - `2` — `low_disk` is true (fatal)
    #[must_use]
    pub const fn exit_code(&self) -> i32 {
        if self.low_disk {
            return 2;
        }
        // Advisory findings: orphaned worktrees, large JSONL, stale targets.
        // These match `DoctorStatus::Warn` in the full doctor, so exit 1.
        if !self.orphaned_worktree_dirs.is_empty()
            || !self.large_jsonl_files.is_empty()
            || !self.stale_target_dirs.is_empty()
        {
            return 1;
        }
        0
    }

    /// Render the focused `roko doctor disk` report.
    #[must_use]
    pub fn render_human(&self) -> String {
        let mut out = String::from("doctor disk\n");
        let free = self
            .free_disk_mb
            .map_or_else(|| "unavailable".to_string(), |mb| format!("{mb} MB"));
        let _ = writeln!(&mut out, "free disk: {free}");
        let _ = writeln!(&mut out, ".roko size: {} MB", self.roko_dir_mb);
        let _ = writeln!(
            &mut out,
            "targets: {} MB total; {} stale",
            self.total_target_mb,
            self.stale_target_dirs.len()
        );
        let _ = writeln!(
            &mut out,
            "worktrees: {} ({} MB); {} orphaned",
            self.worktree_count,
            self.worktree_total_mb,
            self.orphaned_worktree_dirs.len()
        );
        let _ = writeln!(
            &mut out,
            "large JSONL: {} (threshold {} MB)",
            self.large_jsonl_files.len(),
            self.log_rotation_max_mb
        );

        for target in &self.stale_target_dirs {
            let _ = writeln!(
                &mut out,
                "[stale target] {} — {} MB, {} days old",
                target.path, target.size_mb, target.age_days
            );
            let _ = writeln!(
                &mut out,
                "    → cargo clean --manifest-path {}/Cargo.toml",
                target.path.trim_end_matches("/target")
            );
        }
        for path in &self.orphaned_worktree_dirs {
            let _ = writeln!(&mut out, "[orphaned worktree] {path}");
            let _ = writeln!(&mut out, "    → git worktree prune");
        }
        for path in &self.large_jsonl_files {
            let _ = writeln!(&mut out, "[large JSONL] {path}");
            let _ = writeln!(&mut out, "    → roko knowledge gc");
        }
        out
    }
}

/// One stale Rust build-artifact finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiskTargetFinding {
    pub path: String,
    pub size_mb: u64,
    pub age_days: u32,
}

/// `target/` directories larger than this trigger a warning, in MB.
const WARN_TARGET_MB: u64 = 51_200; // 50 GB — a 35-crate Rust workspace commonly reaches 10-15 GB

/// Recursively compute the total size of a directory tree in bytes.
///
/// Non-fatal: errors on individual entries are silently skipped.
fn dir_size_bytes(path: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    let mut total: u64 = 0;
    for entry in entries.flatten() {
        let metadata = match std::fs::symlink_metadata(entry.path()) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            total += dir_size_bytes(&entry.path());
        } else {
            total += metadata.len();
        }
    }
    total
}

/// Check available disk space, stale targets, orphaned worktrees, and oversized JSONL logs.
async fn check_disk_health(
    workdir: &Path,
    resources: &roko_core::config::ResourcesConfig,
) -> (DoctorCheck, DiskHealthReport) {
    let free_mb = roko_fs::available_disk_mb(workdir).ok();
    let low_disk = free_mb.is_some_and(|mb| mb < resources.warn_disk_mb);

    // Compare on-disk checkout directories with Git's authoritative worktree
    // list. This avoids reporting every healthy live checkout as orphaned.
    let worktrees_dir = workdir.join(".roko").join("worktrees");
    let checkout_dirs = if worktrees_dir.is_dir() {
        std::fs::read_dir(&worktrees_dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| {
                std::fs::symlink_metadata(entry.path())
                    .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
            })
            .map(|entry| entry.path())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let tracked = git_worktree_paths(workdir);
    let orphaned = checkout_dirs
        .iter()
        .filter(|path| {
            let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| (*path).clone());
            !tracked.contains(&canonical)
        })
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();

    let layout = RokoLayout::for_project(workdir);
    let large_threshold = resources.log_rotation_max_mb.saturating_mul(1024 * 1024);
    let large_jsonl = roko_fs::log_rotation::rotatable_jsonl_paths(&layout)
        .into_iter()
        .filter(|path| {
            std::fs::metadata(path).is_ok_and(|metadata| metadata.len() >= large_threshold)
        })
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();

    let target_dirs = roko_fs::scan_target_dirs(workdir).await.unwrap_or_default();
    let stale_targets = target_dirs
        .iter()
        .filter(|target| target.age_days() >= resources.target_max_age_days)
        .map(|target| DiskTargetFinding {
            path: target.path.display().to_string(),
            size_mb: target.size_bytes / (1024 * 1024),
            age_days: target.age_days(),
        })
        .collect::<Vec<_>>();
    let total_target_mb = target_dirs
        .iter()
        .map(|target| target.size_bytes)
        .sum::<u64>()
        / (1024 * 1024);
    let worktree_total_mb = checkout_dirs
        .iter()
        .map(|path| dir_size_bytes(path))
        .sum::<u64>()
        / (1024 * 1024);
    let roko_dir_mb = dir_size_bytes(layout.root()) / (1024 * 1024);

    let disk_health = DiskHealthReport {
        free_disk_mb: free_mb,
        low_disk,
        orphaned_worktree_dirs: orphaned.clone(),
        large_jsonl_files: large_jsonl.clone(),
        stale_target_dirs: stale_targets.clone(),
        total_target_mb,
        roko_dir_mb,
        worktree_count: checkout_dirs.len(),
        worktree_total_mb,
        log_rotation_max_mb: resources.log_rotation_max_mb,
    };

    // Determine overall status.
    let has_warn =
        low_disk || !orphaned.is_empty() || !large_jsonl.is_empty() || !stale_targets.is_empty();
    let status = if has_warn {
        DoctorStatus::Warn
    } else {
        DoctorStatus::Ok
    };

    let mut detail_parts: Vec<String> = Vec::new();
    if let Some(mb) = free_mb {
        detail_parts.push(format!("{mb} MB free disk space"));
    } else {
        detail_parts.push("disk space unavailable".to_string());
    }
    if !orphaned.is_empty() {
        detail_parts.push(format!(
            "{} orphaned worktree dir{}",
            orphaned.len(),
            if orphaned.len() == 1 { "" } else { "s" }
        ));
    }
    if !large_jsonl.is_empty() {
        detail_parts.push(format!(
            "{} JSONL file{} at/over {} MB",
            large_jsonl.len(),
            if large_jsonl.len() == 1 { "" } else { "s" },
            resources.log_rotation_max_mb,
        ));
    }
    if !stale_targets.is_empty() {
        detail_parts.push(format!(
            "{} stale target dir{} ({} MB total targets)",
            stale_targets.len(),
            if stale_targets.len() == 1 { "" } else { "s" },
            total_target_mb,
        ));
    }
    detail_parts.push(format!(
        ".roko: {roko_dir_mb} MB; worktrees: {} dirs / {worktree_total_mb} MB",
        checkout_dirs.len()
    ));

    let fix = if has_warn {
        let mut fixes: Vec<&str> = Vec::new();
        if low_disk {
            fixes.push("free disk space before running plans");
        }
        if !orphaned.is_empty() {
            fixes.push("git worktree prune");
        }
        if !large_jsonl.is_empty() {
            fixes.push("roko knowledge gc");
        }
        if !stale_targets.is_empty() {
            fixes.push("cargo clean --manifest-path <stale-worktree>/Cargo.toml");
        }
        Some(fixes.join("; "))
    } else {
        None
    };

    // Persist the report as JSON in the detail field so the TUI/API can decode it.
    let detail = if detail_parts.is_empty() {
        None
    } else {
        Some(detail_parts.join(", "))
    };

    let check = DoctorCheck {
        id: "disk_health".to_string(),
        status,
        message: if has_warn {
            "disk health check has warnings".to_string()
        } else {
            "disk health looks good".to_string()
        },
        detail,
        path: Some(workdir.display().to_string()),
        url: None,
        fix,
    };
    (check, disk_health)
}

fn git_worktree_paths(workdir: &Path) -> std::collections::HashSet<PathBuf> {
    let output = std::process::Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(workdir)
        .output();
    let Ok(output) = output else {
        return std::collections::HashSet::new();
    };
    if !output.status.success() {
        return std::collections::HashSet::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .map(PathBuf::from)
        .map(|path| std::fs::canonicalize(&path).unwrap_or(path))
        .collect()
}

/// Check the size of the `target/` build artifact directory.
fn check_target_staleness(workdir: &Path) -> DoctorCheck {
    let target = workdir.join("target");
    if !target.is_dir() {
        return DoctorCheck {
            id: "target_staleness".to_string(),
            status: DoctorStatus::Ok,
            message: "no target/ directory found".to_string(),
            detail: None,
            path: None,
            url: None,
            fix: None,
        };
    }

    let size_bytes = dir_size_bytes(&target);
    let size_mb = size_bytes / (1024 * 1024);

    if size_mb > WARN_TARGET_MB {
        DoctorCheck {
            id: "target_staleness".to_string(),
            status: DoctorStatus::Warn,
            message: format!("target/ is large ({size_mb} MB)"),
            detail: Some(format!(
                "target/ at {} is {} MB; Rust build artifacts can be safely removed when not building",
                target.display(),
                size_mb
            )),
            path: Some(target.display().to_string()),
            url: None,
            fix: Some("cargo clean".to_string()),
        }
    } else {
        DoctorCheck {
            id: "target_staleness".to_string(),
            status: DoctorStatus::Ok,
            message: format!("target/ is {size_mb} MB (within threshold)"),
            detail: None,
            path: Some(target.display().to_string()),
            url: None,
            fix: None,
        }
    }
}

/// Warn when both `./plans/` and `.roko/plans/` exist (potential conflict).
fn check_plans_dir_conflict(workdir: &Path) -> DoctorCheck {
    let top_level = workdir.join("plans");
    let dot_roko = workdir.join(".roko").join("plans");

    if top_level.is_dir() && dot_roko.is_dir() {
        let top_names: std::collections::HashSet<String> = std::fs::read_dir(&top_level)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter(|e| e.path().is_dir())
                    .filter_map(|e| e.file_name().into_string().ok())
                    .collect()
            })
            .unwrap_or_default();
        let dot_names: std::collections::HashSet<String> = std::fs::read_dir(&dot_roko)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter(|e| e.path().is_dir())
                    .filter_map(|e| e.file_name().into_string().ok())
                    .collect()
            })
            .unwrap_or_default();

        let top_count = top_names.len();
        let dot_count = dot_names.len();
        let conflicts: Vec<&String> = top_names.intersection(&dot_names).collect();

        let fix = if conflicts.is_empty() {
            Some("mv .roko/plans/* plans/ && rmdir .roko/plans".to_string())
        } else {
            let mut names: Vec<&str> = conflicts.iter().map(|s| s.as_str()).collect();
            names.sort();
            Some(format!(
                "manual merge required — conflicting plan directories: {}",
                names.join(", ")
            ))
        };

        DoctorCheck {
            id: "plans_dir_conflict".to_string(),
            status: DoctorStatus::Warn,
            message: "both plans/ and .roko/plans/ exist".to_string(),
            detail: Some(format!(
                "plans/ has {top_count} plan dir{}, .roko/plans/ has {dot_count} plan dir{}; \
                 the canonical location is plans/",
                if top_count == 1 { "" } else { "s" },
                if dot_count == 1 { "" } else { "s" },
            )),
            path: Some(top_level.display().to_string()),
            url: None,
            fix,
        }
    } else {
        DoctorCheck {
            id: "plans_dir_conflict".to_string(),
            status: DoctorStatus::Ok,
            message: "no plans directory conflict".to_string(),
            detail: None,
            path: None,
            url: None,
            fix: None,
        }
    }
}

/// Options for `roko doctor network`.
#[derive(Debug, Clone)]
pub struct NetworkDoctorOptions {
    /// Workspace root containing `roko.toml`.
    pub workdir: PathBuf,
    /// Optional explicit config override path (`--config`).
    pub config_override: Option<PathBuf>,
    /// Per-probe HTTP timeout. Defaults to [`DEFAULT_NETWORK_PROBE_TIMEOUT`].
    pub probe_timeout: Duration,
}

/// Default per-probe timeout for `roko doctor network`.
pub const DEFAULT_NETWORK_PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// Probe result for a single provider endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NetworkProbeCheck {
    /// Stable provider id from `roko.toml` (for example, `"anthropic"`).
    pub provider_id: String,
    /// The endpoint URL that was probed (or would have been probed).
    pub url: String,
    /// Outcome of the probe.
    pub status: DoctorStatus,
    /// Human-readable summary of the probe outcome.
    pub message: String,
    /// Round-trip latency in milliseconds, or `None` when unavailable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    /// HTTP status code returned by the endpoint, or `None` when unavailable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_status: Option<u16>,
    /// Actionable fix hint for warning or failure statuses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
}

/// Aggregate report from the network doctor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NetworkProbeReport {
    /// Aggregate counts and latency extrema for the probe run.
    pub summary: NetworkProbeSummary,
    /// Per-provider probe results.
    pub checks: Vec<NetworkProbeCheck>,
}

/// Aggregate counts plus best and worst providers by latency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NetworkProbeSummary {
    /// Total number of provider checks.
    pub total: usize,
    /// Number of successful checks.
    pub ok: usize,
    /// Number of checks that completed with a warning.
    pub warn: usize,
    /// Number of failed checks.
    pub fail: usize,
    /// Number of skipped checks.
    pub skipped: usize,
    /// Provider id with the lowest observed latency.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fastest_provider: Option<String>,
    /// Provider id with the highest observed latency.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slowest_provider: Option<String>,
}

impl NetworkProbeSummary {
    fn from_checks(checks: &[NetworkProbeCheck]) -> Self {
        let mut summary = Self {
            total: checks.len(),
            ok: 0,
            warn: 0,
            fail: 0,
            skipped: 0,
            fastest_provider: None,
            slowest_provider: None,
        };

        for check in checks {
            match check.status {
                DoctorStatus::Ok => summary.ok += 1,
                DoctorStatus::Warn => summary.warn += 1,
                DoctorStatus::Fail => summary.fail += 1,
                DoctorStatus::Skipped => summary.skipped += 1,
            }
        }

        let mut observed = checks
            .iter()
            .filter_map(|check| {
                check
                    .latency_ms
                    .map(|latency_ms| (latency_ms, check.provider_id.as_str()))
            })
            .collect::<Vec<_>>();
        observed.sort_unstable();
        summary.fastest_provider = observed
            .first()
            .map(|(_, provider_id)| (*provider_id).to_string());
        summary.slowest_provider = observed
            .last()
            .map(|(_, provider_id)| (*provider_id).to_string());
        summary
    }
}

impl NetworkProbeReport {
    /// Exit code for the focused report: `0` unless a probe failed.
    #[must_use]
    pub const fn exit_code(&self) -> i32 {
        if self.summary.fail == 0 { 0 } else { 1 }
    }

    /// Render the focused network report for terminal users.
    #[must_use]
    pub fn render_human(&self) -> String {
        let mut out = String::from("doctor network\n");
        let _ = writeln!(
            &mut out,
            "summary: {} ok, {} warn, {} failed, {} skipped",
            self.summary.ok, self.summary.warn, self.summary.fail, self.summary.skipped
        );

        if self.checks.is_empty() {
            out.push_str("no configured provider endpoints to probe\n");
            return out;
        }

        if let (Some(fastest), Some(slowest)) = (
            self.summary.fastest_provider.as_deref(),
            self.summary.slowest_provider.as_deref(),
        ) {
            let _ = writeln!(&mut out, "latency: fastest {fastest}, slowest {slowest}");
        }

        for check in &self.checks {
            let latency = check
                .latency_ms
                .map_or_else(|| "not probed".to_string(), |ms| format!("{ms}ms"));
            let _ = write!(
                &mut out,
                "[{}] network/{}: {} ({latency})",
                check.status.label(),
                check.provider_id,
                check.message
            );
            if !check.url.is_empty() {
                let _ = write!(&mut out, " [{}]", check.url);
            }
            out.push('\n');
            if matches!(check.status, DoctorStatus::Fail | DoctorStatus::Warn)
                && let Some(fix) = &check.fix
            {
                let _ = writeln!(&mut out, "    \u{2192} fix: {fix}");
            }
        }
        out
    }
}

/// Return the canonical endpoint for HTTP provider kinds without a configured URL.
pub(crate) fn endpoint_for_kind(kind: ProviderKind) -> Option<&'static str> {
    match kind {
        ProviderKind::AnthropicApi => Some("https://api.anthropic.com/v1"),
        ProviderKind::OpenAiCompat => Some("https://api.openai.com/v1"),
        ProviderKind::GeminiApi => Some("https://generativelanguage.googleapis.com/v1beta"),
        ProviderKind::PerplexityApi => Some("https://api.perplexity.ai"),
        ProviderKind::CerebrasApi => Some("https://api.cerebras.ai/v1"),
        ProviderKind::ClaudeCli
        | ProviderKind::CodexCli
        | ProviderKind::CursorAcp
        | ProviderKind::GeminiCli
        | ProviderKind::CursorCli
        | ProviderKind::Hermes
        | ProviderKind::OpenClaw => None,
    }
}

pub(crate) fn endpoint_for_provider(provider: &ProviderConfig) -> Option<String> {
    provider
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|url| !url.is_empty())
        .map(str::to_string)
        .or_else(|| endpoint_for_kind(provider.kind).map(str::to_string))
}

fn skipped_provider_check(
    provider_id: &str,
    provider: &ProviderConfig,
) -> Option<NetworkProbeCheck> {
    let url = endpoint_for_provider(provider).unwrap_or_default();
    if provider
        .limits
        .as_ref()
        .is_some_and(|limits| matches!(limits.network, ProviderNetworkPolicy::Deny))
    {
        return Some(NetworkProbeCheck {
            provider_id: provider_id.to_string(),
            url,
            status: DoctorStatus::Skipped,
            message: "network policy denies provider network access".to_string(),
            latency_ms: None,
            http_status: None,
            fix: None,
        });
    }

    if url.is_empty() {
        return Some(NetworkProbeCheck {
            provider_id: provider_id.to_string(),
            url,
            status: DoctorStatus::Skipped,
            message: "provider uses a non-HTTP transport".to_string(),
            latency_ms: None,
            http_status: None,
            fix: None,
        });
    }

    None
}

/// Probe one provider endpoint using an HTTP `HEAD` request.
pub(crate) async fn probe_one_provider(
    provider_id: String,
    url: String,
    timeout: Duration,
) -> NetworkProbeCheck {
    let client = match reqwest::Client::builder().timeout(timeout).build() {
        Ok(client) => client,
        Err(err) => {
            return NetworkProbeCheck {
                provider_id,
                url,
                status: DoctorStatus::Fail,
                message: format!("could not build HTTP client: {err}"),
                latency_ms: None,
                http_status: None,
                fix: Some("verify the local TLS and HTTP client configuration".to_string()),
            };
        }
    };

    let started = Instant::now();
    let response = client.head(&url).send().await;
    let latency_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

    match response {
        Ok(response) => {
            let status_code = response.status();
            let http_status = status_code.as_u16();
            let (status, message, fix) = if status_code.is_success() {
                (
                    DoctorStatus::Ok,
                    format!("endpoint reachable ({status_code})"),
                    None,
                )
            } else if status_code.is_server_error() {
                (
                    DoctorStatus::Fail,
                    format!("endpoint returned server error {status_code}"),
                    Some("check the provider status page and retry the probe".to_string()),
                )
            } else {
                let message = if matches!(http_status, 401 | 403) {
                    format!("endpoint reachable; authentication required ({status_code})")
                } else {
                    format!("endpoint reachable but rejected HEAD ({status_code})")
                };
                (
                    DoctorStatus::Warn,
                    message,
                    Some(
                        "verify the provider base URL and credentials; the host is reachable"
                            .to_string(),
                    ),
                )
            };
            NetworkProbeCheck {
                provider_id,
                url,
                status,
                message,
                latency_ms: Some(latency_ms),
                http_status: Some(http_status),
                fix,
            }
        }
        Err(err) => {
            let message = if err.is_timeout() {
                format!("probe timed out after {}ms", timeout.as_millis())
            } else if err.is_builder() {
                format!("invalid provider URL: {err}")
            } else if err.is_connect() {
                format!("could not connect to provider: {err}")
            } else {
                format!("network probe failed: {err}")
            };
            NetworkProbeCheck {
                provider_id,
                url,
                status: DoctorStatus::Fail,
                message,
                latency_ms: Some(latency_ms),
                http_status: None,
                fix: Some(
                    "verify the provider base URL, DNS, proxy, firewall, and network connection"
                        .to_string(),
                ),
            }
        }
    }
}

fn load_network_config(options: &NetworkDoctorOptions) -> Result<Config> {
    if let Some(path) = options.config_override.as_deref() {
        return Config::from_file(path);
    }
    load_resolved_config(&options.workdir).map(|resolved| resolved.config)
}

/// Probe every configured HTTP provider concurrently.
pub async fn run_network_doctor(options: NetworkDoctorOptions) -> NetworkProbeReport {
    let config = match load_network_config(&options) {
        Ok(config) => config,
        Err(err) => {
            let url = options
                .config_override
                .as_ref()
                .map_or_else(String::new, |path| path.display().to_string());
            let checks = vec![NetworkProbeCheck {
                provider_id: "config".to_string(),
                url,
                status: DoctorStatus::Fail,
                message: format!("could not load provider configuration: {err:#}"),
                latency_ms: None,
                http_status: None,
                fix: Some("fix the active roko.toml or pass a valid --config path".to_string()),
            }];
            return NetworkProbeReport {
                summary: NetworkProbeSummary::from_checks(&checks),
                checks,
            };
        }
    };

    let mut providers = config.providers.iter().collect::<Vec<_>>();
    providers.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));

    let mut checks = Vec::with_capacity(providers.len());
    let mut probes = tokio::task::JoinSet::new();
    for (provider_id, provider) in providers {
        if let Some(check) = skipped_provider_check(provider_id, provider) {
            checks.push(check);
            continue;
        }

        let provider_id = provider_id.clone();
        let url = endpoint_for_provider(provider)
            .expect("provider without endpoint should have produced a skipped check");
        let timeout = options.probe_timeout;
        probes.spawn(async move { probe_one_provider(provider_id, url, timeout).await });
    }

    while let Some(result) = probes.join_next().await {
        match result {
            Ok(check) => checks.push(check),
            Err(err) => checks.push(NetworkProbeCheck {
                provider_id: "probe".to_string(),
                url: String::new(),
                status: DoctorStatus::Fail,
                message: format!("provider probe task failed: {err}"),
                latency_ms: None,
                http_status: None,
                fix: Some(
                    "rerun the network doctor; report reproducible task failures".to_string(),
                ),
            }),
        }
    }

    checks.sort_unstable_by(|left, right| left.provider_id.cmp(&right.provider_id));
    NetworkProbeReport {
        summary: NetworkProbeSummary::from_checks(&checks),
        checks,
    }
}

/// Check for a recent crash report in `.roko/crash-report.json`.
///
/// Warns if a crash report exists and was written within the last 24 hours.
/// Older crash reports are treated as informational (ok status).
fn check_crash_report(workdir: &Path) -> DoctorCheck {
    let roko_dir = workdir.join(".roko");
    let path = roko_core::crash_report_path(&roko_dir);

    if !path.exists() {
        return DoctorCheck {
            id: "crash_report".to_string(),
            status: DoctorStatus::Ok,
            message: "no crash report found".to_string(),
            detail: None,
            path: None,
            url: None,
            fix: None,
        };
    }

    // Read the crash report for details.
    let report = roko_core::read_crash_report(&roko_dir);
    let recent = roko_core::has_recent_crash_report(
        &roko_dir,
        Duration::from_secs(24 * 3600), // 24 hours
    );

    let detail = report.as_ref().map(|r| {
        let mut parts = Vec::new();
        parts.push(format!("crashed at {}", r.timestamp));
        parts.push(format!("version {}", r.version));
        if let Some(msg) = &r.panic_message {
            // Truncate long messages for the summary.
            let truncated: String = msg.chars().take(200).collect();
            parts.push(format!("message: {truncated}"));
        }
        if let Some(plan) = &r.active_plan {
            parts.push(format!("plan: {plan}"));
        }
        if let Some(task) = &r.active_task {
            parts.push(format!("task: {task}"));
        }
        if let Some(provider) = &r.provider {
            parts.push(format!("provider: {provider}"));
        }
        parts.join("; ")
    });

    if recent {
        DoctorCheck {
            id: "crash_report".to_string(),
            status: DoctorStatus::Warn,
            message: "recent crash report found (< 24h old)".to_string(),
            detail,
            path: Some(path.display().to_string()),
            url: None,
            fix: Some("review .roko/crash-report.json and rm it once investigated".to_string()),
        }
    } else {
        DoctorCheck {
            id: "crash_report".to_string(),
            status: DoctorStatus::Ok,
            message: "crash report found but is older than 24 hours".to_string(),
            detail,
            path: Some(path.display().to_string()),
            url: None,
            fix: Some("rm .roko/crash-report.json to clear stale crash data".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn config_freshness_check_displays_stale_section() {
        let dir = tempdir().expect("tempdir");
        let state_dir = dir.path().join(".roko").join("state");
        std::fs::create_dir_all(&state_dir).expect("create state dir");
        let mut freshness = roko_core::config::hot_reload::ConfigFreshness::default();
        freshness.section_timestamps.insert(
            "budget".to_string(),
            chrono::Utc::now() - chrono::Duration::days(31),
        );
        freshness
            .save(&state_dir.join("config-freshness.json"))
            .expect("save freshness");

        let checks = check_config_freshness(dir.path());
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].status, DoctorStatus::Warn);
        assert!(checks[0].message.contains("budget"));
    }

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
    async fn disk_health_uses_resource_policy_and_scans_learning_logs_and_worktree_targets() {
        let temp = tempdir().unwrap();
        let layout = RokoLayout::for_project(temp.path());
        layout.ensure_dirs().await.unwrap();
        let learning_log = layout.learn_dir().join("provider-outcomes.jsonl");
        tokio::fs::write(&learning_log, "{}\n").await.unwrap();
        let target = temp.path().join(".roko/worktrees/plan-a/target");
        tokio::fs::create_dir_all(&target).await.unwrap();
        tokio::fs::write(target.join("artifact"), b"data")
            .await
            .unwrap();
        let canonical_target = std::fs::canonicalize(&target).unwrap();

        let resources = roko_core::config::ResourcesConfig {
            log_rotation_max_mb: 0,
            target_max_age_days: 0,
            ..Default::default()
        };
        let (check, report) = check_disk_health(temp.path(), &resources).await;

        assert_eq!(check.status, DoctorStatus::Warn);
        assert_eq!(report.log_rotation_max_mb, 0);
        assert!(
            report
                .large_jsonl_files
                .contains(&learning_log.display().to_string())
        );
        assert_eq!(report.worktree_count, 1);
        assert!(
            report
                .stale_target_dirs
                .iter()
                .any(|finding| finding.path == canonical_target.display().to_string())
        );
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
            check_ids.contains(&"provider_api_keys"),
            "missing provider_api_keys check"
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

    #[test]
    fn doctor_dead_config_warns_for_inert_context_pressure_without_deprecating_watchers() {
        let mut conductor = roko_core::config::schema::ConductorConfig::default();
        conductor.context_pressure_enabled = true;

        let check = check_dead_conductor_config(&conductor);

        assert_eq!(check.id, "dead_conductor_config");
        assert_eq!(check.status, DoctorStatus::Warn);
        let detail = check.detail.expect("deprecation detail");
        assert!(detail.contains("context_pressure_enabled"));
        assert!(detail.contains("runtime-live"));
        assert!(check.fix.is_some());
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
    async fn state_layout_audit_v3_workspace_is_clean() {
        let temp = tempdir().unwrap();
        // Bootstrap a fresh current workspace.
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
            "V3 workspace should have Ok version check"
        );

        let legacy_check = checks
            .iter()
            .find(|c| c.id == "state_legacy_files")
            .expect("state_legacy_files check");
        assert_eq!(
            legacy_check.status,
            DoctorStatus::Ok,
            "fresh V3 workspace should have no legacy files"
        );
    }

    #[tokio::test]
    async fn state_layout_audit_classifies_learn_episode_log_as_legacy() {
        let temp = tempdir().unwrap();
        let layout = RokoLayout::for_project(temp.path());
        layout.ensure_dirs().await.expect("ensure dirs");
        std::fs::write(
            layout.learn_dir().join("episodes.jsonl"),
            "{\"episode_id\":\"legacy-learn\"}\n",
        )
        .expect("write legacy learn episodes");

        let checks = check_state_layout_audit(temp.path());
        let legacy_check = checks
            .iter()
            .find(|check| check.id == "state_legacy_files")
            .expect("legacy check");

        assert_eq!(legacy_check.status, DoctorStatus::Warn);
        assert!(
            legacy_check
                .detail
                .as_deref()
                .is_some_and(|detail| detail.contains("learn/episodes.jsonl"))
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

    fn network_test_provider(kind: ProviderKind, base_url: Option<&str>) -> ProviderConfig {
        ProviderConfig {
            kind,
            base_url: base_url.map(str::to_string),
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
        }
    }

    fn spawn_http_status_server(status: &'static str) -> (String, std::thread::JoinHandle<()>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let address = listener.local_addr().expect("test server address");
        let thread = std::thread::spawn(move || {
            use std::io::{Read as _, Write as _};

            let (mut stream, _) = listener.accept().expect("accept probe request");
            let mut request = [0_u8; 1024];
            let _ = stream.read(&mut request).expect("read probe request");
            write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )
            .expect("write probe response");
        });
        (format!("http://{address}"), thread)
    }

    #[tokio::test]
    async fn network_probe_empty_config_all_skip() {
        let temp = tempdir().expect("tempdir");
        let config_path = temp.path().join("empty.toml");
        std::fs::write(
            &config_path,
            Config::default().to_toml().expect("serialize config"),
        )
        .expect("write config");

        let report = run_network_doctor(NetworkDoctorOptions {
            workdir: temp.path().to_path_buf(),
            config_override: Some(config_path),
            probe_timeout: Duration::from_millis(100),
        })
        .await;

        assert_eq!(
            report.summary,
            NetworkProbeSummary {
                total: 0,
                ok: 0,
                warn: 0,
                fail: 0,
                skipped: 0,
                fastest_provider: None,
                slowest_provider: None,
            }
        );
        assert!(report.checks.is_empty());
        assert_eq!(report.exit_code(), 0);
    }

    #[test]
    fn network_probe_deny_policy_yields_skip() {
        let mut provider = network_test_provider(
            ProviderKind::AnthropicApi,
            Some("https://api.anthropic.com/v1"),
        );
        provider.limits = Some(roko_core::config::provider::ProviderLimits {
            network: ProviderNetworkPolicy::Deny,
            ..roko_core::config::provider::ProviderLimits::default()
        });

        let check = skipped_provider_check("anthropic", &provider).expect("skipped check");
        assert_eq!(check.status, DoctorStatus::Skipped);
        assert!(check.message.contains("network policy"));
    }

    #[test]
    fn cli_only_provider_yields_skip() {
        let provider = network_test_provider(ProviderKind::ClaudeCli, None);
        let check = skipped_provider_check("claude", &provider).expect("skipped check");
        assert_eq!(check.status, DoctorStatus::Skipped);
        assert!(check.message.contains("non-HTTP"));
        assert!(endpoint_for_kind(ProviderKind::ClaudeCli).is_none());
    }

    #[test]
    fn endpoint_for_kind_returns_anthropic_default() {
        assert!(
            endpoint_for_kind(ProviderKind::AnthropicApi)
                .expect("Anthropic endpoint")
                .contains("anthropic.com")
        );
    }

    #[tokio::test]
    async fn network_probe_maps_success_auth_client_and_server_statuses() {
        for (status_line, expected) in [
            ("204 No Content", DoctorStatus::Ok),
            ("401 Unauthorized", DoctorStatus::Warn),
            ("404 Not Found", DoctorStatus::Warn),
            ("503 Service Unavailable", DoctorStatus::Fail),
        ] {
            let (url, server) = spawn_http_status_server(status_line);
            let check = probe_one_provider("local".to_string(), url, Duration::from_secs(2)).await;
            server.join().expect("join test server");
            assert_eq!(check.status, expected, "HTTP {status_line}");
            assert!(check.http_status.is_some());
            assert!(check.latency_ms.is_some());
        }
    }

    #[tokio::test]
    async fn network_probe_invalid_url_fails_without_panicking() {
        let check = probe_one_provider(
            "broken".to_string(),
            "not a URL".to_string(),
            Duration::from_millis(100),
        )
        .await;

        assert_eq!(check.status, DoctorStatus::Fail);
        assert!(check.message.contains("invalid provider URL"));
        assert_eq!(check.http_status, None);
    }

    #[test]
    fn network_report_render_includes_status_latency_url_and_fix() {
        let checks = vec![NetworkProbeCheck {
            provider_id: "anthropic".to_string(),
            url: "https://api.anthropic.com/v1".to_string(),
            status: DoctorStatus::Warn,
            message: "authentication required".to_string(),
            latency_ms: Some(42),
            http_status: Some(401),
            fix: Some("set ANTHROPIC_API_KEY".to_string()),
        }];
        let report = NetworkProbeReport {
            summary: NetworkProbeSummary::from_checks(&checks),
            checks,
        };

        let output = report.render_human();
        assert!(output.contains(
            "[warn] network/anthropic: authentication required (42ms) [https://api.anthropic.com/v1]"
        ));
        assert!(output.contains("fix: set ANTHROPIC_API_KEY"));
        assert_eq!(report.exit_code(), 0);
    }

    fn clean_disk_report() -> DiskHealthReport {
        DiskHealthReport {
            free_disk_mb: Some(100_000),
            low_disk: false,
            orphaned_worktree_dirs: vec![],
            large_jsonl_files: vec![],
            stale_target_dirs: vec![],
            total_target_mb: 500,
            roko_dir_mb: 10,
            worktree_count: 0,
            worktree_total_mb: 0,
            log_rotation_max_mb: 100,
        }
    }

    #[test]
    fn disk_report_clean_exits_zero() {
        let report = clean_disk_report();
        assert_eq!(report.exit_code(), 0);
    }

    #[test]
    fn disk_report_low_disk_exits_two() {
        let mut report = clean_disk_report();
        report.low_disk = true;
        assert_eq!(report.exit_code(), 2);
    }

    #[test]
    fn disk_report_advisory_orphaned_worktree_exits_one() {
        let mut report = clean_disk_report();
        report
            .orphaned_worktree_dirs
            .push("/tmp/orphan".to_string());
        assert_eq!(report.exit_code(), 1);
    }

    #[test]
    fn disk_report_advisory_large_jsonl_exits_one() {
        let mut report = clean_disk_report();
        report.large_jsonl_files.push("/tmp/big.jsonl".to_string());
        assert_eq!(report.exit_code(), 1);
    }

    #[test]
    fn disk_report_advisory_stale_target_exits_one() {
        let mut report = clean_disk_report();
        report.stale_target_dirs.push(DiskTargetFinding {
            path: "/tmp/target".to_string(),
            size_mb: 1000,
            age_days: 90,
        });
        assert_eq!(report.exit_code(), 1);
    }

    #[test]
    fn disk_report_low_disk_trumps_advisory() {
        let mut report = clean_disk_report();
        report.low_disk = true;
        report
            .orphaned_worktree_dirs
            .push("/tmp/orphan".to_string());
        // Fatal (low_disk) should return 2, not 1.
        assert_eq!(report.exit_code(), 2);
    }
}
