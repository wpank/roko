//! Persistent group, membership, knowledge, pheromone, and event APIs.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use roko_core::groups::{
    CoordinationMode, Group, GroupConfig, GroupEvent, GroupId, InvitationId, InviteRequest,
    KnowledgePolicy, MemberPermissions, MemberRole,
};
use roko_core::{Body, Bus, Kind, Pulse, Topic};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use super::middleware::AuthContext;
use crate::error::ApiError;
use crate::extract::{RequestPayload, ValidJson};
use crate::group_runtime::{
    CreateGroupInput, GroupEventRecord, GroupMutation, GroupRuntimeError, UpdateGroupInput,
    UpdateMemberInput,
};
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/groups", get(list_groups).post(create_group))
        .route(
            "/groups/{id}",
            get(get_group).patch(update_group).delete(delete_group),
        )
        .route("/groups/{id}/invite", post(invite_agent))
        .route("/groups/{id}/invitations", get(list_invitations))
        .route(
            "/invitations/{invitation_id}/accept",
            post(accept_invitation),
        )
        .route(
            "/invitations/{invitation_id}/reject",
            post(reject_invitation),
        )
        .route("/groups/{id}/members", get(list_members))
        .route(
            "/groups/{id}/members/{agent_id}",
            axum::routing::patch(update_member).delete(remove_member),
        )
        .route(
            "/groups/{id}/knowledge",
            get(list_knowledge).post(publish_knowledge),
        )
        .route(
            "/groups/{id}/pheromones",
            get(list_pheromones).post(deposit_pheromone),
        )
        .route("/groups/{id}/message", post(publish_message))
        .route("/groups/{id}/events", get(list_events))
}

#[derive(Debug, Deserialize)]
struct CreateGroupRequest {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    coordination: CoordinationMode,
    #[serde(default)]
    config: GroupConfig,
}

impl RequestPayload for CreateGroupRequest {
    fn validate_payload(&self) -> Result<(), ApiError> {
        if self.name.trim().is_empty() {
            return Err(ApiError::bad_request("group name must not be blank"));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct UpdateGroupRequest {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    coordination: Option<CoordinationMode>,
    #[serde(default)]
    config: Option<GroupConfig>,
}

impl RequestPayload for UpdateGroupRequest {
    fn validate_payload(&self) -> Result<(), ApiError> {
        if self
            .name
            .as_deref()
            .is_some_and(|name| name.trim().is_empty())
        {
            return Err(ApiError::bad_request("group name must not be blank"));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct UpdateMemberRequest {
    #[serde(default)]
    role: Option<MemberRole>,
    #[serde(default)]
    permissions: Option<MemberPermissions>,
}

#[derive(Debug, Deserialize)]
struct PheromoneParams {
    #[serde(default)]
    signal_type: Option<String>,
    #[serde(default)]
    min_balance: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct DepositPheromoneRequest {
    depositor: String,
    signal_type: String,
    #[serde(default)]
    position_hint: Option<String>,
    #[serde(default)]
    metadata: Value,
}

impl RequestPayload for DepositPheromoneRequest {
    fn validate_payload(&self) -> Result<(), ApiError> {
        if self.depositor.trim().is_empty() || self.signal_type.trim().is_empty() {
            return Err(ApiError::bad_request(
                "depositor and signal_type must not be blank",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct KnowledgeParams {
    #[serde(default)]
    topic: Option<String>,
    #[serde(default)]
    min_confidence: Option<f64>,
    #[serde(default = "default_knowledge_limit")]
    limit: usize,
}

const fn default_knowledge_limit() -> usize {
    100
}

#[derive(Debug, Deserialize)]
struct PublishKnowledgeRequest {
    author: String,
    topic: String,
    content: String,
    #[serde(default = "default_confidence")]
    confidence: f64,
    #[serde(default)]
    tags: Vec<String>,
}

const fn default_confidence() -> f64 {
    0.5
}

impl RequestPayload for PublishKnowledgeRequest {
    fn validate_payload(&self) -> Result<(), ApiError> {
        if self.author.trim().is_empty()
            || self.topic.trim().is_empty()
            || self.content.trim().is_empty()
        {
            return Err(ApiError::bad_request(
                "author, topic, and content must not be blank",
            ));
        }
        if !self.confidence.is_finite() || !(0.0..=1.0).contains(&self.confidence) {
            return Err(ApiError::bad_request(
                "confidence must be finite and between 0 and 1",
            ));
        }
        if self.tags.len() > 128 || self.tags.iter().any(|tag| tag.len() > 128) {
            return Err(ApiError::bad_request(
                "knowledge tags exceed the bounded size",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct MessageRequest {
    from: String,
    content: String,
    #[serde(default)]
    tags: Vec<String>,
}

impl RequestPayload for MessageRequest {
    fn validate_payload(&self) -> Result<(), ApiError> {
        if self.from.trim().is_empty() || self.content.trim().is_empty() {
            return Err(ApiError::bad_request("from and content must not be blank"));
        }
        if self.content.len() > 64 * 1024 || self.tags.len() > 128 {
            return Err(ApiError::bad_request("group message exceeds size limits"));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct EventParams {
    #[serde(default)]
    after_seq: Option<u64>,
    #[serde(default = "default_event_limit")]
    limit: usize,
}

const fn default_event_limit() -> usize {
    100
}

async fn list_groups(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let groups = state.groups.list(&actor(&auth, &headers)).await;
    let total = groups.len();
    Ok(Json(json!({ "groups": groups, "total": total })))
}

async fn create_group(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    headers: HeaderMap,
    ValidJson(request): ValidJson<CreateGroupRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let actor = actor(&auth, &headers);
    let mutation = state
        .groups
        .create(
            &actor,
            CreateGroupInput {
                name: request.name,
                description: request.description,
                coordination: request.coordination,
                config: request.config,
            },
        )
        .await
        .map_err(map_runtime_error)?;
    publish_mutation(&state, &mutation)?;
    Ok((StatusCode::CREATED, Json(group_json(mutation.value))))
}

async fn get_group(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let group_id = parse_group_id(id)?;
    let group = state
        .groups
        .get(&group_id, &actor(&auth, &headers))
        .await
        .map_err(map_runtime_error)?;
    Ok(Json(group_json(group)))
}

async fn update_group(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    ValidJson(request): ValidJson<UpdateGroupRequest>,
) -> Result<Json<Value>, ApiError> {
    let group_id = parse_group_id(id)?;
    let mutation = state
        .groups
        .update(
            &group_id,
            &actor(&auth, &headers),
            UpdateGroupInput {
                name: request.name,
                description: request.description,
                coordination: request.coordination,
                config: request.config,
            },
        )
        .await
        .map_err(map_runtime_error)?;
    publish_mutation(&state, &mutation)?;
    Ok(Json(group_json(mutation.value)))
}

async fn delete_group(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let mutation = state
        .groups
        .delete(&parse_group_id(id)?, &actor(&auth, &headers))
        .await
        .map_err(map_runtime_error)?;
    publish_mutation(&state, &mutation)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn invite_agent(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<InviteRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let group_id = parse_group_id(id)?;
    let actor = actor(&auth, &headers);
    let owner = {
        let agents = state.discovered_agents.read().await;
        let agent = agents.get(request.agent_id.trim()).ok_or_else(|| {
            ApiError::not_found(format!("agent '{}' is not registered", request.agent_id))
        })?;
        if agent.owner.trim().is_empty() {
            actor.clone()
        } else {
            agent.owner.clone()
        }
    };
    let mutation = state
        .groups
        .invite(&group_id, &actor, &owner, request)
        .await
        .map_err(map_runtime_error)?;
    publish_mutation(&state, &mutation)?;
    let status = if mutation.value.status == roko_core::InvitationStatus::Pending {
        StatusCode::ACCEPTED
    } else {
        StatusCode::CREATED
    };
    let outcome = if mutation.value.status == roko_core::InvitationStatus::Accepted {
        "joined"
    } else {
        "pending"
    };
    Ok((
        status,
        Json(json!({
            "status": outcome,
            "invitation_status": mutation.value.status,
            "invitation_id": mutation.value.invitation_id,
            "agent_id": mutation.value.agent_id,
            "group_id": mutation.value.group_id,
        })),
    ))
}

async fn list_invitations(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let invitations = state
        .groups
        .invitations(&parse_group_id(id)?, &actor(&auth, &headers))
        .await
        .map_err(map_runtime_error)?;
    let total = invitations.len();
    Ok(Json(json!({ "invitations": invitations, "total": total })))
}

async fn accept_invitation(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    headers: HeaderMap,
    Path(invitation_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let invitation_id = parse_invitation_id(invitation_id)?;
    let mutation = state
        .groups
        .accept_invitation(&invitation_id, &actor(&auth, &headers))
        .await
        .map_err(map_runtime_error)?;
    publish_mutation(&state, &mutation)?;
    Ok(Json(json!(mutation.value)))
}

async fn reject_invitation(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    headers: HeaderMap,
    Path(invitation_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let invitation_id = parse_invitation_id(invitation_id)?;
    let mutation = state
        .groups
        .reject_invitation(&invitation_id, &actor(&auth, &headers))
        .await
        .map_err(map_runtime_error)?;
    publish_mutation(&state, &mutation)?;
    Ok(Json(json!(mutation.value)))
}

async fn list_members(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let group = state
        .groups
        .get(&parse_group_id(id)?, &actor(&auth, &headers))
        .await
        .map_err(map_runtime_error)?;
    let total = group.members.len();
    Ok(Json(json!({ "members": group.members, "total": total })))
}

async fn update_member(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    headers: HeaderMap,
    Path((id, agent_id)): Path<(String, String)>,
    Json(request): Json<UpdateMemberRequest>,
) -> Result<Json<Value>, ApiError> {
    validate_agent_path(&agent_id)?;
    let mutation = state
        .groups
        .update_member(
            &parse_group_id(id)?,
            &agent_id,
            &actor(&auth, &headers),
            UpdateMemberInput {
                role: request.role,
                permissions: request.permissions,
            },
        )
        .await
        .map_err(map_runtime_error)?;
    publish_mutation(&state, &mutation)?;
    Ok(Json(json!(mutation.value)))
}

async fn remove_member(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    headers: HeaderMap,
    Path((id, agent_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    validate_agent_path(&agent_id)?;
    let mutation = state
        .groups
        .remove_member(&parse_group_id(id)?, &agent_id, &actor(&auth, &headers))
        .await
        .map_err(map_runtime_error)?;
    publish_mutation(&state, &mutation)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_knowledge(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(params): Query<KnowledgeParams>,
) -> Result<Json<Value>, ApiError> {
    let group_id = parse_group_id(id)?;
    state
        .groups
        .get(&group_id, &actor(&auth, &headers))
        .await
        .map_err(map_runtime_error)?;
    if params
        .min_confidence
        .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
    {
        return Err(ApiError::bad_request(
            "min_confidence must be finite and between 0 and 1",
        ));
    }
    let workdir = state.workdir.clone();
    let group_tag = format!("group:{group_id}");
    let topic = params.topic.map(|value| value.trim().to_ascii_lowercase());
    let minimum = params.min_confidence.unwrap_or(0.0);
    let limit = params.limit.clamp(1, 500);
    let entries = tokio::task::spawn_blocking(move || {
        let store = roko_neuro::KnowledgeStore::for_workdir(&workdir);
        store.read_all()
    })
    .await
    .map_err(|error| ApiError::internal(format!("knowledge query task failed: {error}")))?
    .map_err(|error| ApiError::internal(format!("knowledge query failed: {error}")))?;
    let entries = entries
        .into_iter()
        .filter(|entry| entry.tags.iter().any(|tag| tag == &group_tag))
        .filter(|entry| entry.confidence >= minimum)
        .filter(|entry| {
            topic.as_ref().is_none_or(|topic| {
                entry.content.to_ascii_lowercase().contains(topic)
                    || entry
                        .tags
                        .iter()
                        .any(|tag| tag.to_ascii_lowercase().contains(topic))
            })
        })
        .take(limit)
        .collect::<Vec<_>>();
    let total = entries.len();
    Ok(Json(
        json!({ "group_id": group_id, "entries": entries, "total": total }),
    ))
}

async fn publish_knowledge(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    ValidJson(request): ValidJson<PublishKnowledgeRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let group_id = parse_group_id(id)?;
    let actor = actor(&auth, &headers);
    let group = state
        .groups
        .get(&group_id, &actor)
        .await
        .map_err(map_runtime_error)?;
    let member = require_owned_member(&group, &request.author, &actor)?;
    let policy_allows_write = match group.config.knowledge_policy {
        KnowledgePolicy::Open => member.permissions.write,
        KnowledgePolicy::WriteLeader | KnowledgePolicy::Curated => {
            member.permissions.write && member.role == MemberRole::Leader
        }
    };
    if !policy_allows_write {
        return Err(ApiError::forbidden(
            "group knowledge policy denies this write",
        ));
    }

    let entry_id = format!("know-{}", Uuid::new_v4().simple());
    let mut tags = request.tags;
    tags.push(format!("group:{group_id}"));
    tags.push(request.topic.trim().to_string());
    tags.sort();
    tags.dedup();
    let entry: roko_neuro::KnowledgeEntry = serde_json::from_value(json!({
        "id": entry_id,
        "kind": "insight",
        "source": format!("group-agent:{}", request.author),
        "content": request.content,
        "confidence": request.confidence,
        "tags": tags,
        "created_at": chrono::Utc::now(),
    }))
    .map_err(|error| ApiError::internal(format!("build group knowledge: {error}")))?;
    let workdir = state.workdir.clone();
    let persisted = entry.clone();
    tokio::task::spawn_blocking(move || {
        roko_neuro::KnowledgeStore::for_workdir(&workdir).add(persisted)
    })
    .await
    .map_err(|error| ApiError::internal(format!("knowledge write task failed: {error}")))?
    .map_err(|error| ApiError::internal(format!("knowledge write failed: {error}")))?;
    let event = state
        .groups
        .record_event(
            &group_id,
            &actor,
            GroupEvent::KnowledgePublished {
                entry_id: entry.id.clone(),
                author: request.author,
                topic: request.topic,
            },
        )
        .await
        .map_err(map_runtime_error)?;
    publish_event(&state, &event)?;
    Ok((StatusCode::CREATED, Json(json!(entry))))
}

async fn list_pheromones(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(params): Query<PheromoneParams>,
) -> Result<Json<Value>, ApiError> {
    let group_id = parse_group_id(id)?;
    let pheromones = state
        .groups
        .pheromones(
            &group_id,
            &actor(&auth, &headers),
            params.signal_type.as_deref(),
            params.min_balance,
        )
        .await
        .map_err(map_runtime_error)?;
    let field_size = pheromones.len();
    Ok(Json(
        json!({ "group_id": group_id, "pheromones": pheromones, "field_size": field_size }),
    ))
}

async fn deposit_pheromone(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    ValidJson(request): ValidJson<DepositPheromoneRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let mutation = state
        .groups
        .deposit_pheromone(
            &parse_group_id(id)?,
            &actor(&auth, &headers),
            &request.depositor,
            &request.signal_type,
            request.position_hint,
            request.metadata,
        )
        .await
        .map_err(map_runtime_error)?;
    publish_mutation(&state, &mutation)?;
    Ok((StatusCode::CREATED, Json(json!(mutation.value))))
}

async fn publish_message(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    ValidJson(request): ValidJson<MessageRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let group_id = parse_group_id(id)?;
    let actor = actor(&auth, &headers);
    let group = state
        .groups
        .get(&group_id, &actor)
        .await
        .map_err(map_runtime_error)?;
    let member = require_owned_member(&group, &request.from, &actor)?;
    if !member.permissions.write || member.role == MemberRole::Observer {
        return Err(ApiError::forbidden("member lacks group write permission"));
    }
    let event = state
        .groups
        .record_event(
            &group_id,
            &actor,
            GroupEvent::Message {
                from: request.from,
                content: request.content,
                tags: request.tags,
            },
        )
        .await
        .map_err(map_runtime_error)?;
    publish_event(&state, &event)?;
    Ok((StatusCode::ACCEPTED, Json(json!(event))))
}

async fn list_events(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(params): Query<EventParams>,
) -> Result<Json<Value>, ApiError> {
    let group_id = parse_group_id(id)?;
    let events = state
        .groups
        .events(
            &group_id,
            &actor(&auth, &headers),
            params.after_seq,
            params.limit,
        )
        .await
        .map_err(map_runtime_error)?;
    let total = events.len();
    Ok(Json(
        json!({ "group_id": group_id, "events": events, "total": total }),
    ))
}

// Axum supplies this extractor as an owned `Option`; keeping the helper signature aligned
// avoids repeating `as_ref()` across every group handler.
#[allow(clippy::ref_option)]
fn actor(auth: &Option<Extension<AuthContext>>, headers: &HeaderMap) -> String {
    if let Some(Extension(context)) = auth {
        return context
            .user_id
            .as_deref()
            .filter(|identity| !identity.trim().is_empty())
            .unwrap_or("local")
            .trim()
            .to_string();
    }
    headers
        .get("x-user-id")
        .and_then(|value| value.to_str().ok())
        .filter(|identity| !identity.trim().is_empty())
        .unwrap_or("local")
        .trim()
        .to_string()
}

fn parse_group_id(value: String) -> Result<GroupId, ApiError> {
    validate_identifier(&value, "group id")?;
    Ok(GroupId::new(value))
}

fn parse_invitation_id(value: String) -> Result<InvitationId, ApiError> {
    validate_identifier(&value, "invitation id")?;
    Ok(InvitationId::new(value))
}

fn validate_agent_path(value: &str) -> Result<(), ApiError> {
    validate_identifier(value, "agent id")
}

fn validate_identifier(value: &str, label: &str) -> Result<(), ApiError> {
    if value.is_empty()
        || value.len() > 256
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | ':' | '.')
        })
    {
        return Err(ApiError::bad_request(format!(
            "invalid {label}: expected 1..=256 identifier characters"
        )));
    }
    Ok(())
}

fn require_owned_member<'a>(
    group: &'a Group,
    agent_id: &str,
    actor: &str,
) -> Result<&'a roko_core::GroupMember, ApiError> {
    let member = group
        .member(agent_id)
        .ok_or_else(|| ApiError::forbidden("agent is not a group member"))?;
    if member.owner != actor && group.owner != actor {
        return Err(ApiError::forbidden(
            "caller does not own the acting group member",
        ));
    }
    Ok(member)
}

fn group_json(group: Group) -> Value {
    let relay_room = group.id.room();
    json!({
        "id": group.id,
        "name": group.name,
        "description": group.description,
        "owner": group.owner,
        "members": group.members,
        "coordination": group.coordination,
        "config": group.config,
        "relay_room": relay_room,
        "created_at": group.created_at,
        "updated_at": group.updated_at,
    })
}

fn map_runtime_error(error: GroupRuntimeError) -> ApiError {
    match error {
        GroupRuntimeError::NotFound(message) => ApiError::not_found(message),
        GroupRuntimeError::Forbidden(message) => ApiError::forbidden(message),
        GroupRuntimeError::Conflict(message) => ApiError::conflict(message),
        GroupRuntimeError::Invalid(message) => ApiError::bad_request(message),
        GroupRuntimeError::Storage(message) => ApiError::internal(message),
    }
}

fn publish_mutation<T>(state: &AppState, mutation: &GroupMutation<T>) -> Result<(), ApiError> {
    publish_event(state, &mutation.event)
}

fn publish_event(state: &AppState, record: &GroupEventRecord) -> Result<(), ApiError> {
    let kind = match &record.event {
        GroupEvent::PheromoneDeposited { .. } | GroupEvent::PheromoneDecayed { .. } => {
            Kind::Pheromone
        }
        GroupEvent::KnowledgePublished { .. } | GroupEvent::KnowledgeValidated { .. } => {
            Kind::Insight
        }
        _ => Kind::Custom("dev.roko.group_event".to_string()),
    };
    let body = Body::from_json(&record.event)
        .map_err(|error| ApiError::internal(format!("serialize group event: {error}")))?;
    let mut pulse = Pulse::new(record.seq, Topic::new(record.room.clone()), kind, body);
    pulse
        .tags
        .insert("group_id".into(), record.group_id.to_string());
    pulse
        .tags
        .insert("event_type".into(), record.event.event_type().to_string());
    state
        .pulse_bus
        .publish(pulse)
        .map_err(|error| ApiError::internal(format!("publish group event: {error}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifier_validation_rejects_path_and_control_characters() {
        assert!(parse_group_id("valid:id-1".into()).is_ok());
        assert!(parse_group_id("../escape".into()).is_err());
        assert!(parse_group_id("line\nbreak".into()).is_err());
        assert!(parse_group_id("x".repeat(257)).is_err());
    }

    #[test]
    fn actor_prefers_authenticated_context_over_spoofable_header() {
        let auth = Some(Extension(AuthContext {
            method: super::super::middleware::AuthMethod::Jwt,
            scope: "read".into(),
            user_id: Some("trusted".into()),
        }));
        let mut headers = HeaderMap::new();
        headers.insert("x-user-id", "spoofed".parse().unwrap());
        assert_eq!(actor(&auth, &headers), "trusted");

        let bearer_without_user = Some(Extension(AuthContext {
            method: super::super::middleware::AuthMethod::Bearer,
            scope: "write".into(),
            user_id: None,
        }));
        assert_eq!(actor(&bearer_without_user, &headers), "local");
    }

    #[test]
    fn knowledge_request_rejects_nan_and_oversized_tags() {
        let invalid = PublishKnowledgeRequest {
            author: "agent".into(),
            topic: "topic".into(),
            content: "content".into(),
            confidence: f64::NAN,
            tags: Vec::new(),
        };
        assert!(invalid.validate_payload().is_err());
        let invalid = PublishKnowledgeRequest {
            confidence: 0.5,
            tags: vec!["x".repeat(129)],
            ..invalid
        };
        assert!(invalid.validate_payload().is_err());
    }
}
