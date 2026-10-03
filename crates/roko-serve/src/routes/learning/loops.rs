//! The loop audit's routes (S03 §5; backlog 5132), which S10's Loop Health
//! view renders.
//!
//! - `GET /api/learn/loops`, and its showcase alias `/api/showcase/m2/loops`:
//!   one row per registered loop, from the registry and the loop-audit
//!   ledger.
//! - `GET /api/learn/loops/{id}`: the loop's row, its latest health row, and
//!   its transitions and canaries.
//! - `GET /api/learn/loops/{id}/decisions?limit=`: the runs' decision rows
//!   that name the loop, newest first, at most [`MAX_DECISIONS`].
//! - `GET /api/showcase/m2/loops/{id}/ledger`: the loop's ledger rows.
//!
//! These routes only read. Plan runs append the ledger at each run's close
//! (5126), and their `LoopHealth` and `LoopTransition` events reach
//! `/api/events` through StateHub like every other dashboard event.
//!
//! The admin routes (5133) need a caller with the admin or owner scope, and
//! refuse anyone else with a 403:
//! - `POST /api/learn/loops/{id}/canary`: a dry canary trace through the
//!   runner `roko serve` injects ([`AppState::loop_canary`]); 503 without one.
//! - `POST /api/learn/loops/{id}/fault {kind, ttl_s, max_decisions}` and the
//!   showcase's `POST /api/showcase/m2/loops/{id}/break {action}` (`sever_read`
//!   = CUT, `freeze_state` = STALE, `randomize` = DEGENERATE; decision 5101
//!   §9.9), in fault-injection builds only: they set a flag in serve's
//!   process, with its ground truth in `.roko/learn/serve-faults.jsonl`. A
//!   dry-run kind reaches only dry runs, such as the canary route's; HARMFUL
//!   reaches live runs under its spend cap (decision 5101 §9.10). A flag
//!   lives at most 1800 s (422).

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use axum::extract::{Path as UrlPath, Query, State};
use axum::http::StatusCode;
use axum::{Extension, Json, Router};
use roko_core::config::learning::LearningAuditConfig;
use roko_fs::RokoLayout;
use roko_learn::loop_audit::census::read_runs;
#[cfg(feature = "fault-injection")]
use roko_learn::loop_audit::faults::{self, FaultActor, FaultError, FaultKind, FaultSpec};
use roko_learn::loop_audit::ledger::{
    CanaryRow, HealthRow, Ledger, LoopAuditRecord, LoopAuditRow, latest_health,
};
use roko_learn::loop_audit::{Lifecycle, LoopAuditor, LoopSpec, ReasonCode, Registry};
use roko_learn::telemetry::ContentDecisionPoint;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::ApiError;
use crate::routes::middleware::AuthContext;
use crate::state::AppState;

/// The most decision rows `/learn/loops/{id}/decisions` returns.
pub(crate) const MAX_DECISIONS: usize = 500;

/// The decision rows `/learn/loops/{id}/decisions` returns without a limit.
const DEFAULT_DECISIONS: usize = 50;

/// The loop a route row names when it carries no loop id (as the census
/// reads it).
const ROUTE_LOOP: &str = "L-route";

/// One registered loop, as the Loop Health view lists it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct LoopRow {
    /// Registry id, e.g. `L-know`.
    pub loop_id: String,
    /// Where it stands in the registry.
    pub lifecycle: Lifecycle,
    /// Its audit state; `None` for a retired loop.
    pub state: Option<&'static str>,
    /// The state's reason, e.g. `dormant:unlogged`.
    pub reason: Option<&'static str>,
    /// Qualifiers; the ledger keeps none, so this lists none yet.
    pub qualifiers: Vec<String>,
    /// The holdout rate it runs at in its state.
    pub h: f64,
    /// Opportunities, at its latest health row.
    pub n_opp: Option<u64>,
    /// ε̂, at its latest health row.
    pub eps: Option<f64>,
    /// ι_net, at its latest health row.
    pub iota_net: Option<f64>,
    /// β̂, once judged.
    pub beta: Option<f64>,
}

/// A workspace's loop auditor and its ledger rows, in order.
struct Audit {
    auditor: LoopAuditor,
    records: Vec<LoopAuditRecord>,
}

impl Audit {
    /// The audit of `state`'s workspace, read off the async runtime.
    async fn load(state: &AppState) -> Result<Self, ApiError> {
        let workdir = state.workdir.clone();
        let config = state.roko_config.load().learning.audit.clone();
        tokio::task::spawn_blocking(move || Self::read(&workdir, &config))
            .await
            .map_err(|error| ApiError::internal(format!("loop audit read failed: {error}")))?
    }

    /// The registry of `workdir` and its ledger, read once.
    fn read(workdir: &Path, config: &LearningAuditConfig) -> Result<Self, ApiError> {
        let registry = Registry::load(workdir)
            .map_err(|error| ApiError::internal(format!("loop registry: {error}")))?;
        let learn_dir = RokoLayout::for_project(workdir).learn_dir();
        let records = Ledger::in_learn_dir(&learn_dir)
            .read()
            .map_err(|error| ApiError::internal(format!("loop-audit ledger: {error}")))?;
        let auditor = LoopAuditor::from_records(registry, config, &records);
        Ok(Self { auditor, records })
    }

    /// One row per registered loop, in registry order.
    fn rows(&self) -> Vec<LoopRow> {
        let latest = latest_health(&self.records);
        self.auditor
            .registry()
            .loops()
            .iter()
            .map(|spec| self.row(spec, &latest))
            .collect()
    }

    /// `spec`'s row, its numbers from its `latest` health row.
    fn row(&self, spec: &LoopSpec, latest: &BTreeMap<&str, &HealthRow>) -> LoopRow {
        let id = spec.id.as_str();
        let retired = matches!(spec.lifecycle, Lifecycle::Retired { .. });
        let health = latest.get(id).copied();
        LoopRow {
            loop_id: id.to_string(),
            lifecycle: spec.lifecycle.clone(),
            state: (!retired).then_some(self.auditor.state(id).as_str()),
            reason: self.auditor.reason(id).map(ReasonCode::as_str),
            qualifiers: Vec::new(),
            h: self.auditor.layer_spec(id, "", 0).map_or(0.0, |layer| layer.h),
            n_opp: health.map(|health| health.n_opp),
            eps: health.map(|health| health.eps.est),
            iota_net: health.map(|health| health.iota.net),
            beta: health.and_then(|health| health.beta.est),
        }
    }

    /// `loop_id`'s ledger rows, in order.
    fn ledger_of(&self, loop_id: &str) -> Vec<&LoopAuditRecord> {
        self.records
            .iter()
            .filter(|record| record.loop_id == loop_id)
            .collect()
    }

    /// `loop_id`'s spec, or a 404.
    fn spec(&self, loop_id: &str) -> Result<&LoopSpec, ApiError> {
        self.auditor
            .registry()
            .get(loop_id)
            .ok_or_else(|| ApiError::not_found(format!("no registered loop {loop_id}")))
    }
}

/// `GET /api/learn/loops`: one row per registered loop.
pub(super) async fn loops(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<LoopRow>>, ApiError> {
    Ok(Json(Audit::load(&state).await?.rows()))
}

/// `GET /api/learn/loops/{id}`: the loop's row, its latest health row, and
/// its transitions and canaries.
pub(super) async fn loop_detail(
    State(state): State<Arc<AppState>>,
    UrlPath(id): UrlPath<String>,
) -> Result<Json<Value>, ApiError> {
    let audit = Audit::load(&state).await?;
    let spec = audit.spec(&id)?;
    let row = audit.row(spec, &latest_health(&audit.records));
    let (mut health, mut transitions, mut canaries) = (None, Vec::new(), Vec::new());
    for record in audit.ledger_of(&id) {
        match &record.row {
            LoopAuditRow::Health(_) => health = Some(record),
            LoopAuditRow::Transition(_) => transitions.push(record),
            LoopAuditRow::Canary(_) => canaries.push(record),
            _ => {}
        }
    }
    Ok(Json(json!({
        "loop": row,
        "health": health,
        "transitions": transitions,
        "canaries": canaries,
    })))
}

/// `GET /api/showcase/m2/loops/{id}/ledger`: the loop's ledger rows.
pub(super) async fn loop_ledger(
    State(state): State<Arc<AppState>>,
    UrlPath(id): UrlPath<String>,
) -> Result<Json<Vec<LoopAuditRecord>>, ApiError> {
    let audit = Audit::load(&state).await?;
    audit.spec(&id)?;
    Ok(Json(audit.ledger_of(&id).into_iter().cloned().collect()))
}

/// The `limit` of `/learn/loops/{id}/decisions`.
#[derive(Debug, Default, Deserialize)]
pub(super) struct DecisionsQuery {
    /// At most this many rows, capped at [`MAX_DECISIONS`].
    limit: Option<usize>,
}

/// `GET /api/learn/loops/{id}/decisions?limit=`: the runs' decision rows that
/// name the loop, newest first.
pub(super) async fn loop_decisions(
    State(state): State<Arc<AppState>>,
    UrlPath(id): UrlPath<String>,
    Query(query): Query<DecisionsQuery>,
) -> Result<Json<Vec<Value>>, ApiError> {
    let limit = query.limit.unwrap_or(DEFAULT_DECISIONS).min(MAX_DECISIONS);
    let workdir = state.workdir.clone();
    let rows = tokio::task::spawn_blocking(move || decisions_of(&workdir, &id, limit))
        .await
        .map_err(|error| ApiError::internal(format!("decision read failed: {error}")))?;
    Ok(Json(rows))
}

/// The decision rows of every run of `workdir` that name `loop_id`, newest
/// first by their `ts`, at most `limit`. A route row without a loop id is
/// L-route's; knowledge and playbook rows without one are L-know's and
/// L-play's.
fn decisions_of(workdir: &Path, loop_id: &str, limit: usize) -> Vec<Value> {
    let runs = read_runs(&RokoLayout::for_project(workdir).runs_dir());
    let mut rows: Vec<(String, Value)> = Vec::new();
    for run in &runs {
        for line in &run.decisions {
            let named = line.record.audit.loop_id.as_deref().unwrap_or(ROUTE_LOOP);
            if named == loop_id {
                rows.push((line.ts.clone(), json!(line)));
            }
        }
        for line in &run.content_decisions {
            let record = &line.record;
            let named = record.audit.loop_id.as_deref();
            if named.or_else(|| content_loop(record.decision_point)) == Some(loop_id) {
                rows.push((line.ts.clone(), json!(line)));
            }
        }
    }
    rows.sort_by(|a, b| b.0.cmp(&a.0));
    rows.into_iter().take(limit).map(|(_, row)| row).collect()
}

/// The admin routes' check: a caller with the admin or owner scope. Anyone
/// else, and any caller while serve auth is off, is refused with a 403.
fn require_admin(auth: Option<&Extension<AuthContext>>) -> Result<(), ApiError> {
    let admin = auth.is_some_and(|auth| matches!(auth.scope.as_str(), "admin" | "owner"));
    if admin {
        Ok(())
    } else {
        Err(ApiError::forbidden("the loop audit's canary and fault routes are admin-only"))
    }
}

/// `POST /api/learn/loops/{id}/canary` (admin): trace the loop's canary,
/// dry, through the runner `roko serve` injected; its `loop.canary` row.
pub(super) async fn loop_canary(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    UrlPath(id): UrlPath<String>,
) -> Result<Json<CanaryRow>, ApiError> {
    require_admin(auth.as_ref())?;
    let Some(runner) = state.loop_canary.get().cloned() else {
        return Err(ApiError {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "canary_unavailable".to_string(),
            message: "this server has no canary runner; `roko serve` sets one".to_string(),
            details: None,
        });
    };
    let row = tokio::task::spawn_blocking(move || runner.run(&id))
        .await
        .map_err(|error| ApiError::internal(format!("canary trace failed: {error}")))?
        .map_err(ApiError::unprocessable_entity)?;
    Ok(Json(row))
}

/// The fault routes, in fault-injection builds: `POST
/// /api/learn/loops/{id}/fault` and the showcase's `…/break`.
#[cfg(feature = "fault-injection")]
pub(super) fn fault_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/learn/loops/{id}/fault", axum::routing::post(loop_fault))
        .route("/showcase/m2/loops/{id}/break", axum::routing::post(loop_break))
}

/// No fault routes: this build has no fault flags.
#[cfg(not(feature = "fault-injection"))]
pub(super) fn fault_routes() -> Router<Arc<AppState>> {
    Router::new()
}

/// The ground truth of the flags serve's admin routes set, under the learn
/// directory.
#[cfg(feature = "fault-injection")]
const SERVE_FAULTS_FILE: &str = "serve-faults.jsonl";

/// What `POST /api/learn/loops/{id}/fault` asks for.
#[cfg(feature = "fault-injection")]
#[derive(Debug, Deserialize)]
pub(super) struct FaultRequest {
    /// How to break the loop, e.g. `cut`.
    kind: FaultKind,
    /// The flag's life, 1 to 1800 s.
    ttl_s: u64,
    /// The decisions it may affect, at least 1.
    max_decisions: u64,
    /// HARMFUL's spend cap, at most $1.50.
    #[serde(default)]
    spend_cap_usd: Option<f64>,
}

/// `POST /api/learn/loops/{id}/fault` (admin, fault-injection builds): set
/// a fault flag on the loop.
#[cfg(feature = "fault-injection")]
pub(super) async fn loop_fault(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    UrlPath(id): UrlPath<String>,
    Json(request): Json<FaultRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(auth.as_ref())?;
    set_fault(&state, &id, request).await
}

/// The showcase's "break a loop" actions (decision 5101 §9.9).
#[cfg(feature = "fault-injection")]
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum BreakAction {
    /// Cut the loop's reader: CUT.
    SeverRead,
    /// Pin the reader to an old state version: STALE.
    FreezeState,
    /// Rank the same whatever the task: DEGENERATE.
    Randomize,
}

/// What `POST /api/showcase/m2/loops/{id}/break` asks for.
#[cfg(feature = "fault-injection")]
#[derive(Debug, Deserialize)]
pub(super) struct BreakRequest {
    /// How to break the loop.
    action: BreakAction,
    /// The flag's life; ten minutes unless stated.
    #[serde(default = "default_break_ttl")]
    ttl_s: u64,
    /// The decisions it may affect; a hundred unless stated.
    #[serde(default = "default_break_decisions")]
    max_decisions: u64,
}

/// A showcase break's life, in seconds.
#[cfg(feature = "fault-injection")]
const fn default_break_ttl() -> u64 {
    600
}

/// A showcase break's decision budget.
#[cfg(feature = "fault-injection")]
const fn default_break_decisions() -> u64 {
    100
}

/// `POST /api/showcase/m2/loops/{id}/break` (admin, fault-injection builds):
/// the showcase's names for the fault route's dry-run kinds.
#[cfg(feature = "fault-injection")]
pub(super) async fn loop_break(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    UrlPath(id): UrlPath<String>,
    Json(request): Json<BreakRequest>,
) -> Result<Json<Value>, ApiError> {
    require_admin(auth.as_ref())?;
    let kind = match request.action {
        BreakAction::SeverRead => FaultKind::Cut,
        BreakAction::FreezeState => FaultKind::Stale,
        BreakAction::Randomize => FaultKind::Degenerate,
    };
    let fault = FaultRequest {
        kind,
        ttl_s: request.ttl_s,
        max_decisions: request.max_decisions,
        spend_cap_usd: None,
    };
    set_fault(&state, &id, fault).await
}

/// Set `request`'s flag on the registered loop `loop_id` in this process,
/// with its ground truth in [`SERVE_FAULTS_FILE`].
#[cfg(feature = "fault-injection")]
async fn set_fault(
    state: &AppState,
    loop_id: &str,
    request: FaultRequest,
) -> Result<Json<Value>, ApiError> {
    let ttl_s = request.ttl_s;
    if ttl_s == 0 || ttl_s > faults::MAX_TTL_SECS {
        let most = faults::MAX_TTL_SECS;
        let message = format!("a fault flag lives 1 to {most} s, not {ttl_s} s");
        return Err(ApiError::unprocessable_entity(message));
    }
    Audit::load(state).await?.spec(loop_id)?;
    let learn_dir = RokoLayout::for_project(&state.workdir).learn_dir();
    faults::enable(FaultActor::Admin, learn_dir.join(SERVE_FAULTS_FILE));
    let spec = FaultSpec {
        loop_id: loop_id.to_string(),
        kind: request.kind,
        ttl_secs: ttl_s,
        max_decisions: request.max_decisions,
        spend_cap_usd: request.spend_cap_usd,
    };
    let fault_id = faults::set(spec).map_err(fault_error)?;
    Ok(Json(json!({
        "fault_id": fault_id,
        "loop_id": loop_id,
        "kind": request.kind,
        "ttl_s": ttl_s,
        "max_decisions": request.max_decisions,
    })))
}

/// The status a flag that could not be set answers with.
#[cfg(feature = "fault-injection")]
fn fault_error(error: FaultError) -> ApiError {
    match error {
        FaultError::Busy(_) => ApiError::conflict(error.to_string()),
        FaultError::Disabled | FaultError::GroundTruth(_) => ApiError::internal(error.to_string()),
        FaultError::Ttl(_) | FaultError::NoDecisions | FaultError::SpendCap(_) => {
            ApiError::unprocessable_entity(error.to_string())
        }
    }
}

/// The loop a content decision point's rows belong to without a loop id.
const fn content_loop(point: ContentDecisionPoint) -> Option<&'static str> {
    match point {
        ContentDecisionPoint::Knowledge => Some("L-know"),
        ContentDecisionPoint::Playbooks => Some("L-play"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use roko_core::config::ServeAuthConfig;
    use roko_core::config::schema::RokoConfig;
    use roko_learn::loop_audit::AuditState;
    use roko_learn::loop_audit::ledger::{BetaFields, EpsilonFields, IotaFields, LOOP_AUDIT_SCHEMA};
    use tower::ServiceExt;

    use super::*;
    use crate::deploy::create_backend;
    use crate::routes::build_router;
    use crate::runtime::NoOpRuntime;

    /// Serve's state over a fresh workspace.
    fn state() -> (tempfile::TempDir, Arc<AppState>) {
        let dir = tempfile::tempdir().expect("tempdir");
        let backend = create_backend("manual", None, None, None).expect("a manual backend");
        let runtime = Arc::new(NoOpRuntime);
        let config = RokoConfig::default();
        let workdir = dir.path().to_path_buf();
        let state = AppState::new(workdir, runtime, config, Arc::from(backend));
        (dir, Arc::new(state.expect("AppState::new")))
    }

    /// GET `uri` from serve's router over `state`: its status and JSON body.
    async fn get(state: &Arc<AppState>, uri: &str) -> (StatusCode, Value) {
        let auth = ServeAuthConfig {
            enabled: false,
            ..ServeAuthConfig::default()
        };
        let app = build_router(Arc::clone(state), &[], auth);
        let request = Request::builder()
            .method("GET")
            .uri(uri)
            .body(Body::empty())
            .expect("a request");
        let response = app.oneshot(request).await.expect("a response");
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("a body");
        (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
    }

    /// L-know's health row: flagged `dormant:unlogged` after 40
    /// opportunities.
    fn flagged_know() -> LoopAuditRecord {
        let health = HealthRow {
            state: AuditState::Flagged,
            reason: Some(ReasonCode::Unlogged),
            h: 0.5,
            n_opp: 40,
            n_learned: 32,
            n_default: 8,
            eps: EpsilonFields {
                est: 0.0,
                ucb: 0.31,
                read: 1.0,
                reach: 1.0,
                honest: 1.0,
                receipt: 0.0,
            },
            iota: IotaFields {
                act: 1.0,
                aa: 0.0,
                net: 1.0,
                lcb: 0.8,
            },
            beta: BetaFields {
                est: None,
                lcb: None,
                ucb: None,
                reason: Some("eps_below_min".to_string()),
            },
            srm_evalue: 1.0,
            placebo_ok: true,
            evidence: "measured".to_string(),
        };
        LoopAuditRecord {
            schema_version: LOOP_AUDIT_SCHEMA.to_string(),
            record_id: Some("b3:health".to_string()),
            ts: Some("2026-10-03T10:00:00Z".to_string()),
            loop_id: "L-know".to_string(),
            harness_sha: None,
            config_hash: None,
            audit_epoch: Some("2026-10-03".to_string()),
            run_id: Some(Some("gr-1".to_string())),
            row: LoopAuditRow::Health(health),
        }
    }

    /// S03 §5 (backlog 5132): `/api/learn/loops` lists one row per
    /// registered loop with its state: L-know flagged from the ledger, the
    /// retired loops without a state, every other loop on probation. The
    /// showcase alias lists the same rows, a loop's detail carries its latest
    /// health, an unknown loop is a 404, and a workspace without runs has no
    /// decision rows.
    #[tokio::test(flavor = "multi_thread")]
    async fn learn_loops_lists_one_state_per_registered_loop() {
        let (dir, state) = state();
        let learn = dir.path().join(".roko/learn");
        let ledger = Ledger::in_learn_dir(&learn);
        ledger.append(&flagged_know()).expect("append a health row");

        let (status, loops) = get(&state, "/api/learn/loops").await;
        assert_eq!(status, StatusCode::OK, "{loops}");
        let registry = Registry::load(dir.path()).expect("the registry");
        let rows = loops.as_array().expect("an array of loops");
        assert_eq!(rows.len(), registry.loops().len());
        for (row, spec) in rows.iter().zip(registry.loops()) {
            let id = spec.id.as_str();
            assert_eq!(row["loop_id"], id);
            let expected = match spec.lifecycle {
                Lifecycle::Retired { .. } => Value::Null,
                _ if id == "L-know" => json!("flagged"),
                _ => json!("probation"),
            };
            assert_eq!(row["state"], expected, "{row}");
        }
        let know = rows.iter().find(|row| row["loop_id"] == "L-know");
        let know = know.expect("L-know's row");
        assert_eq!(know["reason"], "dormant:unlogged");
        assert_eq!((know["n_opp"].as_u64(), know["h"].as_f64()), (Some(40), Some(0.5)));
        let (_, showcase) = get(&state, "/api/showcase/m2/loops").await;
        assert_eq!(showcase, loops);

        let (status, detail) = get(&state, "/api/learn/loops/L-know").await;
        assert_eq!(status, StatusCode::OK, "{detail}");
        assert_eq!(detail["health"]["kind"], "loop.health");
        assert_eq!(detail["loop"]["state"], "flagged");
        let (status, ledger_rows) = get(&state, "/api/showcase/m2/loops/L-know/ledger").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(ledger_rows.as_array().map(Vec::len), Some(1));
        let (status, _) = get(&state, "/api/learn/loops/L-missing").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, decisions) = get(&state, "/api/learn/loops/L-know/decisions?limit=5").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(decisions, json!([]));
    }
    /// A caller with `scope`.
    fn caller(scope: &str) -> Option<Extension<AuthContext>> {
        Some(Extension(AuthContext {
            method: crate::routes::middleware::AuthMethod::ApiKey,
            scope: scope.to_string(),
            user_id: None,
        }))
    }

    /// The status of a handler's error, if it failed.
    fn refused<T>(result: Result<T, ApiError>) -> Option<StatusCode> {
        result.err().map(|error| error.status)
    }

    /// A canary runner that returns a passing trace of P1 alone.
    struct OneProbe;

    impl crate::state::LoopCanaryRunner for OneProbe {
        fn run(&self, _loop_id: &str) -> Result<CanaryRow, String> {
            Ok(CanaryRow {
                nonce: "c-1".to_string(),
                dry_run: true,
                first_failure: None,
                probes: Vec::new(),
                cost_usd: 0.0,
            })
        }
    }

    /// S03 §5 (backlog 5133): the canary route refuses a caller without the
    /// admin scope (403), answers 503 until `roko serve` injects a runner,
    /// and then returns the runner's trace.
    #[tokio::test(flavor = "multi_thread")]
    async fn loop_canary_route_needs_an_admin_and_a_runner() {
        let (_dir, state) = state();
        let id = || UrlPath("L-know".to_string());
        let read = loop_canary(State(Arc::clone(&state)), caller("read"), id()).await;
        assert_eq!(refused(read), Some(StatusCode::FORBIDDEN));
        let nobody = loop_canary(State(Arc::clone(&state)), None, id()).await;
        assert_eq!(refused(nobody), Some(StatusCode::FORBIDDEN));
        let bare = loop_canary(State(Arc::clone(&state)), caller("admin"), id()).await;
        assert_eq!(refused(bare), Some(StatusCode::SERVICE_UNAVAILABLE));

        let runner: Arc<dyn crate::state::LoopCanaryRunner> = Arc::new(OneProbe);
        assert!(state.loop_canary.set(runner).is_ok());
        let traced = loop_canary(State(Arc::clone(&state)), caller("owner"), id()).await;
        let row = traced.expect("a trace").0;
        assert_eq!((row.nonce.as_str(), row.first_failure), ("c-1", None));
    }

    /// S03 §5 and decision 5101 (backlog 5133): the fault route refuses a
    /// caller without the admin scope (403) and a flag that would live past
    /// 1800 s (422). An admin's flag is set, and a second one on the same
    /// loop is a conflict until the first is cleared; the showcase's break
    /// names the dry-run kinds.
    #[cfg(feature = "fault-injection")]
    #[tokio::test(flavor = "multi_thread")]
    async fn loop_fault_route_rejects_non_admin_and_long_ttl() {
        let (_dir, state) = state();
        let id = || UrlPath("L-know".to_string());
        let request = |ttl_s| {
            Json(FaultRequest {
                kind: FaultKind::Cut,
                ttl_s,
                max_decisions: 10,
                spend_cap_usd: None,
            })
        };
        let fault = |auth, ttl_s| loop_fault(State(Arc::clone(&state)), auth, id(), request(ttl_s));

        assert_eq!(refused(fault(None, 60).await), Some(StatusCode::FORBIDDEN));
        assert_eq!(refused(fault(caller("read"), 60).await), Some(StatusCode::FORBIDDEN));
        let long = fault(caller("admin"), 3_600).await;
        assert_eq!(refused(long), Some(StatusCode::UNPROCESSABLE_ENTITY));

        let set = fault(caller("admin"), 60).await.expect("an admin's flag").0;
        assert_eq!((set["loop_id"].as_str(), set["kind"].as_str()), (Some("L-know"), Some("cut")));
        assert_eq!(refused(fault(caller("admin"), 60).await), Some(StatusCode::CONFLICT));
        assert!(faults::clear("L-know"), "the flag was set");

        let freeze = Json(BreakRequest {
            action: BreakAction::FreezeState,
            ttl_s: default_break_ttl(),
            max_decisions: default_break_decisions(),
        });
        let broken = loop_break(State(Arc::clone(&state)), caller("admin"), id(), freeze).await;
        let broken = broken.expect("a showcase break").0;
        assert_eq!(broken["kind"], "stale");
        assert!(faults::clear("L-know"), "the break was set");
        faults::disable();
    }
}
