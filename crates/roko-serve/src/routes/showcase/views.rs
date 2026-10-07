//! The showcase read routes (S10 §5.2): the manifest, the bundles and their files, and the
//! precomputed views, all from bundles the loader verified.

use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::http::header::{CONTENT_TYPE, ETAG};
use axum::response::{IntoResponse, Response};
use roko_core::config::schema::RokoConfig;
use serde::Deserialize;
use serde_json::{Value, json};

use super::bundles::{BundleStatus, Catalog, LoadedBundle, sha256_hex};
use crate::state::AppState;

/// `bundles/index.json`'s schema, which the manifest route shares (S10 §5.2).
pub const MANIFEST_SCHEMA: &str = "showcase-manifest/1";

/// `?source=` on a view route: `bundle:<id>`, or `live`.
#[derive(Debug, Default, Deserialize)]
pub struct ViewQuery {
    pub source: Option<String>,
}

/// `[showcase] bundle_root`, against the workspace when it is relative.
pub fn bundle_root(state: &AppState, config: &RokoConfig) -> PathBuf {
    let root = FsPath::new(&config.showcase.bundle_root);
    if root.is_absolute() {
        root.to_path_buf()
    } else {
        state.workdir.join(root)
    }
}

/// The configuration, and the catalogue of its bundle root.
pub(super) fn catalog(state: &AppState) -> (Arc<RokoConfig>, Arc<Catalog>) {
    let config = state.load_roko_config();
    let catalog = state.showcase_bundles.catalog(&bundle_root(state, &config));
    (config, catalog)
}

pub(super) fn error(status: StatusCode, code: &str) -> Response {
    (status, Json(json!({ "error": code }))).into_response()
}

/// `409 bundle_rejected`, with the loader's reason.
pub(super) fn rejected(reason: &str) -> Response {
    let body = json!({ "error": "bundle_rejected", "reason": reason });
    (StatusCode::CONFLICT, Json(body)).into_response()
}

/// A bundle's entry in the manifest: `id`, `title`, `status`, `created_at`, `featured`, and the
/// `reason` of a rejected one.
pub fn summary(bundle: &LoadedBundle) -> Value {
    let mut summary = json!({
        "id": bundle.id,
        "title": bundle.title,
        "status": bundle.status_word(),
        "created_at": bundle.created_at,
        "featured": bundle.featured,
    });
    if let BundleStatus::Rejected(reason) = &bundle.status {
        summary["reason"] = json!(reason);
    }
    summary
}

/// `GET /api/showcase/manifest`: the bundles this deployment can replay (S10 §5.2).
pub async fn manifest(State(state): State<Arc<AppState>>) -> Response {
    let (config, catalog) = catalog(&state);
    let featured = catalog.featured();
    let bundles: Vec<Value> = catalog.bundles.iter().map(summary).collect();
    Json(json!({
        "schema": MANIFEST_SCHEMA,
        "showcase_mode": config.showcase.enabled,
        "live_enabled": config.showcase.live_enabled,
        "featured_bundle": featured.map(|bundle| bundle.id.clone()),
        "bundles": bundles,
        "harness_commit": featured.and_then(|bundle| bundle.harness_commit.clone()),
    }))
    .into_response()
}

/// `GET /api/showcase/bundles`: every bundle, with its status.
pub async fn bundles(State(state): State<Arc<AppState>>) -> Response {
    let (_, catalog) = catalog(&state);
    let bundles: Vec<Value> = catalog.bundles.iter().map(summary).collect();
    Json(json!({ "bundles": bundles })).into_response()
}

/// `GET /api/showcase/bundles/{id}`: the bundle's `bundle.json`, once verified.
pub async fn bundle(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    let (_, catalog) = catalog(&state);
    match verified(&catalog, &id) {
        Ok(bundle) => serve_file(bundle, "bundle.json"),
        Err(response) => response,
    }
}

/// `GET /api/showcase/bundles/{id}/files/{*path}`: a file the bundle's `SHA256SUMS` lists, with
/// its SHA-256 as the `ETag`. Any other path, one outside the bundle included, is not found.
pub async fn bundle_file(
    State(state): State<Arc<AppState>>,
    Path((id, path)): Path<(String, String)>,
) -> Response {
    let (_, catalog) = catalog(&state);
    match verified(&catalog, &id) {
        Ok(bundle) => serve_file(bundle, &path),
        Err(response) => response,
    }
}

/// The verified bundle called `id`: `404` when there is none, `409` when it was rejected.
#[allow(clippy::result_large_err)]
fn verified<'a>(catalog: &'a Catalog, id: &str) -> Result<&'a LoadedBundle, Response> {
    let bundle = catalog
        .get(id)
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "bundle_not_found"))?;
    match &bundle.status {
        BundleStatus::Verified => Ok(bundle),
        BundleStatus::Rejected(reason) => Err(rejected(reason)),
    }
}

/// A listed file of `bundle`, once it still matches its digest; `409` when it no longer does.
pub(super) fn serve_file(bundle: &LoadedBundle, path: &str) -> Response {
    let Some(digest) = bundle.sums.get(path) else {
        return error(StatusCode::NOT_FOUND, "file_not_found");
    };
    let bytes = std::fs::read(bundle.dir.join(path)).ok();
    let Some(bytes) = bytes.filter(|bytes| sha256_hex(bytes) == *digest) else {
        return rejected("integrity");
    };
    let extension = FsPath::new(path).extension().and_then(|ext| ext.to_str());
    let content_type = match extension {
        Some("json") => "application/json",
        Some("jsonl") => "application/x-ndjson",
        _ => "text/plain; charset=utf-8",
    };
    let headers = [
        (CONTENT_TYPE, content_type.to_string()),
        (ETAG, format!("\"{digest}\"")),
    ];
    (headers, bytes).into_response()
}

/// `GET /api/showcase/<view>?source=bundle:<id>`: `views/<view>.json` of the bundle `source`
/// names, or of the featured one when it names none, for a view the bundle lists.
///
/// The values are the file's. Each provenance source is marked `sha256_verified` only when its
/// file and its recorded digest match `SHA256SUMS`, as the static loader marks them, so the
/// client's guard reads the same view from either source. `source=live` is `404 live_disabled`
/// until the live slices land.
pub fn serve_bundle_view(state: &AppState, view: &str, query: &ViewQuery) -> Response {
    let (_, catalog) = catalog(state);
    let bundle = match query.source.as_deref() {
        None => catalog.featured(),
        Some("live") => return error(StatusCode::NOT_FOUND, "live_disabled"),
        Some(source) => match source.strip_prefix("bundle:") {
            Some(id) => catalog.get(id),
            None => return error(StatusCode::BAD_REQUEST, "invalid_source"),
        },
    };
    let Some(bundle) = bundle else {
        return error(StatusCode::NOT_FOUND, "bundle_not_found");
    };
    if let BundleStatus::Rejected(reason) = &bundle.status {
        return rejected(reason);
    }
    let path = format!("views/{view}.json");
    let listed = bundle.views.iter().any(|listed| listed == view);
    let Some(digest) = bundle.sums.get(&path).filter(|_| listed) else {
        return error(StatusCode::NOT_FOUND, "view_not_found");
    };
    let parsed = std::fs::read(bundle.dir.join(&path))
        .ok()
        .filter(|bytes| sha256_hex(bytes) == *digest)
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
    let Some(parsed) = parsed else {
        return rejected("integrity");
    };
    let etag = format!("\"{digest}\"");
    ([(ETAG, etag)], Json(mark_sources(bundle, parsed))).into_response()
}

/// `view` with each provenance source marked `sha256_verified` from the files of `bundle`.
fn mark_sources(bundle: &LoadedBundle, mut view: Value) -> Value {
    let sources = view
        .pointer_mut("/provenance/sources")
        .and_then(Value::as_array_mut);
    if let Some(sources) = sources {
        for source in sources {
            let verified = source_verified(bundle, source);
            if let Some(source) = source.as_object_mut() {
                source.insert("sha256_verified".to_string(), Value::Bool(verified));
            }
        }
    }
    view
}

/// Whether `source`'s file and its recorded digest both match the bundle's `SHA256SUMS`.
fn source_verified(bundle: &LoadedBundle, source: &Value) -> bool {
    let Some(path) = source.get("path").and_then(Value::as_str) else {
        return false;
    };
    let Some(digest) = bundle.sums.get(path) else {
        return false;
    };
    let recorded = source.get("sha256").and_then(Value::as_str);
    recorded == Some(digest.as_str())
        && std::fs::read(bundle.dir.join(path)).is_ok_and(|bytes| sha256_hex(&bytes) == *digest)
}

/// A view route's handler: [`serve_bundle_view`] for one view.
macro_rules! view_route {
    ($name:ident, $view:literal) => {
        #[doc = concat!("`GET` of the `", $view, "` view, from a bundle.")]
        pub async fn $name(
            State(state): State<Arc<AppState>>,
            Query(query): Query<ViewQuery>,
        ) -> Response {
            serve_bundle_view(&state, $view, &query)
        }
    };
}

view_route!(overview, "overview");
view_route!(p1_head_to_head, "p1-head-to-head");
view_route!(m4_audits, "m4-audits");

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt as _;

    use super::*;
    use crate::deploy::manual::ManualBackend;
    use crate::runtime::NoOpRuntime;

    const METRICS: &str = "{\"metric\":\"resolve\",\"run_ids\":[\"r1\"],\"simulated\":false}\n";
    const RECORDS: &str = "{\"run_id\":\"r1\",\"execution\":{\"status\":\"completed\"}}\n";

    /// A replay bundle `id` under `root` whose metrics and records are `metrics` and `records`,
    /// with one view and a `SHA256SUMS` that matches it.
    fn write_bundle(root: &FsPath, id: &str, metrics: &str, records: &str) -> PathBuf {
        let dir = root.join(id);
        let manifest = json!({
            "schema": "showcase-bundle/1",
            "bundle_id": id,
            "kind": "replay",
            "simulated": false,
            "title": format!("Bundle {id}"),
            "created_at": "2026-10-04T00:00:00Z",
            "featured": true,
            "harness_commit": "0123456789abcdef",
            "views": ["overview"],
        });
        let view = json!({
            "provenance": {
                "kind": "replay",
                "simulated": false,
                "run_ids": ["r1"],
                "sources": [{
                    "path": "data/metrics.jsonl",
                    "sha256": sha256_hex(metrics.as_bytes()),
                    "simulated": false,
                }],
            },
            "tiles": [],
        });
        let files = [
            ("bundle.json", manifest.to_string()),
            ("data/metrics.jsonl", metrics.to_string()),
            ("data/records.jsonl", records.to_string()),
            ("views/overview.json", view.to_string()),
        ];
        for (path, contents) in &files {
            let file = dir.join(path);
            std::fs::create_dir_all(file.parent().expect("parent")).expect("bundle dir");
            std::fs::write(file, contents).expect("bundle file");
        }
        reseal(&dir);
        dir
    }

    /// The paths of every file under `dir`, relative to `root`.
    fn files_under(root: &FsPath, dir: &FsPath, files: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).expect("read the bundle") {
            let path = entry.expect("bundle entry").path();
            if path.is_dir() {
                files_under(root, &path, files);
            } else {
                let relative = path.strip_prefix(root).expect("a path in the bundle");
                files.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }

    /// Rewrite `SHA256SUMS` so it lists every file of the bundle as it is now.
    fn reseal(dir: &FsPath) {
        let mut files = Vec::new();
        files_under(dir, dir, &mut files);
        let sums: BTreeMap<String, String> = files
            .into_iter()
            .filter(|path| path != "SHA256SUMS")
            .map(|path| {
                let bytes = std::fs::read(dir.join(&path)).expect("bundle file");
                (path, sha256_hex(&bytes))
            })
            .collect();
        let text: String = sums
            .iter()
            .map(|(path, digest)| format!("{digest}  {path}\n"))
            .collect();
        std::fs::write(dir.join("SHA256SUMS"), text).expect("SHA256SUMS");
    }

    /// A server whose bundle root is `root`, and the showcase routes over it.
    fn app(root: &FsPath) -> (tempfile::TempDir, Arc<AppState>, axum::Router) {
        let workdir = tempfile::tempdir().expect("workdir");
        let mut config = RokoConfig::default();
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
        let router = super::super::routes().with_state(Arc::clone(&state));
        (workdir, state, router)
    }

    async fn get(app: &axum::Router, uri: &str) -> (StatusCode, Value) {
        let request = Request::builder()
            .uri(uri)
            .body(Body::empty())
            .expect("request");
        let response = app.clone().oneshot(request).await.expect("response");
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    #[tokio::test]
    async fn showcase_bundle_manifest_lists_a_good_bundle_as_verified() {
        let root = tempfile::tempdir().expect("bundle root");
        write_bundle(root.path(), "b-good", METRICS, RECORDS);
        let (_workdir, _state, app) = app(root.path());

        let (status, manifest) = get(&app, "/showcase/manifest").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(manifest["schema"], MANIFEST_SCHEMA);
        assert_eq!(manifest["featured_bundle"], "b-good");
        assert_eq!(manifest["harness_commit"], "0123456789abcdef");
        assert_eq!(manifest["bundles"][0]["status"], "verified");

        let (status, view) = get(&app, "/showcase/overview?source=bundle:b-good").await;
        assert_eq!(status, StatusCode::OK);
        let source = &view["provenance"]["sources"][0];
        assert_eq!(source["sha256_verified"], true, "{view}");
        let (status, _) = get(&app, "/showcase/overview").await;
        assert_eq!(status, StatusCode::OK);
        let (status, bundle) = get(&app, "/showcase/bundles/b-good").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(bundle["kind"], "replay");
        let (status, view) = get(&app, "/showcase/m4/audits?source=bundle:b-good").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(view["error"], "view_not_found");
        let (status, view) = get(&app, "/showcase/overview?source=live").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(view["error"], "live_disabled");
    }

    #[tokio::test]
    async fn showcase_bundle_checksum_mismatch_is_409() {
        let root = tempfile::tempdir().expect("bundle root");
        let tampered = write_bundle(root.path(), "b-tampered", METRICS, RECORDS);
        std::fs::write(
            tampered.join("data/metrics.jsonl"),
            METRICS.replace("r1", "r2"),
        )
        .expect("tamper");
        let late = write_bundle(root.path(), "b-late", METRICS, RECORDS);
        let (_workdir, state, app) = app(root.path());

        let (status, body) = get(&app, "/showcase/overview?source=bundle:b-tampered").await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["error"], "bundle_rejected");
        assert_eq!(body["reason"], "integrity");
        let (status, _) = get(&app, "/showcase/bundles/b-tampered").await;
        assert_eq!(status, StatusCode::CONFLICT);

        // A file changed after loading is caught when it is served.
        let (status, _) = get(&app, "/showcase/overview?source=bundle:b-late").await;
        assert_eq!(status, StatusCode::OK);
        std::fs::write(late.join("views/overview.json"), "{}").expect("tamper");
        let (status, body) = get(&app, "/showcase/overview?source=bundle:b-late").await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["reason"], "integrity");

        // A reload sees the change in the manifest.
        let config = state.load_roko_config();
        let catalog = state.showcase_bundles.reload(&bundle_root(&state, &config));
        let late = catalog.get("b-late").expect("b-late");
        assert_eq!(late.status, BundleStatus::Rejected("integrity".to_string()));
    }

    #[tokio::test]
    async fn showcase_bundle_simulated_or_run_id_less_is_rejected() {
        let root = tempfile::tempdir().expect("bundle root");
        let simulated = METRICS.replace("\"simulated\":false", "\"simulated\":true");
        write_bundle(root.path(), "b-simulated", &simulated, RECORDS);
        let run_id_less = METRICS.replace("[\"r1\"]", "[]");
        write_bundle(root.path(), "b-no-run-ids", &run_id_less, RECORDS);
        let infra = RECORDS.replace("completed", "infra_error");
        write_bundle(root.path(), "b-infra", METRICS, &infra);
        let fixture = write_bundle(root.path(), "b-fixture", METRICS, RECORDS);
        let manifest = std::fs::read_to_string(fixture.join("bundle.json")).expect("manifest");
        let manifest = manifest.replace("\"replay\"", "\"fixture\"");
        std::fs::write(fixture.join("bundle.json"), manifest).expect("manifest");
        reseal(&fixture);
        let (_workdir, _state, app) = app(root.path());

        let (_, manifest) = get(&app, "/showcase/manifest").await;
        let reasons: BTreeMap<String, String> = manifest["bundles"]
            .as_array()
            .expect("bundles")
            .iter()
            .map(|bundle| {
                let reason = bundle["reason"].as_str().unwrap_or("none");
                (
                    bundle["id"].as_str().expect("id").to_string(),
                    reason.to_string(),
                )
            })
            .collect();
        let expected: BTreeMap<String, String> = [
            ("b-fixture", "kind"),
            ("b-infra", "infra_error_counted"),
            ("b-no-run-ids", "no_run_ids"),
            ("b-simulated", "simulated"),
        ]
        .into_iter()
        .map(|(id, reason)| (id.to_string(), reason.to_string()))
        .collect();
        assert_eq!(reasons, expected);
        assert_eq!(manifest["featured_bundle"], Value::Null);
    }

    #[tokio::test]
    async fn showcase_bundle_path_outside_the_bundle_is_refused() {
        let root = tempfile::tempdir().expect("bundle root");
        write_bundle(root.path(), "b-good", METRICS, RECORDS);
        std::fs::write(root.path().join("secret.txt"), "not a bundle file").expect("secret");
        let (_workdir, _state, app) = app(root.path());

        let listed = "/showcase/bundles/b-good/files/data/metrics.jsonl";
        let (status, _) = get(&app, listed).await;
        assert_eq!(status, StatusCode::OK);
        for uri in [
            "/showcase/bundles/b-good/files/../secret.txt",
            "/showcase/bundles/b-good/files/%2E%2E/secret.txt",
            "/showcase/bundles/b-good/files/SHA256SUMS",
            "/showcase/bundles/..%2Fb-good/files/bundle.json",
        ] {
            let (status, _) = get(&app, uri).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
        }
    }
}
