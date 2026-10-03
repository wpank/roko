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

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Path as UrlPath, Query, State};
use roko_core::config::learning::LearningAuditConfig;
use roko_fs::RokoLayout;
use roko_learn::loop_audit::census::read_runs;
use roko_learn::loop_audit::ledger::{
    HealthRow, Ledger, LoopAuditRecord, LoopAuditRow, latest_health,
};
use roko_learn::loop_audit::{Lifecycle, LoopAuditor, LoopSpec, ReasonCode, Registry};
use roko_learn::telemetry::ContentDecisionPoint;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::ApiError;
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
}
