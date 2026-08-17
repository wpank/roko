//! Route-level typed RBAC enforcement.

use std::sync::Arc;

use axum::Json;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use super::middleware::{AuthContext, AuthMethod, scope_to_role};
use super::route_permissions::required_permission_for;
use crate::auth_audit::{AuthAuditAction, AuthAuditEvent, AuthOutcome};
use crate::rbac::{Role, check_permission};
use crate::state::AppState;

/// Enforce the typed permission associated with the current route.
///
/// This runs after credential authentication and coarse scope enforcement.
/// JWT callers resolve their role from the persisted team membership registry;
/// a JWT claim or caller-controlled header can therefore never grant a role.
/// Agent and worker credentials are already constrained by their dedicated,
/// operation-specific validators and do not enter the human workspace RBAC
/// hierarchy.
pub(crate) async fn require_route_permission(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let Some(required) = required_permission_for(req.method(), req.uri().path()) else {
        return next.run(req).await;
    };

    let route = format!("{} {}", req.method(), req.uri().path());
    let Some(context) = req.extensions().get::<AuthContext>() else {
        audit_denial(&state, "unauthenticated", Role::Viewer, required, &route);
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized" })),
        )
            .into_response();
    };

    // These identities have already passed their narrower protocol-specific
    // checks in `require_api_key`: concrete AgentCapability for agents and an
    // exact deployment callback token for workers.
    if context.method == AuthMethod::WorkerToken || context.scope == "agent:capability" {
        return next.run(req).await;
    }

    let actor = context.user_id.as_deref().unwrap_or("unknown");
    let role = if context.method == AuthMethod::Jwt {
        context
            .user_id
            .as_deref()
            .and_then(|user_id| super::team::role_for_member(&state.workdir, user_id))
            .unwrap_or(Role::Viewer)
    } else {
        scope_to_role(&context.scope)
    };

    if !check_permission(role, required) {
        audit_denial(&state, actor, role, required, &route);
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "forbidden",
                "permission": required.as_str(),
                "role": role.as_str(),
            })),
        )
            .into_response();
    }

    next.run(req).await
}

fn audit_denial(
    state: &AppState,
    actor: &str,
    role: Role,
    permission: crate::rbac::Permission,
    route: &str,
) {
    tracing::warn!(
        actor,
        role = role.as_str(),
        permission = permission.as_str(),
        route,
        "rbac: route permission denied",
    );
    if let Some(log) = state.auth_audit.as_ref() {
        log.append(
            &AuthAuditEvent::new(
                actor,
                AuthAuditAction::PermissionDenied,
                route,
                AuthOutcome::Denied,
            )
            .with_meta("permission", permission.as_str())
            .with_meta("role", role.as_str()),
        );
    }
}
