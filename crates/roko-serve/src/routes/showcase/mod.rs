//! The showcase routes under `/api` (S10 §5.2, S11): read routes over bundles the loader
//! verified, and admin routes.
//!
//! The R2 views (`/showcase/m1/*`, `/showcase/m2/*`, `/showcase/m3/*`, ...) belong to the
//! packages that serve them live; given `?source=bundle:<id>` they answer through
//! [`views::serve_bundle_view`], so a recorded view needs no route here.

pub(crate) mod admin;
pub(crate) mod bundles;
pub(crate) mod views;

use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};

use crate::state::AppState;

/// The showcase routes, mounted under `/api` behind the auth layers.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/showcase/manifest", get(views::manifest))
        .route("/showcase/bundles", get(views::bundles))
        .route("/showcase/bundles/{id}", get(views::bundle))
        .route("/showcase/bundles/{id}/files/{*path}", get(views::bundle_file))
        .route("/showcase/overview", get(views::overview))
        .route("/showcase/p1/head-to-head", get(views::p1_head_to_head))
        .route("/showcase/m4/audits", get(views::m4_audits))
        .route("/showcase/admin/bundles/reload", post(admin::reload_bundles))
}
