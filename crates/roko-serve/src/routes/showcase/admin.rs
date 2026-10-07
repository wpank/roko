//! The showcase admin routes (S11 §4.3): the admin key with `X-Roko-CSRF: 1`, and audited.
//!
//! `require_scope` holds every `/api/showcase/admin/*` mutation to the `admin` scope, and a
//! showcase session never reaches these routes.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use super::views::{bundle_root, summary};
use crate::auth_audit::{AuthAuditAction, AuthAuditEvent, AuthOutcome};
use crate::routes::middleware::AuthContext;
use crate::showcase::auth::CSRF_HEADER;
use crate::state::AppState;

/// Refuse an admin call without `X-Roko-CSRF: 1`; otherwise audit it under the caller's name.
#[allow(clippy::result_large_err)]
pub(super) fn admit(state: &AppState, req: &Request, route: &str) -> Result<(), Response> {
    let csrf = req
        .headers()
        .get(CSRF_HEADER)
        .and_then(|value| value.to_str().ok());
    if csrf != Some("1") {
        let body = Json(json!({ "error": "csrf_required" }));
        return Err((StatusCode::FORBIDDEN, body).into_response());
    }
    let actor = req
        .extensions()
        .get::<AuthContext>()
        .and_then(|context| context.user_id.clone())
        .unwrap_or_else(|| "admin".to_string());
    if let Some(log) = state.auth_audit.as_ref() {
        let action = AuthAuditAction::PermissionGranted;
        let event = AuthAuditEvent::new(actor, action, route, AuthOutcome::Success);
        log.append(&event);
    }
    Ok(())
}

/// `POST /api/showcase/admin/login-unlock`: clear every login block and failure count.
pub async fn unlock_login(State(state): State<Arc<AppState>>, req: Request) -> Response {
    if let Err(response) = admit(&state, &req, "POST /api/showcase/admin/login-unlock") {
        return response;
    }
    state.local_access.login_lockout().unlock();
    Json(json!({ "unlocked": true })).into_response()
}

/// `POST /api/showcase/admin/bundles/reload`: load the bundles again, and list them.
pub async fn reload_bundles(State(state): State<Arc<AppState>>, req: Request) -> Response {
    if let Err(response) = admit(&state, &req, "POST /api/showcase/admin/bundles/reload") {
        return response;
    }
    let config = state.load_roko_config();
    let catalog = state.showcase_bundles.reload(&bundle_root(&state, &config));
    let bundles: Vec<_> = catalog.bundles.iter().map(summary).collect();
    Json(json!({ "reloaded": true, "bundles": bundles })).into_response()
}
