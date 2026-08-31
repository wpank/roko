//! Bounded, run-scoped observability routes.
//!
//! Writers maintain hashed per-run JSONL indexes beside the compatibility
//! `events.jsonl` and `runtime-events.jsonl` logs. Handlers read only those
//! indexes (or a validated evidence-bundle directory); they never replay the
//! potentially hundreds-of-megabytes global logs on a request path.

use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;
use std::io::{BufRead, BufReader, Read as _, Seek, SeekFrom};
use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use futures::stream::{self, StreamExt};
use roko_core::obs::LogScrubber;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::broadcast;

use crate::error::ApiError;
use crate::state::AppState;

const DEFAULT_PAGE_LIMIT: usize = 100;
const MAX_PAGE_LIMIT: usize = 200;
const MAX_PAGE_SCAN_BYTES: u64 = 4 * 1024 * 1024;
const MAX_DETAIL_SCAN_BYTES: u64 = 8 * 1024 * 1024;
const MAX_LINE_BYTES: usize = 256 * 1024;
const MAX_DETAIL_EVENTS: usize = 10_000;
const MAX_MALFORMED_RECORDS: usize = 32;
const MAX_TYPE_FILTERS: usize = 16;
const MAX_BUNDLE_MANIFEST_BYTES: u64 = 256 * 1024;
const MAX_BUNDLE_ENTRIES: usize = 256;
const MAX_SCREENSHOTS: usize = 64;
const MAX_LOG_PREVIEW_CHARS: usize = 4_096;
const MAX_SSE_REPLAY: usize = 256;
const MAX_DASHBOARD_RUNS: usize = 128;
const MAX_DASHBOARD_INDEX_ENTRIES: usize = 512;
const MAX_DASHBOARD_SCAN_BYTES_PER_RUN: u64 = 256 * 1024;
const MAX_DASHBOARD_TOTAL_SCAN_BYTES: u64 = 8 * 1024 * 1024;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/dashboard/runs", get(get_dashboard_runs))
        .route("/runs/{run_id}", get(get_run_detail))
        .route("/runs/{run_id}/events", get(get_run_events))
        .route("/runs/{run_id}/events/stream", get(get_run_events_stream))
        .route("/runs/{run_id}/tasks", get(get_run_tasks))
        .route(
            "/runs/{run_id}/tasks/{task_id}/attempts",
            get(get_task_attempts),
        )
        .route("/runs/{run_id}/gates", get(get_run_gates))
        .route("/runs/{run_id}/logs", get(get_run_logs))
        .route("/runs/{run_id}/metrics", get(get_run_metrics))
        .route("/runs/{run_id}/artifacts", get(get_run_artifacts))
        .route("/runs/{run_id}/screenshots", get(get_run_screenshots))
        .route("/runs/{run_id}/bundle", get(get_run_bundle))
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct EventQuery {
    cursor: Option<u64>,
    limit: Option<usize>,
    types: Option<String>,
    source: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct LogQuery {
    cursor: Option<u64>,
    limit: Option<usize>,
    source: Option<String>,
    level: Option<String>,
    since: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventSource {
    Runner,
    Runtime,
}

impl EventSource {
    const fn label(self) -> &'static str {
        match self {
            Self::Runner => "runner",
            Self::Runtime => "runtime",
        }
    }
}

#[derive(Debug)]
struct IndexedPage {
    source: EventSource,
    cursor: u64,
    next_cursor: u64,
    has_more: bool,
    scanned_bytes: u64,
    quarantined_records: usize,
    partial_tail: bool,
    events: Vec<IndexedEvent>,
}

#[derive(Debug, Clone)]
struct IndexedEvent {
    cursor: u64,
    value: Value,
}

/// `GET /api/dashboard/runs` — summarize bounded per-run indexes.
///
/// The previous implementation replayed the complete global compatibility log
/// on every dashboard refresh. This version enumerates a bounded number of
/// hashed per-run indexes and applies both per-file and aggregate byte caps.
async fn get_dashboard_runs(State(state): State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    ensure_observability_allowed(&state)?;
    let io_state = Arc::clone(&state);
    let (runs, truncated, scanned_bytes) =
        blocking_io(move || Ok(discover_indexed_runs(&io_state))).await?;
    Ok(Json(json!({
        "runs": runs,
        "truncated": truncated,
        "scanned_bytes": scanned_bytes,
        "source": "per_run_indexes",
    })))
}

/// `GET /api/runs/{run_id}` — bounded detail assembled from one run index.
async fn get_run_detail(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    ensure_observability_allowed(&state)?;
    validate_id(&run_id, "run id")?;
    let active = state.active_runs.read().await.contains_key(&run_id);
    let io_state = Arc::clone(&state);
    let io_run_id = run_id.clone();
    let (page, bundle) = blocking_io(move || {
        let page = read_for_run(
            &io_state,
            &io_run_id,
            None,
            0,
            MAX_DETAIL_EVENTS,
            MAX_DETAIL_SCAN_BYTES,
            &BTreeSet::new(),
        )?;
        Ok((page, find_bundle_dir(&io_state, &io_run_id)))
    })
    .await?;
    if page.is_none() && !active && bundle.is_none() {
        return Err(ApiError::not_found(format!("run '{run_id}' not found")));
    }

    let (source, events, integrity) = match page {
        Some(page) => {
            let integrity = page_integrity(&page);
            (Some(page.source.label()), page.events, integrity)
        }
        None => (None, Vec::new(), json!({"state": "no_event_index"})),
    };
    let summary = summarize_events(&events);
    Ok(Json(json!({
        "run_id": run_id,
        "active": active,
        "status": summary["status"],
        "started_at": summary["started_at"],
        "finished_at": summary["finished_at"],
        "plans": summary["plans"],
        "tasks": summary["tasks"],
        "attempts": summary["attempts"],
        "gates": summary["gates"],
        "metrics": summary["metrics"],
        "event_source": source,
        "event_index": integrity,
        "bundle_available": bundle.is_some(),
        "links": run_links(&run_id),
    })))
}

/// `GET /api/runs/{run_id}/events` — cursor-paginated JSON, or SSE when the
/// request sends `Accept: text/event-stream`.
async fn get_run_events(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
    Query(query): Query<EventQuery>,
) -> Result<Response, ApiError> {
    if accepts_sse(&headers) {
        return run_events_sse(state, run_id, headers, query).await;
    }
    ensure_observability_allowed(&state)?;
    validate_id(&run_id, "run id")?;
    let types = parse_types(query.types.as_deref())?;
    let limit = bounded_limit(query.limit)?;
    let source = parse_source(query.source.as_deref())?;
    let io_state = Arc::clone(&state);
    let io_run_id = run_id.clone();
    let cursor = query.cursor.unwrap_or(0);
    let page = blocking_io(move || {
        read_for_run(
            &io_state,
            &io_run_id,
            source,
            cursor,
            limit,
            MAX_PAGE_SCAN_BYTES,
            &types,
        )
    })
    .await?
    .ok_or_else(|| ApiError::not_found(format!("run '{run_id}' has no event index")))?;

    Ok(Json(page_json(&run_id, page)).into_response())
}

/// Explicit alias for clients that cannot set an Accept header.
async fn get_run_events_stream(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
    headers: HeaderMap,
    Query(query): Query<EventQuery>,
) -> Result<Response, ApiError> {
    run_events_sse(state, run_id, headers, query).await
}

async fn run_events_sse(
    state: Arc<AppState>,
    run_id: String,
    headers: HeaderMap,
    mut query: EventQuery,
) -> Result<Response, ApiError> {
    ensure_observability_allowed(&state)?;
    validate_id(&run_id, "run id")?;
    let active = state.active_runs.read().await.contains_key(&run_id);
    let known = if active {
        true
    } else {
        let io_state = Arc::clone(&state);
        let io_run_id = run_id.clone();
        blocking_io(move || Ok(run_is_known_on_disk(&io_state, &io_run_id))).await?
    };
    if !known {
        return Err(ApiError::not_found(format!("run '{run_id}' not found")));
    }

    query.source = Some("runtime".to_string());
    query.limit = Some(query.limit.unwrap_or(MAX_SSE_REPLAY).min(MAX_SSE_REPLAY));
    let types = parse_types(query.types.as_deref())?;
    let header_cursor = headers
        .get("Last-Event-ID")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    let cursor = header_cursor.or(query.cursor).unwrap_or(0);

    // Subscribe before reading the durable suffix. Live frames whose persisted
    // byte cursor is not beyond the suffix are then recognized as duplicates.
    let rx = state.sse_adapter.subscribe();
    let io_state = Arc::clone(&state);
    let io_run_id = run_id.clone();
    let io_types = types.clone();
    let replay_limit = query.limit.unwrap_or(MAX_SSE_REPLAY);
    let historical = blocking_io(move || {
        read_for_run(
            &io_state,
            &io_run_id,
            Some(EventSource::Runtime),
            cursor,
            replay_limit,
            MAX_PAGE_SCAN_BYTES,
            &io_types,
        )
    })
    .await?;
    let live_floor = historical.as_ref().map_or(cursor, |page| page.next_cursor);
    let scrubber = Arc::clone(&state.scrubber);
    let replay = historical
        .map(|page| {
            page.events
                .into_iter()
                .map(|item| indexed_sse_event(item, &scrubber))
                .map(Ok::<_, Infallible>)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let live_state = Arc::clone(&state);
    let live_run_id = run_id.clone();
    let live_types = types.clone();
    let live = stream::unfold(
        (rx, live_state, live_run_id, live_floor, live_types),
        |(mut rx, state, run_id, mut cursor, types)| async move {
            loop {
                match rx.recv().await {
                    Ok(event) if event.run_id == run_id => {
                        if !types.is_empty() && !types.contains(&event.kind) {
                            continue;
                        }
                        let data = state.scrubber.scrub(
                            &serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string()),
                        );
                        let mut frame = Event::default().event(event.kind).data(data);
                        if let Some(persisted_cursor) = event.cursor {
                            if persisted_cursor <= cursor {
                                continue;
                            }
                            cursor = persisted_cursor;
                            frame = frame.id(cursor.to_string());
                        }
                        return Some((Ok::<_, Infallible>(frame), (rx, state, run_id, cursor, types)));
                    }
                    Ok(_) => continue,
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        let data = json!({
                            "run_id": run_id,
                            "missed_events": n,
                            "resume_cursor": cursor,
                        });
                        let frame = Event::default()
                            .event("gap")
                            .id(cursor.to_string())
                            .data(data.to_string());
                        return Some((Ok::<_, Infallible>(frame), (rx, state, run_id, cursor, types)));
                    }
                    Err(broadcast::error::RecvError::Closed) => return None,
                }
            }
        },
    );

    let sse = Sse::new(stream::iter(replay).chain(live)).keep_alive(
        KeepAlive::new()
            .interval(std::time::Duration::from_secs(8))
            .text("keepalive"),
    );
    Ok((super::sse::sse_response_headers(), sse).into_response())
}

async fn get_run_tasks(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    ensure_observability_allowed(&state)?;
    validate_id(&run_id, "run id")?;
    let io_state = Arc::clone(&state);
    let io_run_id = run_id.clone();
    let page = blocking_io(move || detail_page(&io_state, &io_run_id)).await?;
    let mut tasks: BTreeMap<String, Value> = BTreeMap::new();
    for item in &page.events {
        let Some(task_id) = event_task_id(&item.value) else {
            continue;
        };
        let entry = tasks.entry(task_id.to_string()).or_insert_with(|| {
            json!({
                "task_id": task_id,
                "plan_id": event_plan_id(&item.value),
                "attempts": BTreeSet::<u64>::new(),
                "event_count": 0,
                "status": "observed",
            })
        });
        entry["event_count"] = json!(entry["event_count"].as_u64().unwrap_or(0) + 1);
        if let Some(attempt) = event_attempt(&item.value) {
            let mut attempts = entry["attempts"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            if !attempts.iter().any(|value| value.as_u64() == Some(attempt)) {
                attempts.push(json!(attempt));
                attempts.sort_by_key(Value::as_u64);
            }
            entry["attempts"] = Value::Array(attempts);
        }
        if is_task_terminal(event_type(&item.value).unwrap_or_default()) {
            entry["status"] = terminal_status(&item.value);
        }
    }
    Ok(Json(json!({
        "run_id": run_id,
        "source": page.source.label(),
        "tasks": tasks.into_values().take(MAX_PAGE_LIMIT).collect::<Vec<_>>(),
        "integrity": page_integrity(&page),
    })))
}

async fn get_task_attempts(
    State(state): State<Arc<AppState>>,
    Path((run_id, task_id)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    ensure_observability_allowed(&state)?;
    validate_id(&run_id, "run id")?;
    validate_id(&task_id, "task id")?;
    let io_state = Arc::clone(&state);
    let io_run_id = run_id.clone();
    let page = blocking_io(move || detail_page(&io_state, &io_run_id)).await?;
    let mut attempts: BTreeMap<u64, Vec<Value>> = BTreeMap::new();
    for item in &page.events {
        if event_task_id(&item.value) != Some(task_id.as_str()) {
            continue;
        }
        let attempt = event_attempt(&item.value).unwrap_or(0);
        let rows = attempts.entry(attempt).or_default();
        if rows.len() < MAX_PAGE_LIMIT {
            rows.push(indexed_event_json(item.clone(), page.source));
        }
    }
    if attempts.is_empty() {
        return Err(ApiError::not_found(format!(
            "task '{task_id}' has no attempts in run '{run_id}'"
        )));
    }
    Ok(Json(json!({
        "run_id": run_id,
        "task_id": task_id,
        "source": page.source.label(),
        "attempts": attempts.into_iter().map(|(attempt, events)| json!({
            "attempt": attempt,
            "status": events.last().map(|value| terminal_status(&value["event"])).unwrap_or_else(|| json!("observed")),
            "events": events,
        })).collect::<Vec<_>>(),
        "integrity": page_integrity(&page),
    })))
}

async fn get_run_gates(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
    Query(query): Query<EventQuery>,
) -> Result<Json<Value>, ApiError> {
    ensure_observability_allowed(&state)?;
    validate_id(&run_id, "run id")?;
    let limit = bounded_limit(query.limit)?;
    let source = parse_source(query.source.as_deref())?;
    let cursor = query.cursor.unwrap_or(0);
    let io_state = Arc::clone(&state);
    let io_run_id = run_id.clone();
    let page = blocking_io(move || {
        read_for_run_filtered(
            &io_state,
            &io_run_id,
            source,
            cursor,
            limit,
            MAX_PAGE_SCAN_BYTES,
            &BTreeSet::new(),
            &|value| event_type(value).is_some_and(is_gate_event),
        )
    })
    .await?
    .ok_or_else(|| ApiError::not_found(format!("run '{run_id}' has no event index")))?;
    let gates = page
        .events
        .iter()
        .cloned()
        .map(|item| indexed_event_json(item, page.source))
        .collect::<Vec<_>>();
    Ok(Json(json!({
        "run_id": run_id,
        "source": page.source.label(),
        "cursor": page.cursor,
        "next_cursor": page.next_cursor,
        "has_more": page.has_more,
        "gates": gates,
        "integrity": page_integrity(&page),
    })))
}

async fn get_run_logs(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
    Query(query): Query<LogQuery>,
) -> Result<Json<Value>, ApiError> {
    ensure_observability_allowed(&state)?;
    validate_id(&run_id, "run id")?;
    let source = parse_source(query.source.as_deref().filter(|value| *value != "events"))?;
    let limit = bounded_limit(query.limit)?;
    let wanted_level = query.level.as_deref().map(str::to_ascii_lowercase);
    let since = query.since;
    let cursor = query.cursor.unwrap_or(0);
    let io_state = Arc::clone(&state);
    let io_run_id = run_id.clone();
    let page = blocking_io(move || {
        read_for_run_filtered(
            &io_state,
            &io_run_id,
            source,
            cursor,
            limit,
            MAX_PAGE_SCAN_BYTES,
            &BTreeSet::new(),
            &|value| {
                if since.is_some_and(|since| event_timestamp_ms(value).unwrap_or(0) < since) {
                    return false;
                }
                let level = event_level(value);
                !wanted_level.as_deref().is_some_and(|wanted| wanted != level)
            },
        )
    })
    .await?
    .ok_or_else(|| ApiError::not_found(format!("run '{run_id}' has no event index")))?;
    let mut logs = Vec::new();
    for item in &page.events {
        let level = event_level(&item.value);
        let serialized = serde_json::to_string(&item.value).unwrap_or_default();
        let message = serialized.chars().take(MAX_LOG_PREVIEW_CHARS).collect::<String>();
        logs.push(json!({
            "cursor": item.cursor,
            "timestamp_ms": event_timestamp_ms(&item.value),
            "source": page.source.label(),
            "level": level,
            "type": event_type(&item.value),
            "message": message,
            "truncated": serialized.chars().count() > MAX_LOG_PREVIEW_CHARS,
        }));
    }
    Ok(Json(json!({
        "run_id": run_id,
        "source": page.source.label(),
        "cursor": page.cursor,
        "next_cursor": page.next_cursor,
        "has_more": page.has_more,
        "logs": logs,
        "integrity": page_integrity(&page),
    })))
}

async fn get_run_metrics(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    ensure_observability_allowed(&state)?;
    validate_id(&run_id, "run id")?;
    let io_state = Arc::clone(&state);
    let io_run_id = run_id.clone();
    let page = blocking_io(move || detail_page(&io_state, &io_run_id)).await?;
    let summary = summarize_events(&page.events);
    Ok(Json(json!({
        "run_id": run_id,
        "source": page.source.label(),
        "metrics": summary["metrics"],
        "integrity": page_integrity(&page),
    })))
}

async fn get_run_artifacts(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    ensure_observability_allowed(&state)?;
    validate_id(&run_id, "run id")?;
    let io_state = Arc::clone(&state);
    let io_run_id = run_id.clone();
    let (mut artifacts, bundle_available) = blocking_io(move || {
        let bundle = find_bundle_dir(&io_state, &io_run_id);
        let mut artifacts = bundle
            .as_deref()
            .map(bundle_files)
            .unwrap_or_default()
            .into_iter()
            .filter(|entry| {
                entry["path"]
                    .as_str()
                    .is_some_and(|path| !path.starts_with("screenshots/"))
            })
            .collect::<Vec<_>>();
        if let Ok(page) = detail_page(&io_state, &io_run_id) {
            for item in page.events {
                if let Some(name) = checkpoint_name(&item.value) {
                    artifacts.push(json!({
                        "kind": "checkpoint",
                        "name": name,
                        "content_available": false,
                        "note": "event reference only; arbitrary path serving is disabled",
                    }));
                }
            }
        }
        Ok((artifacts, bundle.is_some()))
    })
    .await?;
    artifacts.truncate(MAX_BUNDLE_ENTRIES);
    if artifacts.is_empty() && !bundle_available {
        return Err(ApiError::not_found(format!("run '{run_id}' has no artifacts")));
    }
    Ok(Json(json!({
        "run_id": run_id,
        "artifacts": artifacts,
        "content_serving": false,
    })))
}

async fn get_run_screenshots(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    ensure_observability_allowed(&state)?;
    validate_id(&run_id, "run id")?;
    let io_state = Arc::clone(&state);
    let io_run_id = run_id.clone();
    let (manifest, files) = blocking_io(move || {
        let bundle = find_bundle_dir(&io_state, &io_run_id).ok_or_else(|| {
            ApiError::not_found(format!("run '{io_run_id}' has no evidence bundle"))
        })?;
        let screenshots = bundle.join("screenshots");
        if !safe_child_dir(&bundle, &screenshots) {
            return Err(ApiError::not_found(format!(
                "run '{io_run_id}' has no screenshots"
            )));
        }
        Ok((
            read_bounded_json(
                &screenshots.join("manifest.json"),
                MAX_BUNDLE_MANIFEST_BYTES,
                io_state.scrubber.as_ref(),
            ),
            list_screenshot_files(&screenshots),
        ))
    })
    .await?;
    Ok(Json(json!({
        "run_id": run_id,
        "manifest": manifest,
        "screenshots": files,
        "content_serving": false,
    })))
}

async fn get_run_bundle(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    ensure_observability_allowed(&state)?;
    validate_id(&run_id, "run id")?;
    let io_state = Arc::clone(&state);
    let io_run_id = run_id.clone();
    let (manifest, files) = blocking_io(move || {
        let bundle = find_bundle_dir(&io_state, &io_run_id).ok_or_else(|| {
            ApiError::not_found(format!("run '{io_run_id}' has no evidence bundle"))
        })?;
        Ok((
            read_bounded_json(
                &bundle.join("manifest.json"),
                MAX_BUNDLE_MANIFEST_BYTES,
                io_state.scrubber.as_ref(),
            ),
            bundle_files(&bundle),
        ))
    })
    .await?;
    let total_bytes = files
        .iter()
        .filter_map(|entry| entry["bytes"].as_u64())
        .fold(0u64, u64::saturating_add);
    Ok(Json(json!({
        "run_id": run_id,
        "manifest": manifest,
        "files": files,
        "total_bytes": total_bytes,
        "download_available": false,
        "note": "manifest only; arbitrary archive and file serving are intentionally disabled",
    })))
}

async fn blocking_io<T, F>(operation: F) -> Result<T, ApiError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, ApiError> + Send + 'static,
{
    tokio::task::spawn_blocking(operation)
        .await
        .map_err(|error| ApiError::internal(format!("run observability worker failed: {error}")))?
}

fn ensure_observability_allowed(state: &AppState) -> Result<(), ApiError> {
    let config = state.load_roko_config();
    if config.serve.auth.enabled || super::bind_is_loopback(&config.server.bind) {
        return Ok(());
    }
    Err(ApiError::forbidden(
        "run observability requires loopback binding or enabled API authentication",
    ))
}

fn validate_id(value: &str, label: &str) -> Result<(), ApiError> {
    roko_fs::run_index::validate_scoped_id(value)
        .map_err(|reason| ApiError::bad_request(format!("invalid {label}: {reason}")))
}

fn bounded_limit(limit: Option<usize>) -> Result<usize, ApiError> {
    let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);
    if limit == 0 || limit > MAX_PAGE_LIMIT {
        return Err(ApiError::bad_request(format!(
            "limit must be between 1 and {MAX_PAGE_LIMIT}"
        )));
    }
    Ok(limit)
}

fn parse_types(raw: Option<&str>) -> Result<BTreeSet<String>, ApiError> {
    let Some(raw) = raw else {
        return Ok(BTreeSet::new());
    };
    let mut types = BTreeSet::new();
    for item in raw.split(',').map(str::trim).filter(|item| !item.is_empty()) {
        if item.len() > 64
            || !item
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        {
            return Err(ApiError::bad_request("invalid event type filter"));
        }
        types.insert(item.to_string());
        if types.len() > MAX_TYPE_FILTERS {
            return Err(ApiError::bad_request(format!(
                "at most {MAX_TYPE_FILTERS} event types may be requested"
            )));
        }
    }
    Ok(types)
}

fn parse_source(raw: Option<&str>) -> Result<Option<EventSource>, ApiError> {
    match raw {
        None | Some("") | Some("auto") | Some("events") => Ok(None),
        Some("runner") => Ok(Some(EventSource::Runner)),
        Some("runtime") => Ok(Some(EventSource::Runtime)),
        Some(_) => Err(ApiError::bad_request(
            "source must be one of auto, runner, or runtime",
        )),
    }
}

fn source_path(state: &AppState, run_id: &str, source: EventSource) -> Option<PathBuf> {
    match source {
        EventSource::Runner => roko_fs::run_index::run_index_path(
            &state.layout.events_jsonl_path(),
            run_id,
        )
        .ok(),
        EventSource::Runtime => {
            // Materialize this run's bounded buffer on demand. This does not
            // flush or scan the global compatibility log.
            let _ = state.runtime_event_logger.flush_run(run_id);
            state.runtime_event_logger.run_path(run_id).ok()
        }
    }
}

fn discover_indexed_runs(state: &AppState) -> (Vec<Value>, bool, u64) {
    let mut summaries = BTreeMap::<String, Value>::new();
    let mut scanned_bytes = 0u64;
    let mut inspected_entries = 0usize;
    let mut truncated = false;
    // Runtime summaries preserve workflow/template detail. Runner indexes are
    // the fallback for self-hosted plan runs that did not use HTTP ingest.
    for source in [EventSource::Runtime, EventSource::Runner] {
        let Some(sample) = source_path(state, "index-probe", source) else {
            continue;
        };
        let Some(dir) = sample.parent() else {
            continue;
        };
        let Ok(metadata) = std::fs::symlink_metadata(dir) else {
            continue;
        };
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries {
            inspected_entries = inspected_entries.saturating_add(1);
            if summaries.len() >= MAX_DASHBOARD_RUNS
                || inspected_entries > MAX_DASHBOARD_INDEX_ENTRIES
                || scanned_bytes >= MAX_DASHBOARD_TOTAL_SCAN_BYTES
            {
                truncated = true;
                break;
            }
            let Ok(entry) = entry else {
                continue;
            };
            let path = entry.path();
            let Ok(metadata) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            if metadata.file_type().is_symlink()
                || !metadata.is_file()
                || path.extension().and_then(|value| value.to_str()) != Some("jsonl")
            {
                continue;
            }
            let budget = MAX_DASHBOARD_SCAN_BYTES_PER_RUN.min(
                MAX_DASHBOARD_TOTAL_SCAN_BYTES.saturating_sub(scanned_bytes),
            );
            let (run_id, discovery_bytes) = discover_run_id(&path, budget);
            scanned_bytes = scanned_bytes.saturating_add(discovery_bytes);
            let Some(run_id) = run_id else {
                continue;
            };
            if summaries.contains_key(&run_id) {
                continue;
            }
            let page_budget = budget
                .saturating_sub(discovery_bytes)
                .min(MAX_DASHBOARD_TOTAL_SCAN_BYTES.saturating_sub(scanned_bytes));
            if page_budget == 0 {
                truncated = true;
                break;
            }
            let Ok(page) = read_index_page(
                &path,
                &run_id,
                source,
                0,
                MAX_DETAIL_EVENTS,
                page_budget,
                &BTreeSet::new(),
                state.scrubber.as_ref(),
            ) else {
                continue;
            };
            scanned_bytes = scanned_bytes.saturating_add(page.scanned_bytes);
            let aggregate = summarize_events(&page.events);
            let template = page.events.iter().find_map(|item| {
                (event_type(&item.value) == Some("workflow_started"))
                    .then(|| event_data(&item.value).get("template").cloned())
                    .flatten()
            });
            let prompt = page.events.iter().find_map(|item| {
                matches!(event_type(&item.value), Some("workflow_started" | "run_started"))
                    .then(|| event_data(&item.value).get("prompt").cloned())
                    .flatten()
            });
            let status = aggregate["status"].clone();
            summaries.insert(
                run_id.clone(),
                json!({
                    "run_id": run_id,
                    "template": template,
                    "prompt": prompt,
                    "current_phase": current_phase(&page.events),
                    "phases_visited": phases_visited(&page.events),
                    "gates_passed": aggregate["metrics"]["gates_passed"],
                    "gates_failed": aggregate["metrics"]["gates_failed"],
                    "agents_spawned": count_event_types(&page.events, &["agent.dispatch.started", "agent_spawned"]),
                    "is_complete": matches!(status.as_str(), Some("completed" | "failed" | "cancelled")),
                    "outcome": status,
                    "source": source.label(),
                    "truncated": page.has_more,
                }),
            );
        }
    }
    (summaries.into_values().collect(), truncated, scanned_bytes)
}

fn discover_run_id(path: &FsPath, max_bytes: u64) -> (Option<String>, u64) {
    let Ok(file) = roko_fs::run_index::open_existing_run_index(path) else {
        return (None, 0);
    };
    let mut reader = BufReader::new(file);
    let mut scanned = 0u64;
    let mut line = String::new();
    while scanned < max_bytes {
        line.clear();
        let bytes = match reader
            .by_ref()
            .take(max_bytes.saturating_sub(scanned).min((MAX_LINE_BYTES + 1) as u64))
            .read_line(&mut line)
        {
            Ok(bytes) => bytes,
            Err(_) => return (None, max_bytes),
        };
        if bytes == 0 {
            break;
        }
        scanned = scanned.saturating_add(bytes as u64);
        if bytes > MAX_LINE_BYTES {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(line.trim_end()) else {
            continue;
        };
        let Some(run_id) = validated_embedded_run_id(&value) else {
            continue;
        };
        if validate_id(run_id, "run id").is_ok() {
            return (Some(run_id.to_string()), scanned);
        }
    }
    (None, scanned)
}

fn current_phase(events: &[IndexedEvent]) -> Option<String> {
    events.iter().rev().find_map(|item| {
        let kind = event_type(&item.value)?;
        if !matches!(kind, "phase_transition" | "pipeline_phase") {
            return None;
        }
        event_data(&item.value)
            .get("to")
            .or_else(|| event_data(&item.value).get("phase"))
            .and_then(Value::as_str)
            .map(str::to_string)
    })
}

fn phases_visited(events: &[IndexedEvent]) -> Vec<String> {
    let mut phases = Vec::new();
    for item in events {
        let Some(kind) = event_type(&item.value) else {
            continue;
        };
        if !matches!(kind, "phase_transition" | "pipeline_phase") {
            continue;
        }
        if let Some(phase) = event_data(&item.value)
            .get("to")
            .or_else(|| event_data(&item.value).get("phase"))
            .and_then(Value::as_str)
            && phases.last().map(String::as_str) != Some(phase)
        {
            phases.push(phase.to_string());
        }
    }
    phases
}

fn count_event_types(events: &[IndexedEvent], kinds: &[&str]) -> usize {
    events
        .iter()
        .filter(|item| event_type(&item.value).is_some_and(|kind| kinds.contains(&kind)))
        .count()
}

fn read_for_run(
    state: &AppState,
    run_id: &str,
    requested_source: Option<EventSource>,
    cursor: u64,
    limit: usize,
    scan_bytes: u64,
    types: &BTreeSet<String>,
) -> Result<Option<IndexedPage>, ApiError> {
    read_for_run_filtered(
        state,
        run_id,
        requested_source,
        cursor,
        limit,
        scan_bytes,
        types,
        &|_| true,
    )
}

fn read_for_run_filtered(
    state: &AppState,
    run_id: &str,
    requested_source: Option<EventSource>,
    cursor: u64,
    limit: usize,
    scan_bytes: u64,
    types: &BTreeSet<String>,
    include: &dyn Fn(&Value) -> bool,
) -> Result<Option<IndexedPage>, ApiError> {
    let candidates: &[EventSource] = match requested_source {
        Some(EventSource::Runner) => &[EventSource::Runner],
        Some(EventSource::Runtime) => &[EventSource::Runtime],
        None => &[EventSource::Runner, EventSource::Runtime],
    };
    for source in candidates {
        let Some(path) = source_path(state, run_id, *source) else {
            continue;
        };
        if roko_fs::run_index::open_existing_run_index(&path).is_ok() {
            return read_index_page_filtered(
                &path,
                run_id,
                *source,
                cursor,
                limit,
                scan_bytes,
                types,
                state.scrubber.as_ref(),
                include,
            )
            .map(Some);
        }
    }
    Ok(None)
}

fn read_index_page(
    path: &FsPath,
    run_id: &str,
    source: EventSource,
    cursor: u64,
    limit: usize,
    scan_bytes: u64,
    types: &BTreeSet<String>,
    scrubber: &LogScrubber,
) -> Result<IndexedPage, ApiError> {
    read_index_page_filtered(
        path,
        run_id,
        source,
        cursor,
        limit,
        scan_bytes,
        types,
        scrubber,
        &|_| true,
    )
}

#[allow(clippy::too_many_arguments)]
fn read_index_page_filtered(
    path: &FsPath,
    run_id: &str,
    source: EventSource,
    cursor: u64,
    limit: usize,
    scan_bytes: u64,
    types: &BTreeSet<String>,
    scrubber: &LogScrubber,
    include: &dyn Fn(&Value) -> bool,
) -> Result<IndexedPage, ApiError> {
    let mut file = roko_fs::run_index::open_existing_run_index(path)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let file_len = file
        .metadata()
        .map_err(|error| ApiError::internal(error.to_string()))?
        .len();
    if cursor > file_len {
        return Err(ApiError::bad_request("cursor is beyond the event index"));
    }
    if cursor > 0 {
        file.seek(SeekFrom::Start(cursor - 1))
            .map_err(|error| ApiError::internal(error.to_string()))?;
        let mut boundary = [0u8; 1];
        std::io::Read::read_exact(&mut file, &mut boundary)
            .map_err(|error| ApiError::bad_request(format!("invalid cursor: {error}")))?;
        if boundary[0] != b'\n' {
            return Err(ApiError::bad_request(
                "cursor must be a next_cursor returned by this endpoint",
            ));
        }
    }
    file.seek(SeekFrom::Start(cursor))
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let mut reader = BufReader::new(file);
    let mut offset = cursor;
    let mut scanned_bytes = 0u64;
    let mut quarantined = 0usize;
    let mut partial_tail = false;
    let mut events = Vec::new();
    let mut line = String::new();

    while offset < file_len && scanned_bytes < scan_bytes && events.len() < limit {
        line.clear();
        let record_start = offset;
        let bytes = reader
            .by_ref()
            .take((MAX_LINE_BYTES + 1) as u64)
            .read_line(&mut line)
            .map_err(|error| ApiError::internal(format!("read run event index: {error}")))?;
        if bytes == 0 {
            break;
        }
        scanned_bytes = scanned_bytes.saturating_add(bytes as u64);
        offset = offset.saturating_add(bytes as u64);
        if bytes > MAX_LINE_BYTES {
            return Err(ApiError::unprocessable_entity(
                "run event index contains an oversized record",
            ));
        }
        if !line.ends_with('\n') {
            // A concurrent append can make metadata visible before the whole
            // JSONL record. Never expose or advance beyond that partial tail;
            // the same safe cursor will retry it on the next request.
            offset = record_start;
            partial_tail = true;
            break;
        }
        let parsed = serde_json::from_str::<Value>(line.trim_end());
        let Ok(mut value) = parsed else {
            quarantined += 1;
            if quarantined > MAX_MALFORMED_RECORDS {
                return Err(ApiError::unprocessable_entity(
                    "run event index contains too many malformed records",
                ));
            }
            continue;
        };
        if validated_embedded_run_id(&value) != Some(run_id) {
            quarantined += 1;
            if quarantined > MAX_MALFORMED_RECORDS {
                return Err(ApiError::unprocessable_entity(
                    "run event index failed run-id integrity checks",
                ));
            }
            continue;
        }
        if !types.is_empty()
            && !event_type(&value).is_some_and(|kind| types.contains(kind))
        {
            continue;
        }
        if !include(&value) {
            continue;
        }
        scrub_json_value(&mut value, scrubber);
        events.push(IndexedEvent {
            cursor: offset,
            value,
        });
    }

    Ok(IndexedPage {
        source,
        cursor,
        next_cursor: offset,
        has_more: offset < file_len,
        scanned_bytes,
        quarantined_records: quarantined,
        partial_tail,
        events,
    })
}

fn page_json(run_id: &str, page: IndexedPage) -> Value {
    let integrity = page_integrity(&page);
    json!({
        "run_id": run_id,
        "source": page.source.label(),
        "cursor": page.cursor,
        "next_cursor": page.next_cursor,
        "has_more": page.has_more,
        "events": page.events.into_iter().map(|item| indexed_event_json(item, page.source)).collect::<Vec<_>>(),
        "integrity": integrity,
    })
}

fn page_integrity(page: &IndexedPage) -> Value {
    json!({
        "state": if page.partial_tail {
            "partial_tail"
        } else if page.quarantined_records == 0 {
            "ok"
        } else {
            "degraded"
        },
        "quarantined_records": page.quarantined_records,
        "partial_tail": page.partial_tail,
        "scanned_bytes": page.scanned_bytes,
        "response_bounded": page.has_more,
        "index_durability": "derived_best_effort",
        "repair_source": "global compatibility log (offline repair only)",
    })
}

fn indexed_event_json(item: IndexedEvent, source: EventSource) -> Value {
    json!({
        "cursor": item.cursor,
        "source": source.label(),
        "type": event_type(&item.value),
        "event": item.value,
    })
}

fn indexed_sse_event(item: IndexedEvent, scrubber: &LogScrubber) -> Event {
    let kind = event_type(&item.value).unwrap_or("event").to_string();
    let data = scrubber.scrub(&serde_json::to_string(&item.value).unwrap_or_default());
    Event::default()
        .event(kind)
        .id(item.cursor.to_string())
        .data(data)
}

fn accepts_sse(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.split(',').any(|part| part.trim() == "text/event-stream"))
}

fn run_is_known_on_disk(state: &AppState, run_id: &str) -> bool {
    [EventSource::Runner, EventSource::Runtime]
        .into_iter()
        .filter_map(|source| source_path(state, run_id, source))
        .any(|path| roko_fs::run_index::open_existing_run_index(&path).is_ok())
        || find_bundle_dir(state, run_id).is_some()
}

fn detail_page(state: &AppState, run_id: &str) -> Result<IndexedPage, ApiError> {
    read_for_run(
        state,
        run_id,
        None,
        0,
        MAX_DETAIL_EVENTS,
        MAX_DETAIL_SCAN_BYTES,
        &BTreeSet::new(),
    )?
    .ok_or_else(|| ApiError::not_found(format!("run '{run_id}' has no event index")))
}

fn validated_embedded_run_id(value: &Value) -> Option<&str> {
    let run_id = value.get("run_id")?.as_str()?;
    roko_fs::run_index::validate_scoped_id(run_id).ok()?;
    for pointer in ["/event/run_id", "/payload/data/run_id"] {
        if let Some(nested) = value.pointer(pointer) {
            let nested = nested.as_str()?;
            roko_fs::run_index::validate_scoped_id(nested).ok()?;
            if nested != run_id {
                return None;
            }
        }
    }
    Some(run_id)
}

fn event_type(value: &Value) -> Option<&str> {
    value
        .get("type")
        .and_then(Value::as_str)
        .or_else(|| value.get("event")?.get("type")?.as_str())
        .or_else(|| value.get("payload")?.get("kind")?.as_str())
        .or_else(|| value.get("kind")?.as_str())
}

fn event_data(value: &Value) -> &Value {
    value
        .get("event")
        .unwrap_or(value)
        .get("payload")
        .and_then(|payload| payload.get("data"))
        .or_else(|| value.get("data"))
        .unwrap_or(value.get("event").unwrap_or(value))
}

fn event_task_id(value: &Value) -> Option<&str> {
    value
        .get("task_id")
        .and_then(Value::as_str)
        .or_else(|| event_data(value).get("task_id").and_then(Value::as_str))
        .or_else(|| {
            event_data(value)
                .get("attempt")
                .and_then(|attempt| attempt.get("task_id"))
                .and_then(Value::as_str)
        })
}

fn event_plan_id(value: &Value) -> Option<&str> {
    value
        .get("plan_id")
        .and_then(Value::as_str)
        .or_else(|| event_data(value).get("plan_id").and_then(Value::as_str))
}

fn event_attempt(value: &Value) -> Option<u64> {
    value
        .get("attempt")
        .and_then(Value::as_u64)
        .or_else(|| event_data(value).get("attempt").and_then(Value::as_u64))
}

fn event_timestamp_ms(value: &Value) -> Option<u64> {
    value
        .get("timestamp_ms")
        .and_then(Value::as_u64)
        .or_else(|| value.get("event")?.get("timestamp_ms")?.as_u64())
}

fn event_level(value: &Value) -> &'static str {
    let kind = event_type(value).unwrap_or_default();
    if kind.contains("failed") || kind.contains("error") || kind.contains("timeout") {
        "error"
    } else if kind.contains("cancel") || kind.contains("retry") || kind.contains("budget") {
        "warn"
    } else {
        "info"
    }
}

fn is_gate_event(kind: &str) -> bool {
    kind.starts_with("gate.") || kind.starts_with("gate_") || kind.contains("gate")
}

fn is_task_terminal(kind: &str) -> bool {
    matches!(kind, "task.attempt.completed" | "task_completed" | "task_failed")
}

fn terminal_status(value: &Value) -> Value {
    let kind = event_type(value).unwrap_or_default();
    if kind.contains("cancel") {
        return json!("cancelled");
    }
    let data = event_data(value);
    if data.get("passed").and_then(Value::as_bool) == Some(true)
        || data.get("success").and_then(Value::as_bool) == Some(true)
        || value.get("outcome").and_then(Value::as_str) == Some("passed")
    {
        json!("passed")
    } else if is_task_terminal(kind) {
        json!("failed")
    } else {
        json!("observed")
    }
}

fn summarize_events(events: &[IndexedEvent]) -> Value {
    let mut plans = BTreeSet::new();
    let mut tasks = BTreeSet::new();
    let mut attempts = BTreeSet::new();
    let mut gates_total = 0u64;
    let mut gates_passed = 0u64;
    let mut gates_failed = 0u64;
    let mut input_tokens = 0u64;
    let mut output_tokens = 0u64;
    let mut cache_read_tokens = 0u64;
    let mut cache_write_tokens = 0u64;
    let mut total_tokens = 0u64;
    let mut cost_usd = 0.0f64;
    let mut runner_attempt_costs = BTreeMap::<(String, String, u64), f64>::new();
    let mut duration_ms = None;
    let mut started_at = None;
    let mut finished_at = None;
    let mut status = "observed";
    let mut saw_inference_usage = false;
    let mut final_cost = None;

    for item in events {
        let value = &item.value;
        let kind = event_type(value).unwrap_or_default();
        if let Some(plan_id) = event_plan_id(value) {
            plans.insert(plan_id.to_string());
        }
        if let Some(task_id) = event_task_id(value) {
            tasks.insert(task_id.to_string());
            if let Some(attempt) = event_attempt(value) {
                attempts.insert((task_id.to_string(), attempt));
            }
        }
        let timestamp = value
            .get("timestamp")
            .and_then(Value::as_str)
            .or_else(|| value.get("ts").and_then(Value::as_str));
        if kind == "run.started" || kind == "workflow_started" || kind == "run_started" {
            status = "running";
            started_at = timestamp.map(str::to_string).or_else(|| {
                event_timestamp_ms(value).map(|value| value.to_string())
            });
        }
        if matches!(kind, "run.completed" | "workflow_completed" | "run_completed") {
            status = completion_status(value);
            finished_at = timestamp.map(str::to_string).or_else(|| {
                event_timestamp_ms(value).map(|value| value.to_string())
            });
            duration_ms = numeric(value, "duration_ms");
            final_cost = decimal(value, "total_cost_usd").or_else(|| decimal(value, "cost_usd"));
        }
        if is_gate_event(kind) && (kind.contains("completed") || kind.contains("passed") || kind.contains("failed")) {
            gates_total = gates_total.saturating_add(1);
            let passed = bool_field(value, "passed").unwrap_or_else(|| kind.contains("passed"));
            if passed {
                gates_passed = gates_passed.saturating_add(1);
            } else {
                gates_failed = gates_failed.saturating_add(1);
            }
        }
        if kind == "inference_completed" {
            saw_inference_usage = true;
            input_tokens = input_tokens.saturating_add(numeric(value, "input_tokens").unwrap_or(0));
            output_tokens = output_tokens.saturating_add(numeric(value, "output_tokens").unwrap_or(0));
            cost_usd += decimal(value, "cost_usd").unwrap_or(0.0);
        } else if kind == "agent.token_usage" {
            input_tokens = input_tokens.saturating_add(numeric(value, "input_tokens").unwrap_or(0));
            output_tokens = output_tokens.saturating_add(numeric(value, "output_tokens").unwrap_or(0));
            cache_read_tokens = cache_read_tokens
                .saturating_add(numeric(value, "cache_read_tokens").unwrap_or(0));
            cache_write_tokens = cache_write_tokens
                .saturating_add(numeric(value, "cache_write_tokens").unwrap_or(0));
        } else if kind == "agent.turn_completed" {
            if let (Some(plan_id), Some(task_id), Some(attempt), Some(cost)) = (
                event_plan_id(value),
                event_task_id(value),
                event_attempt(value),
                decimal(value, "total_cost_usd"),
            ) {
                runner_attempt_costs
                    .entry((plan_id.to_string(), task_id.to_string(), attempt))
                    .and_modify(|current| *current = current.max(cost))
                    .or_insert(cost);
            }
        } else if !saw_inference_usage && matches!(kind, "agent_completed" | "agent.completed") {
            total_tokens = total_tokens.saturating_add(numeric(value, "tokens_used").unwrap_or(0));
            cost_usd += decimal(value, "cost_usd").unwrap_or(0.0);
        }
    }
    if let Some(final_cost) = final_cost {
        cost_usd = final_cost;
    }
    if saw_inference_usage {
        total_tokens = input_tokens.saturating_add(output_tokens);
    } else {
        if input_tokens != 0 || output_tokens != 0 {
            total_tokens = input_tokens.saturating_add(output_tokens);
        }
        if final_cost.is_none() && !runner_attempt_costs.is_empty() {
            cost_usd = runner_attempt_costs.values().sum();
        }
    }
    json!({
        "status": status,
        "started_at": started_at,
        "finished_at": finished_at,
        "plans": plans,
        "tasks": tasks,
        "attempts": attempts.len(),
        "gates": {
            "total": gates_total,
            "passed": gates_passed,
            "failed": gates_failed,
        },
        "metrics": {
            "events": events.len(),
            "plans": plans.len(),
            "tasks": tasks.len(),
            "attempts": attempts.len(),
            "gates_total": gates_total,
            "gates_passed": gates_passed,
            "gates_failed": gates_failed,
            "input_tokens": input_tokens,
            "output_tokens": output_tokens,
            "cache_read_tokens": cache_read_tokens,
            "cache_write_tokens": cache_write_tokens,
            "total_tokens": total_tokens,
            "cost_usd": cost_usd,
            "duration_ms": duration_ms,
        }
    })
}

fn completion_status(value: &Value) -> &'static str {
    if bool_field(value, "success") == Some(true) {
        return "completed";
    }
    let serialized = serde_json::to_string(event_data(value)).unwrap_or_default().to_ascii_lowercase();
    if serialized.contains("cancel") {
        "cancelled"
    } else if serialized.contains("success") || serialized.contains("succeeded") {
        "completed"
    } else {
        "failed"
    }
}

fn numeric(value: &Value, field: &str) -> Option<u64> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .or_else(|| event_data(value).get(field).and_then(Value::as_u64))
}

fn decimal(value: &Value, field: &str) -> Option<f64> {
    value
        .get(field)
        .and_then(Value::as_f64)
        .or_else(|| event_data(value).get(field).and_then(Value::as_f64))
}

fn bool_field(value: &Value, field: &str) -> Option<bool> {
    value
        .get(field)
        .and_then(Value::as_bool)
        .or_else(|| event_data(value).get(field).and_then(Value::as_bool))
}

fn checkpoint_name(value: &Value) -> Option<String> {
    if event_type(value)? != "state_checkpointed" {
        return None;
    }
    let path = event_data(value).get("path")?.as_str()?;
    FsPath::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
}

fn run_links(run_id: &str) -> Value {
    json!({
        "events": format!("/api/runs/{run_id}/events"),
        "stream": format!("/api/runs/{run_id}/events/stream"),
        "tasks": format!("/api/runs/{run_id}/tasks"),
        "gates": format!("/api/runs/{run_id}/gates"),
        "logs": format!("/api/runs/{run_id}/logs"),
        "metrics": format!("/api/runs/{run_id}/metrics"),
        "artifacts": format!("/api/runs/{run_id}/artifacts"),
        "screenshots": format!("/api/runs/{run_id}/screenshots"),
        "bundle": format!("/api/runs/{run_id}/bundle"),
    })
}

fn find_bundle_dir(state: &AppState, run_id: &str) -> Option<PathBuf> {
    let root = state.layout.root().join("runs");
    if !root.is_dir() {
        return None;
    }
    let direct = root.join(run_id);
    if safe_child_dir(&root, &direct) && manifest_matches_run(&direct, run_id) {
        return Some(direct);
    }
    let entries = std::fs::read_dir(&root).ok()?;
    for entry in entries.take(MAX_BUNDLE_ENTRIES).flatten() {
        if !entry.file_type().ok()?.is_dir() {
            continue;
        }
        let path = entry.path();
        if safe_child_dir(&root, &path) && manifest_matches_run(&path, run_id) {
            return Some(path);
        }
    }
    None
}

fn manifest_matches_run(dir: &FsPath, run_id: &str) -> bool {
    let path = dir.join("manifest.json");
    let Ok(metadata) = std::fs::symlink_metadata(&path) else {
        return dir.file_name().and_then(|name| name.to_str()) == Some(run_id);
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > MAX_BUNDLE_MANIFEST_BYTES {
        return false;
    }
    let Ok(data) = std::fs::read_to_string(path) else {
        return false;
    };
    let Ok(manifest) = serde_json::from_str::<Value>(&data) else {
        return false;
    };
    manifest
        .get("run_id")
        .or_else(|| manifest.get("id"))
        .and_then(Value::as_str)
        == Some(run_id)
}

fn safe_child_dir(root: &FsPath, candidate: &FsPath) -> bool {
    if std::fs::symlink_metadata(candidate)
        .map(|metadata| metadata.file_type().is_symlink() || !metadata.is_dir())
        .unwrap_or(true)
    {
        return false;
    }
    let Ok(root) = std::fs::canonicalize(root) else {
        return false;
    };
    std::fs::canonicalize(candidate).is_ok_and(|path| path.starts_with(root))
}

fn bundle_files(bundle: &FsPath) -> Vec<Value> {
    const ALLOWED: &[&str] = &[
        "manifest.json", "command.txt", "stdout.log", "stderr.log", "events.jsonl",
        "status.jsonl", "commands.jsonl", "usage.jsonl", "endpoints.json", "gates.json",
        "processes.json", "timings.json", "diff.patch", "diff-stat.json", "summary.json",
        "score.json", "DEBRIEF.md",
    ];
    let mut files = ALLOWED
        .iter()
        .filter_map(|name| file_metadata(bundle, &bundle.join(name), name))
        .collect::<Vec<_>>();
    let screenshots = bundle.join("screenshots");
    if safe_child_dir(bundle, &screenshots) {
        files.extend(list_screenshot_files(&screenshots).into_iter().map(|mut value| {
            if let Some(path) = value.get_mut("path")
                && let Some(name) = path.as_str()
            {
                *path = json!(format!("screenshots/{name}"));
            }
            value
        }));
    }
    files.truncate(MAX_BUNDLE_ENTRIES);
    files
}

fn list_screenshot_files(dir: &FsPath) -> Vec<Value> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files = entries
        .take(MAX_SCREENSHOTS + 1)
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_str()?;
            if name.len() > 128
                || !name.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')
                })
            {
                return None;
            }
            let extension = FsPath::new(name).extension().and_then(|value| value.to_str());
            if !matches!(extension, Some("png" | "txt" | "json")) {
                return None;
            }
            file_metadata(dir, &entry.path(), name)
        })
        .collect::<Vec<_>>();
    files.truncate(MAX_SCREENSHOTS);
    files
}

fn file_metadata(root: &FsPath, path: &FsPath, relative: &str) -> Option<Value> {
    let metadata = std::fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return None;
    }
    let canonical_root = std::fs::canonicalize(root).ok()?;
    if !std::fs::canonicalize(path).ok()?.starts_with(canonical_root) {
        return None;
    }
    let modified_ms = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64);
    Some(json!({
        "path": relative,
        "bytes": metadata.len(),
        "modified_ms": modified_ms,
        "content_available": false,
    }))
}

fn read_bounded_json(path: &FsPath, max_bytes: u64, scrubber: &LogScrubber) -> Option<Value> {
    let metadata = std::fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > max_bytes {
        return None;
    }
    let data = std::fs::read_to_string(path).ok()?;
    let mut value = serde_json::from_str::<Value>(&data).ok()?;
    scrub_json_value(&mut value, scrubber);
    Some(value)
}

fn scrub_json_value(value: &mut Value, scrubber: &LogScrubber) {
    match value {
        Value::String(text) => *text = scrubber.scrub(text),
        Value::Array(values) => {
            for value in values {
                scrub_json_value(value, scrubber);
            }
        }
        Value::Object(values) => {
            for value in values.values_mut() {
                scrub_json_value(value, scrubber);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_index(path: &FsPath, run_id: &str) {
        let rows = [
            json!({"type":"run.started","run_id":run_id,"timestamp_ms":1}),
            json!({"type":"task.attempt.started","run_id":run_id,"timestamp_ms":2,"plan_id":"p1","task_id":"t1","attempt":1}),
            json!({"type":"gate.completed","run_id":run_id,"timestamp_ms":3,"plan_id":"p1","task_id":"t1","attempt":1,"passed":true,"rung":0}),
            json!({"type":"run.completed","run_id":run_id,"timestamp_ms":4,"outcome":"succeeded","total_cost_usd":0.1,"duration_ms":10}),
        ];
        let body = rows
            .into_iter()
            .map(|row| serde_json::to_string(&row).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(path, format!("{body}\n")).unwrap();
    }

    #[test]
    fn cursor_pages_are_bounded_and_resume_at_line_boundary() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("run.jsonl");
        write_index(&path, "r1");
        let first = read_index_page(
            &path,
            "r1",
            EventSource::Runner,
            0,
            2,
            MAX_PAGE_SCAN_BYTES,
            &BTreeSet::new(),
            &LogScrubber::new(),
        )
        .unwrap();
        assert_eq!(first.events.len(), 2);
        assert!(first.has_more);
        let second = read_index_page(
            &path,
            "r1",
            EventSource::Runner,
            first.next_cursor,
            2,
            MAX_PAGE_SCAN_BYTES,
            &BTreeSet::new(),
            &LogScrubber::new(),
        )
        .unwrap();
        assert_eq!(second.events.len(), 2);
        assert!(!second.has_more);
        assert!(read_index_page(
            &path,
            "r1",
            EventSource::Runner,
            1,
            2,
            MAX_PAGE_SCAN_BYTES,
            &BTreeSet::new(),
            &LogScrubber::new(),
        )
        .is_err());
    }

    #[test]
    fn mismatched_and_malformed_records_are_quarantined() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("run.jsonl");
        std::fs::write(
            &path,
            "{bad}\n{\"type\":\"run.started\",\"run_id\":\"other\"}\n{\"type\":\"run.started\",\"run_id\":\"r1\",\"event\":{\"run_id\":\"other\"}}\n{\"type\":\"run.started\",\"run_id\":\"r1\"}\n",
        )
        .unwrap();
        let page = read_index_page(
            &path,
            "r1",
            EventSource::Runner,
            0,
            10,
            MAX_PAGE_SCAN_BYTES,
            &BTreeSet::new(),
            &LogScrubber::new(),
        )
        .unwrap();
        assert_eq!(page.events.len(), 1);
        assert_eq!(page.quarantined_records, 3);
    }

    #[test]
    fn partial_tail_does_not_advance_the_external_cursor() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("run.jsonl");
        let complete = "{\"type\":\"run.started\",\"run_id\":\"r1\"}\n";
        std::fs::write(
            &path,
            format!("{complete}{{\"type\":\"run.completed\""),
        )
        .unwrap();
        let page = read_index_page(
            &path,
            "r1",
            EventSource::Runner,
            0,
            10,
            MAX_PAGE_SCAN_BYTES,
            &BTreeSet::new(),
            &LogScrubber::new(),
        )
        .unwrap();
        assert_eq!(page.events.len(), 1);
        assert_eq!(page.next_cursor, complete.len() as u64);
        assert!(page.partial_tail);
        assert!(page.has_more);
    }

    #[test]
    fn filtered_pagination_resumes_after_last_emitted_match() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("run.jsonl");
        std::fs::write(
            &path,
            [
                json!({"type":"gate.completed","run_id":"r1","rung":0}),
                json!({"type":"agent.message","run_id":"r1","text":"between"}),
                json!({"type":"gate.completed","run_id":"r1","rung":1}),
            ]
            .into_iter()
            .map(|value| serde_json::to_string(&value).unwrap())
            .collect::<Vec<_>>()
            .join("\n")
                + "\n",
        )
        .unwrap();
        let first = read_index_page_filtered(
            &path,
            "r1",
            EventSource::Runner,
            0,
            1,
            MAX_PAGE_SCAN_BYTES,
            &BTreeSet::new(),
            &LogScrubber::new(),
            &|value| event_type(value).is_some_and(is_gate_event),
        )
        .unwrap();
        let second = read_index_page_filtered(
            &path,
            "r1",
            EventSource::Runner,
            first.next_cursor,
            1,
            MAX_PAGE_SCAN_BYTES,
            &BTreeSet::new(),
            &LogScrubber::new(),
            &|value| event_type(value).is_some_and(is_gate_event),
        )
        .unwrap();
        assert_eq!(first.events[0].value["rung"], 0);
        assert_eq!(second.events[0].value["rung"], 1);
    }

    #[test]
    fn type_filter_and_secret_scrubbing_apply_before_response() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("run.jsonl");
        std::fs::write(
            &path,
            "{\"type\":\"agent.completed\",\"run_id\":\"r1\",\"message\":\"Bearer abcdefghijklmnopqrstuvwxyz1234\"}\n{\"type\":\"run.completed\",\"run_id\":\"r1\"}\n",
        )
        .unwrap();
        let page = read_index_page(
            &path,
            "r1",
            EventSource::Runner,
            0,
            10,
            MAX_PAGE_SCAN_BYTES,
            &BTreeSet::from(["agent.completed".to_string()]),
            &LogScrubber::new(),
        )
        .unwrap();
        assert_eq!(page.events.len(), 1);
        assert!(page.events[0].value.to_string().contains("[REDACTED]"));
    }

    #[test]
    fn summary_reports_tasks_gates_and_terminal_metrics() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("run.jsonl");
        write_index(&path, "r1");
        let page = read_index_page(
            &path,
            "r1",
            EventSource::Runner,
            0,
            10,
            MAX_PAGE_SCAN_BYTES,
            &BTreeSet::new(),
            &LogScrubber::new(),
        )
        .unwrap();
        let summary = summarize_events(&page.events);
        assert_eq!(summary["status"], "completed");
        assert_eq!(summary["metrics"]["tasks"], 1);
        assert_eq!(summary["metrics"]["gates_passed"], 1);
        assert_eq!(summary["metrics"]["duration_ms"], 10);
    }
}
