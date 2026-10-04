//! M1's routes (S06 §5; backlog 8131), which S10's Homeostat panel renders.
//!
//! - `GET /api/learn/homeostasis`: `{mode, state, theta, theta0,
//!   lkg_versions, episode}`, from the controller's saved state, the mode a
//!   person set and θ's guarded store. `state` is the controller's phase;
//!   without saved state it, θ and the episode are `null`.
//! - `GET /api/showcase/m1/essential-variables`: the controller ledger's EV
//!   rows (`ev.sample`, `ev.breach`, `ev.restore`), oldest first.
//! - `GET /api/showcase/m1/episodes?since=`: its episode, change, evaluation
//!   and HOLD rows written at or after `since` (RFC 3339).
//! - `POST /api/learn/homeostasis/mode {mode}` and `POST
//!   /api/learn/homeostasis/ack` (admin, a person): set the mode M1's next
//!   runs open in (`shadow` or `on`; `off` stays the config's), or release a
//!   HOLD in the saved state. Each appends a `controller.mode` or
//!   `controller.hold` row with `actor = human`. A worker's token, and any
//!   caller without the admin or owner scope, an agent's included, is
//!   refused with a 403. A run already going keeps its mode, and its end
//!   saves its own phase.
//!
//! The live `ev.update` and `m1.episode` events reach `/api/events` through
//! StateHub (8130).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::{Extension, Json};
use roko_core::config::homeostasis::HomeostasisMode;
use roko_fs::RokoLayout;
use roko_learn::guarded_commit::{COMMITS_DIR, GuardMode, GuardedStore};
use roko_learn::homeostasis::controller::{
    Controller, STATE_SCHEMA, operator_mode, set_operator_mode,
};
use roko_learn::homeostasis::ledger::{
    Actor, ControllerRecord, ControllerRow, Envelope, HoldEvent, append, ledger_path,
};
use roko_learn::homeostasis::lkg::STORE;
use roko_learn::homeostasis::policy::ViabilityPolicy;
use roko_learn::telemetry::Arm;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::ApiError;
use crate::routes::middleware::{AuthContext, AuthMethod};
use crate::state::AppState;

/// The controller ledger's rows the episodes route returns.
const EPISODE_KINDS: [&str; 4] = [
    "homeostasis.episode",
    "param.change",
    "param.evaluate",
    "controller.hold",
];

/// `GET /api/learn/homeostasis`.
pub(super) async fn homeostasis(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    let roko = roko_dir(&state.workdir);
    let config_mode = state.roko_config.load().homeostasis.mode;
    let view = tokio::task::spawn_blocking(move || view(&roko, config_mode))
        .await
        .map_err(|error| ApiError::internal(format!("M1's state was not read: {error}")))?;
    Ok(Json(view))
}

/// The homeostat's view of the workspace whose `.roko` directory is `roko`.
fn view(roko: &Path, config_mode: HomeostasisMode) -> Value {
    let saved = saved_state(roko).unwrap_or(Value::Null);
    let field = |name: &str| saved.get(name).cloned().unwrap_or(Value::Null);
    let mode = operator_mode(roko).unwrap_or(config_mode);
    json!({
        "mode": mode,
        "state": field("phase"),
        "theta": field("theta"),
        "theta0": field("theta0"),
        "lkg_versions": lkg_versions(roko),
        "episode": field("episode"),
    })
}

/// `GET /api/showcase/m1/essential-variables`.
pub(super) async fn essential_variables(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Value>>, ApiError> {
    let roko = roko_dir(&state.workdir);
    let rows = tokio::task::spawn_blocking(move || {
        ledger_rows(&roko)
            .into_iter()
            .filter(|row| row["kind"].as_str().unwrap_or_default().starts_with("ev."))
            .collect()
    })
    .await
    .map_err(|error| ApiError::internal(format!("M1's ledger was not read: {error}")))?;
    Ok(Json(rows))
}

/// `?since=` of the episodes route.
#[derive(Debug, Default, Deserialize)]
pub(super) struct Since {
    /// The earliest row time returned (RFC 3339); every row without one.
    since: Option<String>,
}

/// `GET /api/showcase/m1/episodes?since=`.
pub(super) async fn episodes(
    State(state): State<Arc<AppState>>,
    Query(query): Query<Since>,
) -> Result<Json<Vec<Value>>, ApiError> {
    let roko = roko_dir(&state.workdir);
    let rows = tokio::task::spawn_blocking(move || {
        let since = query.since.unwrap_or_default();
        ledger_rows(&roko)
            .into_iter()
            .filter(|row| EPISODE_KINDS.iter().any(|kind| row["kind"] == *kind))
            .filter(|row| row["ts"].as_str().unwrap_or_default() >= since.as_str())
            .collect()
    })
    .await
    .map_err(|error| ApiError::internal(format!("M1's ledger was not read: {error}")))?;
    Ok(Json(rows))
}

/// The body of the mode route.
#[derive(Debug, Deserialize)]
pub(super) struct ModeBody {
    /// `shadow` or `on`.
    mode: HomeostasisMode,
}

/// `POST /api/learn/homeostasis/mode {mode}` (admin, a person).
pub(super) async fn set_mode(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
    Json(body): Json<ModeBody>,
) -> Result<Json<Value>, ApiError> {
    require_person(auth.as_ref())?;
    if body.mode == HomeostasisMode::Off {
        return Err(ApiError::unprocessable_entity(
            "M1 is switched off in roko.toml ([homeostasis] mode), not here",
        ));
    }
    let roko = roko_dir(&state.workdir);
    let config_mode = state.roko_config.load().homeostasis.mode;
    let to = body.mode;
    tokio::task::spawn_blocking(move || {
        let from = operator_mode(&roko).unwrap_or(config_mode);
        set_operator_mode(&roko, to)?;
        let row = ControllerRow::Mode {
            from,
            to,
            actor: Actor::Human,
        };
        record(&roko, row)
    })
    .await
    .map_err(|error| ApiError::internal(format!("M1's mode was not set: {error}")))?
    .map_err(|error| ApiError::internal(format!("M1's mode was not set: {error}")))?;
    Ok(Json(json!({ "mode": to })))
}

/// `POST /api/learn/homeostasis/ack` (admin, a person): release a HOLD.
pub(super) async fn ack(
    State(state): State<Arc<AppState>>,
    auth: Option<Extension<AuthContext>>,
) -> Result<Json<Value>, ApiError> {
    require_person(auth.as_ref())?;
    let roko = roko_dir(&state.workdir);
    let released = tokio::task::spawn_blocking(move || release_hold(&roko))
        .await
        .map_err(|error| ApiError::internal(format!("the HOLD was not released: {error}")))?
        .map_err(|error| ApiError::internal(format!("the HOLD was not released: {error}")))?;
    Ok(Json(json!({ "released": released })))
}

/// Release the HOLD the saved state is in, as `Controller::ack` does, and
/// log it; `false` when it holds nothing.
fn release_hold(roko: &Path) -> std::io::Result<bool> {
    let Some(mut state) = saved_state(roko) else {
        return Ok(false);
    };
    if state["phase"] != "hold" {
        return Ok(false);
    }
    state["phase"] = json!("idle");
    let path = Controller::state_path(roko);
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, serde_json::to_string_pretty(&state)?)?;
    std::fs::rename(&temp, &path)?;
    let row = ControllerRow::Hold {
        event: HoldEvent::Release,
        reason: "ack".to_string(),
        episode_id: state["episode"]["id"].as_str().map(str::to_string),
        actor: Actor::Human,
    };
    record(roko, row)?;
    Ok(true)
}

/// Append `row`, a person's act, to the workspace's controller ledger.
fn record(roko: &Path, row: ControllerRow) -> std::io::Result<()> {
    let policy_version = ViabilityPolicy::load_optional(roko)
        .ok()
        .flatten()
        .map_or(0, |policy| policy.policy_version);
    let resolutions = saved_state(roko).and_then(|state| state["resolutions"].as_u64());
    let envelope = Envelope {
        ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        run_id: None,
        policy_version,
        arm: Arm::Learned,
        seq: resolutions.unwrap_or(0),
    };
    append(&ledger_path(roko), &[ControllerRecord::new(&envelope, row)])
}

/// The admin routes' check: an admin or owner, never a worker's token.
fn require_person(auth: Option<&Extension<AuthContext>>) -> Result<(), ApiError> {
    let person = auth.is_some_and(|auth| {
        matches!(auth.scope.as_str(), "admin" | "owner")
            && !matches!(auth.method, AuthMethod::WorkerToken)
    });
    if person {
        Ok(())
    } else {
        Err(ApiError::forbidden(
            "M1's mode and HOLD are a person's to change: an admin's or owner's credentials",
        ))
    }
}

/// The workspace's `.roko` directory.
fn roko_dir(workdir: &Path) -> PathBuf {
    RokoLayout::for_project(workdir).root().to_path_buf()
}

/// The controller's saved state, as JSON, when it is M1's.
fn saved_state(roko: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(Controller::state_path(roko)).ok()?;
    let state: Value = serde_json::from_str(&text).ok()?;
    (state["schema_version"] == STATE_SCHEMA).then_some(state)
}

/// The versions θ's guarded store keeps, oldest first; none before its
/// first commit.
fn lkg_versions(roko: &Path) -> Vec<u64> {
    let learn = roko.join("learn");
    if !learn.join(COMMITS_DIR).join(STORE).is_dir() {
        return Vec::new();
    }
    GuardedStore::open(&learn, STORE, GuardMode::Observe)
        .and_then(|store| store.versions())
        .unwrap_or_default()
}

/// The workspace's controller ledger rows, oldest first.
fn ledger_rows(roko: &Path) -> Vec<Value> {
    std::fs::read_to_string(ledger_path(roko))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use roko_core::config::ServeAuthConfig;
    use roko_core::config::harness_params::{HarnessLadders, HarnessParams, Knob, Step};
    use roko_core::config::homeostasis::HomeostasisConfig;
    use roko_core::config::schema::RokoConfig;
    use roko_learn::homeostasis::detect::Baseline;
    use roko_learn::homeostasis::lkg::ThetaLkg;
    use tower::ServiceExt;

    use super::*;
    use crate::deploy::create_backend;
    use crate::routes::build_router;
    use crate::runtime::NoOpRuntime;

    const POLICY: &str = "policy_version = 3\n\
        ev.pass_rate = { lo = 0.70 }\nev.usd_per_verified_success = { hi = 0.12 }\n\
        ev.false_green = { hi = 0.10 }\nev.latency_p90_s = { hi = 900 }\n";

    /// Serve's state over a fresh workspace.
    fn state() -> (tempfile::TempDir, Arc<AppState>) {
        let dir = tempfile::tempdir().expect("tempdir");
        let backend = create_backend("manual", None, None, None).expect("a manual backend");
        let runtime = Arc::new(NoOpRuntime);
        let workdir = dir.path().to_path_buf();
        let state = AppState::new(workdir, runtime, RokoConfig::default(), Arc::from(backend));
        (dir, Arc::new(state.expect("AppState::new")))
    }

    /// GET `uri` from serve's router over `state`, auth off.
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

    /// The status of a handler's error, if it failed.
    fn status_of<T>(result: Result<T, ApiError>) -> Option<StatusCode> {
        result.err().map(|error| error.status)
    }

    /// A caller with `scope` who signed in with `method`.
    fn caller(scope: &str, method: AuthMethod) -> Option<Extension<AuthContext>> {
        Some(Extension(AuthContext {
            method,
            scope: scope.to_string(),
            user_id: None,
        }))
    }

    /// S06 §5 (8131): the homeostat route reports the mode, the
    /// controller's phase, θ and θ₀, the versions θ's guarded store keeps
    /// and the episode; the showcase routes read the controller ledger. A
    /// person switches the mode with a logged row.
    #[tokio::test]
    async fn homeostasis_route_reports_mode_state_theta_lkg() {
        let (dir, state) = state();
        let (status, fresh) = get(&state, "/api/learn/homeostasis").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(fresh["mode"], "shadow");
        assert!(fresh["state"].is_null(), "{fresh}");
        assert_eq!(fresh["lkg_versions"], json!([]));

        // A controller's saved state and two committed versions of θ.
        let roko = dir.path().join(".roko");
        let policy_path = ViabilityPolicy::path_in(&roko);
        std::fs::create_dir_all(policy_path.parent().expect("policy dir")).expect("mkdir");
        std::fs::write(&policy_path, POLICY).expect("write the policy");
        let config = RokoConfig::default();
        let theta0 = HarnessParams::baseline(&config);
        let ladders = HarnessLadders::from_config(&config);
        let policy = ViabilityPolicy::parse(POLICY).expect("the policy parses");
        let baseline = Baseline {
            pass_rate: 0.80,
            usd_per_resolution: 0.05,
            wall_ms: 300_000.0,
        };
        let controller = Controller::new(
            &HomeostasisConfig::default(),
            policy.clone(),
            theta0.clone(),
            ladders.clone(),
            baseline,
            0,
        );
        controller
            .save(&Controller::state_path(&roko))
            .expect("the state is saved");
        let mut lkg = ThetaLkg::open(&roko.join("learn"), theta0.clone(), ladders.clone(), policy)
            .expect("θ's guarded store");
        let raised = theta0
            .step(Knob::RetryDelta, Step::Up, &ladders)
            .expect("one more retry");
        for theta in [&theta0, &raised] {
            lkg.commit(theta, "homeostat:test", true).expect("a commit");
        }

        let (status, view) = get(&state, "/api/learn/homeostasis").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(view["mode"], "shadow");
        assert_eq!(view["state"], "idle");
        assert_eq!(
            view["theta0"],
            serde_json::to_value(&theta0).expect("θ₀ as JSON")
        );
        assert_eq!(view["theta"], view["theta0"]);
        assert_eq!(view["lkg_versions"], json!([1, 2]));
        assert!(view["episode"].is_null(), "{view}");

        // A person turns M1 on: the next runs open on, and the ledger says who.
        let admin = caller("admin", AuthMethod::Session);
        let body = Json(ModeBody {
            mode: HomeostasisMode::On,
        });
        let Json(set) = set_mode(State(Arc::clone(&state)), admin, body)
            .await
            .expect("an admin sets the mode");
        assert_eq!(set["mode"], "on", "{set}");
        let (_, view) = get(&state, "/api/learn/homeostasis").await;
        assert_eq!(view["mode"], "on");
        let rows = ledger_rows(&roko);
        let mode_row = rows.last().expect("a mode row");
        assert_eq!(mode_row["kind"], "controller.mode");
        assert_eq!(
            (mode_row["from"].as_str(), mode_row["to"].as_str()),
            (Some("shadow"), Some("on"))
        );
        assert_eq!(mode_row["actor"], "human");
        assert_eq!(mode_row["policy_version"], 3);
        let (status, episodes) = get(&state, "/api/showcase/m1/episodes?since=2000-01-01").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(episodes, json!([]));
        let (status, evs) = get(&state, "/api/showcase/m1/essential-variables").await;
        assert_eq!((status, evs), (StatusCode::OK, json!([])));
    }

    /// An agent's token, a worker's token and a read-only caller cannot
    /// change M1's mode or release its HOLD.
    #[tokio::test]
    async fn agent_token_cannot_change_homeostasis_mode() {
        let (dir, state) = state();
        for auth in [
            caller("agent:write", AuthMethod::Bearer),
            caller("admin", AuthMethod::WorkerToken),
            caller("read", AuthMethod::ApiKey),
            None,
        ] {
            let body = Json(ModeBody {
                mode: HomeostasisMode::On,
            });
            let mode = set_mode(State(Arc::clone(&state)), auth.clone(), body).await;
            assert_eq!(status_of(mode), Some(StatusCode::FORBIDDEN));
            let release = ack(State(Arc::clone(&state)), auth).await;
            assert_eq!(status_of(release), Some(StatusCode::FORBIDDEN));
        }
        let roko = dir.path().join(".roko");
        assert_eq!(operator_mode(&roko), None);
        assert!(ledger_rows(&roko).is_empty());
    }
}
