//! Serve two SPAs — disk-first, embedded-fallback.
//!
//! **Routes**
//! - `/demo` and `/demo/*` → demo app (`demo/demo-app/dist/`)
//! - Everything else       → portal  (`apps/portal/out/`)
//!
//! **Dev workflow**:
//! - Portal  : run `npm run build:export` in `apps/portal`,  then refresh.
//! - Demo app: run `npm run build`        in `demo/demo-app`, then refresh.
//!
//! No Rust recompile needed for disk-served assets.
//!
//! **Production**: the binary carries baked-in copies of both UIs via
//! `rust-embed`. When the on-disk directories don't exist the embedded copies
//! are served automatically.
//!
//! Override the portal disk path with `ROKO_SPA_DIR=/path/to/dir`.
//!
//! **SPA fallback rule**: a request for a path *without* a file extension is
//! treated as a client-side route and falls back to the UI's `index.html`.
//! A request for a path *with* a file extension that is not found returns a
//! plain `404` — never an HTML page.

use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// ── Embedded asset bundles ────────────────────────────────────────────────────

/// Embedded portal assets (`apps/portal/out/`).
///
/// Falls back to `assets/frontend-fallback/` when `apps/portal/out/index.html`
/// is absent at compile time (controlled by the `roko_portal_fallback` cfg).
#[derive(rust_embed::Embed)]
#[cfg_attr(roko_portal_fallback, folder = "assets/frontend-fallback/")]
#[cfg_attr(not(roko_portal_fallback), folder = "../../apps/portal/out/")]
struct EmbeddedPortalAssets;

/// Embedded demo app assets (`demo/demo-app/dist/`).
///
/// Falls back to `assets/frontend-fallback/` when the dist directory is absent
/// at compile time (controlled by the `roko_frontend_fallback` cfg).
#[derive(rust_embed::Embed)]
#[cfg_attr(roko_frontend_fallback, folder = "assets/frontend-fallback/")]
#[cfg_attr(not(roko_frontend_fallback), folder = "../../demo/demo-app/dist/")]
struct EmbeddedDemoAssets;

// ── Disk-root resolution (resolved once at startup) ───────────────────────────

/// The portal root: `ROKO_SPA_DIR` override first, then `apps/portal/out/`
/// beside the crate.
fn disk_portal_dir() -> Option<&'static PathBuf> {
    static DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
    DIR.get_or_init(|| {
        // 1. Explicit env override
        if let Ok(dir) = std::env::var("ROKO_SPA_DIR") {
            let p = PathBuf::from(&dir);
            if p.join("index.html").is_file() {
                tracing::info!(path = %p.display(), "serving portal from ROKO_SPA_DIR");
                return Some(p);
            }
            tracing::warn!(
                path = %dir,
                "ROKO_SPA_DIR set but index.html not found, falling back to embedded portal"
            );
        }

        // 2. Relative to the compile-time crate directory (works during local dev)
        let compile_time: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../apps/portal/out");
        let p = Path::new(compile_time);
        if p.join("index.html").is_file() {
            let canon = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
            tracing::info!(path = %canon.display(), "serving portal from disk (dev mode)");
            return Some(canon);
        }

        tracing::debug!("no on-disk apps/portal/out found, serving embedded portal assets");
        None
    })
    .as_ref()
}

/// The demo app root: `demo/demo-app/dist/` beside the crate.
fn disk_demo_dir() -> Option<&'static PathBuf> {
    static DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
    DIR.get_or_init(|| {
        let compile_time: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../demo/demo-app/dist");
        let p = Path::new(compile_time);
        if p.join("index.html").is_file() {
            let canon = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
            tracing::info!(path = %canon.display(), "serving demo app from disk (dev mode)");
            return Some(canon);
        }

        tracing::debug!("no on-disk demo/demo-app/dist found, serving embedded demo assets");
        None
    })
    .as_ref()
}

// ── Path helpers ──────────────────────────────────────────────────────────────

/// Returns `true` when the URL path belongs to the demo app mount point.
///
/// `/demo` and `/demo/anything` are demo paths; `/demon` or `/demos` are not.
fn is_demo_path(path: &str) -> bool {
    path == "/demo" || path.starts_with("/demo/")
}

/// Strip the `/demo` prefix from a demo path, returning the relative asset
/// path (without a leading slash) that the demo app's file tree uses.
///
/// `/demo` → `""`, `/demo/assets/foo.js` → `"assets/foo.js"`.
fn strip_demo_prefix(path: &str) -> &str {
    if path == "/demo" {
        ""
    } else {
        // path starts with "/demo/"; skip exactly 6 bytes
        path.strip_prefix("/demo/").unwrap_or("")
    }
}

fn is_safe_relative_asset_path(path: &str) -> bool {
    path.is_empty()
        || (!path.contains("..")
            && !path.starts_with('/')
            && !path.starts_with('\\')
            && !path.contains(':')) // Windows drive letters in requests
}

/// Returns `true` when the last path segment contains a file extension.
///
/// Used to distinguish client-side routes (no extension → SPA fallback) from
/// asset requests (have extension → plain 404 when missing).
fn has_file_extension(path: &str) -> bool {
    let segment = path.rsplit('/').next().unwrap_or(path);
    // Segments starting with '.' (e.g. ".well-known") need a *second* dot to
    // count as having an extension (e.g. ".well-known/openid-configuration" vs
    // ".htaccess.bak").
    let s = segment.strip_prefix('.').unwrap_or(segment);
    s.contains('.')
}

// ── Cache-control helper ──────────────────────────────────────────────────────

/// Returns the appropriate `Cache-Control` value for the given served path.
///
/// Content-hashed assets are served as immutable:
/// - Vite: paths that start with or contain `/assets/`
///   (e.g. `assets/index-abc123.js`)
/// - Next.js: paths that start with or contain `/_next/static/`
///   (e.g. `_next/static/chunks/main-abc123.js`)
///
/// Everything else (HTML, non-hashed resources) uses `no-cache`.
fn cache_header(served_path: &str) -> &'static str {
    if served_path.starts_with("assets/")
        || served_path.contains("/assets/")
        || served_path.starts_with("_next/static/")
        || served_path.contains("/_next/static/")
    {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    }
}

// ── Read from disk ────────────────────────────────────────────────────────────

/// Try to read `rel_path` from `dir`.
///
/// Applies the SPA fallback rule:
/// - Found file → return it.
/// - Not found + has extension → `None` (404).
/// - Not found + no extension (or empty `rel_path`) → fall back to `index.html`.
fn read_from_disk_root(dir: &Path, rel_path: &str) -> Option<(Vec<u8>, String)> {
    if !rel_path.is_empty() {
        if !is_safe_relative_asset_path(rel_path) {
            return None;
        }
        let full = dir.join(rel_path);
        if full.is_file() {
            let base = dir.canonicalize().ok()?;
            let canon = full.canonicalize().ok()?;
            if !canon.starts_with(&base) {
                return None;
            }
            let rel = canon
                .strip_prefix(&base)
                .ok()?
                .to_string_lossy()
                .to_string();
            return std::fs::read(&canon).ok().map(|b| (b, rel));
        }
        // File not found: asset (has extension) → 404; route → SPA fallback
        if has_file_extension(rel_path) {
            return None;
        }
    }

    // SPA fallback: serve index.html for client-side routes and root
    let index = dir.join("index.html");
    let index = index.canonicalize().ok()?;
    let base = dir.canonicalize().ok()?;
    if !index.starts_with(&base) {
        return None;
    }
    std::fs::read(&index)
        .ok()
        .map(|b| (b, "index.html".to_string()))
}

// ── Read from embedded assets ─────────────────────────────────────────────────

fn read_from_embedded_portal(rel_path: &str) -> Option<(Vec<u8>, String)> {
    if !rel_path.is_empty() {
        if !is_safe_relative_asset_path(rel_path) {
            return None;
        }
        if let Some(file) = EmbeddedPortalAssets::get(rel_path) {
            return Some((file.data.into_owned(), rel_path.to_string()));
        }
        if has_file_extension(rel_path) {
            return None;
        }
    }
    // SPA fallback
    EmbeddedPortalAssets::get("index.html").map(|f| (f.data.into_owned(), "index.html".to_string()))
}

fn read_from_embedded_demo(rel_path: &str) -> Option<(Vec<u8>, String)> {
    if !rel_path.is_empty() {
        if !is_safe_relative_asset_path(rel_path) {
            return None;
        }
        if let Some(file) = EmbeddedDemoAssets::get(rel_path) {
            return Some((file.data.into_owned(), rel_path.to_string()));
        }
        if has_file_extension(rel_path) {
            return None;
        }
    }
    // SPA fallback
    EmbeddedDemoAssets::get("index.html").map(|f| (f.data.into_owned(), "index.html".to_string()))
}

// ── Response builder ──────────────────────────────────────────────────────────

fn build_response(result: Option<(Vec<u8>, String)>) -> Response {
    let Some((body, served_path)) = result else {
        return (StatusCode::NOT_FOUND, "not found").into_response();
    };
    let mime = mime_guess::from_path(&served_path)
        .first_or_octet_stream()
        .to_string();
    let cache = cache_header(&served_path);
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, mime),
            (header::CACHE_CONTROL, cache.to_string()),
        ],
        body,
    )
        .into_response()
}

// ── Axum handlers ─────────────────────────────────────────────────────────────

/// Serve the portal for the given request (paths outside `/demo`).
async fn serve_portal(req: axum::extract::Request) -> Response {
    let path = req.uri().path();
    let rel = path.trim_start_matches('/');
    let result = disk_portal_dir()
        .and_then(|dir| read_from_disk_root(dir, rel))
        .or_else(|| read_from_embedded_portal(rel));
    build_response(result)
}

/// Serve the demo app for the given request (`/demo` and `/demo/*`).
async fn serve_demo(req: axum::extract::Request) -> Response {
    let path = req.uri().path();
    let rel = strip_demo_prefix(path);
    let result = disk_demo_dir()
        .and_then(|dir| read_from_disk_root(dir, rel))
        .or_else(|| read_from_embedded_demo(rel));
    build_response(result)
}

/// Axum fallback handler: routes to the portal or the demo app by path prefix.
///
/// Called by `serve_api_or_spa_fallback` in `lib.rs` which has already
/// rejected `/api/*`, `/ws*`, and `/roko-ws*` paths with a JSON 404.
pub async fn serve_embedded(req: axum::extract::Request) -> Response {
    if is_demo_path(req.uri().path()) {
        serve_demo(req).await
    } else {
        serve_portal(req).await
    }
}

/// Serve only the portal index, ignoring the request path.
///
/// Used by handlers that fall back to the SPA for client-side routing — for
/// example `GET /runs/{id}` when no share transcript exists for that ID.  The
/// browser's SPA router then handles the path on the client side.
pub async fn serve_portal_index() -> Response {
    let result = disk_portal_dir()
        .and_then(|dir| read_from_disk_root(dir, ""))
        .or_else(|| read_from_embedded_portal(""));
    build_response(result)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── is_demo_path ──────────────────────────────────────────────────────────

    #[test]
    fn demo_path_exact() {
        assert!(is_demo_path("/demo"));
    }

    #[test]
    fn demo_path_with_child() {
        assert!(is_demo_path("/demo/x"));
        assert!(is_demo_path("/demo/assets/foo.js"));
    }

    #[test]
    fn demo_prefix_not_matched_for_other_names() {
        // /demon must NOT be treated as the demo mount
        assert!(!is_demo_path("/demon"));
        assert!(!is_demo_path("/demos"));
        assert!(!is_demo_path("/demo-app"));
        assert!(!is_demo_path("/"));
        assert!(!is_demo_path("/api/v1"));
    }

    // ── strip_demo_prefix ─────────────────────────────────────────────────────

    #[test]
    fn strip_demo_exact_gives_empty() {
        assert_eq!(strip_demo_prefix("/demo"), "");
    }

    #[test]
    fn strip_demo_child() {
        assert_eq!(strip_demo_prefix("/demo/x"), "x");
        assert_eq!(strip_demo_prefix("/demo/assets/foo.js"), "assets/foo.js");
    }

    // ── has_file_extension ────────────────────────────────────────────────────

    #[test]
    fn extension_detected() {
        assert!(has_file_extension("assets/foo.js"));
        assert!(has_file_extension("_next/static/chunks/main.js"));
        assert!(has_file_extension("favicon.ico"));
        assert!(has_file_extension("styles.css"));
        assert!(has_file_extension("file.min.js"));
    }

    #[test]
    fn no_extension() {
        assert!(!has_file_extension(""));
        assert!(!has_file_extension("dashboard"));
        assert!(!has_file_extension("settings/profile"));
        // A path starting with a dot but with no second dot has no extension.
        assert!(!has_file_extension(".well-known/openid-configuration"));
    }

    // ── cache_header ──────────────────────────────────────────────────────────

    #[test]
    fn cache_immutable_for_vite_assets() {
        assert_eq!(
            cache_header("assets/index-abc123.js"),
            "public, max-age=31536000, immutable"
        );
        // Leading slash variant (should also be immutable)
        assert_eq!(
            cache_header("some/prefix/assets/chunk.js"),
            "public, max-age=31536000, immutable"
        );
    }

    #[test]
    fn cache_immutable_for_next_static() {
        assert_eq!(
            cache_header("_next/static/chunks/main-abc123.js"),
            "public, max-age=31536000, immutable"
        );
        assert_eq!(
            cache_header("_next/static/css/app.css"),
            "public, max-age=31536000, immutable"
        );
    }

    #[test]
    fn cache_no_cache_for_html_and_routes() {
        assert_eq!(cache_header("index.html"), "no-cache");
        assert_eq!(cache_header("about"), "no-cache");
        assert_eq!(cache_header("_next/data/build-id/page.json"), "no-cache");
    }

    // ── SPA fallback: no extension → index.html; extension → None ────────────
    //
    // These tests exercise `read_from_embedded_portal` against the always-
    // present fallback bundle (which only contains `index.html`), so they run
    // without any real build output on disk.

    #[test]
    fn missing_js_asset_is_404() {
        let result = read_from_embedded_portal("assets/nonexistent.js");
        assert!(
            result.is_none(),
            "expected None (404) for missing .js asset"
        );
    }

    #[test]
    fn missing_css_asset_is_404() {
        let result = read_from_embedded_portal("styles/app.css");
        assert!(
            result.is_none(),
            "expected None (404) for missing .css asset"
        );
    }

    #[test]
    fn missing_route_falls_back_to_index() {
        let result = read_from_embedded_portal("settings");
        assert!(
            result.is_some(),
            "expected index.html fallback for extension-less route"
        );
        let (_, served_path) = result.unwrap();
        assert_eq!(served_path, "index.html");
    }

    #[test]
    fn nested_missing_route_falls_back_to_index() {
        let result = read_from_embedded_portal("settings/profile");
        assert!(
            result.is_some(),
            "expected index.html fallback for nested extension-less route"
        );
        let (_, served_path) = result.unwrap();
        assert_eq!(served_path, "index.html");
    }

    #[test]
    fn empty_rel_path_falls_back_to_index() {
        // Root path — both portal and demo
        let result = read_from_embedded_portal("");
        assert!(
            result.is_some(),
            "expected index.html fallback for root (empty rel_path)"
        );
        let result = read_from_embedded_demo("");
        assert!(
            result.is_some(),
            "expected index.html fallback for demo root (empty rel_path)"
        );
    }

    // ── Demo missing asset is 404 ─────────────────────────────────────────────

    #[test]
    fn demo_missing_js_asset_is_404() {
        let result = read_from_embedded_demo("assets/nonexistent.js");
        assert!(
            result.is_none(),
            "expected None (404) for missing demo .js asset"
        );
    }

    #[test]
    fn demo_missing_route_falls_back_to_index() {
        let result = read_from_embedded_demo("some-page");
        assert!(
            result.is_some(),
            "expected index.html fallback for demo extension-less route"
        );
        let (_, served_path) = result.unwrap();
        assert_eq!(served_path, "index.html");
    }
}
