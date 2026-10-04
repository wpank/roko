//! What a showcase session may reach (S11 §4.2): a default-deny allowlist, checked before any
//! scope rule.
//!
//! A passphrase is shared and may be forwarded, so its sessions reach only the showcase: GET and
//! HEAD on `/api/showcase/**` except `/api/showcase/admin/**`, the session endpoint, and the live
//! actions of S10 §5.3 while `[showcase] live_enabled` is on. Anything else answers
//! `403 forbidden_for_session`.

use axum::http::Method;

/// The live actions a session may post while the live slices are on (S10 §5.3); a `*` segment
/// matches any one segment.
const LIVE_POSTS: [&str; 6] = [
    "/showcase/runs",
    "/showcase/runs/*/cancel",
    "/showcase/m4/audits/draw",
    "/showcase/m1/disturbances",
    "/showcase/m2/loops/*/break",
    "/showcase/p1/spec-duel",
];

/// Whether a `showcase` session may make the request `method path`.
///
/// `path` may carry the `/api` prefix or not: the API router's middleware sees it stripped.
pub fn session_may_access(method: &Method, path: &str, live_enabled: bool) -> bool {
    let path = path
        .strip_prefix("/api")
        .filter(|rest| rest.starts_with('/'))
        .unwrap_or(path);
    if path.split('/').any(|segment| segment == "..") {
        return false;
    }
    if path == "/auth/session" {
        return matches!(
            *method,
            Method::GET | Method::HEAD | Method::POST | Method::DELETE
        );
    }
    let showcase = path == "/showcase" || path.starts_with("/showcase/");
    let admin = path == "/showcase/admin" || path.starts_with("/showcase/admin/");
    if !showcase || admin {
        return false;
    }
    if matches!(*method, Method::GET | Method::HEAD) {
        return true;
    }
    live_enabled && *method == Method::POST && is_live_post(path)
}

/// Whether `path` is one of the live actions.
fn is_live_post(path: &str) -> bool {
    LIVE_POSTS.iter().any(|pattern| {
        let mut segments = path.split('/');
        pattern.split('/').all(|want| {
            segments
                .next()
                .is_some_and(|got| want == "*" || want == got)
        }) && segments.next().is_none()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn showcase_scope_allowlist_is_default_deny() {
        let allowed = [
            (Method::GET, "/api/showcase/manifest"),
            (Method::GET, "/showcase/p1/head-to-head"),
            (
                Method::HEAD,
                "/showcase/bundles/b-1/files/data/metrics.jsonl",
            ),
            (Method::GET, "/api/showcase/stream"),
            (Method::DELETE, "/api/auth/session"),
        ];
        for (method, path) in allowed {
            assert!(session_may_access(&method, path, false), "{method} {path}");
        }
        let refused = [
            (Method::GET, "/api/config"),
            (Method::GET, "/secrets"),
            (Method::POST, "/run"),
            (Method::GET, "/api/showcase/admin/bundles/reload"),
            (Method::POST, "/showcase/admin/login-unlock"),
            (Method::POST, "/showcase/runs"),
            (Method::GET, "/showcase/../config"),
            (Method::GET, "/apishowcase/manifest"),
            (Method::GET, "/ws/terminal/x"),
        ];
        for (method, path) in refused {
            assert!(!session_may_access(&method, path, false), "{method} {path}");
        }
        let live = |method: Method, path: &str| session_may_access(&method, path, true);
        assert!(live(Method::POST, "/api/showcase/runs/sr-1/cancel"));
        assert!(!live(Method::POST, "/showcase/runs/sr-1/cancel/x"));
        assert!(!live(Method::DELETE, "/showcase/runs"));
    }
}
