//! Workspace diagnostic endpoint.
//!
//! `GET /api/doctor` returns a structured report analogous to `roko doctor`.
//! This implementation runs the subset of checks that are available without
//! the full CLI tool chain: config presence, disk health, provider keys,
//! layout directories, and the serve-side runtime.
//!
//! The response schema mirrors the CLI's `DoctorReport` (workdir, checks[],
//! summary, healthy) so callers that parse the CLI's `--json` output can also
//! consume this endpoint.

use std::path::Path;
use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::routing::get;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::ApiError;
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/doctor", get(doctor_report))
}

/// Status of a single diagnostic check — matches the CLI's `DoctorStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorStatus {
    Ok,
    Warn,
    Fail,
    Skipped,
}

impl DoctorStatus {
    #[allow(dead_code)]
    fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warn => "warn",
            Self::Fail => "fail",
            Self::Skipped => "skipped",
        }
    }
}

/// One named check result — matches the CLI's `DoctorCheck`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorCheck {
    pub id: String,
    pub status: DoctorStatus,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
}

impl DoctorCheck {
    fn ok(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: DoctorStatus::Ok,
            message: message.into(),
            detail: None,
            path: None,
            fix: None,
        }
    }

    fn warn(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: DoctorStatus::Warn,
            message: message.into(),
            detail: None,
            path: None,
            fix: None,
        }
    }

    fn fail(id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: DoctorStatus::Fail,
            message: message.into(),
            detail: None,
            path: None,
            fix: None,
        }
    }

    fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    fn with_fix(mut self, fix: impl Into<String>) -> Self {
        self.fix = Some(fix.into());
        self
    }
}

/// `GET /api/doctor` — run workspace diagnostics and return a structured report.
async fn doctor_report(State(state): State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    let workdir = state.workdir.clone();
    let config = state.load_roko_config();

    let mut checks: Vec<DoctorCheck> = Vec::new();

    // ── 1. Workspace directory ────────────────────────────────────────────
    checks.push(check_workdir(&workdir));

    // ── 2. roko.toml presence ─────────────────────────────────────────────
    checks.push(check_config_presence(&workdir));

    // ── 3. .roko/ layout directories ─────────────────────────────────────
    checks.extend(check_layout_dirs(&workdir));

    // ── 4. Disk health ────────────────────────────────────────────────────
    let disk_check = tokio::task::spawn_blocking({
        let wd = workdir.clone();
        move || check_disk_health(&wd)
    })
    .await
    .map_err(|e| ApiError::internal(format!("disk check panicked: {e}")))?;
    checks.push(disk_check);

    // ── 5. Provider API keys ──────────────────────────────────────────────
    checks.extend(check_provider_keys(&config));

    // ── 6. Default model ──────────────────────────────────────────────────
    checks.push(check_default_model(&config));

    // ── 7. Budget guard rails ─────────────────────────────────────────────
    checks.push(check_budget(&config));

    // ── 8. Serve auth ─────────────────────────────────────────────────────
    checks.push(check_serve_auth(&config));

    // ── Build summary ─────────────────────────────────────────────────────
    let mut ok = 0usize;
    let mut warn = 0usize;
    let mut fail = 0usize;
    let mut skipped = 0usize;
    for c in &checks {
        match c.status {
            DoctorStatus::Ok => ok += 1,
            DoctorStatus::Warn => warn += 1,
            DoctorStatus::Fail => fail += 1,
            DoctorStatus::Skipped => skipped += 1,
        }
    }
    let healthy = fail == 0;

    Ok(Json(json!({
        "workdir": workdir.display().to_string(),
        "healthy": healthy,
        "summary": {
            "total": checks.len(),
            "ok": ok,
            "warn": warn,
            "fail": fail,
            "skipped": skipped,
        },
        "checks": checks,
    })))
}

// ── individual checks ────────────────────────────────────────────────────────

fn check_workdir(workdir: &Path) -> DoctorCheck {
    if workdir.is_dir() {
        DoctorCheck::ok("workdir", "workspace directory exists")
            .with_path(workdir.display().to_string())
    } else {
        DoctorCheck::fail("workdir", "workspace directory not found")
            .with_path(workdir.display().to_string())
            .with_fix("roko init")
    }
}

fn check_config_presence(workdir: &Path) -> DoctorCheck {
    let toml_path = workdir.join("roko.toml");
    if toml_path.exists() {
        DoctorCheck::ok("config_presence", "roko.toml found")
            .with_path(toml_path.display().to_string())
    } else {
        DoctorCheck::warn("config_presence", "roko.toml not found; using defaults")
            .with_path(toml_path.display().to_string())
            .with_fix("roko init")
    }
}

fn check_layout_dirs(workdir: &Path) -> Vec<DoctorCheck> {
    let roko_dir = workdir.join(".roko");
    let required = ["plans", "learn", "sessions"];
    let mut checks = Vec::new();

    if roko_dir.is_dir() {
        checks.push(DoctorCheck::ok(
            "layout_roko_dir",
            ".roko/ directory present",
        ));
    } else {
        checks.push(
            DoctorCheck::warn("layout_roko_dir", ".roko/ directory missing (first run?)")
                .with_fix("roko init"),
        );
        // Without .roko/ the sub-dirs won't exist either — skip them.
        for name in required {
            checks.push(DoctorCheck {
                id: format!("layout_{name}"),
                status: DoctorStatus::Skipped,
                message: format!(".roko/{name}/ skipped (parent absent)"),
                detail: None,
                path: None,
                fix: None,
            });
        }
        return checks;
    }

    for name in required {
        let dir = roko_dir.join(name);
        if dir.is_dir() {
            checks.push(DoctorCheck::ok(
                format!("layout_{name}"),
                format!(".roko/{name}/ present"),
            ));
        } else {
            checks.push(
                DoctorCheck::warn(
                    format!("layout_{name}"),
                    format!(".roko/{name}/ missing (created on first use)"),
                )
                .with_path(dir.display().to_string()),
            );
        }
    }

    checks
}

fn check_disk_health(workdir: &Path) -> DoctorCheck {
    let config = roko_core::config::schema::ResourcesConfig::default();
    let monitor = roko_fs::DiskMonitor::new(config.min_free_disk_mb, config.warn_disk_mb);

    if monitor.check_pre_run(workdir).is_err() {
        return DoctorCheck::fail("disk_space", "disk space critically low")
            .with_fix("roko doctor disk")
            .with_path(workdir.display().to_string());
    }
    if let Some(warning) = monitor.check_warning(workdir) {
        return DoctorCheck::warn("disk_space", "disk space is low")
            .with_detail(format!(
                "{} MiB free (threshold {} MiB)",
                warning.free_mb, warning.threshold_mb
            ))
            .with_path(workdir.display().to_string());
    }

    // Report free space as a detail.
    let detail = roko_fs::get_disk_usage(workdir)
        .map(|u| {
            format!(
                "{} MiB free of {} MiB ({:.0}% used)",
                u.available_mb, u.total_mb, u.percentage,
            )
        })
        .unwrap_or_else(|_| "disk usage unavailable".into());

    DoctorCheck::ok("disk_space", "disk space ok").with_detail(detail)
}

fn check_provider_keys(config: &roko_core::config::schema::RokoConfig) -> Vec<DoctorCheck> {
    let mut checks = Vec::new();
    if config.providers.is_empty() {
        checks.push(
            DoctorCheck::warn(
                "provider_keys",
                "no providers configured; add at least one in roko.toml",
            )
            .with_fix("roko config providers add"),
        );
        return checks;
    }

    for (name, provider) in &config.providers {
        let key_present = has_provider_key(provider);
        let id = format!("provider_key_{name}");
        if key_present {
            checks.push(DoctorCheck::ok(
                id,
                format!("provider '{name}' key configured"),
            ));
        } else {
            checks.push(
                DoctorCheck::warn(id, format!("provider '{name}' has no API key set"))
                    .with_fix(format!("roko config set-secret {name}_api_key <value>")),
            );
        }
    }

    checks
}

fn has_provider_key(provider: &roko_core::config::ProviderConfig) -> bool {
    // CLI-backed providers (claude, codex, gemini) do not require an API key.
    use roko_core::agent::ProviderKind;
    if matches!(
        provider.kind,
        ProviderKind::ClaudeCli | ProviderKind::CodexCli | ProviderKind::GeminiCli
    ) {
        return true;
    }
    // For HTTP providers, check if the env-var key resolves.
    provider.resolve_api_key().is_some()
}

fn check_default_model(config: &roko_core::config::schema::RokoConfig) -> DoctorCheck {
    let key = &config.agent.default_model;
    if key.is_empty() {
        DoctorCheck::warn(
            "default_model",
            "agent.default_model is not set; using built-in fallback",
        )
        .with_fix("roko config set agent.default_model <model-key>")
    } else {
        DoctorCheck::ok("default_model", format!("default model is '{key}'"))
    }
}

fn check_budget(config: &roko_core::config::schema::RokoConfig) -> DoctorCheck {
    let max_plan = config.budget.max_plan_usd;
    if max_plan <= 0.0 {
        DoctorCheck::warn(
            "budget",
            "budget.max_plan_usd is not set or zero; no per-plan USD cap enforced",
        )
        .with_fix("roko config preset budget")
    } else {
        DoctorCheck::ok("budget", format!("plan budget ${max_plan:.2} USD"))
    }
}

fn check_serve_auth(config: &roko_core::config::schema::RokoConfig) -> DoctorCheck {
    if config.serve.auth.enabled {
        DoctorCheck::ok("serve_auth", "HTTP authentication is enabled")
    } else {
        DoctorCheck::warn(
            "serve_auth",
            "HTTP authentication is disabled; ensure the server is not internet-exposed",
        )
        .with_fix("roko config set serve.auth.enabled true")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::tempdir;

    use crate::deploy::create_backend;
    use crate::runtime::NoOpRuntime;
    use crate::state::AppState;

    fn test_state() -> (tempfile::TempDir, Arc<AppState>) {
        let dir = tempdir().expect("tempdir");
        let workdir = dir.path().to_path_buf();
        let deploy_backend =
            Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
        let state = Arc::new(
            AppState::new(
                workdir,
                Arc::new(NoOpRuntime),
                roko_core::config::schema::RokoConfig::default(),
                deploy_backend,
            )
            .expect("AppState::new"),
        );
        (dir, state)
    }

    #[tokio::test]
    async fn doctor_report_returns_structured_response() {
        let (_dir, state) = test_state();
        let result = doctor_report(State(state)).await;
        assert!(result.is_ok(), "doctor_report should succeed");
        let body = result.unwrap().0;
        assert!(body["workdir"].as_str().is_some());
        assert!(body["healthy"].as_bool().is_some());
        assert!(body["summary"]["total"].as_u64().is_some());
        assert!(body["checks"].as_array().is_some_and(|a| !a.is_empty()));
    }

    #[tokio::test]
    async fn doctor_report_includes_workdir_check_ok() {
        let (_dir, state) = test_state();
        let result = doctor_report(State(state)).await.unwrap();
        let body = result.0;
        let checks = body["checks"].as_array().unwrap();
        let workdir_check = checks
            .iter()
            .find(|c| c["id"] == "workdir")
            .expect("workdir check present");
        assert_eq!(workdir_check["status"], "ok");
    }

    #[tokio::test]
    async fn check_workdir_fails_for_nonexistent_path() {
        let check = check_workdir(Path::new("/nonexistent/path/unlikely"));
        assert_eq!(check.status, DoctorStatus::Fail);
    }

    #[tokio::test]
    async fn check_config_presence_warns_when_missing() {
        let dir = tempdir().expect("tempdir");
        let check = check_config_presence(dir.path());
        assert_eq!(check.status, DoctorStatus::Warn);
        assert!(check.fix.is_some());
    }

    #[tokio::test]
    async fn check_disk_health_passes_for_tempdir() {
        let dir = tempdir().expect("tempdir");
        let check = check_disk_health(dir.path());
        // On any reasonable dev machine tempdir should not be critically low.
        assert_ne!(
            check.status,
            DoctorStatus::Fail,
            "tempdir should have enough space"
        );
    }
}
