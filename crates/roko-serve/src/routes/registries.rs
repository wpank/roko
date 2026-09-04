//! Durable local registry lifecycle and optional read-only chain indexing.
//!
//! Local passport and knowledge operations are restart-safe and remain usable
//! without chain configuration. When an RPC client and registry addresses are
//! configured, the same API exposes a finality-aware normalized event index;
//! it never turns ordinary local agent operation into a chain dependency.

use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use roko_chain::agent_registry::{
    AgentRegistryEvent, CAP_ANALYTICS, CAP_DATA_TRANSFORM, CAP_FINE_TUNE, CAP_INFERENCE,
    CAP_KNOWLEDGE, CAP_MULTI_AGENT, CAP_RAG, CAP_SECURITY, CAP_STRATEGY, CAP_TRADING,
    RegistryError,
};
use roko_chain::{
    AgentRegistry, AgentRegistrySnapshot, DelegationCaveat, EventIndexer, EventIndexerConfig,
    KnowledgeEntryState, KnowledgeRegistry, KnowledgeRegistryEntry, KnowledgeRegistrySnapshot,
    RegistryContract, RegistryEventQuery,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::Mutex;

use crate::error::ApiError;
use crate::extract::ApiJson;
use crate::state::AppState;

const LOCAL_ADMIN: &str = "roko-serve";
const LOCAL_SCHEMA_VERSION: u32 = 1;
const DEFAULT_PAGE_SIZE: usize = 50;
const MAX_PAGE_SIZE: usize = 250;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Shared durable registry runtime owned by [`AppState`].
pub(crate) struct RegistryRuntime {
    state_path: PathBuf,
    local: Mutex<LocalRegistryState>,
    local_error: Option<String>,
    indexer: Mutex<Option<EventIndexer>>,
    indexer_config: Option<EventIndexerConfig>,
    indexer_error: Option<String>,
    chain: Option<Arc<dyn roko_chain::ChainClient>>,
}

#[derive(Debug, Clone)]
struct LocalRegistryState {
    passports: AgentRegistry,
    knowledge: KnowledgeRegistry,
}

impl Default for LocalRegistryState {
    fn default() -> Self {
        Self {
            passports: AgentRegistry::new(LOCAL_ADMIN),
            knowledge: KnowledgeRegistry::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LocalRegistrySnapshot {
    schema_version: u32,
    passports: AgentRegistrySnapshot,
    knowledge: KnowledgeRegistrySnapshot,
}

impl RegistryRuntime {
    /// Open local state and, independently, an optional chain indexer.
    pub(crate) fn open(
        workdir: &FsPath,
        config: &roko_core::config::schema::RokoConfig,
        chain: Option<Arc<dyn roko_chain::ChainClient>>,
    ) -> Self {
        let state_path = workdir
            .join(".roko")
            .join("chain")
            .join("registry-state.json");
        let (local, local_error) = match load_local_state(&state_path) {
            Ok(state) => (state, None),
            Err(error) => (LocalRegistryState::default(), Some(error)),
        };

        let contracts = configured_contracts(&config.chain);
        let (indexer, indexer_config, indexer_error) = match (chain.clone(), contracts.is_empty()) {
            (Some(chain), false) => {
                let indexer_config = EventIndexerConfig {
                    contracts,
                    store_path: workdir.join(".roko").join("chain").join("events.jsonl"),
                    start_block: 0,
                    finality_confirmations: config.chain.finality_confirmations.unwrap_or(0),
                    max_batch_size: 256,
                    max_retained_events: 100_000,
                };
                match EventIndexer::open(chain, indexer_config.clone()) {
                    Ok(indexer) => (Some(indexer), Some(indexer_config), None),
                    Err(error) => (None, Some(indexer_config), Some(error.to_string())),
                }
            }
            _ => (None, None, None),
        };

        Self {
            state_path,
            local: Mutex::new(local),
            local_error,
            indexer: Mutex::new(indexer),
            indexer_config,
            indexer_error,
            chain,
        }
    }

    fn ensure_local(&self) -> Result<(), ApiError> {
        self.local_error.as_ref().map_or(Ok(()), |error| {
            Err(service_unavailable(
                "local registry state is unavailable",
                Some(error),
            ))
        })
    }

    async fn mutate<T>(
        &self,
        operation: impl FnOnce(&mut LocalRegistryState) -> Result<T, ApiError>,
    ) -> Result<T, ApiError> {
        self.ensure_local()?;
        let mut state = self.local.lock().await;
        let previous = state.clone();
        let output = match operation(&mut state) {
            Ok(output) => output,
            Err(error) => {
                *state = previous;
                return Err(error);
            }
        };
        if let Err(error) = persist_local_state(&self.state_path, &state) {
            *state = previous;
            return Err(ApiError::internal(format!(
                "persist local registry state: {error}"
            )));
        }
        Ok(output)
    }
}

/// Registry and passport routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/registries/passports",
            get(list_passports).post(mint_passport),
        )
        .route("/registries/passports/{id}", get(get_passport))
        .route("/registries/passports/{id}/history", get(passport_history))
        .route(
            "/registries/passports/{id}/transfer",
            post(transfer_passport),
        )
        .route(
            "/registries/passports/{id}/metadata",
            put(update_passport_metadata),
        )
        .route(
            "/registries/passports/{id}/delegations",
            post(add_delegation),
        )
        .route(
            "/registries/passports/{id}/delegations/{delegatee}",
            delete(revoke_delegation),
        )
        .route(
            "/registries/knowledge",
            get(list_knowledge).post(publish_knowledge),
        )
        .route("/registries/knowledge/{id}", get(get_knowledge))
        .route(
            "/registries/knowledge/{id}/validate",
            post(validate_knowledge),
        )
        .route(
            "/registries/knowledge/{id}/challenge",
            post(challenge_knowledge),
        )
        .route(
            "/registries/knowledge/challenges/{id}/resolve",
            post(resolve_knowledge_challenge),
        )
        .route("/registries/events", get(list_events))
        .route("/registries/stats", get(registry_stats))
        .route("/registries/indexer/sync", post(sync_indexer))
        .route("/registries/indexer/rebuild", post(rebuild_indexer))
}

#[derive(Debug, Default, Deserialize)]
struct PageQuery {
    #[serde(default)]
    offset: usize,
    #[serde(default)]
    limit: usize,
    tier: Option<roko_chain::PassportTier>,
    capability: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MintPassportRequest {
    /// Registry lifecycle owner value. The route is restricted to an
    /// authenticated workspace admin; this string is not derived from or
    /// cryptographically bound to the HTTP bearer identity.
    owner: String,
    #[serde(default)]
    capabilities: Vec<String>,
    system_prompt_hash: String,
    #[serde(default)]
    initial_stake: u128,
}

#[derive(Debug, Deserialize)]
struct TransferPassportRequest {
    from: String,
    to: String,
    block: u64,
}

#[derive(Debug, Deserialize)]
struct UpdateMetadataRequest {
    owner: String,
    #[serde(default)]
    service_endpoints: Vec<String>,
    #[serde(default)]
    feeds: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct AddDelegationRequest {
    owner: String,
    delegatee: u128,
    capabilities: Vec<String>,
    expiry_block: u64,
    max_spend: Option<u128>,
    scope: Option<String>,
    #[serde(default)]
    current_block: u64,
}

#[derive(Debug, Deserialize)]
struct OwnerQuery {
    owner: String,
}

async fn list_passports(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PageQuery>,
) -> Result<Json<Value>, ApiError> {
    state.registries.ensure_local()?;
    let capability = query
        .capability
        .as_deref()
        .map(capability_bit)
        .transpose()?;
    let state = state.registries.local.lock().await;
    let matching = state
        .passports
        .passports()
        .into_iter()
        .filter(|passport| query.tier.is_none_or(|tier| passport.tier == tier))
        .filter(|passport| capability.is_none_or(|bit| passport.capability_list & bit == bit))
        .cloned()
        .collect::<Vec<_>>();
    let total = matching.len();
    let limit = page_size(query.limit);
    let passports = matching
        .into_iter()
        .skip(query.offset)
        .take(limit)
        .collect::<Vec<_>>();
    Ok(Json(json!({
        "source": "local_durable",
        "passports": passports,
        "offset": query.offset,
        "limit": limit,
        "total": total,
    })))
}

async fn mint_passport(
    State(state): State<Arc<AppState>>,
    ApiJson(request): ApiJson<MintPassportRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let owner = nonblank(request.owner, "owner")?;
    let capabilities = capabilities_mask(&request.capabilities)?;
    let system_prompt_hash = parse_hash(&request.system_prompt_hash, "system_prompt_hash")?;
    let passport = state
        .registries
        .mutate(|local| {
            let id = local
                .passports
                .mint(
                    LOCAL_ADMIN,
                    owner,
                    capabilities,
                    system_prompt_hash,
                    request.initial_stake,
                )
                .map_err(registry_error)?;
            local
                .passports
                .get_passport(id)
                .cloned()
                .ok_or_else(|| ApiError::internal("minted passport missing"))
        })
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "source": "local_durable", "passport": passport })),
    ))
}

async fn get_passport(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u128>,
) -> Result<Json<Value>, ApiError> {
    state.registries.ensure_local()?;
    let state = state.registries.local.lock().await;
    let passport = state
        .passports
        .get_passport(id)
        .cloned()
        .ok_or_else(|| ApiError::not_found(format!("passport {id} not found")))?;
    Ok(Json(
        json!({ "source": "local_durable", "passport": passport }),
    ))
}

async fn passport_history(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u128>,
) -> Result<Json<Value>, ApiError> {
    state.registries.ensure_local()?;
    let state = state.registries.local.lock().await;
    if state.passports.get_passport(id).is_none() {
        return Err(ApiError::not_found(format!("passport {id} not found")));
    }
    let events = state
        .passports
        .events()
        .iter()
        .filter(|event| agent_event_passport(event) == id)
        .cloned()
        .collect::<Vec<_>>();
    Ok(Json(json!({
        "passport_id": id,
        "transfers": state.passports.transfer_history(id),
        "events": events,
    })))
}

async fn transfer_passport(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u128>,
    ApiJson(request): ApiJson<TransferPassportRequest>,
) -> Result<Json<Value>, ApiError> {
    let from = nonblank(request.from, "from")?;
    let to = nonblank(request.to, "to")?;
    let passport = state
        .registries
        .mutate(|local| {
            local
                .passports
                .transfer_at_block(id, &from, &to, request.block)
                .map_err(registry_error)?;
            local
                .passports
                .get_passport(id)
                .cloned()
                .ok_or_else(|| ApiError::internal("transferred passport missing"))
        })
        .await?;
    Ok(Json(
        json!({ "source": "local_durable", "passport": passport }),
    ))
}

async fn update_passport_metadata(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u128>,
    ApiJson(request): ApiJson<UpdateMetadataRequest>,
) -> Result<Json<Value>, ApiError> {
    let owner = nonblank(request.owner, "owner")?;
    validate_uris(&request.service_endpoints, "service_endpoints")?;
    validate_uris(&request.feeds, "feeds")?;
    let passport = state
        .registries
        .mutate(|local| {
            local
                .passports
                .update_service_endpoints(id, &owner, request.service_endpoints)
                .map_err(registry_error)?;
            local
                .passports
                .update_feeds(id, &owner, request.feeds)
                .map_err(registry_error)?;
            local
                .passports
                .get_passport(id)
                .cloned()
                .ok_or_else(|| ApiError::internal("updated passport missing"))
        })
        .await?;
    Ok(Json(
        json!({ "source": "local_durable", "passport": passport }),
    ))
}

async fn add_delegation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<u128>,
    ApiJson(request): ApiJson<AddDelegationRequest>,
) -> Result<Json<Value>, ApiError> {
    let owner = nonblank(request.owner, "owner")?;
    let allowed_capabilities = capabilities_mask(&request.capabilities)?;
    let caveat = DelegationCaveat {
        delegatee: request.delegatee,
        allowed_capabilities,
        expiry_block: request.expiry_block,
        max_spend: request.max_spend,
        scope: request.scope,
    };
    let passport = state
        .registries
        .mutate(|local| {
            if local.passports.get_passport(caveat.delegatee).is_none() {
                return Err(ApiError::bad_request(
                    "delegatee must reference a local passport",
                ));
            }
            local.passports.set_block(request.current_block);
            local
                .passports
                .add_caveat(id, &owner, caveat)
                .map_err(registry_error)?;
            local
                .passports
                .get_passport(id)
                .cloned()
                .ok_or_else(|| ApiError::internal("delegating passport missing"))
        })
        .await?;
    Ok(Json(
        json!({ "source": "local_durable", "passport": passport }),
    ))
}

async fn revoke_delegation(
    State(state): State<Arc<AppState>>,
    Path((id, delegatee)): Path<(u128, u128)>,
    Query(query): Query<OwnerQuery>,
) -> Result<StatusCode, ApiError> {
    let owner = nonblank(query.owner, "owner")?;
    state
        .registries
        .mutate(|local| {
            local
                .passports
                .revoke_caveat(id, &owner, delegatee)
                .map_err(registry_error)
        })
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Default, Deserialize)]
struct KnowledgeQuery {
    tag: Option<String>,
    state: Option<KnowledgeEntryState>,
    #[serde(default)]
    offset: usize,
    #[serde(default)]
    limit: usize,
}

#[derive(Debug, Deserialize)]
struct PublishKnowledgeRequest {
    publisher_id: u128,
    content_hash: String,
    hdc_fingerprint: Option<Vec<u64>>,
    #[serde(default)]
    tags: Vec<String>,
    published_at: u64,
}

#[derive(Debug, Deserialize)]
struct ValidateKnowledgeRequest {
    validator_id: u128,
}

#[derive(Debug, Deserialize)]
struct ChallengeKnowledgeRequest {
    challenger_id: u128,
    evidence_hash: String,
    reason: String,
    resolution_deadline: u64,
}

#[derive(Debug, Deserialize)]
struct ResolveChallengeRequest {
    upheld: bool,
}

async fn list_knowledge(
    State(state): State<Arc<AppState>>,
    Query(query): Query<KnowledgeQuery>,
) -> Result<Json<Value>, ApiError> {
    state.registries.ensure_local()?;
    let state = state.registries.local.lock().await;
    let mut entries = if let Some(tag) = query.tag.as_deref() {
        state
            .knowledge
            .query_by_tag(tag)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>()
    } else {
        state
            .knowledge
            .entries()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>()
    };
    entries.retain(|entry| query.state.is_none_or(|wanted| entry.state == wanted));
    let total = entries.len();
    let limit = page_size(query.limit);
    let entries = entries
        .into_iter()
        .skip(query.offset)
        .take(limit)
        .map(knowledge_json)
        .collect::<Vec<_>>();
    Ok(Json(json!({
        "source": "local_durable",
        "entries": entries,
        "offset": query.offset,
        "limit": limit,
        "total": total,
    })))
}

async fn publish_knowledge(
    State(state): State<Arc<AppState>>,
    ApiJson(request): ApiJson<PublishKnowledgeRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    if request.publisher_id == 0 || request.published_at == 0 {
        return Err(ApiError::bad_request(
            "publisher_id and published_at must be nonzero",
        ));
    }
    validate_tags(&request.tags)?;
    let content_hash = parse_hash(&request.content_hash, "content_hash")?;
    let entry = state
        .registries
        .mutate(|local| {
            if local.passports.get_passport(request.publisher_id).is_none() {
                return Err(ApiError::bad_request(
                    "publisher_id must reference a local passport",
                ));
            }
            let id = local
                .knowledge
                .publish(KnowledgeRegistryEntry::draft(
                    request.publisher_id,
                    content_hash,
                    request.hdc_fingerprint,
                    request.tags,
                    request.published_at,
                ))
                .map_err(knowledge_error)?;
            local
                .knowledge
                .get_entry(&id)
                .cloned()
                .ok_or_else(|| ApiError::internal("published knowledge entry missing"))
        })
        .await?;
    Ok((StatusCode::CREATED, Json(knowledge_json(entry))))
}

async fn get_knowledge(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    state.registries.ensure_local()?;
    let id = parse_hash(&id, "knowledge id")?;
    let state = state.registries.local.lock().await;
    let entry = state
        .knowledge
        .get_entry(&id)
        .cloned()
        .ok_or_else(|| ApiError::not_found("knowledge entry not found"))?;
    Ok(Json(knowledge_json(entry)))
}

async fn validate_knowledge(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    ApiJson(request): ApiJson<ValidateKnowledgeRequest>,
) -> Result<Json<Value>, ApiError> {
    let id = parse_hash(&id, "knowledge id")?;
    if request.validator_id == 0 {
        return Err(ApiError::bad_request("validator_id must be nonzero"));
    }
    let effect = state
        .registries
        .mutate(|local| {
            if local.passports.get_passport(request.validator_id).is_none() {
                return Err(ApiError::bad_request(
                    "validator_id must reference a local passport",
                ));
            }
            local
                .knowledge
                .validate(&id, request.validator_id)
                .map_err(knowledge_error)
        })
        .await?;
    Ok(Json(
        json!({ "entry_id": format_hash(&id), "reputation_effect": effect }),
    ))
}

async fn challenge_knowledge(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    ApiJson(request): ApiJson<ChallengeKnowledgeRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let id = parse_hash(&id, "knowledge id")?;
    let evidence_hash = parse_hash(&request.evidence_hash, "evidence_hash")?;
    let reason = nonblank(request.reason, "reason")?;
    let challenge_id = state
        .registries
        .mutate(|local| {
            if local
                .passports
                .get_passport(request.challenger_id)
                .is_none()
            {
                return Err(ApiError::bad_request(
                    "challenger_id must reference a local passport",
                ));
            }
            local
                .knowledge
                .challenge(
                    &id,
                    request.challenger_id,
                    evidence_hash,
                    reason,
                    request.resolution_deadline,
                )
                .map_err(knowledge_error)
        })
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "challenge_id": format_hash(&challenge_id) })),
    ))
}

async fn resolve_knowledge_challenge(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    ApiJson(request): ApiJson<ResolveChallengeRequest>,
) -> Result<Json<Value>, ApiError> {
    let id = parse_hash(&id, "challenge id")?;
    let effect = state
        .registries
        .mutate(|local| {
            local
                .knowledge
                .resolve_challenge(&id, request.upheld)
                .map_err(knowledge_error)
        })
        .await?;
    Ok(Json(json!({
        "challenge_id": format_hash(&id),
        "upheld": request.upheld,
        "reputation_effect": effect,
    })))
}

#[derive(Debug, Default, Deserialize)]
struct EventsQuery {
    contract: Option<String>,
    event_type: Option<String>,
    from_block: Option<u64>,
    to_block: Option<u64>,
    #[serde(default)]
    limit: usize,
}

async fn list_events(
    State(state): State<Arc<AppState>>,
    Query(query): Query<EventsQuery>,
) -> Result<Json<Value>, ApiError> {
    let (chain_events, chain_indexer_configured) = {
        let indexer = state.registries.indexer.lock().await;
        let events = indexer.as_ref().map_or_else(Vec::new, |indexer| {
            indexer.query(&RegistryEventQuery {
                contract: query.contract,
                event_type: query.event_type,
                from_block: query.from_block,
                to_block: query.to_block,
                limit: query.limit,
            })
        });
        (events, indexer.is_some())
    };
    state.registries.ensure_local()?;
    let local = state.registries.local.lock().await;
    Ok(Json(json!({
        "chain_events": chain_events,
        "local": {
            "passport_events": local.passports.events(),
            "knowledge_events": local.knowledge.events(),
        },
        "chain_indexer_configured": chain_indexer_configured,
    })))
}

async fn registry_stats(State(state): State<Arc<AppState>>) -> Json<Value> {
    let local = state.registries.local.lock().await;
    let passport_count = local.passports.passport_count();
    let knowledge_count = local.knowledge.entries().len();
    drop(local);
    let chain_tip = match state.registries.chain.as_ref() {
        Some(chain) => chain.block_number().await.ok(),
        None => None,
    };
    let indexer_status = {
        let indexer = state.registries.indexer.lock().await;
        indexer.as_ref().map(|indexer| indexer.status(chain_tip))
    };
    let indexer_error = if indexer_status.is_none() {
        state.registries.indexer_error.as_deref()
    } else {
        None
    };
    Json(json!({
        "local": {
            "available": state.registries.local_error.is_none(),
            "error": state.registries.local_error.as_deref(),
            "passport_count": passport_count,
            "knowledge_count": knowledge_count,
        },
        "chain": {
            "configured": state.registries.chain.is_some(),
            "tip": chain_tip,
            "indexer": indexer_status,
            "error": indexer_error,
        }
    }))
}

async fn sync_indexer(State(state): State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    let mut indexer = state.registries.indexer.lock().await;
    let indexer = indexer.as_mut().ok_or_else(|| {
        service_unavailable(
            "chain registry indexer is not configured",
            state.registries.indexer_error.as_ref(),
        )
    })?;
    let outcome = indexer
        .sync_once()
        .await
        .map_err(|error| ApiError::internal(error.to_string()))?;
    Ok(Json(json!({ "outcome": outcome })))
}

async fn rebuild_indexer(State(state): State<Arc<AppState>>) -> Result<StatusCode, ApiError> {
    let mut indexer = state.registries.indexer.lock().await;
    if let Some(indexer) = indexer.as_mut() {
        indexer
            .rebuild()
            .map_err(|error| ApiError::internal(error.to_string()))?;
    } else {
        let chain = state.registries.chain.clone().ok_or_else(|| {
            service_unavailable(
                "chain registry indexer is not configured",
                state.registries.indexer_error.as_ref(),
            )
        })?;
        let config = state.registries.indexer_config.clone().ok_or_else(|| {
            service_unavailable(
                "chain registry indexer is not configured",
                state.registries.indexer_error.as_ref(),
            )
        })?;
        *indexer = Some(
            EventIndexer::rebuild_open(chain, config)
                .map_err(|error| ApiError::internal(error.to_string()))?,
        );
    }
    Ok(StatusCode::NO_CONTENT)
}

fn configured_contracts(config: &roko_core::config::ChainConfig) -> Vec<RegistryContract> {
    let candidates = [
        ("identity", config.identity_registry.as_ref()),
        ("reputation", config.reputation_registry.as_ref()),
        ("validation", config.validation_registry.as_ref()),
        ("knowledge", config.knowledge_registry.as_ref()),
        ("agent", config.agent_registry.as_ref()),
    ];
    candidates
        .into_iter()
        .filter_map(|(name, address)| {
            let address = address?.trim().to_ascii_lowercase();
            Some(RegistryContract {
                name: name.to_owned(),
                address,
                topics: Vec::new(),
            })
        })
        .collect()
}

fn load_local_state(path: &FsPath) -> Result<LocalRegistryState, String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(LocalRegistryState::default());
        }
        Err(error) => return Err(error.to_string()),
    };
    let snapshot: LocalRegistrySnapshot =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if snapshot.schema_version != LOCAL_SCHEMA_VERSION {
        return Err(format!(
            "unsupported local registry schema {}",
            snapshot.schema_version
        ));
    }
    if !local_snapshot_links_are_valid(&snapshot) {
        return Err("local registry snapshot contains orphan passport references".to_owned());
    }
    let passports =
        AgentRegistry::from_snapshot(snapshot.passports).map_err(|error| format!("{error:?}"))?;
    let knowledge =
        KnowledgeRegistry::from_snapshot(snapshot.knowledge).map_err(|error| error.to_string())?;
    Ok(LocalRegistryState {
        passports,
        knowledge,
    })
}

fn local_snapshot_links_are_valid(snapshot: &LocalRegistrySnapshot) -> bool {
    let passports = snapshot
        .passports
        .passports
        .iter()
        .map(|passport| passport.passport_id)
        .collect::<HashSet<_>>();
    snapshot
        .passports
        .passports
        .iter()
        .flat_map(|passport| passport.delegation_caveats.iter())
        .all(|caveat| passports.contains(&caveat.delegatee))
        && snapshot
            .knowledge
            .entries
            .iter()
            .all(|entry| passports.contains(&entry.publisher_id))
        && snapshot
            .knowledge
            .challenges
            .iter()
            .all(|challenge| passports.contains(&challenge.challenger_id))
        && snapshot
            .knowledge
            .validators
            .iter()
            .all(|(_, validators)| validators.iter().all(|id| passports.contains(id)))
        && snapshot.knowledge.events.iter().all(|event| match event {
            roko_chain::knowledge_registry::KnowledgeRegistryEvent::Published {
                publisher_id,
                ..
            } => passports.contains(publisher_id),
            roko_chain::knowledge_registry::KnowledgeRegistryEvent::Validated {
                validator_id,
                ..
            } => passports.contains(validator_id),
            roko_chain::knowledge_registry::KnowledgeRegistryEvent::Challenged {
                challenger_id,
                ..
            } => passports.contains(challenger_id),
            roko_chain::knowledge_registry::KnowledgeRegistryEvent::ChallengeResolved {
                ..
            }
            | roko_chain::knowledge_registry::KnowledgeRegistryEvent::StateChanged { .. } => true,
        })
}

fn persist_local_state(path: &FsPath, state: &LocalRegistryState) -> std::io::Result<()> {
    let snapshot = LocalRegistrySnapshot {
        schema_version: LOCAL_SCHEMA_VERSION,
        passports: state.passports.snapshot(),
        knowledge: state.knowledge.snapshot(),
    };
    let bytes = serde_json::to_vec_pretty(&snapshot)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    durable_atomic_write(path, &bytes)
}

fn durable_atomic_write(path: &FsPath, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "state path has no parent")
    })?;
    std::fs::create_dir_all(parent)?;
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(
        ".registry-state.tmp.{}.{}",
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

fn registry_error(error: RegistryError) -> ApiError {
    match error {
        RegistryError::PassportNotFound(id) => {
            ApiError::not_found(format!("passport {id} not found"))
        }
        RegistryError::NotOwner | RegistryError::AdminOnly => {
            ApiError::forbidden(format!("registry authorization failed: {error:?}"))
        }
        RegistryError::InvalidTransferTarget | RegistryError::InvalidDelegation => {
            ApiError::bad_request(format!("invalid passport operation: {error:?}"))
        }
        RegistryError::CaveatNotFound | RegistryError::NoPendingUpdate => {
            ApiError::not_found(format!("registry lifecycle item not found: {error:?}"))
        }
        RegistryError::InvalidSnapshot => ApiError::internal("invalid registry snapshot"),
        other => ApiError::conflict(format!("passport lifecycle conflict: {other:?}")),
    }
}

fn knowledge_error(error: roko_chain::knowledge_registry::KnowledgeRegistryError) -> ApiError {
    use roko_chain::knowledge_registry::KnowledgeRegistryError;
    match error {
        KnowledgeRegistryError::EntryNotFound | KnowledgeRegistryError::ChallengeNotFound => {
            ApiError::not_found(error.to_string())
        }
        KnowledgeRegistryError::SelfAttestation
        | KnowledgeRegistryError::InvalidDeadline
        | KnowledgeRegistryError::NotPublisher => ApiError::bad_request(error.to_string()),
        KnowledgeRegistryError::InvalidSnapshot => ApiError::internal(error.to_string()),
        _ => ApiError::conflict(error.to_string()),
    }
}

fn service_unavailable(message: &str, detail: Option<&String>) -> ApiError {
    ApiError {
        status: StatusCode::SERVICE_UNAVAILABLE,
        code: "service_unavailable".to_owned(),
        message: message.to_owned(),
        details: detail.map(|detail| Box::new(json!({ "reason": detail }))),
    }
}

fn nonblank(value: String, field: &str) -> Result<String, ApiError> {
    let value = value.trim();
    if value.is_empty() || value.len() > 512 {
        Err(ApiError::bad_request(format!(
            "{field} must contain 1..=512 characters"
        )))
    } else {
        Ok(value.to_owned())
    }
}

fn validate_uris(values: &[String], field: &str) -> Result<(), ApiError> {
    if values.len() > 64
        || values
            .iter()
            .any(|value| value.trim().is_empty() || value.len() > 2_048)
    {
        return Err(ApiError::bad_request(format!(
            "{field} must contain at most 64 non-empty values of at most 2048 characters"
        )));
    }
    Ok(())
}

fn validate_tags(tags: &[String]) -> Result<(), ApiError> {
    if tags.len() > 32
        || tags
            .iter()
            .any(|tag| tag.trim().is_empty() || tag.len() > 64)
    {
        return Err(ApiError::bad_request(
            "tags must contain at most 32 non-empty values of at most 64 characters",
        ));
    }
    Ok(())
}

fn page_size(limit: usize) -> usize {
    if limit == 0 {
        DEFAULT_PAGE_SIZE
    } else {
        limit.min(MAX_PAGE_SIZE)
    }
}

fn capabilities_mask(names: &[String]) -> Result<u64, ApiError> {
    if names.is_empty() {
        return Err(ApiError::bad_request("at least one capability is required"));
    }
    names
        .iter()
        .try_fold(0_u64, |mask, name| Ok(mask | capability_bit(name)?))
}

fn capability_bit(name: &str) -> Result<u64, ApiError> {
    match name.trim().to_ascii_lowercase().as_str() {
        "inference" => Ok(CAP_INFERENCE),
        "data_transform" => Ok(CAP_DATA_TRANSFORM),
        "fine_tune" => Ok(CAP_FINE_TUNE),
        "rag" => Ok(CAP_RAG),
        "multi_agent" => Ok(CAP_MULTI_AGENT),
        "trading" => Ok(CAP_TRADING),
        "security" => Ok(CAP_SECURITY),
        "analytics" => Ok(CAP_ANALYTICS),
        "knowledge" => Ok(CAP_KNOWLEDGE),
        "strategy" => Ok(CAP_STRATEGY),
        other => Err(ApiError::bad_request(format!(
            "unknown capability '{other}'"
        ))),
    }
}

fn parse_hash(value: &str, field: &str) -> Result<[u8; 32], ApiError> {
    let value = value.strip_prefix("0x").unwrap_or(value);
    if value.len() != 64 {
        return Err(ApiError::bad_request(format!(
            "{field} must be a 32-byte hexadecimal value"
        )));
    }
    let mut output = [0_u8; 32];
    for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        output[index] = (hex_digit(chunk[0]).ok_or_else(|| {
            ApiError::bad_request(format!("{field} contains non-hexadecimal data"))
        })? << 4)
            | hex_digit(chunk[1]).ok_or_else(|| {
                ApiError::bad_request(format!("{field} contains non-hexadecimal data"))
            })?;
    }
    Ok(output)
}

const fn hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn format_hash(hash: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    hash.iter().fold(String::from("0x"), |mut output, byte| {
        let _ = write!(output, "{byte:02x}");
        output
    })
}

fn knowledge_json(entry: KnowledgeRegistryEntry) -> Value {
    json!({
        "entry_id": format_hash(&entry.entry_id),
        "publisher_id": entry.publisher_id,
        "content_hash": format_hash(&entry.content_hash),
        "hdc_fingerprint": entry.hdc_fingerprint,
        "tags": entry.tags,
        "state": entry.state,
        "validation_count": entry.validation_count,
        "challenge_count": entry.challenge_count,
        "published_at": entry.published_at,
        "last_refreshed": entry.last_refreshed,
    })
}

const fn agent_event_passport(event: &AgentRegistryEvent) -> u128 {
    match event {
        AgentRegistryEvent::Minted { passport_id, .. }
        | AgentRegistryEvent::Transferred { passport_id, .. }
        | AgentRegistryEvent::CaveatUpdated { passport_id, .. }
        | AgentRegistryEvent::CaveatRevoked { passport_id, .. }
        | AgentRegistryEvent::FeedsUpdated { passport_id, .. }
        | AgentRegistryEvent::EndpointsUpdated { passport_id, .. } => *passport_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime(path: &FsPath) -> RegistryRuntime {
        RegistryRuntime::open(
            path,
            &roko_core::config::schema::RokoConfig::default(),
            None,
        )
    }

    #[tokio::test]
    async fn passport_and_knowledge_lifecycle_survives_restart() {
        let directory = tempfile::tempdir().expect("tempdir");
        let registry_runtime = runtime(directory.path());
        let passport_id = registry_runtime
            .mutate(|local| {
                local
                    .passports
                    .mint(LOCAL_ADMIN, "owner".to_owned(), CAP_KNOWLEDGE, [1; 32], 0)
                    .map_err(registry_error)
            })
            .await
            .expect("mint");
        let entry_id = registry_runtime
            .mutate(|local| {
                local
                    .knowledge
                    .publish(KnowledgeRegistryEntry::draft(
                        passport_id,
                        [2; 32],
                        None,
                        vec!["defi".to_owned()],
                        10,
                    ))
                    .map_err(knowledge_error)
            })
            .await
            .expect("publish");
        drop(registry_runtime);

        let restored = runtime(directory.path());
        assert!(restored.local_error.is_none());
        let state = restored.local.lock().await;
        assert_eq!(
            state.passports.get_passport(passport_id).unwrap().owner,
            "owner"
        );
        assert_eq!(state.knowledge.get_entry(&entry_id).unwrap().tags, ["defi"]);
    }

    #[test]
    fn corrupt_state_is_degraded_instead_of_silently_reset() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory
            .path()
            .join(".roko")
            .join("chain")
            .join("registry-state.json");
        std::fs::create_dir_all(path.parent().unwrap()).expect("mkdir");
        std::fs::write(&path, "not-json").expect("fixture");
        let runtime = runtime(directory.path());
        assert!(runtime.local_error.is_some());
        assert!(runtime.ensure_local().is_err());
    }

    #[tokio::test]
    async fn well_formed_orphan_knowledge_state_is_degraded() {
        let directory = tempfile::tempdir().expect("tempdir");
        let registry_runtime = runtime(directory.path());
        let passport_id = registry_runtime
            .mutate(|local| {
                local
                    .passports
                    .mint(LOCAL_ADMIN, "owner".to_owned(), CAP_KNOWLEDGE, [1; 32], 0)
                    .map_err(registry_error)
            })
            .await
            .expect("mint");
        registry_runtime
            .mutate(|local| {
                local
                    .knowledge
                    .publish(KnowledgeRegistryEntry::draft(
                        passport_id,
                        [2; 32],
                        None,
                        Vec::new(),
                        10,
                    ))
                    .map_err(knowledge_error)
            })
            .await
            .expect("publish");
        let path = registry_runtime.state_path.clone();
        drop(registry_runtime);

        let mut snapshot: LocalRegistrySnapshot =
            serde_json::from_slice(&std::fs::read(&path).expect("snapshot bytes"))
                .expect("snapshot JSON");
        snapshot.passports.passports.clear();
        snapshot.passports.events.clear();
        std::fs::write(&path, serde_json::to_vec_pretty(&snapshot).unwrap()).expect("tamper");

        let restored = runtime(directory.path());
        assert!(restored.local_error.is_some());
        assert!(restored.ensure_local().is_err());
    }

    #[test]
    fn hash_and_capability_validation_reject_ambiguous_inputs() {
        assert!(parse_hash("00", "hash").is_err());
        assert_eq!(parse_hash(&"ab".repeat(32), "hash").unwrap(), [0xab; 32]);
        assert!(capabilities_mask(&[]).is_err());
        assert!(capabilities_mask(&["unknown".to_owned()]).is_err());
        assert_eq!(
            capabilities_mask(&["knowledge".to_owned(), "trading".to_owned()]).unwrap(),
            CAP_KNOWLEDGE | CAP_TRADING
        );
    }

    #[tokio::test]
    async fn failed_mutation_restores_in_memory_registry_state() {
        let directory = tempfile::tempdir().expect("tempdir");
        let registry_runtime = runtime(directory.path());
        let passport_id = registry_runtime
            .mutate(|local| {
                local
                    .passports
                    .mint(LOCAL_ADMIN, "owner".to_owned(), CAP_KNOWLEDGE, [1; 32], 0)
                    .map_err(registry_error)
            })
            .await
            .expect("mint");
        registry_runtime
            .mutate(|local| {
                local
                    .passports
                    .mint(
                        LOCAL_ADMIN,
                        "delegatee".to_owned(),
                        CAP_KNOWLEDGE,
                        [2; 32],
                        0,
                    )
                    .map_err(registry_error)
            })
            .await
            .expect("mint delegatee");

        let failed = registry_runtime
            .mutate(|local| {
                local.passports.set_block(50);
                Err::<(), _>(ApiError::bad_request("stop after partial mutation"))
            })
            .await;
        assert!(failed.is_err());

        registry_runtime
            .mutate(|local| {
                local
                    .passports
                    .add_caveat(
                        passport_id,
                        &"owner".to_owned(),
                        DelegationCaveat {
                            delegatee: 2,
                            allowed_capabilities: CAP_KNOWLEDGE,
                            expiry_block: 25,
                            max_spend: None,
                            scope: None,
                        },
                    )
                    .map_err(registry_error)
            })
            .await
            .expect("failed operation must not leak block 50");
    }

    #[test]
    fn duplicate_configured_addresses_are_left_for_fail_closed_validation() {
        let mut config = roko_core::config::ChainConfig::default();
        let address = "0x1111111111111111111111111111111111111111".to_owned();
        config.identity_registry = Some(address.clone());
        config.agent_registry = Some(address);
        let contracts = configured_contracts(&config);
        assert_eq!(contracts.len(), 2);
        assert_eq!(contracts[0].address, contracts[1].address);
    }
}
