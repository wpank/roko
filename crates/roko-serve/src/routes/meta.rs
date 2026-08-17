//! Durable authenticated meta-agent lineage and activation service.
//!
//! Proposals are persisted before validation. Validation terminates as
//! `active` or `rejected`; active authority may later be exactly morphed,
//! rolled back, or durably deactivated. Activation requires schema validation,
//! bounded lineage accounting, non-widening delegation, the canonical
//! five-head safety Graph, and a completed R03 arena evaluation whose external
//! evidence binds the complete activation artifact.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use roko_agent::lifecycle::{
    AgentExtendedManifest, MAX_META_AGENT_DEPTH, MAX_META_AGENT_FANOUT,
    MAX_META_AGENT_LINEAGE_COST_USD, MAX_META_AGENT_RETRIES, MetaAgentLimits, SuccessorMode,
    SuccessorUsage, create_successor_bounded, validate_manifest,
};
use roko_agent::metamorphosis::role_transition_allowed;
use roko_agent::safety::{
    MAX_META_AGENT_GRANT_TTL_SECS, MetaAgentGrant, RecursiveSafetyEvidence, RecursiveSafetyMonitor,
    intersect_tools,
};
use roko_core::AgentRole;
use roko_core::corrigibility::{ActionContext, CorrigibilityHead, HeadVerdict};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::arenas::{ArenaAcceptanceEvidence, verify_acceptance_in_registry};
use crate::error::ApiError;
use crate::extract::ApiJson;
use crate::routes::middleware::AuthContext;
use crate::state::AppState;

const META_SCHEMA_VERSION: u32 = 2;
const VALIDATION_LEASE_SECS: u64 = 300;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Durable lifecycle state for a generated agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MetaAgentState {
    Proposed,
    Validating,
    Active,
    Deactivated,
    Rejected,
}

/// One durable lineage node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MetaAgentRecord {
    id: String,
    owner_principal: String,
    parent_id: Option<String>,
    parent_role_at_proposal: Option<AgentRole>,
    lineage_id: String,
    generation: u32,
    role: AgentRole,
    previous_role: Option<AgentRole>,
    previous_grant: Option<MetaAgentGrant>,
    activation_role: AgentRole,
    manifest: AgentExtendedManifest,
    activation_artifact_hash: String,
    grant: MetaAgentGrant,
    activation_grant: MetaAgentGrant,
    limits: MetaAgentLimits,
    state: MetaAgentState,
    #[serde(default)]
    validation_lease: Option<String>,
    #[serde(default)]
    validation_started_at: Option<u64>,
    rejection_reason: Option<String>,
    safety_evidence: Option<RecursiveSafetyEvidence>,
    acceptance_evidence: Option<ArenaAcceptanceEvidence>,
    created_at: u64,
    updated_at: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct MetaStore {
    records: BTreeMap<String, MetaAgentRecord>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MetaSnapshot {
    schema_version: u32,
    records: BTreeMap<String, MetaAgentRecord>,
    content_hash: String,
}

/// AppState-owned restart-safe meta-agent registry.
pub(crate) struct MetaAgentRuntime {
    path: PathBuf,
    store: Mutex<MetaStore>,
    startup_error: Option<String>,
}

impl MetaAgentRuntime {
    pub(crate) fn open(workdir: &FsPath) -> Self {
        let path = workdir
            .join(".roko")
            .join("agents")
            .join("meta-lineage.json");
        let (store, startup_error) = match load_store(&path) {
            Ok(mut store) => {
                if let Err(error) = reconcile_durable_acceptance_receipts(workdir, &store) {
                    return Self {
                        path,
                        store: Mutex::new(store),
                        startup_error: Some(error),
                    };
                }
                let recovered = recover_interrupted_validations(&mut store);
                let error = recovered
                    .then(|| persist_store(&path, &store).err())
                    .flatten()
                    .map(|error| format!("persist validation recovery: {error}"));
                (store, error)
            }
            Err(error) => (MetaStore::default(), Some(error)),
        };
        Self {
            path,
            store: Mutex::new(store),
            startup_error,
        }
    }

    fn ensure_available(&self) -> Result<(), ApiError> {
        self.startup_error.as_ref().map_or(Ok(()), |reason| {
            Err(ApiError {
                status: StatusCode::SERVICE_UNAVAILABLE,
                code: "meta_state_unavailable".to_string(),
                message: "durable meta-agent state failed validation".to_string(),
                details: Some(Box::new(json!({ "reason": reason }))),
            })
        })
    }

    async fn read<T>(
        &self,
        operation: impl FnOnce(&MetaStore) -> Result<T, ApiError>,
    ) -> Result<T, ApiError> {
        self.ensure_available()?;
        let store = self.store.lock().await;
        operation(&store)
    }

    async fn mutate<T>(
        &self,
        operation: impl FnOnce(&mut MetaStore) -> Result<T, ApiError>,
    ) -> Result<T, ApiError> {
        self.ensure_available()?;
        let mut store = self.store.lock().await;
        let previous = store.clone();
        let value = match operation(&mut store) {
            Ok(value) => value,
            Err(error) => return Err(error),
        };
        if let Err(error) = persist_store(&self.path, &store) {
            *store = previous;
            return Err(ApiError::internal(format!(
                "persist durable meta-agent state: {error}"
            )));
        }
        Ok(value)
    }
}

/// Authenticated meta-agent proposal, validation, and morph routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/meta/agents", get(list_agents).post(propose_agent))
        .route("/meta/agents/{id}", get(get_agent))
        .route("/meta/agents/{id}/validate", post(validate_agent))
        .route("/meta/agents/{id}/morph", post(morph_agent))
        .route("/meta/agents/{id}/morph/rollback", post(rollback_morph))
        .route("/meta/agents/{id}/deactivate", post(deactivate_agent))
}

#[derive(Debug, Deserialize)]
struct ProposeAgentRequest {
    #[serde(default)]
    parent_id: Option<String>,
    #[serde(default)]
    mode: SuccessorMode,
    name: String,
    #[serde(default)]
    manifest: Option<AgentExtendedManifest>,
    role: AgentRole,
    grant: MetaAgentGrant,
}

#[derive(Debug, Deserialize)]
struct ValidateAgentRequest {
    arena_id: String,
    attempt_id: String,
    evidence_hash: String,
}

#[derive(Debug, Deserialize)]
struct MorphAgentRequest {
    role: AgentRole,
}

async fn list_agents(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
) -> Result<Json<Value>, ApiError> {
    let auth = require_auth(auth)?;
    let principal = auth_principal(&auth);
    let admin = is_admin(&auth);
    let records = state
        .meta_agents
        .read(|store| {
            Ok(store
                .records
                .values()
                .filter(|record| admin || record.owner_principal == principal)
                .cloned()
                .collect::<Vec<_>>())
        })
        .await?;
    Ok(Json(
        json!({ "source": "local_durable", "agents": records }),
    ))
}

async fn get_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    auth: Option<Extension<AuthContext>>,
) -> Result<Json<MetaAgentRecord>, ApiError> {
    let auth = require_auth(auth)?;
    let principal = auth_principal(&auth);
    let admin = is_admin(&auth);
    state
        .meta_agents
        .read(|store| {
            let record = store
                .records
                .get(&id)
                .cloned()
                .ok_or_else(|| ApiError::not_found("meta-agent proposal not found"))?;
            require_owner(&record, &principal, admin)?;
            Ok(Json(record))
        })
        .await
}

async fn propose_agent(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    ApiJson(request): ApiJson<ProposeAgentRequest>,
) -> Result<(StatusCode, Json<MetaAgentRecord>), ApiError> {
    let auth = require_auth(auth)?;
    let principal = auth_principal(&auth);
    let admin = is_admin(&auth);
    if request.name.trim().is_empty() {
        return Err(ApiError::bad_request("meta-agent name must not be blank"));
    }
    let now = now_secs()?;
    if request.grant.expires_at.is_none_or(|expiry| {
        expiry <= now || expiry > now.saturating_add(MAX_META_AGENT_GRANT_TTL_SECS)
    }) {
        return Err(ApiError::bad_request(
            "meta-agent grant expiry must be within the trusted maximum lifetime",
        ));
    }
    let id = Uuid::new_v4().to_string();
    let record = state
        .meta_agents
        .mutate(move |store| {
            let (mut manifest, lineage_id, limits, parent_role_at_proposal) =
                if let Some(parent_id) = &request.parent_id {
                    if request.manifest.is_some() {
                        return Err(ApiError::bad_request(
                            "child manifests are server-derived; omit manifest",
                        ));
                    }
                    let parent = store
                        .records
                        .get(parent_id)
                        .cloned()
                        .ok_or_else(|| ApiError::not_found("parent meta-agent not found"))?;
                    require_owner(&parent, &principal, admin)?;
                    if parent.state != MetaAgentState::Active {
                        return Err(ApiError::conflict("parent meta-agent is not active"));
                    }
                    if parent.grant.expires_at.is_none_or(|expiry| expiry <= now) {
                        return Err(ApiError::forbidden("parent meta-agent grant is expired"));
                    }
                    let usage = successor_usage(store, &parent);
                    let effective_limits = effective_child_limits(&parent)?;
                    if usage.direct_children >= effective_limits.max_children_per_parent {
                        return Err(ApiError::conflict("meta-agent child fan-out limit reached"));
                    }
                    if usage.rejected_children > effective_limits.max_retries_per_parent {
                        return Err(ApiError::conflict("meta-agent retry limit reached"));
                    }
                    let child = create_successor_bounded(
                        &parent.manifest,
                        request.mode,
                        Some(request.name.clone()),
                        effective_limits,
                        usage,
                        request.grant.max_cost_usd,
                    )
                    .map_err(|error| ApiError::conflict(error.to_string()))?;
                    (
                        child,
                        parent.lineage_id.clone(),
                        parent.limits,
                        Some(parent.role),
                    )
                } else {
                    if !admin {
                        return Err(ApiError::forbidden(
                            "only an owner/admin credential may establish a root lineage",
                        ));
                    }
                    let mut root = request.manifest.clone().ok_or_else(|| {
                        ApiError::bad_request("root meta-agent proposals require manifest")
                    })?;
                    root.name = Some(request.name.clone());
                    root.generation = 0;
                    root.lineage_id = Some(id.clone());
                    (root, id.clone(), MetaAgentLimits::default(), None)
                };
            manifest.lineage_id = Some(lineage_id.clone());
            let generation = manifest.generation;
            let mut record = MetaAgentRecord {
                id: id.clone(),
                owner_principal: principal.clone(),
                parent_id: request.parent_id.clone(),
                parent_role_at_proposal,
                lineage_id,
                generation,
                role: request.role,
                previous_role: None,
                previous_grant: None,
                activation_role: request.role,
                manifest,
                activation_artifact_hash: String::new(),
                grant: request.grant.clone(),
                activation_grant: request.grant.clone(),
                limits,
                state: MetaAgentState::Proposed,
                validation_lease: None,
                validation_started_at: None,
                rejection_reason: None,
                safety_evidence: None,
                acceptance_evidence: None,
                created_at: now,
                updated_at: now,
            };
            record.activation_artifact_hash = hash_activation_artifact(&record)?;
            store.records.insert(id.clone(), record.clone());
            Ok(record)
        })
        .await?;
    Ok((StatusCode::CREATED, Json(record)))
}

async fn validate_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    auth: Option<Extension<AuthContext>>,
    ApiJson(request): ApiJson<ValidateAgentRequest>,
) -> Result<Json<MetaAgentRecord>, ApiError> {
    let auth = require_auth(auth)?;
    let principal = auth_principal(&auth);
    let admin = is_admin(&auth);
    let arena_id = parse_hash(&request.arena_id)?;
    let attempt_id = parse_hash(&request.attempt_id)?;
    let evidence_hash = parse_hash(&request.evidence_hash)?;
    let lease = Uuid::new_v4().to_string();
    let started_at = now_secs()?;
    let proposal = state
        .meta_agents
        .mutate(|store| {
            let record = store
                .records
                .get_mut(&id)
                .ok_or_else(|| ApiError::not_found("meta-agent proposal not found"))?;
            require_owner(record, &principal, admin)?;
            let stale_lease = record.state == MetaAgentState::Validating
                && record.validation_started_at.is_some_and(|started| {
                    started.saturating_add(VALIDATION_LEASE_SECS) <= started_at
                });
            if record.state != MetaAgentState::Proposed && !stale_lease {
                return Err(ApiError::conflict(
                    "meta-agent proposal is already terminal",
                ));
            }
            record.state = MetaAgentState::Validating;
            record.validation_lease = Some(lease.clone());
            record.validation_started_at = Some(started_at);
            record.updated_at = started_at;
            Ok(record.clone())
        })
        .await?;
    let output_hash = parse_hash(&proposal.activation_artifact_hash)?;
    let validation = validate_proposal(
        &state,
        &proposal,
        arena_id,
        attempt_id,
        output_hash,
        evidence_hash,
    )
    .await;
    let now = now_secs()?;
    let record = state
        .meta_agents
        .mutate(move |store| {
            let current = store
                .records
                .get(&id)
                .cloned()
                .ok_or_else(|| ApiError::not_found("meta-agent proposal not found"))?;
            if current.state != MetaAgentState::Validating
                || current.validation_lease.as_deref() != Some(lease.as_str())
            {
                return Err(ApiError::conflict(
                    "meta-agent validation lease was superseded",
                ));
            }
            let validation = validation.and_then(|evidence| {
                final_activation_check(store, &current, &evidence.1, now)?;
                Ok(evidence)
            });
            let record = store
                .records
                .get_mut(&id)
                .ok_or_else(|| ApiError::not_found("meta-agent proposal not found"))?;
            match validation {
                Ok((safety, acceptance)) => {
                    record.state = MetaAgentState::Active;
                    record.safety_evidence = Some(safety);
                    record.acceptance_evidence = Some(acceptance);
                }
                Err(reason) => {
                    record.state = MetaAgentState::Rejected;
                    record.rejection_reason = Some(reason);
                }
            }
            record.updated_at = now;
            record.validation_lease = None;
            record.validation_started_at = None;
            Ok(record.clone())
        })
        .await?;
    Ok(Json(record))
}

async fn morph_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    auth: Option<Extension<AuthContext>>,
    ApiJson(request): ApiJson<MorphAgentRequest>,
) -> Result<Json<MetaAgentRecord>, ApiError> {
    let auth = require_auth(auth)?;
    let principal = auth_principal(&auth);
    let admin = is_admin(&auth);
    let observed_at = now_secs()?;
    let current = state
        .meta_agents
        .read(|store| {
            let record = store
                .records
                .get(&id)
                .cloned()
                .ok_or_else(|| ApiError::not_found("meta-agent not found"))?;
            require_owner(&record, &principal, admin)?;
            if record.state != MetaAgentState::Active {
                return Err(ApiError::conflict("only active meta-agents may morph"));
            }
            if record.previous_role.is_some() {
                return Err(ApiError::conflict(
                    "the prior morph must be rolled back before another morph",
                ));
            }
            if record.role == request.role || !role_transition_allowed(record.role, request.role) {
                return Err(ApiError::forbidden(
                    "role morph is not in the transition policy",
                ));
            }
            if record
                .grant
                .expires_at
                .is_none_or(|expiry| expiry <= observed_at)
            {
                return Err(ApiError::forbidden("meta-agent grant is expired"));
            }
            Ok(record)
        })
        .await?;
    let safety = RecursiveSafetyMonitor
        .validate_action(
            format!("bounded role morph for meta-agent {id}"),
            morph_action_context(true),
        )
        .await
        .map_err(|error| ApiError::forbidden(error.to_string()))?;
    let now = now_secs()?;
    state
        .meta_agents
        .mutate(move |store| {
            let record = store
                .records
                .get(&id)
                .cloned()
                .ok_or_else(|| ApiError::not_found("meta-agent not found"))?;
            if record.state != MetaAgentState::Active
                || record.role != current.role
                || record.grant != current.grant
                || record.previous_role != current.previous_role
                || record.previous_grant != current.previous_grant
            {
                return Err(ApiError::conflict(
                    "meta-agent changed during morph validation",
                ));
            }
            let mut narrowed_grant = record.grant.clone();
            narrowed_grant.tools =
                intersect_tools(narrowed_grant.tools, request.role.tool_permissions());
            if narrowed_grant.expires_at.is_none_or(|expiry| expiry <= now) {
                return Err(ApiError::forbidden("meta-agent grant expired during morph"));
            }
            for child in store
                .records
                .values()
                .filter(|child| child.parent_id.as_deref() == Some(id.as_str()))
            {
                match child.state {
                    MetaAgentState::Proposed | MetaAgentState::Validating => {
                        return Err(ApiError::conflict(
                            "cannot morph while a direct child proposal is pending",
                        ));
                    }
                    MetaAgentState::Active => {
                        roko_agent::safety::validate_delegation_at(
                            &narrowed_grant,
                            child.role,
                            &child.grant,
                            now,
                        )
                        .map_err(|_| {
                            ApiError::conflict(
                                "morph would leave an active descendant with wider authority",
                            )
                        })?;
                    }
                    MetaAgentState::Deactivated | MetaAgentState::Rejected => {}
                }
            }
            let record = store
                .records
                .get_mut(&id)
                .ok_or_else(|| ApiError::not_found("meta-agent not found"))?;
            let previous_grant = record.grant.clone();
            record.previous_role = Some(record.role);
            record.previous_grant = Some(previous_grant);
            record.grant = narrowed_grant;
            record.role = request.role;
            record.safety_evidence = Some(safety);
            record.updated_at = now;
            Ok(Json(record.clone()))
        })
        .await
}

async fn rollback_morph(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    auth: Option<Extension<AuthContext>>,
) -> Result<Json<MetaAgentRecord>, ApiError> {
    let auth = require_auth(auth)?;
    let principal = auth_principal(&auth);
    let admin = is_admin(&auth);
    let current = state
        .meta_agents
        .read(|store| {
            let record = store
                .records
                .get(&id)
                .cloned()
                .ok_or_else(|| ApiError::not_found("meta-agent not found"))?;
            require_owner(&record, &principal, admin)?;
            if record.state != MetaAgentState::Active {
                return Err(ApiError::conflict(
                    "only active meta-agents may roll back a morph",
                ));
            }
            if record.previous_role.is_none() {
                return Err(ApiError::conflict(
                    "meta-agent has no pending morph rollback",
                ));
            }
            Ok(record)
        })
        .await?;
    let safety = RecursiveSafetyMonitor
        .validate_action(
            format!("rollback bounded role morph for meta-agent {id}"),
            morph_action_context(true),
        )
        .await
        .map_err(|error| ApiError::forbidden(error.to_string()))?;
    let now = now_secs()?;
    state
        .meta_agents
        .mutate(move |store| {
            if store.records.values().any(|child| {
                child.parent_id.as_deref() == Some(id.as_str())
                    && matches!(
                        child.state,
                        MetaAgentState::Proposed | MetaAgentState::Validating
                    )
            }) {
                return Err(ApiError::conflict(
                    "cannot roll back a morph while a child proposal is pending",
                ));
            }
            let record = store
                .records
                .get(&id)
                .cloned()
                .ok_or_else(|| ApiError::not_found("meta-agent not found"))?;
            if record.state != MetaAgentState::Active
                || record.role != current.role
                || record.grant != current.grant
                || record.previous_role != current.previous_role
                || record.previous_grant != current.previous_grant
            {
                return Err(ApiError::conflict(
                    "meta-agent changed during rollback validation",
                ));
            }
            let prior_role = record
                .previous_role
                .ok_or_else(|| ApiError::conflict("meta-agent has no pending morph rollback"))?;
            let prior_grant = record
                .previous_grant
                .clone()
                .ok_or_else(|| ApiError::conflict("meta-agent rollback grant is missing"))?;
            if let Some(parent_id) = &record.parent_id {
                let parent = store
                    .records
                    .get(parent_id)
                    .ok_or_else(|| ApiError::conflict("meta-agent parent disappeared"))?;
                if parent.state != MetaAgentState::Active {
                    return Err(ApiError::conflict("meta-agent parent is no longer active"));
                }
                roko_agent::safety::validate_delegation_at(
                    &parent.grant,
                    prior_role,
                    &prior_grant,
                    now,
                )
                .map_err(|error| ApiError::forbidden(error.to_string()))?;
            }
            for child in store.records.values().filter(|child| {
                child.parent_id.as_deref() == Some(id.as_str())
                    && child.state == MetaAgentState::Active
            }) {
                roko_agent::safety::validate_delegation_at(
                    &prior_grant,
                    child.role,
                    &child.grant,
                    now,
                )
                .map_err(|error| ApiError::conflict(error.to_string()))?;
            }
            let record = store
                .records
                .get_mut(&id)
                .ok_or_else(|| ApiError::not_found("meta-agent not found"))?;
            record.previous_role = None;
            record.previous_grant = None;
            record.grant = prior_grant;
            record.role = prior_role;
            record.safety_evidence = Some(safety);
            record.updated_at = now;
            Ok(Json(record.clone()))
        })
        .await
}

async fn deactivate_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    auth: Option<Extension<AuthContext>>,
) -> Result<Json<MetaAgentRecord>, ApiError> {
    let auth = require_auth(auth)?;
    let principal = auth_principal(&auth);
    let admin = is_admin(&auth);
    let now = now_secs()?;
    state
        .meta_agents
        .mutate(move |store| {
            let current = store
                .records
                .get(&id)
                .cloned()
                .ok_or_else(|| ApiError::not_found("meta-agent not found"))?;
            require_owner(&current, &principal, admin)?;
            if current.state != MetaAgentState::Active {
                return Err(ApiError::conflict(
                    "only active meta-agents may be deactivated",
                ));
            }
            if store.records.values().any(|child| {
                child.parent_id.as_deref() == Some(id.as_str())
                    && matches!(
                        child.state,
                        MetaAgentState::Proposed
                            | MetaAgentState::Validating
                            | MetaAgentState::Active
                    )
            }) {
                return Err(ApiError::conflict(
                    "deactivate live descendants before deactivating their parent",
                ));
            }
            let record = store
                .records
                .get_mut(&id)
                .ok_or_else(|| ApiError::not_found("meta-agent not found"))?;
            record.state = MetaAgentState::Deactivated;
            record.updated_at = now;
            Ok(Json(record.clone()))
        })
        .await
}

async fn validate_proposal(
    state: &AppState,
    proposal: &MetaAgentRecord,
    arena_id: [u8; 32],
    attempt_id: [u8; 32],
    output_hash: [u8; 32],
    evidence_hash: [u8; 32],
) -> Result<(RecursiveSafetyEvidence, ArenaAcceptanceEvidence), String> {
    if proposal.role != proposal.activation_role
        || proposal.grant != proposal.activation_grant
        || proposal.previous_role.is_some()
        || proposal.previous_grant.is_some()
    {
        return Err("proposal authority differs from its bound activation artifact".to_string());
    }
    validate_manifest(&proposal.manifest).map_err(|error| error.to_string())?;
    if hash_activation_artifact(proposal).map_err(|error| error.message)?
        != proposal.activation_artifact_hash
    {
        return Err(
            "persisted activation artifact hash no longer matches its proposal".to_string(),
        );
    }
    let safety = if let Some(parent_id) = &proposal.parent_id {
        let parent = state
            .meta_agents
            .read(|store| {
                store
                    .records
                    .get(parent_id)
                    .cloned()
                    .ok_or_else(|| ApiError::not_found("parent meta-agent not found"))
            })
            .await
            .map_err(|error| error.message)?;
        if parent.state != MetaAgentState::Active
            || parent.lineage_id != proposal.lineage_id
            || parent.generation.checked_add(1) != Some(proposal.generation)
        {
            return Err("durable parent lineage is no longer valid".to_string());
        }
        RecursiveSafetyMonitor
            .validate_activation(
                &parent.grant,
                proposal.role,
                &proposal.grant,
                format!("activate generated meta-agent {}", proposal.id),
                activation_action_context(),
                now_secs().map_err(|error| error.message)?,
            )
            .await
            .map_err(|error| error.to_string())?
    } else {
        validate_root_grant(proposal, now_secs().map_err(|error| error.message)?)?;
        RecursiveSafetyMonitor
            .validate_action(
                format!("activate root meta-agent {}", proposal.id),
                activation_action_context(),
            )
            .await
            .map_err(|error| error.to_string())?
    };
    let acceptance = state
        .arenas
        .verify_acceptance(
            arena_id,
            attempt_id,
            output_hash,
            evidence_hash,
            &proposal.owner_principal,
        )
        .await
        .map_err(|error| error.message)?;
    Ok((safety, acceptance))
}

fn validate_root_grant(proposal: &MetaAgentRecord, observed_at: u64) -> Result<(), String> {
    let mut ceiling = proposal.grant.clone();
    ceiling.tools = proposal.role.tool_permissions();
    ceiling.data_scopes = BTreeSet::from(["*".to_string()]);
    ceiling.network_hosts = BTreeSet::from(["*".to_string()]);
    ceiling.max_cost_usd = proposal.limits.max_lineage_cost_usd;
    ceiling.spawn.remaining_depth = proposal
        .limits
        .max_depth
        .checked_add(1)
        .ok_or_else(|| "root depth ceiling overflow".to_string())?;
    ceiling.spawn.max_children = proposal.limits.max_children_per_parent;
    ceiling.spawn.max_retries = proposal.limits.max_retries_per_parent;
    roko_agent::safety::validate_delegation_at(
        &ceiling,
        proposal.role,
        &proposal.grant,
        observed_at,
    )
    .map_err(|error| error.to_string())
}

fn final_activation_check(
    store: &MetaStore,
    proposal: &MetaAgentRecord,
    acceptance: &ArenaAcceptanceEvidence,
    observed_at: u64,
) -> Result<(), String> {
    if store.records.values().any(|record| {
        record.id != proposal.id
            && matches!(
                record.state,
                MetaAgentState::Active | MetaAgentState::Deactivated
            )
            && record.acceptance_evidence.as_ref().is_some_and(|used| {
                used.attempt_id == acceptance.attempt_id
                    || used.evidence_hash == acceptance.evidence_hash
            })
    }) {
        return Err("arena acceptance attempt or evidence was already consumed".to_string());
    }
    if let Some(parent_id) = &proposal.parent_id {
        let parent = store
            .records
            .get(parent_id)
            .ok_or_else(|| "durable parent disappeared during validation".to_string())?;
        if parent.state != MetaAgentState::Active
            || proposal.parent_role_at_proposal != Some(parent.role)
            || parent.lineage_id != proposal.lineage_id
            || parent.generation.checked_add(1) != Some(proposal.generation)
        {
            return Err(
                "durable parent state, role, or generation changed during validation".to_string(),
            );
        }
        roko_agent::safety::validate_delegation_at(
            &parent.grant,
            proposal.role,
            &proposal.grant,
            observed_at,
        )
        .map_err(|error| format!("final parent grant check failed: {error}"))?;
        let usage = successor_usage(store, parent);
        let effective_limits = effective_child_limits(parent).map_err(|error| error.message)?;
        if usage.direct_children > effective_limits.max_children_per_parent
            || usage.rejected_children > effective_limits.max_retries_per_parent
            || !usage.lineage_cost_usd.is_finite()
            || usage.lineage_cost_usd > parent.limits.max_lineage_cost_usd
        {
            return Err("durable lineage limits changed during validation".to_string());
        }
    } else {
        validate_root_grant(proposal, observed_at)?;
    }
    Ok(())
}

fn effective_child_limits(parent: &MetaAgentRecord) -> Result<MetaAgentLimits, ApiError> {
    if parent.grant.spawn.remaining_depth == 0 || parent.grant.spawn.max_children == 0 {
        return Err(ApiError::conflict(
            "parent meta-agent has no remaining spawn authority",
        ));
    }
    let delegated_depth = parent
        .generation
        .checked_add(parent.grant.spawn.remaining_depth)
        .ok_or_else(|| ApiError::conflict("meta-agent delegated depth overflow"))?;
    Ok(MetaAgentLimits {
        max_depth: parent.limits.max_depth.min(delegated_depth),
        max_children_per_parent: parent
            .limits
            .max_children_per_parent
            .min(parent.grant.spawn.max_children),
        max_retries_per_parent: parent
            .limits
            .max_retries_per_parent
            .min(parent.grant.spawn.max_retries),
        max_lineage_cost_usd: parent.limits.max_lineage_cost_usd,
    })
}

fn successor_usage(store: &MetaStore, parent: &MetaAgentRecord) -> SuccessorUsage {
    let direct_children = store
        .records
        .values()
        .filter(|record| {
            record.parent_id.as_deref() == Some(parent.id.as_str())
                && record.state != MetaAgentState::Rejected
        })
        .count();
    SuccessorUsage {
        direct_children: u32::try_from(direct_children).unwrap_or(u32::MAX),
        rejected_children: u32::try_from(
            store
                .records
                .values()
                .filter(|record| {
                    record.parent_id.as_deref() == Some(parent.id.as_str())
                        && record.state == MetaAgentState::Rejected
                })
                .count(),
        )
        .unwrap_or(u32::MAX),
        lineage_cost_usd: store
            .records
            .values()
            .filter(|record| {
                record.lineage_id == parent.lineage_id && record.state != MetaAgentState::Rejected
            })
            .map(|record| record.grant.max_cost_usd)
            .sum(),
    }
}

fn activation_action_context() -> ActionContext {
    ActionContext {
        // An authenticated principal explicitly requests this transition.
        autonomy_level: Some("assist".to_string()),
        // Active authority can be durably removed through `/deactivate`.
        reversible: Some(true),
        // The route cannot alter the fixed Graph, audit log, or its evidence.
        modifies_audit: Some(false),
        // The full artifact is hash-bound to independent R03 evidence.
        outputs_verifiable: Some(true),
        // Activation is the explicit endpoint task after all validations pass.
        on_task: Some(true),
    }
}

fn morph_action_context(rollback_available: bool) -> ActionContext {
    ActionContext {
        autonomy_level: Some("assist".to_string()),
        // True only when the same durable transaction installs one-step rollback.
        reversible: Some(rollback_available),
        modifies_audit: Some(false),
        // Role and capability intersection are deterministic typed operations.
        outputs_verifiable: Some(true),
        on_task: Some(true),
    }
}

fn require_auth(auth: Option<Extension<AuthContext>>) -> Result<AuthContext, ApiError> {
    auth.map(|auth| auth.0).ok_or_else(|| {
        ApiError::unauthorized(
            "meta-agent routes require authentication; enable serve auth and provide a credential",
        )
    })
}

fn auth_principal(auth: &AuthContext) -> String {
    auth.user_id
        .clone()
        .unwrap_or_else(|| format!("credential:{}", auth.scope))
}

fn is_admin(auth: &AuthContext) -> bool {
    matches!(auth.scope.as_str(), "admin" | "owner")
}

fn require_owner(record: &MetaAgentRecord, principal: &str, admin: bool) -> Result<(), ApiError> {
    if admin || record.owner_principal == principal {
        Ok(())
    } else {
        Err(ApiError::forbidden(
            "meta-agent belongs to another principal",
        ))
    }
}

fn now_secs() -> Result<u64, ApiError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| ApiError::internal(format!("read server clock: {error}")))?
        .as_secs()
        .max(1))
}

fn hash_activation_artifact(record: &MetaAgentRecord) -> Result<String, ApiError> {
    #[derive(Serialize)]
    struct ActivationArtifact<'a> {
        id: &'a str,
        owner_principal: &'a str,
        parent_id: &'a Option<String>,
        parent_role_at_proposal: Option<AgentRole>,
        lineage_id: &'a str,
        generation: u32,
        role: AgentRole,
        manifest: &'a AgentExtendedManifest,
        grant: &'a MetaAgentGrant,
        limits: MetaAgentLimits,
    }
    let artifact = ActivationArtifact {
        id: &record.id,
        owner_principal: &record.owner_principal,
        parent_id: &record.parent_id,
        parent_role_at_proposal: record.parent_role_at_proposal,
        lineage_id: &record.lineage_id,
        generation: record.generation,
        role: record.activation_role,
        manifest: &record.manifest,
        grant: &record.activation_grant,
        limits: record.limits,
    };
    let bytes = canonical_json_bytes(&artifact)
        .map_err(|error| ApiError::internal(format!("serialize activation artifact: {error}")))?;
    Ok(format_hash(blake3::hash(&bytes).as_bytes()))
}

fn parse_hash(value: &str) -> Result<[u8; 32], ApiError> {
    let value = value.strip_prefix("0x").unwrap_or(value);
    if value.len() != 64 {
        return Err(ApiError::bad_request("hash must contain exactly 32 bytes"));
    }
    let mut output = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let pair = std::str::from_utf8(pair)
            .map_err(|_| ApiError::bad_request("hash must be lowercase hexadecimal"))?;
        output[index] = u8::from_str_radix(pair, 16)
            .map_err(|_| ApiError::bad_request("hash must be hexadecimal"))?;
    }
    if output == [0; 32] {
        return Err(ApiError::bad_request("hash must not be zero"));
    }
    Ok(output)
}

fn format_hash(hash: &[u8; 32]) -> String {
    let mut output = String::with_capacity(64);
    for byte in hash {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn load_store(path: &FsPath) -> Result<MetaStore, String> {
    if !path.exists() {
        return Ok(MetaStore::default());
    }
    let bytes = std::fs::read(path).map_err(|error| format!("read snapshot: {error}"))?;
    let snapshot: MetaSnapshot =
        serde_json::from_slice(&bytes).map_err(|error| format!("decode snapshot: {error}"))?;
    if snapshot.schema_version != META_SCHEMA_VERSION {
        return Err(format!(
            "unsupported meta-agent snapshot schema {}",
            snapshot.schema_version
        ));
    }
    let expected = records_hash(&snapshot.records)?;
    if snapshot.content_hash != expected {
        return Err("meta-agent snapshot content hash mismatch".to_string());
    }
    let store = MetaStore {
        records: snapshot.records,
    };
    validate_store(&store)?;
    Ok(store)
}

fn recover_interrupted_validations(store: &mut MetaStore) -> bool {
    let mut recovered = false;
    for record in store.records.values_mut() {
        if record.state == MetaAgentState::Validating {
            record.state = MetaAgentState::Proposed;
            record.validation_lease = None;
            record.validation_started_at = None;
            recovered = true;
        }
    }
    recovered
}

fn reconcile_durable_acceptance_receipts(
    workdir: &FsPath,
    store: &MetaStore,
) -> Result<(), String> {
    let terminal = store.records.values().filter(|record| {
        matches!(
            record.state,
            MetaAgentState::Active | MetaAgentState::Deactivated
        )
    });
    let records = terminal.collect::<Vec<_>>();
    if records.is_empty() {
        return Ok(());
    }
    let arena_path = workdir.join(".roko").join("chain").join("arena-state.json");
    let registry = roko_chain::arena::ArenaRegistry::open(&arena_path)
        .map_err(|error| format!("open durable arena receipts: {error}"))?;
    for record in records {
        let receipt = record
            .acceptance_evidence
            .as_ref()
            .ok_or_else(|| format!("terminal meta-agent `{}` has no receipt", record.id))?;
        let output_hash =
            parse_hash(&record.activation_artifact_hash).map_err(|error| error.message)?;
        let durable = verify_acceptance_in_registry(
            &registry,
            receipt.arena_id,
            receipt.attempt_id,
            output_hash,
            receipt.evidence_hash,
            &record.owner_principal,
        )
        .map_err(|error| {
            format!(
                "terminal meta-agent `{}` has invalid durable arena receipt: {error}",
                record.id
            )
        })?;
        if &durable != receipt {
            return Err(format!(
                "terminal meta-agent `{}` arena receipt differs from durable R03 state",
                record.id
            ));
        }
    }
    Ok(())
}

fn persist_store(path: &FsPath, store: &MetaStore) -> std::io::Result<()> {
    validate_store(store)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    let snapshot = MetaSnapshot {
        schema_version: META_SCHEMA_VERSION,
        records: store.records.clone(),
        content_hash: records_hash(&store.records)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?,
    };
    let bytes = serde_json::to_vec_pretty(&snapshot)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    durable_atomic_write(path, &bytes)
}

fn canonical_safety_evidence(evidence: &RecursiveSafetyEvidence) -> bool {
    let expected = CorrigibilityHead::all_in_order()
        .into_iter()
        .map(|head| (head, HeadVerdict::Pass))
        .collect::<Vec<_>>();
    evidence.evaluated_nodes == expected.len() && evidence.decision.verdicts == expected
}

fn valid_exact_morph_state(record: &MetaAgentRecord) -> bool {
    if intersect_tools(
        record.activation_grant.tools,
        record.activation_role.tool_permissions(),
    ) != record.activation_grant.tools
    {
        return false;
    }
    match (record.previous_role, record.previous_grant.as_ref()) {
        (None, None) => {
            record.role == record.activation_role && record.grant == record.activation_grant
        }
        (Some(previous_role), Some(previous_grant)) => {
            let mut expected_current = record.activation_grant.clone();
            expected_current.tools =
                intersect_tools(expected_current.tools, record.role.tool_permissions());
            previous_role == record.activation_role
                && previous_grant == &record.activation_grant
                && record.role != record.activation_role
                && role_transition_allowed(record.activation_role, record.role)
                && record.grant == expected_current
        }
        _ => false,
    }
}

fn validate_store(store: &MetaStore) -> Result<(), String> {
    let mut active_attempts = BTreeSet::new();
    let mut active_evidence_hashes = BTreeSet::new();
    for (key, record) in &store.records {
        if key != &record.id {
            return Err(format!("meta-agent key `{key}` does not match record id"));
        }
        if record.id.trim().is_empty()
            || record.owner_principal.trim().is_empty()
            || record.lineage_id.trim().is_empty()
            || record
                .parent_id
                .as_ref()
                .is_some_and(|id| id.trim().is_empty())
            || record
                .manifest
                .name
                .as_deref()
                .is_none_or(|name| name.trim().is_empty())
            || parse_hash(&record.activation_artifact_hash).is_err()
            || record.created_at == 0
            || record.updated_at < record.created_at
        {
            return Err(format!(
                "meta-agent `{key}` has empty or invalid identity fields"
            ));
        }
        if record.manifest.generation != record.generation
            || record.manifest.lineage_id.as_deref() != Some(record.lineage_id.as_str())
        {
            return Err(format!("meta-agent `{key}` manifest lineage mismatch"));
        }
        if hash_activation_artifact(record).map_err(|error| error.message)?
            != record.activation_artifact_hash
        {
            return Err(format!(
                "meta-agent `{key}` activation artifact hash mismatch"
            ));
        }
        if !limits_are_bounded(record.limits)
            || !grant_is_structurally_bounded(&record.grant)
            || !grant_is_structurally_bounded(&record.activation_grant)
            || record
                .previous_grant
                .as_ref()
                .is_some_and(|grant| !grant_is_structurally_bounded(grant))
        {
            return Err(format!("meta-agent `{key}` contains invalid hard bounds"));
        }
        match record.state {
            MetaAgentState::Proposed => {
                if record.role != record.activation_role
                    || record.grant != record.activation_grant
                    || record.previous_role.is_some()
                    || record.previous_grant.is_some()
                    || record.validation_lease.is_some()
                    || record.validation_started_at.is_some()
                    || record.safety_evidence.is_some()
                    || record.acceptance_evidence.is_some()
                    || record.rejection_reason.is_some()
                {
                    return Err(format!(
                        "meta-agent `{key}` has incoherent proposed evidence"
                    ));
                }
            }
            MetaAgentState::Validating => {
                if record.role != record.activation_role
                    || record.grant != record.activation_grant
                    || record.previous_role.is_some()
                    || record.previous_grant.is_some()
                    || record.validation_lease.as_deref().is_none_or(str::is_empty)
                    || record
                        .validation_started_at
                        .is_none_or(|started| started == 0)
                    || record.safety_evidence.is_some()
                    || record.acceptance_evidence.is_some()
                    || record.rejection_reason.is_some()
                {
                    return Err(format!(
                        "meta-agent `{key}` has incoherent validation lease"
                    ));
                }
            }
            MetaAgentState::Active | MetaAgentState::Deactivated => {
                let safety = record
                    .safety_evidence
                    .as_ref()
                    .ok_or_else(|| format!("active meta-agent `{key}` has no safety evidence"))?;
                let acceptance = record.acceptance_evidence.as_ref().ok_or_else(|| {
                    format!("active meta-agent `{key}` has no acceptance evidence")
                })?;
                if record.validation_lease.is_some()
                    || record.validation_started_at.is_some()
                    || record.rejection_reason.is_some()
                    || !canonical_safety_evidence(safety)
                    || acceptance.arena_id == [0; 32]
                    || acceptance.attempt_id == [0; 32]
                    || acceptance.evidence_hash == [0; 32]
                    || acceptance.subject_output_hash == [0; 32]
                    || acceptance.scorer_principal.trim().is_empty()
                    || acceptance.observed_at_block == 0
                    || acceptance.subject_output_hash
                        != parse_hash(&record.activation_artifact_hash)
                            .map_err(|error| error.message)?
                    || !active_attempts.insert(acceptance.attempt_id)
                    || !active_evidence_hashes.insert(acceptance.evidence_hash)
                {
                    return Err(format!("active meta-agent `{key}` has incoherent evidence"));
                }
                if !valid_exact_morph_state(record) {
                    return Err(format!(
                        "active meta-agent `{key}` has incoherent morph authority"
                    ));
                }
            }
            MetaAgentState::Rejected => {
                if record.role != record.activation_role
                    || record.grant != record.activation_grant
                    || record.previous_role.is_some()
                    || record.previous_grant.is_some()
                    || record.validation_lease.is_some()
                    || record.validation_started_at.is_some()
                    || record.acceptance_evidence.is_some()
                    || record.safety_evidence.is_some()
                    || record
                        .rejection_reason
                        .as_deref()
                        .is_none_or(|reason| reason.trim().is_empty())
                {
                    return Err(format!(
                        "meta-agent `{key}` has incoherent rejection evidence"
                    ));
                }
            }
        }
        if let Some(parent_id) = &record.parent_id {
            let parent = store
                .records
                .get(parent_id)
                .ok_or_else(|| format!("meta-agent `{key}` has missing parent `{parent_id}`"))?;
            if parent.generation.checked_add(1) != Some(record.generation)
                || parent.lineage_id != record.lineage_id
                || parent.limits != record.limits
            {
                return Err(format!("meta-agent `{key}` has broken inherited lineage"));
            }
            if record.parent_role_at_proposal.is_none()
                || (matches!(
                    record.state,
                    MetaAgentState::Proposed | MetaAgentState::Validating | MetaAgentState::Active
                ) && parent.state != MetaAgentState::Active)
                || (record.state == MetaAgentState::Deactivated
                    && !matches!(
                        parent.state,
                        MetaAgentState::Active | MetaAgentState::Deactivated
                    ))
            {
                return Err(format!("meta-agent `{key}` has a non-active parent"));
            }
            if record.state == MetaAgentState::Active {
                roko_agent::safety::validate_delegation_structure(
                    &parent.grant,
                    record.role,
                    &record.grant,
                )
                .map_err(|error| format!("live child `{key}` widens parent: {error}"))?;
            }
        } else {
            if record.generation != 0
                || record.lineage_id != record.id
                || record.parent_role_at_proposal.is_some()
            {
                return Err(format!("meta-agent root `{key}` has invalid lineage"));
            }
            if matches!(
                record.state,
                MetaAgentState::Active | MetaAgentState::Deactivated
            ) && (record.activation_grant.max_cost_usd > record.limits.max_lineage_cost_usd
                || record.activation_grant.spawn.remaining_depth > record.limits.max_depth
                || record.activation_grant.spawn.max_children
                    > record.limits.max_children_per_parent
                || record.activation_grant.spawn.max_retries > record.limits.max_retries_per_parent)
            {
                return Err(format!(
                    "terminal meta-agent root `{key}` exceeds its lineage limits"
                ));
            }
        }
    }

    for record in store.records.values() {
        let mut cursor = record;
        let mut traversed = 0_u32;
        while let Some(parent_id) = &cursor.parent_id {
            traversed = traversed.saturating_add(1);
            if traversed > record.limits.max_depth {
                return Err(format!(
                    "meta-agent `{}` lineage does not reach a root",
                    record.id
                ));
            }
            cursor = &store.records[parent_id];
        }
        if cursor.id != record.lineage_id || traversed != record.generation {
            return Err(format!(
                "meta-agent `{}` root traversal mismatch",
                record.id
            ));
        }
    }

    for parent in store.records.values() {
        let usage = successor_usage(store, parent);
        let active_children = usage.direct_children;
        let max_children = parent
            .limits
            .max_children_per_parent
            .min(parent.grant.spawn.max_children);
        let max_retries = parent
            .limits
            .max_retries_per_parent
            .min(parent.grant.spawn.max_retries);
        if active_children > max_children
            || usage.rejected_children > max_retries.saturating_add(1)
            || !usage.lineage_cost_usd.is_finite()
            || usage.lineage_cost_usd > parent.limits.max_lineage_cost_usd
        {
            return Err(format!(
                "meta-agent `{}` durable usage exceeds bounds",
                parent.id
            ));
        }
    }
    Ok(())
}

fn limits_are_bounded(limits: MetaAgentLimits) -> bool {
    limits.max_depth > 0
        && limits.max_depth <= MAX_META_AGENT_DEPTH
        && limits.max_children_per_parent > 0
        && limits.max_children_per_parent <= MAX_META_AGENT_FANOUT
        && limits.max_retries_per_parent <= MAX_META_AGENT_RETRIES
        && limits.max_lineage_cost_usd.is_finite()
        && limits.max_lineage_cost_usd > 0.0
        && limits.max_lineage_cost_usd <= MAX_META_AGENT_LINEAGE_COST_USD
}

fn grant_is_structurally_bounded(grant: &MetaAgentGrant) -> bool {
    grant.expires_at.is_some_and(|expiry| expiry > 0)
        && grant.max_cost_usd.is_finite()
        && grant.max_cost_usd >= 0.0
        && grant.max_cost_usd <= MAX_META_AGENT_LINEAGE_COST_USD
        && grant.spawn.remaining_depth <= MAX_META_AGENT_DEPTH
        && grant.spawn.max_children <= MAX_META_AGENT_FANOUT
        && grant.spawn.max_retries <= MAX_META_AGENT_RETRIES
}

fn records_hash(records: &BTreeMap<String, MetaAgentRecord>) -> Result<String, String> {
    canonical_json_bytes(records)
        .map(|bytes| format_hash(blake3::hash(&bytes).as_bytes()))
        .map_err(|error| format!("serialize meta-agent records: {error}"))
}

fn canonical_json_bytes(value: &impl Serialize) -> Result<Vec<u8>, serde_json::Error> {
    fn canonicalize(value: Value) -> Value {
        match value {
            Value::Object(values) => {
                let sorted = values
                    .into_iter()
                    .map(|(key, value)| (key, canonicalize(value)))
                    .collect::<BTreeMap<_, _>>();
                Value::Object(sorted.into_iter().collect())
            }
            Value::Array(values) => Value::Array(values.into_iter().map(canonicalize).collect()),
            scalar => scalar,
        }
    }
    serde_json::to_vec(&canonicalize(serde_json::to_value(value)?))
}

fn durable_atomic_write(path: &FsPath, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "state path has no parent")
    })?;
    std::fs::create_dir_all(parent)?;
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(
        ".meta-lineage.tmp.{}.{}",
        std::process::id(),
        sequence
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        std::fs::rename(&temporary, path)?;
        File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use roko_agent::lifecycle::{
        AgentCoreManifest, CustomPluginConfig, DeploymentMode, DomainPlugin,
    };
    use roko_agent::safety::SpawnAuthority;
    use roko_chain::arena::{
        AggregationRule, Arena, ArenaCategory, ArenaRegistry, ArenaState, AttemptSettlement,
        BinaryCriterion, GroundTruthSource, ScoringEvidence, ScoringFunction, TaskSource,
    };
    use roko_core::ToolPermissions;
    use roko_core::corrigibility::{CorrigibilityDecision, CorrigibilityHead, HeadVerdict};
    use tempfile::tempdir;

    fn unix_test_now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test clock")
            .as_secs()
    }

    fn record(id: &str) -> MetaAgentRecord {
        let mut manifest = AgentExtendedManifest::new(AgentCoreManifest {
            prompt: "A bounded test meta-agent".to_string(),
            mode: DeploymentMode::SelfHosted,
            domain: None,
            schema_version: 1,
        });
        manifest.name = Some(id.to_string());
        manifest.lineage_id = Some(id.to_string());
        let mut record = MetaAgentRecord {
            id: id.to_string(),
            owner_principal: "owner".to_string(),
            parent_id: None,
            parent_role_at_proposal: None,
            lineage_id: id.to_string(),
            generation: 0,
            role: AgentRole::Implementer,
            previous_role: None,
            previous_grant: None,
            activation_role: AgentRole::Implementer,
            manifest,
            activation_artifact_hash: "01".repeat(32),
            grant: MetaAgentGrant {
                tools: ToolPermissions::read_only(),
                data_scopes: BTreeSet::new(),
                network_hosts: BTreeSet::new(),
                max_cost_usd: 1.0,
                expires_at: Some(unix_test_now().saturating_add(3_600)),
                spawn: SpawnAuthority {
                    remaining_depth: 2,
                    max_children: 2,
                    max_retries: 1,
                },
            },
            activation_grant: MetaAgentGrant {
                tools: ToolPermissions::read_only(),
                data_scopes: BTreeSet::new(),
                network_hosts: BTreeSet::new(),
                max_cost_usd: 1.0,
                expires_at: Some(unix_test_now().saturating_add(3_600)),
                spawn: SpawnAuthority {
                    remaining_depth: 2,
                    max_children: 2,
                    max_retries: 1,
                },
            },
            limits: MetaAgentLimits::default(),
            state: MetaAgentState::Proposed,
            validation_lease: None,
            validation_started_at: None,
            rejection_reason: None,
            safety_evidence: None,
            acceptance_evidence: None,
            created_at: 1,
            updated_at: 1,
        };
        record.activation_artifact_hash = hash_activation_artifact(&record).expect("artifact hash");
        record
    }

    fn safety_evidence() -> RecursiveSafetyEvidence {
        RecursiveSafetyEvidence {
            decision: CorrigibilityDecision::new(
                CorrigibilityHead::all_in_order()
                    .into_iter()
                    .map(|head| (head, HeadVerdict::Pass))
                    .collect(),
            ),
            evaluated_nodes: 5,
        }
    }

    fn acceptance(record: &MetaAgentRecord, attempt_id: [u8; 32]) -> ArenaAcceptanceEvidence {
        ArenaAcceptanceEvidence {
            arena_id: [3; 32],
            attempt_id,
            evidence_hash: attempt_id,
            subject_output_hash: parse_hash(&record.activation_artifact_hash).expect("hash"),
            scorer_principal: "scorer".to_string(),
            observed_at_block: 10,
        }
    }

    fn activate(record: &mut MetaAgentRecord, attempt_id: [u8; 32]) {
        record.state = MetaAgentState::Active;
        record.safety_evidence = Some(safety_evidence());
        record.acceptance_evidence = Some(acceptance(record, attempt_id));
        record.validation_lease = None;
        record.validation_started_at = None;
    }

    fn child_record(parent: &MetaAgentRecord, id: &str) -> MetaAgentRecord {
        let mut child = record(id);
        child.parent_id = Some(parent.id.clone());
        child.parent_role_at_proposal = Some(parent.role);
        child.lineage_id = parent.lineage_id.clone();
        child.generation = parent.generation + 1;
        child.manifest.lineage_id = Some(child.lineage_id.clone());
        child.manifest.generation = child.generation;
        child.limits = parent.limits;
        child.grant.spawn.remaining_depth = parent.grant.spawn.remaining_depth - 1;
        child.activation_grant = child.grant.clone();
        child.activation_artifact_hash = hash_activation_artifact(&child).expect("child hash");
        child.state = MetaAgentState::Validating;
        child.validation_lease = Some("lease".to_string());
        child.validation_started_at = Some(1);
        child
    }

    #[test]
    fn durable_store_survives_restart_and_rejects_tampering() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("meta.json");
        let mut store = MetaStore::default();
        store.records.insert("root".to_string(), record("root"));
        persist_store(&path, &store).expect("persist");
        assert_eq!(load_store(&path).expect("reload").records.len(), 1);

        let mut value: Value =
            serde_json::from_slice(&std::fs::read(&path).expect("read")).expect("json");
        value["records"]["root"]["generation"] = json!(9);
        std::fs::write(&path, serde_json::to_vec(&value).expect("serialize")).expect("tamper");
        assert!(load_store(&path).unwrap_err().contains("content hash"));
    }

    #[test]
    fn canonical_hash_survives_restart_with_nonempty_hash_maps() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("meta.json");
        let mut item = record("mapped");
        item.manifest.core.domain = Some(DomainPlugin::Custom(CustomPluginConfig {
            id: "plugin".to_string(),
            params: HashMap::from([
                ("zeta".to_string(), "last".to_string()),
                ("alpha".to_string(), "first".to_string()),
            ]),
        }));
        item.activation_artifact_hash = hash_activation_artifact(&item).expect("artifact hash");
        let expected = item.activation_artifact_hash.clone();
        let mut store = MetaStore::default();
        store.records.insert(item.id.clone(), item);
        persist_store(&path, &store).expect("persist mapped snapshot");
        let loaded = load_store(&path).expect("reload mapped snapshot");
        assert_eq!(loaded.records["mapped"].activation_artifact_hash, expected);
        assert_eq!(records_hash(&loaded.records), records_hash(&store.records));
    }

    #[test]
    fn restart_recovers_interrupted_validation_to_proposed() {
        let mut store = MetaStore::default();
        let mut item = record("validating");
        item.state = MetaAgentState::Validating;
        item.validation_lease = Some("lease".to_string());
        item.validation_started_at = Some(1);
        store.records.insert(item.id.clone(), item);
        assert!(recover_interrupted_validations(&mut store));
        let recovered = &store.records["validating"];
        assert_eq!(recovered.state, MetaAgentState::Proposed);
        assert!(recovered.validation_lease.is_none());
    }

    #[tokio::test]
    async fn runtime_recovers_on_disk_validation_lease_after_restart() {
        let dir = tempdir().expect("tempdir");
        let path = dir
            .path()
            .join(".roko")
            .join("agents")
            .join("meta-lineage.json");
        let mut store = MetaStore::default();
        let mut item = record("validating-disk");
        item.state = MetaAgentState::Validating;
        item.validation_lease = Some("interrupted".to_string());
        item.validation_started_at = Some(1);
        store.records.insert(item.id.clone(), item);
        persist_store(&path, &store).expect("persist validating state");

        let runtime = MetaAgentRuntime::open(dir.path());
        let state = runtime
            .read(|store| Ok(store.records["validating-disk"].state))
            .await
            .expect("runtime read");
        assert_eq!(state, MetaAgentState::Proposed);
        assert_eq!(
            load_store(&path).expect("recovered disk").records["validating-disk"].state,
            MetaAgentState::Proposed
        );
    }

    #[test]
    fn restart_reconciles_terminal_receipts_with_durable_arena_state() {
        let dir = tempdir().expect("tempdir");
        let arena_path = dir
            .path()
            .join(".roko")
            .join("chain")
            .join("arena-state.json");
        let mut registry = ArenaRegistry::new();
        registry.set_block(1);
        registry
            .create_arena(Arena {
                id: [1; 32],
                name: "restart receipt arena".to_string(),
                description: String::new(),
                category: ArenaCategory::Coding,
                state: ArenaState::Draft,
                task_source: TaskSource::Static,
                scoring: ScoringFunction::Binary(BinaryCriterion::TestSuitePass),
                aggregation: AggregationRule::Median,
                weight: 1.0,
                creator_identity_id: 1,
                creator_principal: "arena-owner".to_string(),
                prize_pool_usdc: 0,
                max_attempts_per_agent: 2,
                cooldown_blocks: 0,
                deadline_block: 0,
                ground_truth: GroundTruthSource::TestSuite,
            })
            .expect("create arena");
        registry.activate_arena(&[1; 32]).expect("activate arena");

        let mut item = record("receipt-bound");
        let output_hash = parse_hash(&item.activation_artifact_hash).expect("artifact hash");
        registry.set_block(2);
        let attempt = registry
            .start_attempt_for_principal(&[1; 32], 7, item.owner_principal.clone(), Some([2; 32]))
            .expect("start attempt");
        registry
            .submit_attempt_with_output(&attempt.id, Some(output_hash))
            .expect("submit artifact");
        registry.set_block(4);
        registry
            .settle_attempt(
                &attempt.id,
                ScoringEvidence {
                    source: GroundTruthSource::TestSuite,
                    scorer_identity_id: 9,
                    scorer_principal: "independent-scorer".to_string(),
                    evidence_hash: [4; 32],
                    subject_output_hash: output_hash,
                    observed_at_block: 4,
                },
                AttemptSettlement::Completed {
                    score: 1.0,
                    gate_verdicts: vec![true],
                },
            )
            .expect("settle attempt");
        registry.persist(&arena_path).expect("persist arena");

        item.state = MetaAgentState::Active;
        item.safety_evidence = Some(safety_evidence());
        item.acceptance_evidence = Some(ArenaAcceptanceEvidence {
            arena_id: [1; 32],
            attempt_id: attempt.id,
            evidence_hash: [4; 32],
            subject_output_hash: output_hash,
            scorer_principal: "independent-scorer".to_string(),
            observed_at_block: 4,
        });
        let meta_path = dir
            .path()
            .join(".roko")
            .join("agents")
            .join("meta-lineage.json");
        let store = MetaStore {
            records: BTreeMap::from([(item.id.clone(), item.clone())]),
        };
        persist_store(&meta_path, &store).expect("persist terminal meta state");
        MetaAgentRuntime::open(dir.path())
            .ensure_available()
            .expect("durable receipt reconciles");

        item.state = MetaAgentState::Deactivated;
        let deactivated = MetaStore {
            records: BTreeMap::from([(item.id.clone(), item.clone())]),
        };
        persist_store(&meta_path, &deactivated).expect("persist deactivated receipt");
        MetaAgentRuntime::open(dir.path())
            .ensure_available()
            .expect("deactivated receipt also reconciles");

        item.acceptance_evidence
            .as_mut()
            .expect("receipt")
            .scorer_principal = "forged-scorer".to_string();
        let forged = MetaStore {
            records: BTreeMap::from([(item.id.clone(), item)]),
        };
        persist_store(&meta_path, &forged).expect("persist structurally valid forgery");
        assert!(
            MetaAgentRuntime::open(dir.path())
                .ensure_available()
                .is_err()
        );
    }

    #[test]
    fn correctly_rehashed_incoherent_snapshot_is_rejected() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("meta.json");
        let mut item = record("incoherent");
        item.role = AgentRole::Auditor;
        // The immutable artifact remains internally rehashed and intact; the
        // lifecycle/current-authority mismatch must still fail semantically.
        item.activation_artifact_hash = hash_activation_artifact(&item).expect("artifact");
        let records = BTreeMap::from([(item.id.clone(), item)]);
        let snapshot = MetaSnapshot {
            schema_version: META_SCHEMA_VERSION,
            content_hash: records_hash(&records).expect("records hash"),
            records,
        };
        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&snapshot).expect("snapshot"),
        )
        .expect("write incoherent snapshot");
        assert!(load_store(&path).unwrap_err().contains("proposed evidence"));
    }

    #[test]
    fn correctly_rehashed_noncanonical_safety_evidence_is_rejected() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("meta.json");
        let mut item = record("unsafe-evidence");
        activate(&mut item, [8; 32]);
        let safety = item.safety_evidence.as_mut().expect("safety");
        safety.evaluated_nodes = 5;
        safety.decision.verdicts = vec![
            (CorrigibilityHead::Deference, HeadVerdict::Pass),
            (CorrigibilityHead::Deference, HeadVerdict::Pass),
            (CorrigibilityHead::Deference, HeadVerdict::Pass),
            (CorrigibilityHead::Deference, HeadVerdict::Pass),
            (CorrigibilityHead::Deference, HeadVerdict::Pass),
        ];
        let records = BTreeMap::from([(item.id.clone(), item)]);
        let snapshot = MetaSnapshot {
            schema_version: META_SCHEMA_VERSION,
            content_hash: records_hash(&records).expect("records hash"),
            records,
        };
        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&snapshot).expect("snapshot"),
        )
        .expect("write tampered snapshot");
        assert!(
            load_store(&path)
                .unwrap_err()
                .contains("incoherent evidence")
        );
    }

    #[test]
    fn correctly_rehashed_morph_state_tampering_is_rejected() {
        let mut no_history = record("no-history");
        activate(&mut no_history, [8; 32]);
        no_history.role = AgentRole::Auditor;
        no_history.grant.tools = intersect_tools(
            no_history.activation_grant.tools,
            AgentRole::Auditor.tool_permissions(),
        );
        let store = MetaStore {
            records: BTreeMap::from([(no_history.id.clone(), no_history)]),
        };
        assert!(
            validate_store(&store)
                .unwrap_err()
                .contains("morph authority")
        );

        let mut forged_history = record("forged-history");
        activate(&mut forged_history, [9; 32]);
        forged_history.role = AgentRole::Auditor;
        forged_history.grant.tools = intersect_tools(
            forged_history.activation_grant.tools,
            AgentRole::Auditor.tool_permissions(),
        );
        forged_history.previous_role = Some(forged_history.activation_role);
        let mut widened_history = forged_history.activation_grant.clone();
        widened_history.max_cost_usd = 0.5;
        forged_history.previous_grant = Some(widened_history);
        let store = MetaStore {
            records: BTreeMap::from([(forged_history.id.clone(), forged_history)]),
        };
        assert!(
            validate_store(&store)
                .unwrap_err()
                .contains("morph authority")
        );
    }

    #[test]
    fn accepted_artifact_binds_owner_limits_and_parent_role_history() {
        let mut owner = record("owner-bound");
        activate(&mut owner, [8; 32]);
        owner.owner_principal = "attacker".to_string();
        owner.activation_artifact_hash = hash_activation_artifact(&owner).expect("rehashed owner");
        let store = MetaStore {
            records: BTreeMap::from([(owner.id.clone(), owner)]),
        };
        assert!(
            validate_store(&store)
                .unwrap_err()
                .contains("incoherent evidence")
        );

        let mut limits = record("limits-bound");
        activate(&mut limits, [9; 32]);
        limits.limits.max_depth -= 1;
        limits.activation_artifact_hash =
            hash_activation_artifact(&limits).expect("rehashed limits");
        let store = MetaStore {
            records: BTreeMap::from([(limits.id.clone(), limits)]),
        };
        assert!(
            validate_store(&store)
                .unwrap_err()
                .contains("incoherent evidence")
        );

        let mut parent = record("history-parent");
        activate(&mut parent, [10; 32]);
        let mut child = child_record(&parent, "history-child");
        activate(&mut child, [11; 32]);
        child.parent_role_at_proposal = Some(AgentRole::Auditor);
        child.activation_artifact_hash =
            hash_activation_artifact(&child).expect("rehashed parent role");
        let store = MetaStore {
            records: BTreeMap::from([(parent.id.clone(), parent), (child.id.clone(), child)]),
        };
        assert!(
            validate_store(&store)
                .unwrap_err()
                .contains("incoherent evidence")
        );
    }

    #[test]
    fn deactivated_child_requires_active_or_deactivated_parent() {
        let mut parent = record("inactive-parent");
        parent.state = MetaAgentState::Rejected;
        parent.rejection_reason = Some("rejected".to_string());
        let mut child = child_record(&parent, "deactivated-child");
        activate(&mut child, [8; 32]);
        child.state = MetaAgentState::Deactivated;
        let store = MetaStore {
            records: BTreeMap::from([(parent.id.clone(), parent), (child.id.clone(), child)]),
        };
        assert!(
            validate_store(&store)
                .unwrap_err()
                .contains("non-active parent")
        );
    }

    #[test]
    fn final_activation_rechecks_parent_role_and_single_use_evidence() {
        let mut parent = record("parent");
        activate(&mut parent, [8; 32]);
        let child = child_record(&parent, "child");
        let child_acceptance = acceptance(&child, [9; 32]);
        let mut store = MetaStore::default();
        store.records.insert(parent.id.clone(), parent.clone());
        store.records.insert(child.id.clone(), child.clone());
        final_activation_check(&store, &child, &child_acceptance, unix_test_now())
            .expect("current parent permits activation");

        store.records.get_mut("parent").expect("parent").role = AgentRole::Auditor;
        assert!(
            final_activation_check(&store, &child, &child_acceptance, unix_test_now())
                .unwrap_err()
                .contains("parent state, role")
        );

        store.records.get_mut("parent").expect("parent").role = parent.role;
        let mut used = record("used");
        activate(&mut used, [9; 32]);
        used.state = MetaAgentState::Deactivated;
        store.records.insert(used.id.clone(), used);
        assert!(
            final_activation_check(&store, &child, &child_acceptance, unix_test_now())
                .unwrap_err()
                .contains("already consumed")
        );
    }

    #[test]
    fn delegated_zero_and_narrow_spawn_caps_are_effective() {
        let mut parent = record("spawn-parent");
        parent.grant.spawn.max_children = 0;
        assert!(effective_child_limits(&parent).is_err());

        parent.grant.spawn.max_children = 1;
        parent.grant.spawn.max_retries = 0;
        let limits = effective_child_limits(&parent).expect("narrow limits");
        assert_eq!(limits.max_children_per_parent, 1);
        assert_eq!(limits.max_retries_per_parent, 0);
    }

    #[tokio::test]
    async fn operation_context_fails_closed_without_real_rollback() {
        let error = RecursiveSafetyMonitor
            .validate_action("unrecoverable morph", morph_action_context(false))
            .await
            .expect_err("Impact head must veto an irreversible route operation");
        assert!(error.to_string().contains("vetoed"));
    }

    #[tokio::test]
    async fn persistence_failure_rolls_back_memory() {
        let dir = tempdir().expect("tempdir");
        let blocking_file = dir.path().join("blocking");
        std::fs::write(&blocking_file, b"not a directory").expect("blocking file");
        let runtime = MetaAgentRuntime {
            path: blocking_file.join("meta.json"),
            store: Mutex::new(MetaStore::default()),
            startup_error: None,
        };
        assert!(
            runtime
                .mutate(|store| {
                    store.records.insert("root".to_string(), record("root"));
                    Ok(())
                })
                .await
                .is_err()
        );
        assert_eq!(
            runtime
                .read(|store| Ok(store.records.len()))
                .await
                .expect("read"),
            0
        );
    }
}
