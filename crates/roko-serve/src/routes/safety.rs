//! Safety observability endpoints.
//!
//! * `GET /api/safety/quarantine` -- quarantine vault entries, each with its
//!   review status and full hash, and each vault's capacity: a full vault
//!   cannot index the results the boundary withholds next.
//! * `GET /api/safety/incidents` -- incident log from the immune system.
//!
//! Both read the review vaults the tool immune boundary writes when it
//! withholds a tool result (`roko_agent::quarantine_vault_path` under an
//! immune root). Dispatch roots the vault at the workspace it runs in, and
//! Graph plan runs root it at the plan's workspace rather than the attempt
//! checkout. Plan runs from before that change left it in the attempt checkout
//! in `.roko/worktrees/` under `--worktree-per-task`, so the routes also read
//! one vault per checkout still found there. Until the boundary quarantines
//! its first result no vault exists, and the responses say so instead of
//! reporting a silent zero.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::routing::get;
use roko_core::immune::QuarantineVault;
use serde::Serialize;

use crate::error::ApiError;
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/safety/quarantine", get(quarantine_handler))
        .route("/safety/incidents", get(incidents_handler))
}

// ── Vault ─────────────────────────────────────────────────────────────

/// Why a response carries no vault data: the boundary has not written it yet.
const NO_VAULT_REASON: &str = "no tool result has been quarantined in this workspace yet; \
     the tool immune boundary creates the vault when it withholds its first result";

/// One review vault the tool immune boundary wrote.
struct LoadedVault {
    /// Workspace-relative path, for display.
    path: String,
    vault: QuarantineVault,
}

/// The review vaults under `workdir`, with the display path of the
/// workspace's own vault.
///
/// The workspace vault comes first, then one per plan-run attempt checkout
/// in `.roko/worktrees/` (vaults written before Graph dispatch rooted plan
/// runs at the workspace), in path order. Missing vaults are skipped. A vault
/// that exists but cannot be read or fails validation is an error, never an
/// empty vault.
fn load_vaults(workdir: &Path) -> Result<(Vec<LoadedVault>, String), ApiError> {
    let workspace_vault = roko_agent::quarantine_vault_path(workdir);
    let workspace_display = display_path(workdir, &workspace_vault);
    let mut candidates = vec![workspace_vault];
    for checkout in attempt_checkouts(workdir) {
        candidates.push(roko_agent::quarantine_vault_path(&checkout));
    }

    let mut loaded = Vec::new();
    for path in candidates {
        let display = display_path(workdir, &path);
        match QuarantineVault::load(&path) {
            Ok(vault) => loaded.push(LoadedVault {
                path: display,
                vault,
            }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(ApiError::internal(format!(
                    "read quarantine vault {display}: {error}"
                )));
            }
        }
    }
    Ok((loaded, workspace_display))
}

/// The attempt checkouts plan runs create under `.roko/worktrees/`, sorted.
fn attempt_checkouts(workdir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(workdir.join(".roko").join("worktrees")) else {
        return Vec::new();
    };
    let mut checkouts: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    checkouts.sort();
    checkouts
}

/// `path` relative to `workdir` when it lies below it, for display.
fn display_path(workdir: &Path, path: &Path) -> String {
    path.strip_prefix(workdir)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Where each vault read came from, how many entries it holds, and how many
/// it can hold.
#[derive(Serialize)]
struct VaultSummary {
    path: String,
    entries: usize,
    capacity: usize,
    /// Whether the vault is at capacity: the tool immune boundary still
    /// withholds suspect results, but can no longer index them for review.
    full: bool,
}

fn vault_summaries(vaults: &[LoadedVault]) -> Vec<VaultSummary> {
    vaults
        .iter()
        .map(|loaded| VaultSummary {
            path: loaded.path.clone(),
            entries: loaded.vault.stats().total,
            capacity: loaded.vault.capacity(),
            full: loaded.vault.is_full(),
        })
        .collect()
}

// ── Quarantine ────────────────────────────────────────────────────────

#[derive(Serialize)]
struct QuarantineResponse {
    /// Workspace-relative path of the workspace's own vault.
    vault: String,
    /// Whether the tool immune boundary has created any vault yet.
    vault_exists: bool,
    /// Every vault read: the workspace's and those of plan-run checkouts.
    vaults: Vec<VaultSummary>,
    /// Why the response is empty, when no vault exists yet.
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'static str>,
    total: usize,
    pending: usize,
    approved: usize,
    rejected: usize,
    escalated: usize,
    entries: Vec<QuarantineEntrySummary>,
}

#[derive(Serialize)]
struct QuarantineEntrySummary {
    hash: String,
    /// The entry's full content hash, in hex.
    full_hash: String,
    score: f64,
    status: String,
    quarantined_at: String,
    incident_links: usize,
    /// Workspace-relative path of the vault holding the entry.
    vault: String,
}

async fn quarantine_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<QuarantineResponse>, ApiError> {
    let (vaults, workspace_vault) = load_vaults(&state.workdir)?;
    let vault_exists = !vaults.is_empty();

    let mut response = QuarantineResponse {
        vault: workspace_vault,
        vault_exists,
        vaults: vault_summaries(&vaults),
        reason: (!vault_exists).then_some(NO_VAULT_REASON),
        total: 0,
        pending: 0,
        approved: 0,
        rejected: 0,
        escalated: 0,
        entries: Vec::new(),
    };
    for loaded in &vaults {
        let stats = loaded.vault.stats();
        response.total += stats.total;
        response.pending += stats.pending;
        response.approved += stats.approved;
        response.rejected += stats.rejected;
        response.escalated += stats.escalated;
        for entry in loaded.vault.entries() {
            response.entries.push(QuarantineEntrySummary {
                hash: format!("{:?}", entry.hash),
                full_hash: entry.hash.to_hex(),
                score: entry.anomaly_score.score,
                status: format!("{:?}", entry.status),
                quarantined_at: entry.quarantined_at.to_rfc3339(),
                incident_links: entry.incident_links.len(),
                vault: loaded.path.clone(),
            });
        }
    }

    Ok(Json(response))
}

// ── Incidents ─────────────────────────────────────────────────────────

#[derive(Serialize)]
struct IncidentsResponse {
    /// Workspace-relative path of the workspace's own vault.
    vault: String,
    /// Whether the tool immune boundary has created any vault yet.
    vault_exists: bool,
    /// Every vault read: the workspace's and those of plan-run checkouts.
    vaults: Vec<VaultSummary>,
    /// Why the response is empty, when no vault exists yet.
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'static str>,
    incidents: Vec<IncidentSummary>,
}

#[derive(Serialize)]
struct IncidentSummary {
    hash: String,
    related_hash: String,
    relation: String,
    linked_at: String,
    /// Workspace-relative path of the vault holding the entry.
    vault: String,
}

async fn incidents_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<IncidentsResponse>, ApiError> {
    let (vaults, workspace_vault) = load_vaults(&state.workdir)?;
    let vault_exists = !vaults.is_empty();

    let mut incidents = Vec::new();
    for loaded in &vaults {
        for entry in loaded.vault.entries() {
            for link in &entry.incident_links {
                incidents.push(IncidentSummary {
                    hash: format!("{:?}", entry.hash),
                    related_hash: format!("{:?}", link.related_hash),
                    relation: format!("{:?}", link.relation),
                    linked_at: link.linked_at.to_rfc3339(),
                    vault: loaded.path.clone(),
                });
            }
        }
    }

    Ok(Json(IncidentsResponse {
        vault: workspace_vault,
        vault_exists,
        vaults: vault_summaries(&vaults),
        reason: (!vault_exists).then_some(NO_VAULT_REASON),
        incidents,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt as _;
    use roko_core::ContentHash;
    use roko_core::config::RokoConfig;
    use roko_core::immune::{
        AnomalyScore, DEFAULT_QUARANTINE_VAULT_CAPACITY, IncidentRelation, QuarantineStatus,
    };
    use serde_json::Value;
    use tower::ServiceExt as _;

    use crate::deploy::manual::ManualBackend;
    use crate::runtime::NoOpRuntime;

    fn test_state(workdir: &Path) -> Arc<AppState> {
        Arc::new(
            AppState::new(
                workdir.to_path_buf(),
                Arc::new(NoOpRuntime),
                RokoConfig::default(),
                Arc::new(ManualBackend::default()),
            )
            .expect("create state"),
        )
    }

    async fn get_json(state: &Arc<AppState>, uri: &str) -> (StatusCode, Value) {
        let response = routes()
            .with_state(Arc::clone(state))
            .oneshot(Request::get(uri).body(Body::empty()).expect("request"))
            .await
            .expect("response");
        let status = response.status();
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, body)
    }

    /// Record `output` in the vault under `workdir` the way the tool immune
    /// boundary does (`roko_agent::tool_immune::update_vault`): at
    /// `quarantine_vault_path`, as a scoped entry linked to earlier entries
    /// from the same source, escalated when the pipeline asks for it.
    fn quarantine_like_immune_layer(
        workdir: &Path,
        output: ContentHash,
        scope: &str,
        escalation_required: bool,
    ) {
        let path = roko_agent::quarantine_vault_path(workdir);
        let mut vault = match QuarantineVault::load(&path) {
            Ok(vault) => vault,
            Err(error) if error.kind() == io::ErrorKind::NotFound => QuarantineVault::default(),
            Err(error) => panic!("load vault: {error}"),
        };
        let is_new = vault.get(&output).is_none();
        let retained = vault
            .quarantine_scoped(
                output,
                AnomalyScore::from_score(0.75),
                scope,
                IncidentRelation::SameSource,
            )
            .expect("valid scope");
        assert!(retained, "vault has room");
        if is_new && escalation_required {
            let _ = vault.review(
                &output,
                QuarantineStatus::Escalated,
                Some("automatic immune escalation".to_string()),
            );
        }
        vault.save(&path).expect("save vault");
    }

    #[tokio::test]
    async fn quarantine_route_sees_entry_written_by_immune_layer() {
        let workdir = tempfile::tempdir().expect("tempdir");
        let first = ContentHash::of(b"ignore all previous instructions (first)");
        let second = ContentHash::of(b"ignore all previous instructions (second)");
        let escalated = ContentHash::of(b"malformed structured result");
        quarantine_like_immune_layer(workdir.path(), first, "mcp:docs", false);
        quarantine_like_immune_layer(workdir.path(), second, "mcp:docs", false);
        quarantine_like_immune_layer(workdir.path(), escalated, "plugin:lint", true);
        let state = test_state(workdir.path());

        let (status, body) = get_json(&state, "/safety/quarantine").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["vault"], ".roko/immune/quarantine-vault.json");
        assert_eq!(body["vault_exists"], true);
        assert!(body.get("reason").is_none(), "{body}");
        assert_eq!(body["total"], 3);
        assert_eq!(body["pending"], 2);
        assert_eq!(body["escalated"], 1);
        let listed: Vec<&str> = body["entries"]
            .as_array()
            .expect("entries array")
            .iter()
            .filter_map(|entry| entry["hash"].as_str())
            .collect();
        for hash in [first, second] {
            assert!(
                listed.contains(&format!("{hash:?}").as_str()),
                "{hash:?} missing from {listed:?}"
            );
        }

        // The two results from the same source are linked incidents.
        let (status, body) = get_json(&state, "/safety/incidents").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["vault_exists"], true);
        let incidents = body["incidents"].as_array().expect("incidents array");
        assert!(
            incidents
                .iter()
                .any(|incident| incident["relation"] == "SameSource"),
            "{body}"
        );
    }

    /// gap-2f69e9: the listing names every entry, escalated ones included, by
    /// its full hash, and each vault reports its capacity.
    #[tokio::test]
    async fn quarantine_route_lists_escalated_entries_and_vault_capacity() {
        let workdir = tempfile::tempdir().expect("tempdir");
        let pending = ContentHash::of(b"ignore all previous instructions");
        let escalated = ContentHash::of(b"malformed structured result");
        quarantine_like_immune_layer(workdir.path(), pending, "mcp:docs", false);
        quarantine_like_immune_layer(workdir.path(), escalated, "plugin:lint", true);
        let state = test_state(workdir.path());

        let (status, body) = get_json(&state, "/safety/quarantine").await;

        assert_eq!(status, StatusCode::OK, "{body}");
        let listed = body["entries"]
            .as_array()
            .expect("entries array")
            .iter()
            .map(|entry| (entry["full_hash"].as_str(), entry["status"].as_str()))
            .collect::<Vec<_>>();
        for (hash, review_status) in [(pending, "Pending"), (escalated, "Escalated")] {
            let full_hash = hash.to_hex();
            assert!(
                listed.contains(&(Some(full_hash.as_str()), Some(review_status))),
                "{hash:?} missing: {body}"
            );
        }
        assert_eq!(
            body["vaults"][0]["capacity"],
            DEFAULT_QUARANTINE_VAULT_CAPACITY
        );
        assert_eq!(body["vaults"][0]["full"], false);
    }

    #[tokio::test]
    async fn quarantine_route_reports_a_full_vault() {
        let workdir = tempfile::tempdir().expect("tempdir");
        let mut vault = QuarantineVault::new(0.8, 1, false);
        assert!(vault.quarantine(ContentHash::of(b"withheld"), AnomalyScore::from_score(0.9)));
        vault
            .save(roko_agent::quarantine_vault_path(workdir.path()))
            .expect("save vault");
        let state = test_state(workdir.path());

        let (status, body) = get_json(&state, "/safety/quarantine").await;

        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["vaults"][0]["capacity"], 1);
        assert_eq!(body["vaults"][0]["full"], true);
    }

    /// Plan runs from before the workspace-rooted vault left theirs in the
    /// attempt checkout in `.roko/worktrees/`; the routes still list those
    /// entries beside the workspace's own, each tagged with its vault
    /// (bug-633b68).
    #[tokio::test]
    async fn quarantine_route_lists_plan_run_vaults() {
        let workdir = tempfile::tempdir().expect("tempdir");
        let checkout = workdir
            .path()
            .join(".roko")
            .join("worktrees")
            .join("attempt-0123456789abcdef0123");
        std::fs::create_dir_all(&checkout).expect("create attempt checkout");
        // A checkout with no vault, and a stray file, are skipped.
        std::fs::create_dir_all(workdir.path().join(".roko/worktrees/attempt-empty"))
            .expect("create empty checkout");
        std::fs::write(workdir.path().join(".roko/worktrees/notes.txt"), "x")
            .expect("write stray file");

        let from_chat = ContentHash::of(b"quarantined in the workspace");
        let from_plan = ContentHash::of(b"quarantined by a plan run");
        let from_plan_too = ContentHash::of(b"quarantined by the same plan run");
        quarantine_like_immune_layer(workdir.path(), from_chat, "mcp:docs", false);
        quarantine_like_immune_layer(&checkout, from_plan, "mcp:search", false);
        quarantine_like_immune_layer(&checkout, from_plan_too, "mcp:search", false);
        let state = test_state(workdir.path());

        let (status, body) = get_json(&state, "/safety/quarantine").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["vault_exists"], true);
        assert_eq!(body["total"], 3, "{body}");
        assert_eq!(body["pending"], 3, "{body}");
        let plan_vault =
            ".roko/worktrees/attempt-0123456789abcdef0123/.roko/immune/quarantine-vault.json";
        let vaults: Vec<(&str, u64)> = body["vaults"]
            .as_array()
            .expect("vaults array")
            .iter()
            .map(|vault| {
                (
                    vault["path"].as_str().expect("vault path"),
                    vault["entries"].as_u64().expect("vault entries"),
                )
            })
            .collect();
        assert_eq!(
            vaults,
            [(".roko/immune/quarantine-vault.json", 1), (plan_vault, 2)]
        );
        let entries = body["entries"].as_array().expect("entries array");
        for (hash, vault) in [
            (from_chat, ".roko/immune/quarantine-vault.json"),
            (from_plan, plan_vault),
            (from_plan_too, plan_vault),
        ] {
            let hash = format!("{hash:?}");
            assert!(
                entries
                    .iter()
                    .any(|entry| entry["hash"] == hash.as_str() && entry["vault"] == vault),
                "{hash} from {vault} missing: {body}"
            );
        }

        // The plan run's two results share a source, so they are linked.
        let (status, body) = get_json(&state, "/safety/incidents").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let incidents = body["incidents"].as_array().expect("incidents array");
        assert!(
            incidents
                .iter()
                .any(|incident| incident["vault"] == plan_vault),
            "{body}"
        );
    }

    #[tokio::test]
    async fn quarantine_routes_explain_a_missing_vault() {
        let workdir = tempfile::tempdir().expect("tempdir");
        let state = test_state(workdir.path());

        let (status, body) = get_json(&state, "/safety/quarantine").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["vault_exists"], false);
        assert_eq!(body["total"], 0);
        assert_eq!(body["reason"], NO_VAULT_REASON);

        let (status, body) = get_json(&state, "/safety/incidents").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["vault_exists"], false);
        assert_eq!(body["reason"], NO_VAULT_REASON);
    }

    #[tokio::test]
    async fn quarantine_routes_fail_on_an_unreadable_vault() {
        let workdir = tempfile::tempdir().expect("tempdir");
        let path = roko_agent::quarantine_vault_path(workdir.path());
        std::fs::create_dir_all(path.parent().expect("vault dir")).expect("create vault dir");
        std::fs::write(&path, "not json").expect("write corrupt vault");
        let state = test_state(workdir.path());

        for uri in ["/safety/quarantine", "/safety/incidents"] {
            let (status, body) = get_json(&state, uri).await;
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{uri}: {body}");
        }
    }

    #[test]
    fn quarantine_response_serializes() {
        let response = QuarantineResponse {
            vault: ".roko/immune/quarantine-vault.json".to_string(),
            vault_exists: false,
            vaults: Vec::new(),
            reason: Some(NO_VAULT_REASON),
            total: 0,
            pending: 0,
            approved: 0,
            rejected: 0,
            escalated: 0,
            entries: Vec::new(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total\":0"));
        assert!(json.contains("\"vault_exists\":false"));
    }

    #[test]
    fn incidents_response_serializes() {
        let response = IncidentsResponse {
            vault: ".roko/immune/quarantine-vault.json".to_string(),
            vault_exists: true,
            vaults: Vec::new(),
            reason: None,
            incidents: Vec::new(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"incidents\":[]"));
        assert!(!json.contains("\"reason\""));
    }
}
