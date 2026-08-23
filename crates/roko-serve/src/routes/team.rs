//! Team membership and secure invitation routes.
//!
//! Invitations are persisted in `.roko/team/invitations.json` with only a
//! SHA-256 token hash. The plaintext 32-byte base64url token is returned once
//! when an invitation is created. Joining requires an authenticated JWT and a
//! live, unconsumed token; merely knowing the invited email is never enough.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use axum::extract::{Path as AxumPath, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post, put};
use axum::{Extension, Json, Router};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, TimeDelta, Utc};
use rand::RngCore;
use rand::rngs::OsRng;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::middleware::{AuthContext, AuthMethod, constant_time_eq};
use crate::auth_audit::{AuthAuditAction, AuthAuditEvent, AuthOutcome};
use crate::error::ApiError;
use crate::rbac::{Permission, Role, enforce_no_escalation, enforce_permission};
use crate::state::AppState;

const INVITE_TOKEN_BYTES: usize = 32;
const MAX_PENDING_INVITES: usize = 10;
static TEAM_STORE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// A workspace team member.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TeamMember {
    /// Authenticated identity from the JWT `sub` claim.
    pub id: String,
    /// Email associated with the accepted invitation.
    #[serde(default)]
    pub email: String,
    /// Typed workspace role, persisted as a lowercase string.
    pub role: Role,
    /// ISO-8601 timestamp.
    pub joined_at: String,
}

/// A persisted invitation. `invite_token_hash` is never returned by an API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Invitation {
    pub email: String,
    pub role: Role,
    pub invited_by: String,
    pub created_at: String,
    #[serde(default)]
    pub expires_at: String,
    /// SHA-256 of the random token, encoded as unpadded base64url.
    #[serde(default)]
    pub invite_token_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_at: Option<String>,
    #[serde(default)]
    pub consumed: bool,
}

/// Payload for inviting a member.
#[derive(Debug, Deserialize)]
pub struct InviteRequest {
    pub email: String,
    #[serde(default = "default_role")]
    pub role: Role,
}

/// Payload for accepting an invitation.
#[derive(Debug, Clone, Deserialize)]
pub struct JoinRequest {
    pub invite_token: String,
}

/// Payload for changing a member role.
#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub role: Role,
}

#[derive(Debug, Clone, Serialize)]
struct InvitationView {
    email: String,
    role: Role,
    invited_by: String,
    created_at: String,
    expires_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    accepted_at: Option<String>,
    consumed: bool,
}

impl From<&Invitation> for InvitationView {
    fn from(invitation: &Invitation) -> Self {
        Self {
            email: invitation.email.clone(),
            role: invitation.role,
            invited_by: invitation.invited_by.clone(),
            created_at: invitation.created_at.clone(),
            expires_at: invitation.expires_at.clone(),
            accepted_at: invitation.accepted_at.clone(),
            consumed: invitation.consumed,
        }
    }
}

#[derive(Debug, Serialize)]
struct InvitationCreatedResponse {
    #[serde(flatten)]
    invitation: InvitationView,
    /// Plaintext token, returned exactly once to the inviter.
    invite_token: String,
}

fn default_role() -> Role {
    Role::Member
}

/// Management routes. The caller must also pass the persisted TeamManage check.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/team/me", get(get_me))
        .route("/team/members", get(list_members))
        .route("/team/invites", get(list_invitations))
        .route("/team/invite", post(invite_member))
        .route(
            "/team/members/{did}",
            put(update_member).delete(remove_member),
        )
}

/// Invite acceptance is authenticated but intentionally outside TeamManage RBAC.
/// The aggregate router must merge this separately from [`routes`].
pub fn join_routes() -> Router<Arc<AppState>> {
    Router::new().route("/team/join", post(accept_invitation))
}

/// Return the persisted role for an authenticated member.
///
/// Storage errors and unknown users both resolve to `None`, allowing callers
/// such as auth middleware to fail closed.
pub(crate) fn role_for_member(workdir: &Path, user_id: &str) -> Option<Role> {
    load_members(&members_path(workdir))
        .ok()?
        .into_iter()
        .find(|member| member.id == user_id)
        .map(|member| member.role)
}

async fn get_me(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let caller = extract_caller_id(&headers);
    let caller_email = extract_caller_email(&headers);
    let _guard = team_store_guard();
    let path = members_path(&state.workdir);
    let mut members = load_members(&path)?;

    if let Some(existing) = members.iter().find(|member| member.id == caller) {
        return Ok(Json(json!(existing)));
    }
    if !members.is_empty() {
        return Err(ApiError::forbidden(
            "not a team member; accept a valid invitation via /api/team/join",
        ));
    }

    let member = TeamMember {
        id: caller,
        email: caller_email,
        role: Role::Owner,
        joined_at: Utc::now().to_rfc3339(),
    };
    members.push(member.clone());
    save_members(&path, &members)?;
    Ok(Json(json!(member)))
}

async fn list_members(State(state): State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    let _guard = team_store_guard();
    let members = load_members(&members_path(&state.workdir))?;
    Ok(Json(json!({ "members": members })))
}

async fn list_invitations(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let caller = extract_caller_id(&headers);
    let expiry_days = state.load_roko_config().serve.auth.invite_expiry_days;
    let _guard = team_store_guard();
    let members = load_members(&members_path(&state.workdir))?;
    require_team_manage(&members, &caller)?;
    let invitations = load_and_purge_expired(&state, expiry_days, &caller)?;
    let pending: Vec<_> = invitations
        .iter()
        .filter(|invitation| !invitation.consumed)
        .map(InvitationView::from)
        .collect();
    let count = pending.len();
    Ok(Json(json!({ "invitations": pending, "count": count })))
}

async fn invite_member(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<InviteRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let email = req.email.trim().to_ascii_lowercase();
    if email.is_empty() {
        return Err(ApiError::bad_request("email must not be empty"));
    }

    let caller = extract_caller_id(&headers);
    let expiry_days = state.load_roko_config().serve.auth.invite_expiry_days;
    let _guard = team_store_guard();
    let members = load_members(&members_path(&state.workdir))?;
    let caller_role = require_team_manage(&members, &caller)?;
    enforce_no_escalation(caller_role, req.role)?;

    if members
        .iter()
        .any(|member| member.email.eq_ignore_ascii_case(&email))
    {
        return Err(ApiError::conflict("user is already a team member"));
    }

    let mut invitations = load_and_purge_expired(&state, expiry_days, &caller)?;
    if invitations
        .iter()
        .any(|invitation| !invitation.consumed && invitation.email.eq_ignore_ascii_case(&email))
    {
        return Err(ApiError::conflict("invitation already pending"));
    }
    if invitations
        .iter()
        .filter(|invitation| !invitation.consumed)
        .count()
        >= MAX_PENDING_INVITES
    {
        return Err(ApiError {
            status: StatusCode::TOO_MANY_REQUESTS,
            code: "invite_limit_reached".into(),
            message: format!(
                "workspace already has the maximum of {MAX_PENDING_INVITES} pending invitations"
            ),
            details: None,
        });
    }

    let now = Utc::now();
    let expires_at = invitation_expiry(now, expiry_days)?;
    let invite_token = generate_invite_token();
    let invite_token_hash = hash_invite_token(&invite_token);
    let invitation = Invitation {
        email,
        role: req.role,
        invited_by: caller.clone(),
        created_at: now.to_rfc3339(),
        expires_at: expires_at.to_rfc3339(),
        invite_token_hash: invite_token_hash.clone(),
        accepted_at: None,
        consumed: false,
    };
    invitations.push(invitation.clone());
    save_invitations(&state.workdir, &invitations)?;

    append_audit_event(
        &state,
        AuthAuditEvent::new(
            caller,
            AuthAuditAction::InviteCreated,
            invitation.email.clone(),
            AuthOutcome::Success,
        )
        .with_meta("role", invitation.role.as_str())
        .with_meta("expires_at", invitation.expires_at.clone())
        .with_meta("token_hash_prefix", hash_prefix(&invite_token_hash)),
    );

    let response = InvitationCreatedResponse {
        invitation: InvitationView::from(&invitation),
        invite_token,
    };
    Ok((StatusCode::CREATED, Json(json!(response))))
}

async fn accept_invitation(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    Json(req): Json<JoinRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let auth = auth
        .map(|Extension(context)| context)
        .filter(|context| context.method == AuthMethod::Jwt)
        .ok_or_else(|| ApiError::unauthorized("joining a team requires an authenticated JWT"))?;
    let caller = auth
        .user_id
        .filter(|user_id| !user_id.trim().is_empty())
        .ok_or_else(|| ApiError::unauthorized("authenticated JWT is missing a subject"))?;
    let supplied_hash = validated_invite_token_hash(&req.invite_token).ok_or_else(|| {
        audit_join_denied(&state, &caller);
        ApiError::unauthorized("invalid, expired, or already consumed invitation")
    })?;
    let expiry_days = state.load_roko_config().serve.auth.invite_expiry_days;

    let _guard = team_store_guard();
    let mut invitations = load_and_purge_expired(&state, expiry_days, &caller)?;
    let Some(index) = invitations.iter().position(|invitation| {
        !invitation.consumed
            && constant_time_eq(
                invitation.invite_token_hash.as_bytes(),
                supplied_hash.as_bytes(),
            )
    }) else {
        audit_join_denied(&state, &caller);
        return Err(ApiError::unauthorized(
            "invalid, expired, or already consumed invitation",
        ));
    };

    let invitation = invitations[index].clone();
    let mut members = load_members(&members_path(&state.workdir))?;
    let inviter_role = members
        .iter()
        .find(|member| member.id == invitation.invited_by)
        .map(|member| member.role)
        .ok_or_else(|| ApiError::unauthorized("invitation is no longer authorized"))?;
    enforce_permission(inviter_role, Permission::TeamManage)
        .map_err(|_| ApiError::unauthorized("invitation is no longer authorized"))?;
    enforce_no_escalation(inviter_role, invitation.role)
        .map_err(|_| ApiError::unauthorized("invitation is no longer authorized"))?;

    if members
        .iter()
        .any(|member| member.id == caller || member.email.eq_ignore_ascii_case(&invitation.email))
    {
        return Err(ApiError::conflict("user is already a team member"));
    }

    let accepted_at = Utc::now().to_rfc3339();
    invitations[index].consumed = true;
    invitations[index].accepted_at = Some(accepted_at.clone());
    // Consume first: if the subsequent member write fails, security stays
    // fail-closed and the same token can never be replayed.
    save_invitations(&state.workdir, &invitations)?;

    let member = TeamMember {
        id: caller.clone(),
        email: invitation.email.clone(),
        role: invitation.role,
        joined_at: accepted_at,
    };
    members.push(member.clone());
    if let Err(error) = save_members(&members_path(&state.workdir), &members) {
        append_audit_event(
            &state,
            AuthAuditEvent::new(
                caller,
                AuthAuditAction::InviteAccepted,
                invitation.email,
                AuthOutcome::Failure,
            ),
        );
        return Err(error);
    }

    append_audit_event(
        &state,
        AuthAuditEvent::new(
            caller,
            AuthAuditAction::InviteAccepted,
            invitation.email,
            AuthOutcome::Success,
        )
        .with_meta("role", invitation.role.as_str())
        .with_meta(
            "token_hash_prefix",
            hash_prefix(&invitation.invite_token_hash),
        ),
    );
    Ok((StatusCode::CREATED, Json(json!(member))))
}

async fn update_member(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath(did): AxumPath<String>,
    Json(req): Json<UpdateRoleRequest>,
) -> Result<Json<Value>, ApiError> {
    let caller = extract_caller_id(&headers);
    let _guard = team_store_guard();
    let path = members_path(&state.workdir);
    let mut members = load_members(&path)?;
    let caller_role = require_team_manage(&members, &caller)?;
    enforce_no_escalation(caller_role, req.role)?;

    let target = members
        .iter_mut()
        .find(|member| member.id == did)
        .ok_or_else(|| ApiError::not_found("member not found"))?;
    if target.role == Role::Owner {
        return Err(ApiError::bad_request("cannot change owner role"));
    }

    let previous_role = target.role;
    target.role = req.role;
    let updated = target.clone();
    save_members(&path, &members)?;
    append_audit_event(
        &state,
        AuthAuditEvent::new(
            caller,
            AuthAuditAction::RoleChanged,
            updated.id.clone(),
            AuthOutcome::Success,
        )
        .with_meta("from_role", previous_role.as_str())
        .with_meta("to_role", updated.role.as_str()),
    );
    Ok(Json(json!(updated)))
}

async fn remove_member(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    AxumPath(did): AxumPath<String>,
) -> Result<StatusCode, ApiError> {
    let caller = extract_caller_id(&headers);
    let _guard = team_store_guard();
    let path = members_path(&state.workdir);
    let mut members = load_members(&path)?;
    require_team_manage(&members, &caller)?;

    if caller == did {
        return Err(ApiError::bad_request("cannot remove yourself"));
    }
    if members
        .iter()
        .any(|member| member.id == did && member.role == Role::Owner)
    {
        return Err(ApiError::bad_request("cannot remove the owner"));
    }

    let before = members.len();
    members.retain(|member| member.id != did);
    if members.len() == before {
        return Err(ApiError::not_found("member not found"));
    }
    save_members(&path, &members)?;
    Ok(StatusCode::NO_CONTENT)
}

fn team_store_guard() -> MutexGuard<'static, ()> {
    TEAM_STORE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn team_dir(workdir: &Path) -> PathBuf {
    workdir.join(".roko").join("team")
}

fn members_path(workdir: &Path) -> PathBuf {
    team_dir(workdir).join("members.json")
}

fn invitations_path(workdir: &Path) -> PathBuf {
    team_dir(workdir).join("invitations.json")
}

fn load_members(path: &Path) -> Result<Vec<TeamMember>, ApiError> {
    load_json(path, "team members")
}

fn save_members(path: &Path, members: &[TeamMember]) -> Result<(), ApiError> {
    save_json(path, members, "team members")
}

fn load_invitations(workdir: &Path) -> Result<Vec<Invitation>, ApiError> {
    load_json(&invitations_path(workdir), "team invitations")
}

fn save_invitations(workdir: &Path, invitations: &[Invitation]) -> Result<(), ApiError> {
    save_json(&invitations_path(workdir), invitations, "team invitations")
}

fn load_json<T>(path: &Path, label: &str) -> Result<T, ApiError>
where
    T: DeserializeOwned + Default,
{
    match std::fs::read_to_string(path) {
        Ok(data) => serde_json::from_str(&data)
            .map_err(|error| ApiError::internal(format!("parse {label}: {error}"))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(error) => Err(ApiError::internal(format!("read {label}: {error}"))),
    }
}

fn save_json<T>(path: &Path, value: &T, label: &str) -> Result<(), ApiError>
where
    T: Serialize + ?Sized,
{
    let parent = path
        .parent()
        .ok_or_else(|| ApiError::internal(format!("invalid {label} path")))?;
    std::fs::create_dir_all(parent)
        .map_err(|error| ApiError::internal(format!("create team dir: {error}")))?;
    let data = serde_json::to_vec_pretty(value)
        .map_err(|error| ApiError::internal(format!("serialize {label}: {error}")))?;
    let temp_path = path.with_extension("json.tmp");
    std::fs::write(&temp_path, data)
        .map_err(|error| ApiError::internal(format!("write {label}: {error}")))?;
    std::fs::rename(&temp_path, path)
        .map_err(|error| ApiError::internal(format!("commit {label}: {error}")))?;
    Ok(())
}

fn invitation_expiry(
    created_at: DateTime<Utc>,
    expiry_days: u64,
) -> Result<DateTime<Utc>, ApiError> {
    let days = i64::try_from(expiry_days)
        .map_err(|_| ApiError::internal("serve.auth.invite_expiry_days is too large"))?;
    let duration = TimeDelta::try_days(days)
        .ok_or_else(|| ApiError::internal("serve.auth.invite_expiry_days is too large"))?;
    created_at
        .checked_add_signed(duration)
        .ok_or_else(|| ApiError::internal("invitation expiry timestamp overflow"))
}

fn invitation_is_expired(invitation: &Invitation, now: DateTime<Utc>) -> bool {
    if invitation.consumed {
        return false;
    }
    if invitation.invite_token_hash.is_empty() {
        return true;
    }
    DateTime::parse_from_rfc3339(&invitation.expires_at)
        .map(|expires_at| expires_at.with_timezone(&Utc) <= now)
        .unwrap_or(true)
}

fn load_and_purge_expired(
    state: &AppState,
    expiry_days: u64,
    actor: &str,
) -> Result<Vec<Invitation>, ApiError> {
    let mut invitations = load_invitations(&state.workdir)?;
    let now = Utc::now();
    let mut expired = Vec::new();
    invitations.retain(|invitation| {
        let is_expired = invitation_is_expired(invitation, now);
        if is_expired {
            expired.push(invitation.clone());
        }
        !is_expired
    });
    if !expired.is_empty() {
        save_invitations(&state.workdir, &invitations)?;
        for invitation in expired {
            append_audit_event(
                state,
                AuthAuditEvent::new(
                    actor,
                    AuthAuditAction::InviteExpired,
                    invitation.email,
                    AuthOutcome::Success,
                )
                .with_meta(
                    "token_hash_prefix",
                    hash_prefix(&invitation.invite_token_hash),
                ),
            );
        }
    }
    // Force evaluation of a configured duration even if no invitation exists,
    // so invalid giant values fail deterministically on list/join.
    let _ = invitation_expiry(now, expiry_days)?;
    Ok(invitations)
}

fn generate_invite_token() -> String {
    let mut bytes = [0_u8; INVITE_TOKEN_BYTES];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn hash_invite_token(token: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(token.as_bytes()))
}

fn validated_invite_token_hash(token: &str) -> Option<String> {
    let decoded = URL_SAFE_NO_PAD.decode(token.as_bytes()).ok()?;
    (decoded.len() == INVITE_TOKEN_BYTES).then(|| hash_invite_token(token))
}

fn hash_prefix(hash: &str) -> String {
    hash.chars().take(8).collect()
}

fn extract_caller_id(headers: &HeaderMap) -> String {
    headers
        .get("x-user-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("local-user")
        .to_string()
}

fn extract_caller_email(headers: &HeaderMap) -> String {
    headers
        .get("x-user-email")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string()
}

fn require_team_manage(members: &[TeamMember], caller_id: &str) -> Result<Role, ApiError> {
    let role = members
        .iter()
        .find(|member| member.id == caller_id)
        .map(|member| member.role)
        .ok_or_else(|| ApiError::forbidden("not a team member"))?;
    enforce_permission(role, Permission::TeamManage)?;
    Ok(role)
}

fn append_audit_event(state: &AppState, event: AuthAuditEvent) {
    if let Some(log) = state.auth_audit.as_ref() {
        log.append(&event);
    }
}

fn audit_join_denied(state: &AppState, actor: &str) {
    append_audit_event(
        state,
        AuthAuditEvent::new(
            actor,
            AuthAuditAction::InviteAccepted,
            "team:join",
            AuthOutcome::Denied,
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::config::schema::RokoConfig;

    use crate::deploy::create_backend;
    use crate::runtime::NoOpRuntime;

    fn test_state(workdir: PathBuf, expiry_days: u64) -> Arc<AppState> {
        let deploy_backend =
            Arc::from(create_backend("manual", None, None, None).expect("manual backend"));
        let mut config = RokoConfig::default();
        config.serve.auth.invite_expiry_days = expiry_days;
        Arc::new(
            AppState::new(workdir, Arc::new(NoOpRuntime), config, deploy_backend)
                .expect("AppState::new"),
        )
    }

    fn owner_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("x-user-id", "owner-1".parse().expect("header"));
        headers
    }

    fn seed_owner(workdir: &Path) {
        save_members(
            &members_path(workdir),
            &[TeamMember {
                id: "owner-1".into(),
                email: "owner@example.com".into(),
                role: Role::Owner,
                joined_at: Utc::now().to_rfc3339(),
            }],
        )
        .expect("seed owner");
    }

    async fn create_invite(state: Arc<AppState>, email: &str) -> String {
        let (_, Json(body)) = invite_member(
            State(state),
            owner_headers(),
            Json(InviteRequest {
                email: email.into(),
                role: Role::Member,
            }),
        )
        .await
        .expect("create invite");
        body["invite_token"]
            .as_str()
            .expect("plaintext token in create response")
            .to_string()
    }

    fn jwt_context(user_id: &str) -> Option<Extension<AuthContext>> {
        Some(Extension(AuthContext {
            method: AuthMethod::Jwt,
            scope: "read".into(),
            user_id: Some(user_id.into()),
        }))
    }

    #[test]
    fn roles_persist_as_backward_compatible_lowercase_strings() {
        let member = TeamMember {
            id: "user".into(),
            email: "user@example.com".into(),
            role: Role::Admin,
            joined_at: Utc::now().to_rfc3339(),
        };
        let json = serde_json::to_string(&member).expect("serialize member");
        assert!(json.contains("\"role\":\"admin\""));
        assert_eq!(
            serde_json::from_str::<TeamMember>(&json)
                .expect("deserialize member")
                .role,
            Role::Admin
        );
        assert!(serde_json::from_str::<UpdateRoleRequest>(r#"{"role":"root"}"#).is_err());
    }

    #[test]
    fn role_resolver_returns_typed_role_and_fails_closed() {
        let dir = tempfile::tempdir().expect("tempdir");
        seed_owner(dir.path());
        assert_eq!(role_for_member(dir.path(), "owner-1"), Some(Role::Owner));
        assert_eq!(role_for_member(dir.path(), "missing"), None);
    }

    #[tokio::test]
    async fn invitation_acceptance_is_hashed_audited_and_single_use() {
        let dir = tempfile::tempdir().expect("tempdir");
        seed_owner(dir.path());
        let state = test_state(dir.path().to_path_buf(), 7);
        let token = create_invite(Arc::clone(&state), "new@example.com").await;
        assert_eq!(
            URL_SAFE_NO_PAD.decode(&token).expect("decode token").len(),
            32
        );

        let stored = std::fs::read_to_string(invitations_path(dir.path())).expect("stored invite");
        assert!(!stored.contains(&token));
        assert!(stored.contains("invite_token_hash"));

        let request = JoinRequest {
            invite_token: token.clone(),
        };
        let (status, Json(member)) = accept_invitation(
            State(Arc::clone(&state)),
            jwt_context("did:privy:new"),
            Json(request.clone()),
        )
        .await
        .expect("accept invite");
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(member["id"], "did:privy:new");
        assert_eq!(member["email"], "new@example.com");
        assert_eq!(member["role"], "member");

        let replay = accept_invitation(State(state), jwt_context("did:privy:other"), Json(request))
            .await
            .expect_err("token must be single-use");
        assert_eq!(replay.status, StatusCode::UNAUTHORIZED);

        let invitations = load_invitations(dir.path()).expect("load invitations");
        assert!(invitations[0].consumed);
        assert!(invitations[0].accepted_at.is_some());
        let audit = crate::auth_audit::open_audit_log(dir.path()).expect("open audit");
        let events = audit
            .query(None, None, None, None)
            .expect("query audit events");
        assert!(events.iter().any(|event| {
            event.action == AuthAuditAction::InviteCreated && event.outcome == AuthOutcome::Success
        }));
        assert!(events.iter().any(|event| {
            event.action == AuthAuditAction::InviteAccepted && event.outcome == AuthOutcome::Success
        }));
    }

    #[tokio::test]
    async fn concurrent_acceptance_has_exactly_one_winner() {
        let dir = tempfile::tempdir().expect("tempdir");
        seed_owner(dir.path());
        let state = test_state(dir.path().to_path_buf(), 7);
        let token = create_invite(Arc::clone(&state), "race@example.com").await;
        let left = accept_invitation(
            State(Arc::clone(&state)),
            jwt_context("did:privy:left"),
            Json(JoinRequest {
                invite_token: token.clone(),
            }),
        );
        let right = accept_invitation(
            State(state),
            jwt_context("did:privy:right"),
            Json(JoinRequest {
                invite_token: token,
            }),
        );
        let (left, right) = tokio::join!(left, right);
        assert_ne!(left.is_ok(), right.is_ok());
        let members = load_members(&members_path(dir.path())).expect("members");
        assert_eq!(members.len(), 2);
    }

    #[tokio::test]
    async fn join_fails_closed_without_jwt_context() {
        let dir = tempfile::tempdir().expect("tempdir");
        seed_owner(dir.path());
        let state = test_state(dir.path().to_path_buf(), 7);
        let token = create_invite(Arc::clone(&state), "new@example.com").await;
        let error = accept_invitation(
            State(state),
            None,
            Json(JoinRequest {
                invite_token: token,
            }),
        )
        .await
        .expect_err("missing JWT must fail");
        assert_eq!(error.status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn expired_invitation_is_cleaned_and_audited() {
        let dir = tempfile::tempdir().expect("tempdir");
        seed_owner(dir.path());
        let state = test_state(dir.path().to_path_buf(), 0);
        let token = create_invite(Arc::clone(&state), "expired@example.com").await;
        let error = accept_invitation(
            State(state),
            jwt_context("did:privy:expired"),
            Json(JoinRequest {
                invite_token: token,
            }),
        )
        .await
        .expect_err("expired invite must fail");
        assert_eq!(error.status, StatusCode::UNAUTHORIZED);
        assert!(
            load_invitations(dir.path())
                .expect("invitations")
                .is_empty()
        );

        let audit = crate::auth_audit::open_audit_log(dir.path()).expect("open audit");
        let events = audit.query(None, None, None, None).expect("query audit");
        assert!(
            events
                .iter()
                .any(|event| event.action == AuthAuditAction::InviteExpired)
        );
    }

    #[tokio::test]
    async fn pending_invites_are_limited_and_listing_hides_hashes() {
        let dir = tempfile::tempdir().expect("tempdir");
        seed_owner(dir.path());
        let state = test_state(dir.path().to_path_buf(), 7);
        for index in 0..MAX_PENDING_INVITES {
            create_invite(Arc::clone(&state), &format!("user-{index}@example.com")).await;
        }
        let error = invite_member(
            State(Arc::clone(&state)),
            owner_headers(),
            Json(InviteRequest {
                email: "one-too-many@example.com".into(),
                role: Role::Viewer,
            }),
        )
        .await
        .expect_err("pending invite limit must be enforced");
        assert_eq!(error.status, StatusCode::TOO_MANY_REQUESTS);

        let Json(list) = list_invitations(State(state), owner_headers())
            .await
            .expect("list invitations");
        assert_eq!(list["count"], MAX_PENDING_INVITES);
        let encoded = serde_json::to_string(&list).expect("encode response");
        assert!(!encoded.contains("invite_token"));
        assert!(!encoded.contains("token_hash"));
    }

    #[tokio::test]
    async fn get_me_does_not_accept_an_email_matched_invitation() {
        let dir = tempfile::tempdir().expect("tempdir");
        seed_owner(dir.path());
        let state = test_state(dir.path().to_path_buf(), 7);
        let _ = create_invite(Arc::clone(&state), "new@example.com").await;
        let mut headers = HeaderMap::new();
        headers.insert("x-user-id", "did:privy:new".parse().expect("id header"));
        headers.insert(
            "x-user-email",
            "new@example.com".parse().expect("email header"),
        );
        let error = get_me(State(state), headers)
            .await
            .expect_err("email match must not bypass token acceptance");
        assert_eq!(error.status, StatusCode::FORBIDDEN);
        assert_eq!(
            load_members(&members_path(dir.path()))
                .expect("members")
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn typed_role_policy_prevents_non_owner_team_management() {
        let dir = tempfile::tempdir().expect("tempdir");
        save_members(
            &members_path(dir.path()),
            &[TeamMember {
                id: "admin-1".into(),
                email: "admin@example.com".into(),
                role: Role::Admin,
                joined_at: Utc::now().to_rfc3339(),
            }],
        )
        .expect("seed admin");
        let state = test_state(dir.path().to_path_buf(), 7);
        let mut headers = HeaderMap::new();
        headers.insert("x-user-id", "admin-1".parse().expect("header"));
        let error = invite_member(
            State(state),
            headers,
            Json(InviteRequest {
                email: "new@example.com".into(),
                role: Role::Viewer,
            }),
        )
        .await
        .expect_err("admin lacks TeamManage");
        assert_eq!(error.status, StatusCode::FORBIDDEN);
    }
}
