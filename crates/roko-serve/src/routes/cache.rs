//! Cache inspection and pruning endpoints.
//!
//! Mirrors `roko cache status` (`GET /api/cache/status`) and
//! `roko cache prune` (`POST /api/cache/prune`).
//!
//! The status route is always read-only.  The prune route requires an
//! explicit `"apply": true` in the request body to perform deletions;
//! without it the response is a dry-run plan identical to what
//! `roko cache prune` prints without `--apply`.

use std::sync::Arc;
use std::time::Duration;

use axum::Json;
use axum::extract::State;
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::ApiError;
use crate::state::AppState;
use roko_fs::{CacheCleanupPolicy, CacheCleanupReport, cleanup_workspace_caches};

const MIB: u64 = 1024 * 1024;
const GIB: u64 = 1024 * MIB;

/// Request body for `POST /api/cache/prune`.
#[derive(Debug, Default, Deserialize)]
pub struct PruneRequest {
    /// When `true`, actually delete files.  When absent or `false`, this is a
    /// dry run that computes candidates without removing anything.
    #[serde(default)]
    pub apply: bool,
    /// Target total build-artifact budget in GiB (default: policy default).
    pub target_budget_gb: Option<u64>,
    /// Evidence artifact budget in MiB (default: policy default).
    pub evidence_budget_mb: Option<u64>,
    /// Context-cache budget in MiB (default: policy default).
    pub context_budget_mb: Option<u64>,
    /// Minimum age in hours before an artifact is eligible (default: 1).
    pub min_age_hours: Option<u64>,
    /// Maximum evidence age in days (default: policy default).
    pub max_evidence_age_days: Option<u64>,
    /// Number of most-recent runs to preserve regardless of age (default: policy default).
    pub keep_runs: Option<usize>,
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/cache/status", get(cache_status))
        .route("/cache/prune", post(cache_prune))
}

/// `GET /api/cache/status` — inspect workspace caches (read-only dry run).
async fn cache_status(State(state): State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    let workdir = state.workdir.clone();
    let policy = CacheCleanupPolicy::default();

    let report = cleanup_workspace_caches(&workdir, policy, false)
        .await
        .map_err(|e| ApiError::internal(format!("cache scan failed: {e}")))?;

    Ok(Json(cache_report_to_json(&report)))
}

/// `POST /api/cache/prune` — prune workspace caches with optional `apply`.
///
/// Body is optional JSON. Without `"apply": true` the response is a dry-run
/// plan identical to `roko cache prune` (without `--apply`).
async fn cache_prune(
    State(state): State<Arc<AppState>>,
    body: Option<Json<PruneRequest>>,
) -> Result<Json<Value>, ApiError> {
    let req = body.map(|Json(b)| b).unwrap_or_default();
    let apply = req.apply;
    let workdir = state.workdir.clone();

    let policy = build_policy(&req);

    let report = cleanup_workspace_caches(&workdir, policy, apply)
        .await
        .map_err(|e| ApiError::internal(format!("cache prune failed: {e}")))?;

    Ok(Json(cache_report_to_json(&report)))
}

fn build_policy(req: &PruneRequest) -> CacheCleanupPolicy {
    let default = CacheCleanupPolicy::default();
    CacheCleanupPolicy {
        target_budget_bytes: req
            .target_budget_gb
            .map(|gb| gb.saturating_mul(GIB))
            .unwrap_or(default.target_budget_bytes),
        evidence_budget_bytes: req
            .evidence_budget_mb
            .map(|mb| mb.saturating_mul(MIB))
            .unwrap_or(default.evidence_budget_bytes),
        context_cache_budget_bytes: req
            .context_budget_mb
            .map(|mb| mb.saturating_mul(MIB))
            .unwrap_or(default.context_cache_budget_bytes),
        min_incremental_age: req
            .min_age_hours
            .map(|h| Duration::from_secs(h.saturating_mul(3600)))
            .unwrap_or(default.min_incremental_age),
        max_evidence_age: req
            .max_evidence_age_days
            .map(|d| Duration::from_secs(d.saturating_mul(86_400)))
            .unwrap_or(default.max_evidence_age),
        preserve_evidence_runs: req.keep_runs.unwrap_or(default.preserve_evidence_runs),
        ..default
    }
}

/// Convert a `CacheCleanupReport` to a JSON `Value`.
fn cache_report_to_json(report: &CacheCleanupReport) -> Value {
    let candidates: Vec<Value> = report
        .candidates
        .iter()
        .map(|c| {
            json!({
                "path": c.path.display().to_string(),
                "size_bytes": c.size_bytes,
                "cold_build_risk": format!("{:?}", c.cold_build_risk),
                "reason": c.reason,
            })
        })
        .collect();

    let protected: Vec<Value> = report
        .protected
        .iter()
        .map(|p| {
            json!({
                "path": p.path.display().to_string(),
                "reason": p.reason,
            })
        })
        .collect();

    json!({
        "dry_run": report.dry_run,
        "target_bytes": report.target_bytes,
        "evidence_bytes": report.evidence_bytes,
        "context_cache_bytes": report.context_cache_bytes,
        "log_archive_bytes": report.log_archive_bytes,
        "eligible_bytes": report.eligible_bytes,
        "reclaimed_bytes": report.reclaimed_bytes,
        "removed_count": report.removed_count,
        "failed_count": report.failed_count,
        "candidate_count": report.candidates.len(),
        "candidates": candidates,
        "protected_count": report.protected.len(),
        "protected": protected,
    })
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
    async fn cache_status_returns_dry_run_report() {
        let (_dir, state) = test_state();
        let result = cache_status(State(state)).await;
        assert!(result.is_ok(), "cache_status should succeed");
        let body = result.unwrap().0;
        assert_eq!(body["dry_run"], true, "status is always dry_run");
        assert!(body["target_bytes"].as_u64().is_some());
        assert!(body["removed_count"].as_u64().is_some());
    }

    #[tokio::test]
    async fn cache_prune_without_apply_is_dry_run() {
        let (_dir, state) = test_state();
        let result = cache_prune(State(state), None).await;
        assert!(result.is_ok(), "cache_prune dry run should succeed");
        let body = result.unwrap().0;
        assert_eq!(body["dry_run"], true, "no apply → dry_run");
        assert_eq!(body["removed_count"], 0);
    }

    #[tokio::test]
    async fn cache_prune_with_apply_false_is_dry_run() {
        let (_dir, state) = test_state();
        let req = PruneRequest {
            apply: false,
            ..Default::default()
        };
        let result = cache_prune(State(state), Some(Json(req))).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0["dry_run"], true);
    }

    #[tokio::test]
    async fn build_policy_uses_defaults_when_fields_are_none() {
        let req = PruneRequest::default();
        let policy = build_policy(&req);
        let default = CacheCleanupPolicy::default();
        assert_eq!(policy.target_budget_bytes, default.target_budget_bytes);
        assert_eq!(policy.evidence_budget_bytes, default.evidence_budget_bytes);
    }

    #[tokio::test]
    async fn build_policy_respects_provided_fields() {
        let req = PruneRequest {
            target_budget_gb: Some(10),
            min_age_hours: Some(2),
            ..Default::default()
        };
        let policy = build_policy(&req);
        assert_eq!(policy.target_budget_bytes, 10 * GIB);
        assert_eq!(policy.min_incremental_age, Duration::from_secs(2 * 3600));
    }
}
