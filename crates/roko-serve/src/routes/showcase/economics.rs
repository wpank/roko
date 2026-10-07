//! The showcase's economics view and calibration stream (S04 §4.9, S10; backlog 6134).
//!
//! `GET /api/showcase/economics?experiment_id=<id>` is one more read route over the bundles the
//! loader verified: it serves `econ/<id>/econ-report.json`, M3's economics report as
//! `.roko/econ/<id>/` stored it, from the newest verified bundle that carries one, byte for
//! byte, once the file still matches `SHA256SUMS`. An experiment no bundle reports on is
//! `404 experiment_not_found`. Like every `/api/showcase/**` read, it sits behind the showcase
//! session auth.
//!
//! [`start_calibration_mirror`] watches M3's calibration export (6122) and publishes each new
//! version on the StateHub as a `self_model.calibration` event, which `/api/events` streams.

use std::path::{Component, Path as FsPath, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use roko_core::DashboardEvent;
use roko_learn::self_model::gate::CALIBRATION_FILE;
use serde::Deserialize;
use serde_json::Value;
use tokio::task::JoinHandle;

use super::bundles::{BundleStatus, sha256_hex};
use super::views::{catalog, error, rejected, serve_file};
use crate::state::AppState;

/// The name of M3's economics report in a bundle's `econ/<experiment_id>/`.
pub const ECON_REPORT: &str = "econ-report.json";

/// How often the mirror reads the calibration export.
const MIRROR_INTERVAL: Duration = Duration::from_secs(2);

/// `?experiment_id=` on the economics route.
#[derive(Debug, Default, Deserialize)]
pub struct EconomicsQuery {
    pub experiment_id: Option<String>,
}

/// `GET /api/showcase/economics?experiment_id=<id>`: the economics report of the experiment.
pub async fn economics(
    State(state): State<Arc<AppState>>,
    Query(query): Query<EconomicsQuery>,
) -> Response {
    let Some(experiment) = query.experiment_id.as_deref() else {
        return error(StatusCode::BAD_REQUEST, "experiment_id_required");
    };
    serve_economics(&state, experiment)
}

/// `econ/<experiment>/econ-report.json` of the newest verified bundle that lists it: `404`
/// when no bundle does, or when `experiment` is not one path segment; `409` when every bundle
/// that lists it was rejected, or the file no longer matches its digest.
fn serve_economics(state: &AppState, experiment: &str) -> Response {
    let mut segments = FsPath::new(experiment).components();
    let one_segment = matches!(segments.next(), Some(Component::Normal(_)))
        && segments.next().is_none()
        && !experiment.contains('\\');
    if !one_segment {
        return error(StatusCode::NOT_FOUND, "experiment_not_found");
    }
    let path = format!("econ/{experiment}/{ECON_REPORT}");
    let (_, catalog) = catalog(state);
    let newest = |verified: bool| {
        catalog
            .bundles
            .iter()
            .filter(|bundle| bundle.sums.contains_key(&path))
            .filter(|bundle| !verified || bundle.status == BundleStatus::Verified)
            .max_by(|a, b| a.created_at.cmp(&b.created_at))
    };
    if let Some(bundle) = newest(true) {
        return serve_file(bundle, &path);
    }
    match newest(false).map(|bundle| &bundle.status) {
        Some(BundleStatus::Rejected(reason)) => rejected(reason),
        _ => error(StatusCode::NOT_FOUND, "experiment_not_found"),
    }
}

/// Watch `.roko/learn/self-model/calibration.json` and publish each version the calibration
/// gate writes as a `self_model.calibration` StateHub event, until the server stops.
#[must_use]
pub fn start_calibration_mirror(state: Arc<AppState>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let path = calibration_path(&state);
        let mut seen = None;
        let mut interval = tokio::time::interval(MIRROR_INTERVAL);
        loop {
            tokio::select! {
                _ = state.cancel.cancelled() => break,
                _ = interval.tick() => {
                    mirror_calibration(&state, &path, &mut seen);
                }
            }
        }
    })
}

/// Where the workspace's calibration export lives.
fn calibration_path(state: &AppState) -> PathBuf {
    state.workdir.join(".roko").join(CALIBRATION_FILE)
}

/// Publish the export at `path` when its bytes differ from `seen`, the digest of the version
/// last published. A file that is missing or not JSON yet, as one written midway, is read
/// again on the next tick. Returns whether it published.
fn mirror_calibration(state: &AppState, path: &FsPath, seen: &mut Option<String>) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    let digest = sha256_hex(&bytes);
    if seen.as_deref() == Some(digest.as_str()) {
        return false;
    }
    let Ok(report) = serde_json::from_slice::<Value>(&bytes) else {
        return false;
    };
    *seen = Some(digest);
    state
        .state_hub
        .publish(DashboardEvent::SelfModelCalibration { report });
    true
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::header::CONTENT_TYPE;
    use axum::http::{HeaderMap, Request};
    use roko_core::config::schema::RokoConfig;
    use serde_json::json;
    use tower::ServiceExt as _;

    use super::*;
    use crate::deploy::manual::ManualBackend;
    use crate::routes::middleware::require_api_key;
    use crate::runtime::NoOpRuntime;

    const METRICS: &str = "{\"metric\":\"resolve\",\"run_ids\":[\"r1\"],\"simulated\":false}\n";
    /// A report whose layout a re-serialization would change: unsorted keys, odd spacing.
    const REPORT: &str = "{\n  \"schema\": \"econ-report/1\",\n  \"experiments\": [\"EXP-1\"],\n  \
                          \"arms\" :{}\n}\n";
    /// A PHC string the session store keys its passphrase generation on.
    const PHC: &str = "$argon2id$v=19$m=8,t=1,p=1$c2FsdHNhbHQ$Zmlyc3Q";

    /// A verified bundle `id` under `root` that carries `experiment`'s economics report.
    fn write_bundle(root: &FsPath, id: &str, experiment: &str) {
        let dir = root.join(id);
        let manifest = json!({
            "schema": "showcase-bundle/1",
            "bundle_id": id,
            "kind": "replay",
            "simulated": false,
            "title": "Economics",
            "created_at": "2026-10-04T00:00:00Z",
            "experiment_ids": [experiment],
            "views": [],
        });
        let files = [
            ("bundle.json".to_string(), manifest.to_string()),
            ("data/metrics.jsonl".to_string(), METRICS.to_string()),
            (
                format!("econ/{experiment}/{ECON_REPORT}"),
                REPORT.to_string(),
            ),
        ];
        let mut sums = String::new();
        for (path, contents) in &files {
            let file = dir.join(path);
            std::fs::create_dir_all(file.parent().expect("parent")).expect("bundle dir");
            std::fs::write(&file, contents).expect("bundle file");
            sums.push_str(&format!("{}  {path}\n", sha256_hex(contents.as_bytes())));
        }
        std::fs::write(dir.join("SHA256SUMS"), sums).expect("SHA256SUMS");
    }

    /// A showcase-mode server whose bundle root is `root`, and the showcase routes behind its
    /// auth.
    fn app(root: &FsPath) -> (tempfile::TempDir, Arc<AppState>, axum::Router) {
        let workdir = tempfile::tempdir().expect("workdir");
        let mut config = RokoConfig::default();
        config.serve.auth.enabled = true;
        config.showcase.enabled = true;
        config.showcase.public_origin = Some("https://showcase.test".to_string());
        config.showcase.bundle_root = root.display().to_string();
        let state = Arc::new(
            AppState::new(
                workdir.path().to_path_buf(),
                Arc::new(NoOpRuntime),
                config,
                Arc::new(ManualBackend::default()),
            )
            .expect("AppState::new"),
        );
        let auth = axum::middleware::from_fn_with_state(Arc::clone(&state), require_api_key);
        let router = super::super::routes()
            .layer(auth)
            .with_state(Arc::clone(&state));
        (workdir, state, router)
    }

    /// The `Cookie` header of a new showcase session.
    fn session_cookie(state: &AppState) -> String {
        let access = &state.local_access;
        access.set_passphrase_hash(Some(PHC.to_string()));
        let config = state.load_roko_config();
        let session = &config.showcase.session;
        let grant = crate::state::SessionGrant::showcase(session, access.passphrase_generation());
        let id = access.create_scoped_session(&grant, chrono::Utc::now());
        format!("{}={id}", session.cookie_name)
    }

    async fn get(
        app: &axum::Router,
        uri: &str,
        cookie: Option<&str>,
    ) -> (StatusCode, HeaderMap, Vec<u8>) {
        let mut request = Request::builder().uri(uri);
        if let Some(cookie) = cookie {
            request = request.header("Cookie", cookie);
        }
        let request = request.body(Body::empty()).expect("request");
        let response = app.clone().oneshot(request).await.expect("response");
        let (parts, body) = response.into_parts();
        let bytes = axum::body::to_bytes(body, usize::MAX).await.expect("body");
        (parts.status, parts.headers, bytes.to_vec())
    }

    /// 6134: the economics route serves the stored report's bytes as they are, 404s an
    /// experiment no verified bundle reports on, and answers 401 without a session.
    #[tokio::test]
    async fn economics_route_serves_stored_report_bytes() {
        let root = tempfile::tempdir().expect("bundle root");
        write_bundle(root.path(), "b-econ", "EXP-1");
        let (_workdir, state, app) = app(root.path());
        let cookie = session_cookie(&state);
        let uri = "/showcase/economics?experiment_id=EXP-1";

        let (status, headers, body) = get(&app, uri, Some(&cookie)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, REPORT.as_bytes());
        assert_eq!(headers[CONTENT_TYPE], "application/json");

        for unknown in ["EXP-9", "..", "EXP-1/../EXP-1"] {
            let uri = format!("/showcase/economics?experiment_id={unknown}");
            let (status, _, _) = get(&app, &uri, Some(&cookie)).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{unknown}");
        }
        let (status, _, _) = get(&app, "/showcase/economics", Some(&cookie)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let (status, _, _) = get(&app, uri, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    /// 6134: each new version of the calibration export is published once, as a
    /// `self_model.calibration` event.
    #[tokio::test]
    async fn calibration_export_is_mirrored_once_per_version() {
        let root = tempfile::tempdir().expect("bundle root");
        let (_workdir, state, _) = app(root.path());
        let path = calibration_path(&state);
        let mut events = state.state_hub.subscribe_events();
        let mut seen = None;

        assert!(
            !mirror_calibration(&state, &path, &mut seen),
            "no export yet"
        );
        std::fs::create_dir_all(path.parent().expect("parent")).expect("self-model dir");
        std::fs::write(&path, "{\"eligible\": false}").expect("export");
        assert!(mirror_calibration(&state, &path, &mut seen));
        assert!(!mirror_calibration(&state, &path, &mut seen), "unchanged");
        std::fs::write(&path, "{\"eligible\": true}").expect("export");
        assert!(mirror_calibration(&state, &path, &mut seen));

        let mut reports = Vec::new();
        while let Ok(envelope) = events.try_recv() {
            if let DashboardEvent::SelfModelCalibration { report } = envelope.payload {
                reports.push(report);
            }
        }
        let expected = [json!({ "eligible": false }), json!({ "eligible": true })];
        assert_eq!(reports, expected);
    }
}
