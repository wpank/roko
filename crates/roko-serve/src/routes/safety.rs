//! Safety observability endpoints.
//!
//! * `GET /api/safety/quarantine` -- quarantine vault entries.
//! * `GET /api/safety/incidents` -- incident log from the immune system.

use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::routing::get;
use serde::Serialize;

use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/safety/quarantine", get(quarantine_handler))
        .route("/safety/incidents", get(incidents_handler))
}

// ── Quarantine ────────────────────────────────────────────────────────

#[derive(Serialize)]
struct QuarantineResponse {
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
    score: f64,
    status: String,
    quarantined_at: String,
    incident_links: usize,
}

async fn quarantine_handler(State(state): State<Arc<AppState>>) -> Json<QuarantineResponse> {
    let vault_path = state
        .workdir
        .join(".roko")
        .join("immune")
        .join("quarantine.json");
    let vault = roko_core::immune::QuarantineVault::load(&vault_path).unwrap_or_default();
    let stats = vault.stats();

    let entries: Vec<QuarantineEntrySummary> = vault
        .pending()
        .into_iter()
        .map(|entry| QuarantineEntrySummary {
            hash: format!("{:?}", entry.hash),
            score: entry.anomaly_score.score,
            status: format!("{:?}", entry.status),
            quarantined_at: entry.quarantined_at.to_rfc3339(),
            incident_links: entry.incident_links.len(),
        })
        .collect();

    Json(QuarantineResponse {
        total: stats.total,
        pending: stats.pending,
        approved: stats.approved,
        rejected: stats.rejected,
        escalated: stats.escalated,
        entries,
    })
}

// ── Incidents ─────────────────────────────────────────────────────────

#[derive(Serialize)]
struct IncidentsResponse {
    incidents: Vec<IncidentSummary>,
}

#[derive(Serialize)]
struct IncidentSummary {
    hash: String,
    related_hash: String,
    relation: String,
    linked_at: String,
}

async fn incidents_handler(State(state): State<Arc<AppState>>) -> Json<IncidentsResponse> {
    let vault_path = state
        .workdir
        .join(".roko")
        .join("immune")
        .join("quarantine.json");
    let vault = roko_core::immune::QuarantineVault::load(&vault_path).unwrap_or_default();

    let mut incidents = Vec::new();
    for entry in vault.pending() {
        for link in &entry.incident_links {
            incidents.push(IncidentSummary {
                hash: format!("{:?}", entry.hash),
                related_hash: format!("{:?}", link.related_hash),
                relation: format!("{:?}", link.relation),
                linked_at: link.linked_at.to_rfc3339(),
            });
        }
    }

    Json(IncidentsResponse { incidents })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarantine_response_serializes() {
        let response = QuarantineResponse {
            total: 0,
            pending: 0,
            approved: 0,
            rejected: 0,
            escalated: 0,
            entries: Vec::new(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"total\":0"));
    }

    #[test]
    fn incidents_response_serializes() {
        let response = IncidentsResponse {
            incidents: Vec::new(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"incidents\":[]"));
    }
}
