//! Restart-safe authenticated arena lifecycle and attempt settlement API.
//!
//! The service owns one durable [`ArenaRegistry`] snapshot. Mutations are
//! serialized, rolled back on operation or persistence failure, and projected
//! onto the existing Pulse Bus only after the atomic snapshot rename succeeds.

use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use roko_chain::ChainClient as _;
use roko_chain::arena::{
    AggregationRule, Arena, ArenaCategory, ArenaError, ArenaRegistry, ArenaState,
    AttemptSettlement, AttemptState, GroundTruthSource, Leaderboard, ReleaseCondition,
    ScoringEvidence, ScoringFunction, TaskSource,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::Mutex;

use crate::error::ApiError;
use crate::extract::ApiJson;
use crate::routes::middleware::AuthContext;
use crate::state::AppState;

const DEFAULT_PAGE_SIZE: usize = 50;
const MAX_PAGE_SIZE: usize = 250;

/// AppState-owned durable arena adapter.
pub(crate) struct ArenaRuntime {
    state_path: PathBuf,
    registry: Mutex<ArenaRegistry>,
    startup_error: Option<String>,
}

/// Immutable R03 evidence consumed by meta-agent activation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ArenaAcceptanceEvidence {
    pub(crate) arena_id: [u8; 32],
    pub(crate) attempt_id: [u8; 32],
    pub(crate) evidence_hash: [u8; 32],
    pub(crate) subject_output_hash: [u8; 32],
    pub(crate) scorer_principal: String,
    pub(crate) observed_at_block: u64,
}

struct ArenaMutation<T> {
    value: T,
}

impl ArenaRuntime {
    pub(crate) fn open(workdir: &FsPath) -> Self {
        let state_path = workdir.join(".roko").join("chain").join("arena-state.json");
        let (registry, startup_error) = match ArenaRegistry::open(&state_path) {
            Ok(registry) => (registry, None),
            Err(error) => (ArenaRegistry::new(), Some(error.to_string())),
        };
        Self {
            state_path,
            registry: Mutex::new(registry),
            startup_error,
        }
    }

    fn ensure_available(&self) -> Result<(), ApiError> {
        self.startup_error.as_ref().map_or(Ok(()), |error| {
            Err(ApiError {
                status: StatusCode::SERVICE_UNAVAILABLE,
                code: "arena_state_unavailable".to_string(),
                message: "durable arena state failed validation".to_string(),
                details: Some(Box::new(json!({ "reason": error }))),
            })
        })
    }

    async fn read<T>(
        &self,
        operation: impl FnOnce(&ArenaRegistry) -> Result<T, ArenaError>,
    ) -> Result<T, ApiError> {
        self.ensure_available()?;
        let registry = self.registry.lock().await;
        operation(&registry).map_err(arena_error)
    }

    async fn mutate<T>(
        &self,
        observed_block: u64,
        operation: impl FnOnce(&mut ArenaRegistry) -> Result<T, ArenaError>,
    ) -> Result<ArenaMutation<T>, ApiError> {
        self.ensure_available()?;
        let mut registry = self.registry.lock().await;
        let previous = registry.clone();
        registry.set_block(observed_block);
        let value = match operation(&mut registry) {
            Ok(value) => value,
            Err(error) => {
                *registry = previous;
                return Err(arena_error(error));
            }
        };
        if let Err(error) = registry.persist(&self.state_path) {
            *registry = previous;
            return Err(ApiError::internal(format!(
                "persist durable arena state: {error}"
            )));
        }
        Ok(ArenaMutation { value })
    }

    /// Publish the durable outbox with at-least-once restart semantics.
    pub(crate) async fn project_pending<B: roko_core::Bus>(
        &self,
        bus: &B,
    ) -> Result<usize, ApiError> {
        self.ensure_available()?;
        let mut registry = self.registry.lock().await;
        let previous = registry.clone();
        let previous_cursor = registry.projected_event_count();
        let pending = registry.pending_events().to_vec();
        if pending.is_empty() {
            return Ok(0);
        }
        for event in &pending {
            let pulse = event.to_pulse().map_err(arena_error)?;
            bus.publish(pulse)
                .map_err(|error| ApiError::internal(format!("publish arena event: {error}")))?;
        }
        registry
            .acknowledge_event_projection(previous_cursor + pending.len())
            .map_err(arena_error)?;
        if let Err(error) = registry.persist(&self.state_path) {
            *registry = previous;
            return Err(ApiError::internal(format!(
                "persist arena projection cursor: {error}"
            )));
        }
        Ok(pending.len())
    }

    /// Verify that a completed externally scored attempt accepts exactly the
    /// proposed artifact and belongs to the authenticated proposal owner.
    pub(crate) async fn verify_acceptance(
        &self,
        arena_id: [u8; 32],
        attempt_id: [u8; 32],
        expected_output_hash: [u8; 32],
        expected_evidence_hash: [u8; 32],
        participant_principal: &str,
    ) -> Result<ArenaAcceptanceEvidence, ApiError> {
        self.read(|registry| {
            verify_acceptance_in_registry(
                registry,
                arena_id,
                attempt_id,
                expected_output_hash,
                expected_evidence_hash,
                participant_principal,
            )
        })
        .await
    }
}

/// Apply the exact R03 acceptance predicate to one durable registry snapshot.
///
/// Route-time activation and restart-time receipt reconciliation share this
/// function so their trust decisions cannot drift.
pub(crate) fn verify_acceptance_in_registry(
    registry: &ArenaRegistry,
    arena_id: [u8; 32],
    attempt_id: [u8; 32],
    expected_output_hash: [u8; 32],
    expected_evidence_hash: [u8; 32],
    participant_principal: &str,
) -> Result<ArenaAcceptanceEvidence, ArenaError> {
    let arena = registry.get_arena(&arena_id).ok_or(ArenaError::NotFound)?;
    let attempt = registry
        .get_attempt(&attempt_id)
        .ok_or(ArenaError::NotFound)?;
    if !matches!(arena.scoring, ScoringFunction::Binary(_))
        || attempt.arena_id != arena_id
        || attempt.state != AttemptState::Completed
        || attempt.participant_principal != participant_principal
        || attempt.output_hash != Some(expected_output_hash)
        || attempt.score != Some(1.0)
        || attempt.gate_verdicts.is_empty()
        || !attempt.gate_verdicts.iter().all(|passed| *passed)
    {
        return Err(ArenaError::InvalidEvidence {
            message: "meta-agent acceptance requires a binary score of 1.0 and at least one passing external gate"
                .to_string(),
        });
    }
    let evidence =
        attempt
            .scoring_evidence
            .as_ref()
            .ok_or_else(|| ArenaError::InvalidEvidence {
                message: "completed attempt has no external scoring evidence".to_string(),
            })?;
    if evidence.evidence_hash != expected_evidence_hash
        || evidence.subject_output_hash != expected_output_hash
    {
        return Err(ArenaError::InvalidEvidence {
            message: "acceptance evidence hash does not bind the proposed artifact".to_string(),
        });
    }
    Ok(ArenaAcceptanceEvidence {
        arena_id,
        attempt_id,
        evidence_hash: evidence.evidence_hash,
        subject_output_hash: evidence.subject_output_hash,
        scorer_principal: evidence.scorer_principal.clone(),
        observed_at_block: evidence.observed_at_block,
    })
}

/// Arena lifecycle, attempt execution, settlement, and projection routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/arenas", get(list_arenas).post(create_arena))
        .route("/arenas/{id}", get(get_arena).patch(transition_arena))
        .route("/arenas/{id}/leaderboard", get(get_leaderboard))
        .route(
            "/arenas/{id}/attempts",
            get(list_attempts).post(start_attempt),
        )
        .route("/arenas/{id}/attempts/{attempt_id}", get(get_attempt))
        .route(
            "/arenas/{id}/attempts/{attempt_id}/submit",
            post(submit_attempt),
        )
        .route(
            "/arenas/{id}/attempts/{attempt_id}/settle",
            post(settle_attempt),
        )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CreateArenaRequest {
    name: String,
    #[serde(default)]
    description: String,
    category: ArenaCategory,
    task_source: TaskSource,
    scoring: ScoringFunction,
    aggregation: AggregationRule,
    #[serde(default = "default_weight")]
    weight: f64,
    #[serde(default)]
    creator_identity_id: Option<u128>,
    #[serde(default)]
    prize_pool_usdc: u128,
    #[serde(default)]
    release_condition: Option<ReleaseCondition>,
    #[serde(default)]
    max_attempts_per_agent: u32,
    #[serde(default)]
    cooldown_blocks: u64,
    #[serde(default)]
    deadline_block: u64,
    ground_truth: GroundTruthSource,
}

#[derive(Debug, Default, Deserialize)]
struct ArenaQuery {
    state: Option<ArenaState>,
    category: Option<ArenaCategory>,
    #[serde(default)]
    offset: usize,
    #[serde(default)]
    limit: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ArenaAction {
    Activate,
    Pause,
    Conclude,
}

#[derive(Debug, Deserialize)]
struct TransitionArenaRequest {
    action: ArenaAction,
}

#[derive(Debug, Deserialize)]
struct StartAttemptRequest {
    #[serde(default)]
    agent_identity_id: Option<u128>,
    #[serde(default)]
    task_hash: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct AttemptQuery {
    agent_identity_id: Option<u128>,
    state: Option<AttemptState>,
    #[serde(default)]
    offset: usize,
    #[serde(default)]
    limit: usize,
}

#[derive(Debug, Deserialize)]
struct SubmitAttemptRequest {
    output_hash: String,
}

#[derive(Debug, Deserialize)]
struct SettleAttemptRequest {
    #[serde(default)]
    scorer_identity_id: Option<u128>,
    source: GroundTruthSource,
    evidence_hash: String,
    subject_output_hash: String,
    settlement: AttemptSettlement,
}

#[derive(Debug, Default, Deserialize)]
struct LeaderboardQuery {
    #[serde(default)]
    offset: usize,
    #[serde(default)]
    limit: usize,
}

const fn default_weight() -> f64 {
    1.0
}

async fn create_arena(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    ApiJson(request): ApiJson<CreateArenaRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let auth = require_auth(auth)?;
    let principal = auth_principal(&auth);
    if request.name.trim().is_empty() {
        return Err(ApiError::bad_request("arena name must not be blank"));
    }
    if request.prize_pool_usdc != 0 && request.release_condition.is_none() {
        return Err(ApiError::bad_request(
            "prize arenas require a release_condition before activation",
        ));
    }
    if state.chain_client.is_none() && (request.cooldown_blocks != 0 || request.deadline_block != 0)
    {
        return Err(ApiError::bad_request(
            "cooldown_blocks and deadline_block require a configured trusted chain clock",
        ));
    }
    let id = arena_id(&request, &principal)?;
    let creator_identity_id = bound_identity(request.creator_identity_id, &principal)?;
    let arena = Arena {
        id,
        name: request.name,
        description: request.description,
        category: request.category,
        state: ArenaState::Draft,
        task_source: request.task_source,
        scoring: request.scoring,
        aggregation: request.aggregation,
        weight: request.weight,
        creator_identity_id,
        creator_principal: principal,
        prize_pool_usdc: request.prize_pool_usdc,
        max_attempts_per_agent: request.max_attempts_per_agent,
        cooldown_blocks: request.cooldown_blocks,
        deadline_block: request.deadline_block,
        ground_truth: request.ground_truth,
    };
    let release_condition = request.release_condition;
    let observed = trusted_observation(&state).await?;
    let mutation = state
        .arenas
        .mutate(observed, move |registry| {
            registry.create_arena(arena.clone())?;
            if let Some(condition) = release_condition {
                registry.deposit_prize(
                    &arena.id,
                    arena.creator_identity_id,
                    arena.prize_pool_usdc,
                    condition,
                )?;
            }
            Ok(arena)
        })
        .await?;
    project_committed_events(&state).await;
    let arena_id = format_hash(&mutation.value.id);
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "source": "local_durable",
            "arena_id": arena_id,
            "arena": mutation.value,
        })),
    ))
}

async fn list_arenas(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ArenaQuery>,
) -> Result<Json<Value>, ApiError> {
    let offset = query.offset;
    let limit = page_limit(query.limit);
    let arenas = state
        .arenas
        .read(move |registry| {
            Ok(registry
                .list_arenas()
                .into_iter()
                .filter(|arena| query.state.is_none_or(|value| arena.state == value))
                .filter(|arena| query.category.is_none_or(|value| arena.category == value))
                .cloned()
                .collect::<Vec<_>>())
        })
        .await?;
    let total = arenas.len();
    let arenas = arenas
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    Ok(Json(json!({
        "source": "local_durable",
        "arenas": arenas,
        "total": total,
        "offset": offset,
        "limit": limit,
    })))
}

async fn get_arena(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let id = parse_hash("arena id", &id)?;
    let arena = state
        .arenas
        .read(move |registry| registry.get_arena(&id).cloned().ok_or(ArenaError::NotFound))
        .await?;
    Ok(Json(json!({ "source": "local_durable", "arena": arena })))
}

async fn transition_arena(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    Path(id): Path<String>,
    ApiJson(request): ApiJson<TransitionArenaRequest>,
) -> Result<Json<Value>, ApiError> {
    let id = parse_hash("arena id", &id)?;
    let auth = require_auth(auth)?;
    let principal = auth_principal(&auth);
    let admin = is_admin(&auth);
    let observed = trusted_observation(&state).await?;
    let mutation = state
        .arenas
        .mutate(observed, move |registry| {
            require_owner(registry, &id, &principal, admin)?;
            match request.action {
                ArenaAction::Activate => registry.activate_arena(&id)?,
                ArenaAction::Pause => registry.pause_arena(&id)?,
                ArenaAction::Conclude => registry.conclude_arena(&id)?,
            }
            registry.get_arena(&id).cloned().ok_or(ArenaError::NotFound)
        })
        .await?;
    project_committed_events(&state).await;
    Ok(Json(
        json!({ "source": "local_durable", "arena": mutation.value }),
    ))
}

async fn start_attempt(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    Path(id): Path<String>,
    ApiJson(request): ApiJson<StartAttemptRequest>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let id = parse_hash("arena id", &id)?;
    let task_hash = request
        .task_hash
        .as_deref()
        .map(|value| parse_hash("task hash", value))
        .transpose()?;
    let auth = require_auth(auth)?;
    let principal = auth_principal(&auth);
    let agent_identity_id = bound_identity(request.agent_identity_id, &principal)?;
    let observed = trusted_observation(&state).await?;
    let mutation = state
        .arenas
        .mutate(observed, move |registry| {
            let prior = registry.get_attempts_for_agent(&id, agent_identity_id)?;
            if prior.iter().any(|attempt| {
                !attempt.participant_principal.is_empty()
                    && attempt.participant_principal != principal
            }) {
                return Err(ArenaError::Unauthorized);
            }
            registry.start_attempt_for_principal(&id, agent_identity_id, principal, task_hash)
        })
        .await?;
    project_committed_events(&state).await;
    let attempt_id = format_hash(&mutation.value.id);
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "source": "local_durable",
            "attempt_id": attempt_id,
            "attempt": mutation.value,
        })),
    ))
}

async fn submit_attempt(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    Path((id, attempt_id)): Path<(String, String)>,
    ApiJson(request): ApiJson<SubmitAttemptRequest>,
) -> Result<Json<Value>, ApiError> {
    let id = parse_hash("arena id", &id)?;
    let attempt_id = parse_hash("attempt id", &attempt_id)?;
    let output_hash = parse_hash("output hash", &request.output_hash)?;
    let auth = require_auth(auth)?;
    let principal = auth_principal(&auth);
    let admin = is_admin(&auth);
    let observed = trusted_observation(&state).await?;
    let mutation = state
        .arenas
        .mutate(observed, move |registry| {
            let attempt = registry
                .get_attempt(&attempt_id)
                .ok_or(ArenaError::NotFound)?;
            if attempt.arena_id != id {
                return Err(ArenaError::NotFound);
            }
            if !admin && attempt.participant_principal != principal {
                return Err(ArenaError::Unauthorized);
            }
            registry.submit_attempt_with_output(&attempt_id, Some(output_hash))?;
            registry
                .get_attempt(&attempt_id)
                .cloned()
                .ok_or(ArenaError::NotFound)
        })
        .await?;
    project_committed_events(&state).await;
    Ok(Json(
        json!({ "source": "local_durable", "attempt": mutation.value }),
    ))
}

async fn settle_attempt(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    Path((id, attempt_id)): Path<(String, String)>,
    ApiJson(request): ApiJson<SettleAttemptRequest>,
) -> Result<Json<Value>, ApiError> {
    let id = parse_hash("arena id", &id)?;
    let attempt_id = parse_hash("attempt id", &attempt_id)?;
    let evidence_hash = parse_hash("evidence hash", &request.evidence_hash)?;
    let subject_output_hash =
        parse_hash("evidence subject output hash", &request.subject_output_hash)?;
    let auth = require_auth(auth)?;
    let principal = auth_principal(&auth);
    let admin = is_admin(&auth);
    let scorer_identity_id = bound_identity(request.scorer_identity_id, &principal)?;
    let observed = trusted_observation(&state).await?;
    let mutation = state
        .arenas
        .mutate(observed, move |registry| {
            require_owner(registry, &id, &principal, admin)?;
            let attempt = registry
                .get_attempt(&attempt_id)
                .ok_or(ArenaError::NotFound)?;
            if attempt.arena_id != id {
                return Err(ArenaError::NotFound);
            }
            let evidence = ScoringEvidence {
                source: request.source,
                scorer_identity_id,
                scorer_principal: principal,
                evidence_hash,
                subject_output_hash,
                observed_at_block: registry.current_block(),
            };
            registry.settle_attempt(&attempt_id, evidence, request.settlement)
        })
        .await?;
    project_committed_events(&state).await;
    Ok(Json(
        json!({ "source": "local_durable", "attempt": mutation.value }),
    ))
}

async fn list_attempts(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<AttemptQuery>,
) -> Result<Json<Value>, ApiError> {
    let id = parse_hash("arena id", &id)?;
    let offset = query.offset;
    let limit = page_limit(query.limit);
    let attempts = state
        .arenas
        .read(move |registry| {
            Ok(registry
                .get_arena_attempts(&id)?
                .iter()
                .filter(|attempt| {
                    query
                        .agent_identity_id
                        .is_none_or(|value| attempt.agent_identity_id == value)
                })
                .filter(|attempt| query.state.is_none_or(|value| attempt.state == value))
                .cloned()
                .collect::<Vec<_>>())
        })
        .await?;
    let total = attempts.len();
    let attempts = attempts
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>();
    Ok(Json(json!({
        "source": "local_durable",
        "attempts": attempts,
        "total": total,
        "offset": offset,
        "limit": limit,
    })))
}

async fn get_attempt(
    State(state): State<Arc<AppState>>,
    Path((id, attempt_id)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    let id = parse_hash("arena id", &id)?;
    let attempt_id = parse_hash("attempt id", &attempt_id)?;
    let attempt = state
        .arenas
        .read(move |registry| {
            let attempt = registry
                .get_attempt(&attempt_id)
                .cloned()
                .ok_or(ArenaError::NotFound)?;
            (attempt.arena_id == id)
                .then_some(attempt)
                .ok_or(ArenaError::NotFound)
        })
        .await?;
    Ok(Json(
        json!({ "source": "local_durable", "attempt": attempt }),
    ))
}

async fn get_leaderboard(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<LeaderboardQuery>,
) -> Result<Json<Value>, ApiError> {
    let id = parse_hash("arena id", &id)?;
    let offset = query.offset;
    let limit = page_limit(query.limit);
    let mut board: Leaderboard = state
        .arenas
        .read(move |registry| registry.compute_leaderboard(&id))
        .await?;
    let total = board.entries.len();
    board.entries = board.entries.into_iter().skip(offset).take(limit).collect();
    Ok(Json(json!({
        "source": "local_durable",
        "leaderboard": board,
        "total": total,
        "offset": offset,
        "limit": limit,
    })))
}

fn require_owner(
    registry: &ArenaRegistry,
    id: &[u8; 32],
    principal: &str,
    admin: bool,
) -> Result<(), ArenaError> {
    let arena = registry.get_arena(id).ok_or(ArenaError::NotFound)?;
    if admin || arena.creator_principal == principal {
        Ok(())
    } else {
        Err(ArenaError::Unauthorized)
    }
}

fn require_auth(auth: Option<Extension<AuthContext>>) -> Result<AuthContext, ApiError> {
    auth.map(|auth| auth.0).ok_or_else(|| {
        ApiError::unauthorized(
            "arena mutations require authentication; enable serve auth and provide a credential",
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

async fn trusted_observation(state: &AppState) -> Result<u64, ApiError> {
    if let Some(chain) = &state.chain_client {
        return chain.block_number().await.map_err(|error| ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "chain_observation_unavailable".to_string(),
            message: "configured chain block could not be observed".to_string(),
            details: Some(Box::new(json!({ "reason": error.to_string() }))),
        });
    }
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| ApiError::internal(format!("read server clock: {error}")))?
        .as_secs()
        .max(1))
}

fn bound_identity(supplied: Option<u128>, principal: &str) -> Result<u128, ApiError> {
    let derived = identity_for_principal(principal);
    if supplied.is_some_and(|value| value != derived) {
        return Err(ApiError::forbidden(
            "identity id is bound to the authenticated principal and may not be overridden",
        ));
    }
    Ok(derived)
}

fn identity_for_principal(principal: &str) -> u128 {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"roko-authenticated-local-identity-v1");
    hasher.update(principal.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest.as_bytes()[..16]);
    u128::from_le_bytes(bytes).max(1)
}

async fn project_committed_events(state: &AppState) {
    if let Err(error) = state.arenas.project_pending(state.pulse_bus.as_ref()).await {
        tracing::warn!(
            %error,
            "arena durable event projection remains pending for restart replay"
        );
    }
}

fn arena_id(request: &CreateArenaRequest, principal: &str) -> Result<[u8; 32], ApiError> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"roko-arena-service-v1");
    hasher.update(principal.as_bytes());
    hasher.update(
        &serde_json::to_vec(request)
            .map_err(|error| ApiError::bad_request(format!("serialize arena: {error}")))?,
    );
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    hasher.update(&nonce.to_le_bytes());
    Ok(*hasher.finalize().as_bytes())
}

fn parse_hash(label: &str, value: &str) -> Result<[u8; 32], ApiError> {
    let value = value.strip_prefix("0x").unwrap_or(value);
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ApiError::bad_request(format!(
            "{label} must be exactly 32 hexadecimal bytes"
        )));
    }
    let mut parsed = [0_u8; 32];
    for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        let chunk = std::str::from_utf8(chunk)
            .map_err(|_| ApiError::bad_request(format!("invalid {label}")))?;
        parsed[index] = u8::from_str_radix(chunk, 16)
            .map_err(|_| ApiError::bad_request(format!("invalid {label}")))?;
    }
    if parsed == [0; 32] {
        return Err(ApiError::bad_request(format!("{label} must be non-zero")));
    }
    Ok(parsed)
}

fn format_hash(value: &[u8; 32]) -> String {
    value
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            use std::fmt::Write as _;
            let _ = write!(output, "{byte:02x}");
            output
        })
}

fn page_limit(limit: usize) -> usize {
    if limit == 0 {
        DEFAULT_PAGE_SIZE
    } else {
        limit.min(MAX_PAGE_SIZE)
    }
}

fn arena_error(error: ArenaError) -> ApiError {
    match error {
        ArenaError::NotFound => ApiError::not_found("arena or attempt was not found"),
        ArenaError::DuplicateArena | ArenaError::DuplicateEscrow => {
            ApiError::conflict(error.to_string())
        }
        ArenaError::Unauthorized | ArenaError::SelfGrading => {
            ApiError::forbidden(error.to_string())
        }
        ArenaError::InvalidState
        | ArenaError::CooldownActive
        | ArenaError::MaxAttemptsReached
        | ArenaError::ArenaNotActive
        | ArenaError::DeadlinePassed
        | ArenaError::PrizeEscrowRequired
        | ArenaError::EscrowReleased
        | ArenaError::NoEligibleWinners => ApiError::conflict(error.to_string()),
        ArenaError::InvalidDeclaration { .. }
        | ArenaError::InvalidEvidence { .. }
        | ArenaError::InvalidScore
        | ArenaError::EvaluatorIdentityRequired
        | ArenaError::EscrowNotFound => ApiError::bad_request(error.to_string()),
        ArenaError::Persistence { .. } | ArenaError::EventPublication { .. } => {
            ApiError::internal(error.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use roko_chain::arena::{BinaryCriterion, ScoringFunction};
    use roko_core::{MemoryBus, TopicFilter};

    fn definition(cooldown_blocks: u64, max_attempts_per_agent: u32) -> Arena {
        Arena {
            id: [1; 32],
            name: "durable arena".to_string(),
            description: String::new(),
            category: ArenaCategory::Coding,
            state: ArenaState::Draft,
            task_source: TaskSource::Static,
            scoring: ScoringFunction::Binary(BinaryCriterion::TestSuitePass),
            aggregation: AggregationRule::Median,
            weight: 1.0,
            creator_identity_id: 1,
            creator_principal: "owner".to_string(),
            prize_pool_usdc: 0,
            max_attempts_per_agent,
            cooldown_blocks,
            deadline_block: 0,
            ground_truth: GroundTruthSource::TestSuite,
        }
    }

    async fn seed(runtime: &ArenaRuntime, arena: Arena) {
        runtime
            .mutate(1, move |registry| {
                registry.create_arena(arena)?;
                registry.activate_arena(&[1; 32])
            })
            .await
            .expect("seed arena");
    }

    async fn settle_candidate(
        runtime: &ArenaRuntime,
        agent_identity_id: u64,
        task_hash: [u8; 32],
        output_hash: [u8; 32],
        evidence_hash: [u8; 32],
        score: f64,
        gate_verdicts: Vec<bool>,
    ) -> [u8; 32] {
        let attempt = runtime
            .mutate(2, |registry| {
                registry.start_attempt_for_principal(
                    &[1; 32],
                    agent_identity_id.into(),
                    "proposal-owner".to_string(),
                    Some(task_hash),
                )
            })
            .await
            .expect("start candidate")
            .value;
        runtime
            .mutate(3, |registry| {
                registry.submit_attempt_with_output(&attempt.id, Some(output_hash))
            })
            .await
            .expect("submit candidate");
        runtime
            .mutate(4, |registry| {
                registry.settle_attempt(
                    &attempt.id,
                    ScoringEvidence {
                        source: GroundTruthSource::TestSuite,
                        scorer_identity_id: 99,
                        scorer_principal: "independent-scorer".to_string(),
                        evidence_hash,
                        subject_output_hash: output_hash,
                        observed_at_block: 4,
                    },
                    AttemptSettlement::Completed {
                        score,
                        gate_verdicts,
                    },
                )
            })
            .await
            .expect("settle candidate");
        attempt.id
    }

    #[tokio::test]
    async fn concurrent_attempts_serialize_cooldown_validation() {
        let dir = tempdir().expect("tempdir");
        let runtime = Arc::new(ArenaRuntime::open(dir.path()));
        seed(&runtime, definition(10, 0)).await;
        let left = Arc::clone(&runtime);
        let right = Arc::clone(&runtime);
        let (left, right) = tokio::join!(
            left.mutate(10, |registry| {
                registry.start_attempt_for_principal(
                    &[1; 32],
                    7,
                    "agent".to_string(),
                    Some([2; 32]),
                )
            }),
            right.mutate(10, |registry| {
                registry.start_attempt_for_principal(
                    &[1; 32],
                    7,
                    "agent".to_string(),
                    Some([3; 32]),
                )
            }),
        );
        assert_eq!(usize::from(left.is_ok()) + usize::from(right.is_ok()), 1);
        let attempts = runtime
            .read(|registry| Ok(registry.get_arena_attempts(&[1; 32])?.len()))
            .await
            .expect("attempt count");
        assert_eq!(attempts, 1);
    }

    #[tokio::test]
    async fn terminal_settlement_and_leaderboard_survive_restart() {
        let dir = tempdir().expect("tempdir");
        let runtime = ArenaRuntime::open(dir.path());
        seed(&runtime, definition(0, 2)).await;
        let attempt = runtime
            .mutate(2, |registry| {
                registry.start_attempt_for_principal(
                    &[1; 32],
                    7,
                    "agent".to_string(),
                    Some([2; 32]),
                )
            })
            .await
            .expect("start")
            .value;
        runtime
            .mutate(3, |registry| {
                registry.submit_attempt_with_output(&attempt.id, Some([3; 32]))
            })
            .await
            .expect("submit");
        runtime
            .mutate(4, |registry| {
                registry.settle_attempt(
                    &attempt.id,
                    ScoringEvidence {
                        source: GroundTruthSource::TestSuite,
                        scorer_identity_id: 9,
                        scorer_principal: "owner".to_string(),
                        evidence_hash: [4; 32],
                        subject_output_hash: [3; 32],
                        observed_at_block: 4,
                    },
                    AttemptSettlement::Completed {
                        score: 0.8,
                        gate_verdicts: vec![true],
                    },
                )
            })
            .await
            .expect("settle");
        drop(runtime);

        let reopened = ArenaRuntime::open(dir.path());
        let board = reopened
            .read(|registry| registry.compute_leaderboard(&[1; 32]))
            .await
            .expect("leaderboard");
        assert_eq!(board.entries.len(), 1);
        assert_eq!(board.entries[0].agent_identity_id, 7);
        assert!((board.entries[0].aggregate_score - 0.8).abs() < f64::EPSILON);
        let settled = reopened
            .read(|registry| {
                registry
                    .get_attempt(&attempt.id)
                    .cloned()
                    .ok_or(ArenaError::NotFound)
            })
            .await
            .expect("settled attempt");
        assert_eq!(settled.state, AttemptState::Completed);
        assert_eq!(
            settled.scoring_evidence.expect("evidence").evidence_hash,
            [4; 32]
        );

        let bus = MemoryBus::new(32);
        let projected = reopened
            .project_pending(&bus)
            .await
            .expect("restart outbox projection");
        assert!(projected >= 4);
        assert!(
            bus.replay_from(0, Some(&TopicFilter::Prefix("arena.".to_string())))
                .iter()
                .any(|pulse| pulse.topic.0.as_str() == "arena.attempt_completed")
        );
        drop(reopened);
        let projected = ArenaRuntime::open(dir.path())
            .project_pending(&bus)
            .await
            .expect("persisted projection cursor");
        assert_eq!(projected, 0);
    }

    #[tokio::test]
    async fn meta_acceptance_requires_exact_durable_artifact_evidence_owner_score_and_gates() {
        let dir = tempdir().expect("tempdir");
        let runtime = ArenaRuntime::open(dir.path());
        seed(&runtime, definition(0, 3)).await;
        let accepted_attempt = settle_candidate(
            &runtime,
            7,
            [2; 32],
            [3; 32],
            [4; 32],
            1.0,
            vec![true, true],
        )
        .await;
        let low_score_attempt =
            settle_candidate(&runtime, 8, [5; 32], [6; 32], [7; 32], 0.99, vec![true]).await;
        let failed_gate_attempt = settle_candidate(
            &runtime,
            9,
            [8; 32],
            [9; 32],
            [10; 32],
            1.0,
            vec![true, false],
        )
        .await;
        drop(runtime);

        // Acceptance is read from the persisted R03 service, not from a
        // caller-constructed registry or evidence object.
        let reopened = ArenaRuntime::open(dir.path());
        let accepted = reopened
            .verify_acceptance(
                [1; 32],
                accepted_attempt,
                [3; 32],
                [4; 32],
                "proposal-owner",
            )
            .await
            .expect("strict durable acceptance");
        assert_eq!(accepted.subject_output_hash, [3; 32]);
        assert_eq!(accepted.evidence_hash, [4; 32]);

        for rejected in [
            reopened.verify_acceptance(
                [1; 32],
                accepted_attempt,
                [11; 32],
                [4; 32],
                "proposal-owner",
            ),
            reopened.verify_acceptance(
                [1; 32],
                accepted_attempt,
                [3; 32],
                [12; 32],
                "proposal-owner",
            ),
            reopened.verify_acceptance(
                [1; 32],
                accepted_attempt,
                [3; 32],
                [4; 32],
                "different-owner",
            ),
            reopened.verify_acceptance(
                [1; 32],
                low_score_attempt,
                [6; 32],
                [7; 32],
                "proposal-owner",
            ),
            reopened.verify_acceptance(
                [1; 32],
                failed_gate_attempt,
                [9; 32],
                [10; 32],
                "proposal-owner",
            ),
        ] {
            assert!(rejected.await.is_err());
        }
    }

    #[tokio::test]
    async fn persistence_failure_rolls_back_in_memory_mutation() {
        let dir = tempdir().expect("tempdir");
        let state_path = dir.path().join("state-target");
        std::fs::create_dir(&state_path).expect("blocking target directory");
        let runtime = ArenaRuntime {
            state_path,
            registry: Mutex::new(ArenaRegistry::new()),
            startup_error: None,
        };
        assert!(
            runtime
                .mutate(1, |registry| registry.create_arena(definition(0, 0)))
                .await
                .is_err()
        );
        assert_eq!(
            runtime
                .read(|registry| Ok(registry.arena_count()))
                .await
                .expect("read after rollback"),
            0
        );
    }
}
