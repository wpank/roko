//! Chat session history endpoints.
//!
//! Exposes `GET /api/history` and `GET /api/history/{id}` that mirror what
//! `roko history` and `roko history <id>` return from the CLI.  Sessions are
//! stored as JSON files in `.roko/sessions/` and read directly from disk.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Router, http::StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::ApiError;
use crate::state::AppState;

/// Query parameters for `GET /api/history`.
#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    /// Maximum number of sessions to return (default 50, max 500).
    limit: Option<usize>,
}

/// Summary of a single chat session as persisted in `.roko/sessions/<id>.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub agent_id: String,
    pub provider: String,
    #[serde(default)]
    pub model_key: String,
    pub started_at: String,
    #[serde(default)]
    pub ended_at: String,
    #[serde(default)]
    pub turn_count: u32,
    #[serde(default)]
    pub first_message: String,
    #[serde(default)]
    pub last_message: String,
    #[serde(default)]
    pub total_tokens: u64,
    #[serde(default)]
    pub total_cost_usd: f64,
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/history", get(list_history))
        .route("/history/{id}", get(get_history_session))
}

/// `GET /api/history[?limit=N]` — list past chat session summaries.
///
/// Sessions are returned newest-first sorted by `started_at`.
async fn list_history(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<Value>, ApiError> {
    let limit = query.limit.unwrap_or(50).min(500);
    let workdir = state.workdir.clone();

    let sessions = tokio::task::spawn_blocking(move || load_sessions(&workdir, limit))
        .await
        .map_err(|e| ApiError::internal(format!("history task panicked: {e}")))?;

    Ok(Json(json!({
        "sessions": sessions,
        "count": sessions.len(),
        "limit": limit,
    })))
}

/// `GET /api/history/{id}` — return a single session by ID.
async fn get_history_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SessionSummary>, ApiError> {
    let workdir = state.workdir.clone();
    let id_for_error = id.clone();
    let session = tokio::task::spawn_blocking(move || load_one_session(&workdir, &id))
        .await
        .map_err(|e| ApiError::internal(format!("history task panicked: {e}")))?;

    match session {
        Some(s) => Ok(Json(s)),
        None => Err(ApiError {
            status: StatusCode::NOT_FOUND,
            code: "not_found".into(),
            message: format!("session '{id_for_error}' not found"),
            details: None,
        }),
    }
}

// ── disk helpers ─────────────────────────────────────────────────────────────

fn sessions_dir(workdir: &std::path::Path) -> std::path::PathBuf {
    workdir.join(".roko").join("sessions")
}

/// Load and return sessions sorted by `started_at` descending (newest first).
fn load_sessions(workdir: &std::path::Path, limit: usize) -> Vec<SessionSummary> {
    let dir = sessions_dir(workdir);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };

    let mut summaries: Vec<SessionSummary> = entries
        .flatten()
        .filter(|entry| entry.path().extension().and_then(|s| s.to_str()) == Some("json"))
        .filter_map(|entry| {
            let text = std::fs::read_to_string(entry.path()).ok()?;
            serde_json::from_str(&text).ok()
        })
        .collect();

    summaries.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    summaries.truncate(limit);
    summaries
}

/// Load a single session by ID (the filename stem).
fn load_one_session(workdir: &std::path::Path, session_id: &str) -> Option<SessionSummary> {
    let path = sessions_dir(workdir).join(format!("{session_id}.json"));
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
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
    async fn list_history_returns_empty_when_no_sessions() {
        let (_dir, state) = test_state();
        let result = list_history(State(state), Query(HistoryQuery { limit: None })).await;
        assert!(result.is_ok());
        let body = result.unwrap().0;
        assert_eq!(body["count"], 0);
        assert!(body["sessions"].as_array().is_some_and(|a| a.is_empty()));
    }

    #[tokio::test]
    async fn list_history_returns_sessions_from_disk() {
        let (dir, state) = test_state();
        let sessions_dir = dir.path().join(".roko").join("sessions");
        std::fs::create_dir_all(&sessions_dir).expect("create sessions dir");

        let summary = SessionSummary {
            session_id: "2026-01-01T00:00:00Z-agent1".into(),
            agent_id: "agent1".into(),
            provider: "anthropic_api".into(),
            model_key: "claude-sonnet-4-6".into(),
            started_at: "2026-01-01T00:00:00Z".into(),
            ended_at: "2026-01-01T00:05:00Z".into(),
            turn_count: 3,
            first_message: "Hello".into(),
            last_message: "Goodbye".into(),
            total_tokens: 500,
            total_cost_usd: 0.01,
        };

        let path = sessions_dir.join("2026-01-01T00:00:00Z-agent1.json");
        std::fs::write(&path, serde_json::to_string_pretty(&summary).unwrap())
            .expect("write session");

        let result = list_history(State(state), Query(HistoryQuery { limit: Some(10) })).await;
        assert!(result.is_ok());
        let body = result.unwrap().0;
        assert_eq!(body["count"], 1);
        assert_eq!(
            body["sessions"][0]["session_id"],
            "2026-01-01T00:00:00Z-agent1"
        );
    }

    #[tokio::test]
    async fn get_history_session_returns_404_for_missing_id() {
        let (_dir, state) = test_state();
        let result = get_history_session(State(state), Path("nonexistent".into())).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().status,
            axum::http::StatusCode::NOT_FOUND
        );
    }
}
