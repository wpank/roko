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
use roko_core::agent::resolve_model;
use roko_core::config::schema::RokoConfig;
use roko_core::dashboard_snapshot::{
    TASK_OUTCOME_ACCEPTED_WITH_FAILURES, TASK_OUTCOME_ALREADY_SATISFIED, TASK_OUTCOME_BLOCKED,
    TASK_OUTCOME_CANCELLED, TASK_OUTCOME_PASSED, TASK_OUTCOME_UNVERIFIED, TaskOutcomeClass,
    classify_task_outcome,
};
use roko_core::obs::LogScrubber;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::broadcast;

use crate::error::ApiError;
use crate::state::{AppState, RunState};

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
        .route("/runs/{run_id}/summary", get(get_run_summary))
        .route("/runs/{run_id}/diff", get(get_run_diff))
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

/// `GET /api/runs/{run_id}/diff`'s limits (gap-0d6d21), and which work it
/// shows: the run's verified work (`work=verified`, the default), or the work
/// a plan left that no check ran on (`work=unverified`, bug-db89d4), of plan
/// `plan_id` when given.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct DiffQuery {
    max_files: Option<usize>,
    max_patch_bytes: Option<usize>,
    work: Option<String>,
    plan_id: Option<String>,
}

/// How many files a run's diff lists unless asked, and at most.
const DEFAULT_DIFF_FILES: usize = 200;
const MAX_DIFF_FILES: usize = 500;

/// How many bytes of one file's patch a run's diff returns unless asked, at
/// least, and at most.
const DEFAULT_DIFF_PATCH_BYTES: usize = 65_536;
const MIN_DIFF_PATCH_BYTES: usize = 1_024;
const MAX_DIFF_PATCH_BYTES: usize = 262_144;

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
    if run_state_in_memory(&state, &run_id).await.is_none() {
        settle_if_process_gone(&state, &run_id).await?;
    }
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

/// `GET /api/runs/{run_id}/summary` — what a host can post about a run (B9
/// B4): its state in the words every run route uses, its verdict once it has
/// ended, its cost, how its tasks ended and at most five milestones. Read
/// from the run's own index, with the detail route's scan limits, and from
/// this server's run handles while the run is queued or running. Milestones
/// name event kinds and ids only, never prompts or output.
async fn get_run_summary(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    summarize_run(&state, &run_id).await.map(Json)
}

/// The summary `GET /api/runs/{run_id}/summary` returns for run `run_id`,
/// which the MCP `run_status` tool returns too.
pub(super) async fn summarize_run(state: &Arc<AppState>, run_id: &str) -> Result<Value, ApiError> {
    let (mut summary, plan_ids) = summary_and_plans(state, run_id).await?;
    // gap-8409d0: an ended run's summary says where its verified work is, so a
    // chat host can tell its person what changed and how to take it.
    if summary["active"] == Value::Bool(false) {
        let workdir = state.workdir.clone();
        let io_run_id = run_id.to_string();
        let result = blocking_io(move || {
            Ok(super::run_result::run_result(
                &workdir, &io_run_id, &plan_ids,
            ))
        })
        .await?;
        if let Some(result) = result {
            summary["result"] = result;
        }
    }
    Ok(summary)
}

/// `GET /api/runs/{run_id}/diff` — every change an ended run's verified work
/// makes from the base its batch started at (gap-0d6d21): each file's status,
/// line counts, whether it is binary and its unified patch, within
/// `max_files` and `max_patch_bytes` per file, and totals over every file.
/// With `work=unverified`, the same for the work a plan of the run left that
/// no check ran on, which was never delivered (bug-db89d4). 409 while the run
/// is active; 404 when it has no such work to diff.
async fn get_run_diff(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<String>,
    Query(query): Query<DiffQuery>,
) -> Result<Json<Value>, ApiError> {
    let limits = super::run_result::DiffLimits {
        max_files: bounded_param(
            "max_files",
            query.max_files,
            DEFAULT_DIFF_FILES,
            1,
            MAX_DIFF_FILES,
        )?,
        max_patch_bytes: bounded_param(
            "max_patch_bytes",
            query.max_patch_bytes,
            DEFAULT_DIFF_PATCH_BYTES,
            MIN_DIFF_PATCH_BYTES,
            MAX_DIFF_PATCH_BYTES,
        )?,
    };
    let unverified = match query.work.as_deref() {
        None | Some("verified") => false,
        Some("unverified") => true,
        Some(_) => {
            return Err(ApiError::bad_request("work must be verified or unverified"));
        }
    };
    if let Some(plan_id) = &query.plan_id {
        validate_id(plan_id, "plan id")?;
    }
    let (summary, plan_ids) = summary_and_plans(&state, &run_id).await?;
    if summary["active"] != Value::Bool(false) {
        return Err(ApiError::conflict(format!(
            "run '{run_id}' is still active"
        )));
    }
    let workdir = state.workdir.clone();
    let io_run_id = run_id.clone();
    let diff = blocking_io(move || {
        Ok(if unverified {
            super::run_result::unverified_diff(
                &workdir,
                &io_run_id,
                &plan_ids,
                query.plan_id.as_deref(),
                limits,
            )
        } else {
            super::run_result::run_diff(&workdir, &io_run_id, &plan_ids, limits)
        })
    })
    .await?;
    diff.map(Json).ok_or_else(|| {
        ApiError::not_found(if unverified {
            format!("run '{run_id}' left no unverified work to diff")
        } else {
            format!("run '{run_id}' has no result to diff")
        })
    })
}

/// `value`, or `default` when it is not given, refused when it falls outside
/// `least..=most`.
fn bounded_param(
    name: &str,
    value: Option<usize>,
    default: usize,
    least: usize,
    most: usize,
) -> Result<usize, ApiError> {
    let value = value.unwrap_or(default);
    if !(least..=most).contains(&value) {
        return Err(ApiError::bad_request(format!(
            "{name} must be between {least} and {most}"
        )));
    }
    Ok(value)
}

/// Run `run_id`'s summary without its `result`, and the plans its events
/// name, whose checkpoints may record where its batch started.
async fn summary_and_plans(
    state: &Arc<AppState>,
    run_id: &str,
) -> Result<(Value, Vec<String>), ApiError> {
    ensure_observability_allowed(state)?;
    validate_id(run_id, "run id")?;
    let in_memory = run_state_in_memory(state, run_id).await;
    if in_memory.is_none() {
        settle_if_process_gone(state, run_id).await?;
    }
    let io_state = Arc::clone(state);
    let io_run_id = run_id.to_string();
    let page = blocking_io(move || {
        read_for_run(
            &io_state,
            &io_run_id,
            None,
            0,
            MAX_DETAIL_EVENTS,
            MAX_DETAIL_SCAN_BYTES,
            &BTreeSet::new(),
        )
    })
    .await?;
    if page.is_none() && in_memory.is_none() {
        return Err(ApiError::not_found(format!("run '{run_id}' not found")));
    }
    let events = page.map(|page| page.events).unwrap_or_default();
    let config = state.load_roko_config();
    let summary = run_summary(run_id, &events, in_memory, &config);
    let mut plan_ids = Vec::<String>::new();
    for item in &events {
        if let Some(plan_id) = event_plan_id(&item.value).filter(|id| !id.is_empty())
            && !plan_ids.iter().any(|known| known == plan_id)
        {
            plan_ids.push(plan_id.to_string());
        }
    }
    Ok((summary, plan_ids))
}

/// What this server holds of run `run_id` in memory: the state of its
/// one-shot or plan-run handle, or `queued` with its place in the run queue,
/// which plan and prompt runs share (gap-9a8d66).
async fn run_state_in_memory(state: &AppState, run_id: &str) -> Option<(RunState, Option<usize>)> {
    if let Some(run) = state.active_runs.read().await.get(run_id) {
        return Some((super::run::run_handle_state(run).0, None));
    }
    {
        let plans = state.active_plans.read().await;
        if let Some(plan) = plans.values().find(|plan| plan.id == run_id) {
            return Some((plan.state(), None));
        }
    }
    state
        .queue_position(run_id)
        .map(|position| (RunState::Queued, Some(position)))
}

/// End run `run_id` as `interrupted` when the roko process its manifest names
/// is gone (bug-dceafd), so that its index records the end before it is read.
/// For a run this server holds no handle of: a handle is the run's own word.
async fn settle_if_process_gone(state: &AppState, run_id: &str) -> Result<(), ApiError> {
    let workdir = state.workdir.clone();
    let run_id = run_id.to_string();
    blocking_io(move || Ok(crate::interrupted_runs::settle(&workdir, &run_id))).await?;
    Ok(())
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

    // A Graph run (every plan run, and a prompt run, which is a one-task plan)
    // writes the runner index and publishes to the StateHub, never to the runtime
    // bus this stream used to follow alone, so its stream sent only keepalives
    // (bug-1f41f0). Follow the runner index unless the run has a runtime index.
    let has_runtime_index = {
        let io_state = Arc::clone(&state);
        let io_run_id = run_id.clone();
        blocking_io(move || {
            Ok(source_path(&io_state, &io_run_id, EventSource::Runtime)
                .is_some_and(|path| roko_fs::run_index::open_existing_run_index(&path).is_ok()))
        })
        .await?
    };
    if !has_runtime_index {
        let types = parse_types(query.types.as_deref())?;
        let header_cursor = headers
            .get("Last-Event-ID")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        let cursor = header_cursor.or(query.cursor).unwrap_or(0);
        return Ok(runner_index_sse(state, run_id, cursor, types));
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
                        return Some((
                            Ok::<_, Infallible>(frame),
                            (rx, state, run_id, cursor, types),
                        ));
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
                        return Some((
                            Ok::<_, Infallible>(frame),
                            (rx, state, run_id, cursor, types),
                        ));
                    }
                    Err(broadcast::error::RecvError::Closed) => return None,
                }
            }
        },
    );

    let sse = Sse::new(super::sse::until_shutdown(
        stream::iter(replay).chain(live),
        state.cancel.clone(),
    ))
    .keep_alive(
        KeepAlive::new()
            .interval(std::time::Duration::from_secs(8))
            .text("keepalive"),
    );
    Ok((super::sse::sse_response_headers(), sse).into_response())
}

/// How often [`runner_index_sse`] looks for rows appended to a run's index.
const RUNNER_INDEX_POLL: std::time::Duration = std::time::Duration::from_millis(500);

/// The per-run stream of a Graph run (bug-1f41f0): the rows of its runner index
/// from `cursor` on, then each row appended later, as SSE events whose `id` is
/// the byte cursor after the row (so `Last-Event-ID` resumes), filtered by
/// `types`. It ends after the run's `run_completed`, or when the server shuts
/// down; a cursor the index refuses ends it with an `error` event.
///
/// The runner flushes lifecycle rows as they happen and the rest at most a
/// second apart (bug-0b997a), so the stream can be a second behind.
fn runner_index_sse(
    state: Arc<AppState>,
    run_id: String,
    cursor: u64,
    types: BTreeSet<String>,
) -> Response {
    struct Follow {
        state: Arc<AppState>,
        run_id: String,
        cursor: u64,
        types: BTreeSet<String>,
        pending: std::collections::VecDeque<Event>,
        ended: bool,
    }
    let follow = Follow {
        state: Arc::clone(&state),
        run_id,
        cursor,
        types,
        pending: std::collections::VecDeque::new(),
        ended: false,
    };
    let rows = stream::unfold(follow, |mut follow| async move {
        loop {
            if let Some(frame) = follow.pending.pop_front() {
                return Some((Ok::<_, Infallible>(frame), follow));
            }
            if follow.ended {
                return None;
            }
            let io_state = Arc::clone(&follow.state);
            let io_run_id = follow.run_id.clone();
            let cursor = follow.cursor;
            // Read every row, whatever `types` asks for, so the run's end is seen.
            let page = blocking_io(move || {
                read_for_run(
                    &io_state,
                    &io_run_id,
                    Some(EventSource::Runner),
                    cursor,
                    MAX_SSE_REPLAY,
                    MAX_PAGE_SCAN_BYTES,
                    &BTreeSet::new(),
                )
            })
            .await;
            match page {
                Ok(Some(page)) => {
                    let has_more = page.has_more;
                    follow.cursor = page.next_cursor;
                    for item in page.events {
                        let kind = event_type(&item.value).unwrap_or_default().to_string();
                        if kind == "run_completed" {
                            follow.ended = true;
                        }
                        if follow.types.is_empty() || follow.types.contains(&kind) {
                            follow
                                .pending
                                .push_back(indexed_sse_event(item, &follow.state.scrubber));
                        }
                    }
                    if follow.ended || has_more || !follow.pending.is_empty() {
                        continue;
                    }
                }
                // No index yet: the run is queued or has only just started.
                Ok(None) => {}
                Err(error) => {
                    follow.ended = true;
                    let data = json!({ "run_id": follow.run_id, "message": error.to_string() });
                    follow
                        .pending
                        .push_back(Event::default().event("error").data(data.to_string()));
                    continue;
                }
            }
            tokio::time::sleep(RUNNER_INDEX_POLL).await;
        }
    });
    let sse = Sse::new(super::sse::until_shutdown(rows, state.cancel.clone())).keep_alive(
        KeepAlive::new()
            .interval(std::time::Duration::from_secs(8))
            .text("keepalive"),
    );
    (super::sse::sse_response_headers(), sse).into_response()
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
            let mut attempts = entry["attempts"].as_array().cloned().unwrap_or_default();
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
                !wanted_level
                    .as_deref()
                    .is_some_and(|wanted| wanted != level)
            },
        )
    })
    .await?
    .ok_or_else(|| ApiError::not_found(format!("run '{run_id}' has no event index")))?;
    let mut logs = Vec::new();
    for item in &page.events {
        let level = event_level(&item.value);
        let serialized = serde_json::to_string(&item.value).unwrap_or_default();
        let message = serialized
            .chars()
            .take(MAX_LOG_PREVIEW_CHARS)
            .collect::<String>();
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
        return Err(ApiError::not_found(format!(
            "run '{run_id}' has no artifacts"
        )));
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
    if state.run_observability_allowed() {
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
    for item in raw
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
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
        EventSource::Runner => {
            roko_fs::run_index::run_index_path(&state.layout.events_jsonl_path(), run_id).ok()
        }
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
            let budget = MAX_DASHBOARD_SCAN_BYTES_PER_RUN
                .min(MAX_DASHBOARD_TOTAL_SCAN_BYTES.saturating_sub(scanned_bytes));
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
            // Reserve the whole bounded read up front. A malformed page may
            // return no detailed byte count, but it must still consume the
            // aggregate request budget rather than enabling another read.
            scanned_bytes = scanned_bytes.saturating_add(page_budget);
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
            scanned_bytes = scanned_bytes
                .saturating_sub(page_budget)
                .saturating_add(page.scanned_bytes);
            let aggregate = summarize_events(&page.events);
            let template = page.events.iter().find_map(|item| {
                (event_type(&item.value) == Some("workflow_started"))
                    .then(|| event_data(&item.value).get("template").cloned())
                    .flatten()
            });
            let prompt = page.events.iter().find_map(|item| {
                matches!(
                    event_type(&item.value),
                    Some("workflow_started" | "run_started")
                )
                .then(|| event_data(&item.value).get("prompt").cloned())
                .flatten()
            });
            let mut status = aggregate["status"].clone();
            let complete = |status: &Value| {
                matches!(
                    status.as_str(),
                    Some("completed" | "failed" | "cancelled" | "interrupted")
                )
            };
            // A run whose roko process is gone ends interrupted (bug-dceafd).
            if !complete(&status)
                && crate::interrupted_runs::settle(&state.workdir, &run_id).is_some()
            {
                status = json!(crate::interrupted_runs::INTERRUPTED);
            }
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
                    "is_complete": complete(&status),
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
            .take(
                max_bytes
                    .saturating_sub(scanned)
                    .min((MAX_LINE_BYTES + 1) as u64),
            )
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
        if !types.is_empty() && !event_type(&value).is_some_and(|kind| types.contains(kind)) {
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
        .is_some_and(|value| {
            value
                .split(',')
                .any(|part| part.trim() == "text/event-stream")
        })
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
        // Graph runs' index rows carry the hub's publish time (bug-28aae1).
        .or_else(|| value.get("ts_millis").and_then(Value::as_u64))
}

/// When an indexed event happened, as text: its timestamp, else its time in
/// milliseconds as RFC 3339 (UTC, milliseconds).
fn event_time_text(value: &Value) -> Option<String> {
    value
        .get("timestamp")
        .and_then(Value::as_str)
        .or_else(|| value.get("ts").and_then(Value::as_str))
        .map(str::to_string)
        .or_else(|| {
            let millis = i64::try_from(event_timestamp_ms(value)?).ok()?;
            let at = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(millis)?;
            Some(at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        })
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
    matches!(
        kind,
        "task.attempt.completed" | "task_completed" | "task_failed" | "task_skipped"
    )
}

/// A task's status from one of its records. A dashboard `task_completed`
/// names the outcome its task settled with and reports that outcome's class
/// (bug-54c729), so a task that completed unverified, was skipped or found
/// its work already there is not reported as failed. A runner attempt record
/// passes only with a `passed` outcome.
fn terminal_status(value: &Value) -> Value {
    let kind = event_type(value).unwrap_or_default();
    if kind.contains("cancel") {
        return json!("cancelled");
    }
    if kind == "task_skipped" {
        return json!("skipped");
    }
    let data = event_data(value);
    if kind == "task_completed"
        && let Some(outcome) = value
            .get("outcome")
            .or_else(|| data.get("outcome"))
            .and_then(Value::as_str)
    {
        return json!(outcome_status(outcome));
    }
    if data.get("passed").and_then(Value::as_bool) == Some(true)
        || data.get("success").and_then(Value::as_bool) == Some(true)
        || value.get("outcome").and_then(Value::as_str) == Some(TASK_OUTCOME_PASSED)
    {
        json!("passed")
    } else if is_task_terminal(kind) {
        json!("failed")
    } else {
        json!("observed")
    }
}

/// The status a dashboard task outcome reports: its class, named as the
/// dashboard's outcomes name it.
fn outcome_status(outcome: &str) -> &'static str {
    match classify_task_outcome(outcome) {
        TaskOutcomeClass::Passed => TASK_OUTCOME_PASSED,
        TaskOutcomeClass::Failed => "failed",
        TaskOutcomeClass::AcceptedWithFailures => TASK_OUTCOME_ACCEPTED_WITH_FAILURES,
        TaskOutcomeClass::AlreadySatisfied => TASK_OUTCOME_ALREADY_SATISFIED,
        TaskOutcomeClass::Unverified => TASK_OUTCOME_UNVERIFIED,
        TaskOutcomeClass::Skipped => "skipped",
        TaskOutcomeClass::Blocked => TASK_OUTCOME_BLOCKED,
        TaskOutcomeClass::Cancelled => TASK_OUTCOME_CANCELLED,
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
    // What Graph runs record: `efficiency_event` rows, one metric each (bug-28aae1).
    let mut efficiency = Efficiency::default();
    // The first time any row carries: a Graph run has no start event of its own.
    let mut first_time = None;

    for item in events {
        let value = &item.value;
        let kind = event_type(value).unwrap_or_default();
        // Log entries carry empty ids; they name no plan or task.
        if let Some(plan_id) = event_plan_id(value).filter(|id| !id.is_empty()) {
            plans.insert(plan_id.to_string());
        }
        if let Some(task_id) = event_task_id(value).filter(|id| !id.is_empty()) {
            tasks.insert(task_id.to_string());
            if let Some(attempt) = event_attempt(value) {
                attempts.insert((task_id.to_string(), attempt));
            }
        }
        let time = event_time_text(value);
        if first_time.is_none() {
            first_time.clone_from(&time);
        }
        if kind == "run.started" || kind == "workflow_started" || kind == "run_started" {
            status = "running";
            started_at.clone_from(&time);
        }
        if matches!(
            kind,
            "run.completed" | "workflow_completed" | "run_completed"
        ) {
            status = completion_status(value);
            finished_at.clone_from(&time);
            duration_ms = numeric(value, "duration_ms");
            final_cost = decimal(value, "total_cost_usd").or_else(|| decimal(value, "cost_usd"));
        }
        if kind == "efficiency_event" {
            efficiency.add(value);
        }
        // Graph runs report a rung's verdict as `gate_result` (bug-28aae1).
        if kind == "gate_result"
            || (is_gate_event(kind)
                && (kind.contains("completed")
                    || kind.contains("passed")
                    || kind.contains("failed")))
        {
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
            output_tokens =
                output_tokens.saturating_add(numeric(value, "output_tokens").unwrap_or(0));
            cost_usd += decimal(value, "cost_usd").unwrap_or(0.0);
        } else if kind == "agent.token_usage" {
            input_tokens = input_tokens.saturating_add(numeric(value, "input_tokens").unwrap_or(0));
            output_tokens =
                output_tokens.saturating_add(numeric(value, "output_tokens").unwrap_or(0));
            cache_read_tokens =
                cache_read_tokens.saturating_add(numeric(value, "cache_read_tokens").unwrap_or(0));
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
    // A Graph run records its spend only as efficiency events: use them when no
    // other source reported any.
    if efficiency.seen
        && !saw_inference_usage
        && final_cost.is_none()
        && runner_attempt_costs.is_empty()
        && cost_usd.abs() < f64::EPSILON
    {
        cost_usd = efficiency.cost_usd;
    }
    if efficiency.seen && input_tokens == 0 && output_tokens == 0 {
        input_tokens = efficiency.input_tokens;
        output_tokens = efficiency.output_tokens;
        cache_read_tokens = cache_read_tokens.max(efficiency.cache_read_tokens);
        cache_write_tokens = cache_write_tokens.max(efficiency.cache_write_tokens);
        total_tokens = input_tokens.saturating_add(output_tokens);
    }
    if started_at.is_none() {
        started_at = first_time;
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

/// A Graph run's `efficiency_event` rows, summed: each carries one `metric` and
/// its `value`.
#[derive(Debug, Default)]
struct Efficiency {
    seen: bool,
    cost_usd: f64,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_write_tokens: u64,
}

impl Efficiency {
    fn add(&mut self, value: &Value) {
        let data = event_data(value);
        let metric = data
            .get("metric")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let Some(amount) = data
            .get("value")
            .and_then(Value::as_f64)
            .filter(|amount| amount.is_finite() && *amount >= 0.0)
        else {
            return;
        };
        self.seen = true;
        // Token counts are whole numbers recorded as floats.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let tokens = amount.round() as u64;
        match metric {
            "cost_usd" => self.cost_usd += amount,
            "input_tokens" => self.input_tokens = self.input_tokens.saturating_add(tokens),
            "output_tokens" => self.output_tokens = self.output_tokens.saturating_add(tokens),
            "cache_read_tokens" => {
                self.cache_read_tokens = self.cache_read_tokens.saturating_add(tokens);
            }
            "cache_write_tokens" => {
                self.cache_write_tokens = self.cache_write_tokens.saturating_add(tokens);
            }
            _ => {}
        }
    }
}

fn completion_status(value: &Value) -> &'static str {
    if bool_field(value, "success") == Some(true) {
        return "completed";
    }
    match string_field(value, "outcome") {
        // The end serve records for a run whose roko process was gone
        // (bug-dceafd).
        Some(crate::interrupted_runs::INTERRUPTED) => return crate::interrupted_runs::INTERRUPTED,
        // A run roko refused to start ended, and not as a success (bug-9a5970),
        // whatever words its reason uses.
        Some(crate::run_end::REFUSED) => return "failed",
        _ => {}
    }
    let serialized = serde_json::to_string(event_data(value))
        .unwrap_or_default()
        .to_ascii_lowercase();
    if serialized.contains("cancel") {
        "cancelled"
    } else if serialized.contains("success") || serialized.contains("succeeded") {
        "completed"
    } else {
        "failed"
    }
}

/// The summary of run `run_id` from its indexed `events` and what this
/// server holds of it in memory ([`run_state_in_memory`]).
///
/// A queued or running run's handle is ahead of its index. Once the index
/// records the run's end, its state is [`RunState::of_ended_run`] over the
/// end and the last outcome of each task, the words `GET
/// /api/plans/{id}/status` uses too. A run with neither is still running, as
/// a run another process writes the index of is, until serve finds that
/// process gone and records the run's end as `interrupted`
/// ([`crate::interrupted_runs`]); the summary then says why under
/// `interrupted`. A run roko refused to run, before its plan started or at
/// the spec gate before any dispatch, ends `failed` and says why under
/// `refused` (bug-9a5970).
///
/// Each task the run dispatched an agent for also gets its untrusted-input
/// boundary under `config` ([`untrusted_input_boundary`]).
fn run_summary(
    run_id: &str,
    events: &[IndexedEvent],
    in_memory: Option<(RunState, Option<usize>)>,
    config: &RokoConfig,
) -> Value {
    let mut completion = None;
    // What the run's plans are called (gap-c2efe7), from the plan set it loaded.
    let mut titles = Vec::<String>::new();
    let mut tasks = BTreeMap::<(String, String), TaskOutcomeClass>::new();
    // The models each task's agents were dispatched to.
    let mut task_models = BTreeMap::<(String, String), BTreeSet<String>>::new();
    // The first milestone of each kind, with its event's place in the run.
    let mut milestones = BTreeMap::<&str, (usize, Value)>::new();
    // What the spec gate refused before any dispatch, `<task>: <rule>: <detail>`.
    let mut gate_refusals = Vec::<String>::new();
    for (place, item) in events.iter().enumerate() {
        let value = &item.value;
        let kind = event_type(value).unwrap_or_default();
        if kind == "event_log_entry"
            && string_field(value, "event_type") == Some("spec.gate.refused")
        {
            let message = string_field(value, "message").unwrap_or_default();
            let line = event_task_id(value)
                .filter(|task_id| !task_id.is_empty())
                .map_or_else(
                    || message.to_string(),
                    |task_id| format!("{task_id}: {message}"),
                );
            if !line.is_empty() && !gate_refusals.contains(&line) {
                gate_refusals.push(line);
            }
        }
        if matches!(kind, "agent_spawned" | "agent.dispatch.started")
            && let Some(task_id) = event_task_id(value)
            && let Some(model) = string_field(value, "model")
                .or_else(|| string_field(value, "requested_model"))
                .filter(|model| !model.is_empty())
        {
            let plan_id = event_plan_id(value).unwrap_or_default().to_string();
            task_models
                .entry((plan_id, task_id.to_string()))
                .or_default()
                .insert(model.to_string());
        }
        if kind == "plan_set_loaded" && titles.is_empty() {
            titles = value["plans"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|plan| plan["title"].as_str())
                .map(str::trim)
                .filter(|title| !title.is_empty())
                .map(str::to_string)
                .collect();
        }
        if matches!(
            kind,
            "run.started" | "run_started" | "workflow_started" | "plan_started" | "plan.started"
        ) {
            let text = event_plan_id(value).map_or_else(
                || "run started".to_string(),
                |plan_id| format!("plan {plan_id} started"),
            );
            milestones
                .entry("started")
                .or_insert_with(|| (place, milestone("started", text, value)));
        }
        if is_task_terminal(kind)
            && let Some(task_id) = event_task_id(value)
        {
            let status = terminal_status(value);
            let class = classify_task_outcome(status.as_str().unwrap_or_default());
            let plan_id = event_plan_id(value).unwrap_or_default().to_string();
            tasks.insert((plan_id, task_id.to_string()), class);
            if class == TaskOutcomeClass::Passed {
                let text = format!("task {task_id} passed");
                milestones
                    .entry("first_task_accepted")
                    .or_insert_with(|| (place, milestone("first_task_accepted", text, value)));
            }
        }
        if is_gate_event(kind)
            && bool_field(value, "passed").map_or(kind.contains("failed"), |passed| !passed)
        {
            let gate = string_field(value, "gate")
                .map_or_else(|| "a gate".to_string(), |gate| format!("gate {gate}"));
            let text = event_task_id(value).map_or_else(
                || format!("{gate} failed"),
                |task_id| format!("{gate} failed on task {task_id}"),
            );
            milestones
                .entry("first_gate_failure")
                .or_insert_with(|| (place, milestone("first_gate_failure", text, value)));
        }
        if matches!(
            kind,
            "run.completed" | "workflow_completed" | "run_completed"
        ) {
            completion = Some((place, value));
        }
        // The first delivery that did not complete says how the run's
        // delivery ended, else the first that did (gap-4f6cbe).
        if let Some(delivery) = delivery_end(value)
            && milestones.get("delivery").is_none_or(|(_, held)| {
                delivery != "completed" && held["text"] == "delivery completed"
            })
        {
            let text = format!("delivery {delivery}");
            milestones.insert("delivery", (place, milestone("delivery", text, value)));
        }
    }

    let ended = completion.map(|(_, value)| match completion_status(value) {
        crate::interrupted_runs::INTERRUPTED => RunState::Interrupted,
        end => RunState::of_ended_run(
            end == "cancelled",
            end == "completed",
            tasks.values().copied(),
        ),
    });
    let (state, position) = match in_memory {
        Some((live, position)) if !live.is_terminal() => (live, position),
        remembered => (
            ended
                .or(remembered.map(|(state, _)| state))
                .unwrap_or(RunState::Running),
            None,
        ),
    };
    if let Some((place, value)) = completion {
        let text = format!("run {}", state.as_str());
        milestones.insert("completed", (place, milestone("completed", text, value)));
    }
    // Why a run whose roko process stopped ended, and its last sign of life
    // (bug-dceafd).
    let interrupted = completion
        .filter(|_| state == RunState::Interrupted)
        .map(|(_, value)| {
            json!({
                "reason": string_field(value, "reason")
                    .unwrap_or(crate::interrupted_runs::INTERRUPTED_REASON),
                "at": event_time_text(value),
            })
        });
    // Why roko did not run it (bug-9a5970): refused before its plan started,
    // or by the spec gate before any dispatch.
    let refused = completion
        .filter(|_| state == RunState::Failed)
        .and_then(|(_, value)| {
            if string_field(value, "outcome") == Some(crate::run_end::REFUSED) {
                return Some(json!({
                    "stage": string_field(value, "stage").unwrap_or(crate::run_end::START_STAGE),
                    "reason": string_field(value, "reason").unwrap_or_default(),
                }));
            }
            (!gate_refusals.is_empty() && tasks.is_empty()).then(|| {
                json!({
                    "stage": crate::run_end::SPEC_GATE_STAGE,
                    "reason": gate_refusals.join("; "),
                })
            })
        });
    let mut milestones = milestones.into_values().collect::<Vec<_>>();
    milestones.sort_by_key(|(place, _)| *place);
    let milestones = milestones
        .into_iter()
        .map(|(_, entry)| entry)
        .collect::<Vec<_>>();

    let count = |class: TaskOutcomeClass| tasks.values().filter(|&&each| each == class).count();
    let (passed, failed, unverified, cancelled) = (
        count(TaskOutcomeClass::Passed),
        count(TaskOutcomeClass::Failed),
        count(TaskOutcomeClass::Unverified),
        count(TaskOutcomeClass::Cancelled),
    );
    let boundaries = task_models
        .iter()
        .map(|((plan_id, task_id), models)| {
            json!({
                "plan_id": plan_id,
                "task_id": task_id,
                "boundary": untrusted_input_boundary(config, models),
            })
        })
        .collect::<Vec<_>>();
    let summary = summarize_events(events);
    json!({
        "run_id": run_id,
        "title": (!titles.is_empty()).then(|| titles.join("; ")),
        "state": state.as_str(),
        "verdict": state.is_terminal().then_some(state),
        "position": position,
        "active": !state.is_terminal(),
        "interrupted": interrupted,
        "refused": refused,
        "cost_usd": summary["metrics"]["cost_usd"],
        "tasks": {
            "total": tasks.len(),
            "passed": passed,
            "failed": failed,
            "unverified": unverified,
            // A task a cancel cut off is not a failure (bug-1d09e8).
            "cancelled": cancelled,
            "other": tasks.len() - passed - failed - unverified - cancelled,
        },
        "untrusted_input_boundary": boundaries,
        "milestones": milestones,
        "started_at": summary["started_at"],
        "finished_at": summary["finished_at"],
        "links": run_links(run_id),
    })
}

/// What screens the text a task's agents read, such as a chat host's request
/// or a fetched page, before it reaches their model (9117): `data_llm` when
/// `[agent.data_llm]` is set and every one of the task's `models` runs roko's
/// own tool loop, where the boundary screens tool results (gap-b0d514);
/// `none` when it is unset or a model runs a CLI or ACP agent with its own
/// tool loop, which the boundary cannot reach.
fn untrusted_input_boundary(config: &RokoConfig, models: &BTreeSet<String>) -> &'static str {
    let screened = config.agent.data_llm.is_some()
        && models.iter().all(|model| {
            let kind = resolve_model(config, model).provider_kind;
            roko_agent::adapter_for_kind(kind).supports_local_tool_runtime()
        });
    if screened { "data_llm" } else { "none" }
}

/// A summary milestone: its kind, a line built from event kinds and ids, and
/// when its event happened.
fn milestone(kind: &str, text: String, value: &Value) -> Value {
    json!({
        "kind": kind,
        "text": text,
        "at": event_time(value),
    })
}

/// When an indexed event happened: its timestamp, else its time in
/// milliseconds.
fn event_time(value: &Value) -> Value {
    value
        .get("timestamp")
        .or_else(|| value.get("ts"))
        .filter(|time| time.is_string())
        .cloned()
        .or_else(|| event_timestamp_ms(value).map(Value::from))
        .unwrap_or(Value::Null)
}

/// How a plan's delivery ended, as the run's `delivery` milestone says it,
/// when `value` is the event that says so: a Graph run's `delivery_finished`
/// (gap-4f6cbe), or a `graph.delivery` `completed` or `failed` runtime event.
/// A delivery a stop cut short was `cancelled`: it stopped, it did not fail.
fn delivery_end(value: &Value) -> Option<&'static str> {
    if event_type(value) == Some("delivery_finished") {
        return Some(match string_field(value, "state")? {
            "delivered" => "completed",
            "cancelled" => "cancelled",
            "conflict" => "failed: its merge conflicted",
            "regression_failed" => "failed: the check on its merge failed",
            _ => "failed",
        });
    }
    let data = event_data(value);
    if data.get("namespace").and_then(Value::as_str) != Some("graph.delivery") {
        return None;
    }
    let end = data.get("value")?;
    match end.get("kind")?.as_str()? {
        "completed" => Some("completed"),
        "failed" if end.get("failure_state").and_then(Value::as_str) == Some("cancelled") => {
            Some("cancelled")
        }
        "failed" => Some("failed"),
        _ => None,
    }
}

fn string_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .or_else(|| event_data(value).get(field).and_then(Value::as_str))
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
        "detail": format!("/api/runs/{run_id}"),
        "summary": format!("/api/runs/{run_id}/summary"),
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
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > MAX_BUNDLE_MANIFEST_BYTES
    {
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
        "manifest.json",
        "command.txt",
        "stdout.log",
        "stderr.log",
        "events.jsonl",
        "status.jsonl",
        "commands.jsonl",
        "usage.jsonl",
        "endpoints.json",
        "gates.json",
        "processes.json",
        "timings.json",
        "diff.patch",
        "diff-stat.json",
        "summary.json",
        "score.json",
        "DEBRIEF.md",
    ];
    let mut files = ALLOWED
        .iter()
        .filter_map(|name| file_metadata(bundle, &bundle.join(name), name))
        .collect::<Vec<_>>();
    let screenshots = bundle.join("screenshots");
    if safe_child_dir(bundle, &screenshots) {
        files.extend(
            list_screenshot_files(&screenshots)
                .into_iter()
                .map(|mut value| {
                    if let Some(path) = value.get_mut("path")
                        && let Some(name) = path.as_str()
                    {
                        *path = json!(format!("screenshots/{name}"));
                    }
                    value
                }),
        );
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
                || !name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
            {
                return None;
            }
            let extension = FsPath::new(name)
                .extension()
                .and_then(|value| value.to_str());
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
    if !std::fs::canonicalize(path)
        .ok()?
        .starts_with(canonical_root)
    {
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
        assert!(
            read_index_page(
                &path,
                "r1",
                EventSource::Runner,
                1,
                2,
                MAX_PAGE_SCAN_BYTES,
                &BTreeSet::new(),
                &LogScrubber::new(),
            )
            .is_err()
        );
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
        std::fs::write(&path, format!("{complete}{{\"type\":\"run.completed\"")).unwrap();
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

    /// bug-28aae1: a Graph run records its spend as `efficiency_event` rows,
    /// its gate verdicts as `gate_result`, and its times only as each row's
    /// `ts_millis`; log entries carry empty plan and task ids. The summary
    /// reads all of it.
    #[tokio::test]
    async fn summary_counts_cost_tokens_gates_and_times_of_graph_runs() {
        let dir = tempfile::tempdir().unwrap();
        let deploy_backend = Arc::from(
            crate::deploy::create_backend("manual", None, None, None).expect("manual backend"),
        );
        let state = Arc::new(
            AppState::new(
                dir.path().to_path_buf(),
                Arc::new(crate::runtime::NoOpRuntime),
                roko_core::config::schema::RokoConfig::default(),
                deploy_backend,
            )
            .expect("AppState::new"),
        );
        let index =
            roko_fs::run_index::run_index_path(&state.layout.events_jsonl_path(), "g1").unwrap();
        let efficiency = |metric: &str, value: f64| json!({"type":"efficiency_event","run_id":"g1","plan_id":"p1","task_id":"T1","metric":metric,"value":value,"ts_millis":1_791_550_840_000_u64});
        write_rows(
            &index,
            &[
                json!({"type":"plan_set_loaded","run_id":"g1","plans":[{"plan_id":"p1","tasks_total":1}],"ts_millis":1_791_550_809_314_u64}),
                json!({"type":"event_log_entry","run_id":"g1","plan_id":"","task_id":"","event_type":"spec.gate","message":"allow","timestamp_ms":1_791_550_809_333_u64}),
                json!({"type":"plan_started","run_id":"g1","plan_id":"p1","tasks_total":1,"ts_millis":1_791_550_809_360_u64}),
                json!({"type":"task_started","run_id":"g1","plan_id":"p1","task_id":"T1","title":"t","phase":"x","ts_millis":1_791_550_809_400_u64}),
                json!({"type":"gate_result","run_id":"g1","plan_id":"p1","task_id":"T1","gate":"verify[0:test]","passed":true,"output_text":"$ cargo test","ts_millis":1_791_550_850_000_u64}),
                efficiency("input_tokens", 3352.0),
                efficiency("output_tokens", 655.0),
                efficiency("cache_read_tokens", 10368.0),
                efficiency("cache_write_tokens", 0.0),
                efficiency("cost_usd", 0.010_270_480_066_537_857),
                json!({"type":"task_completed","run_id":"g1","plan_id":"p1","task_id":"T1","outcome":"passed","ts_millis":1_791_550_850_100_u64}),
                json!({"type":"plan_completed","run_id":"g1","plan_id":"p1","success":true,"ts_millis":1_791_550_850_200_u64}),
                json!({"type":"run_completed","run_id":"g1","outcome":"succeeded","duration_ms":42210,"ts_millis":1_791_550_850_893_u64}),
            ],
        );

        let summary = summarize_events(
            &read_index_page(
                &index,
                "g1",
                EventSource::Runner,
                0,
                100,
                MAX_PAGE_SCAN_BYTES,
                &BTreeSet::new(),
                &LogScrubber::new(),
            )
            .unwrap()
            .events,
        );
        assert_eq!(summary["plans"], json!(["p1"]), "{summary}");
        assert_eq!(summary["tasks"], json!(["T1"]), "{summary}");
        assert_eq!(
            summary["gates"],
            json!({"total": 1, "passed": 1, "failed": 0})
        );
        let metrics = &summary["metrics"];
        assert_eq!(metrics["input_tokens"], 3352, "{summary}");
        assert_eq!(metrics["output_tokens"], 655, "{summary}");
        assert_eq!(metrics["cache_read_tokens"], 10368, "{summary}");
        assert_eq!(metrics["total_tokens"], 4007, "{summary}");
        assert!((metrics["cost_usd"].as_f64().unwrap() - 0.010_270_48).abs() < 1e-9);
        assert_eq!(
            summary["started_at"], "2026-10-09T13:00:09.314Z",
            "{summary}"
        );
        assert_eq!(
            summary["finished_at"], "2026-10-09T13:00:50.893Z",
            "{summary}"
        );

        let Json(run) = get_run_summary(State(Arc::clone(&state)), Path("g1".into()))
            .await
            .unwrap();
        assert_eq!(run["state"], "succeeded", "{run}");
        assert!(
            (run["cost_usd"].as_f64().unwrap() - 0.010_270_48).abs() < 1e-9,
            "{run}"
        );
        assert_eq!(run["started_at"], "2026-10-09T13:00:09.314Z", "{run}");
        assert_eq!(run["finished_at"], "2026-10-09T13:00:50.893Z", "{run}");
    }

    /// The next named event on an SSE `body` other than a keepalive, as its
    /// kind and id; `None` once the stream has ended.
    async fn next_sse_event(body: &mut axum::body::Body) -> Option<(String, Option<String>)> {
        use http_body_util::BodyExt as _;
        loop {
            let frame = tokio::time::timeout(std::time::Duration::from_secs(5), body.frame())
                .await
                .expect("a frame in time")?;
            let data = frame.expect("a frame").into_data().expect("a data frame");
            let text = String::from_utf8(data.to_vec()).expect("UTF-8");
            if let Some(kind) = text.lines().find_map(|line| line.strip_prefix("event: "))
                && kind != "keepalive"
            {
                let id = text
                    .lines()
                    .find_map(|line| line.strip_prefix("id: "))
                    .map(str::to_string);
                return Some((kind.to_string(), id));
            }
        }
    }

    /// bug-1f41f0: a Graph run's per-run stream replays its runner index,
    /// delivers rows appended later, keeps `types` out of what it sends, and
    /// ends after the run's `run_completed`.
    #[tokio::test]
    async fn per_run_stream_carries_a_graph_runs_events() {
        let dir = tempfile::tempdir().unwrap();
        let deploy_backend = Arc::from(
            crate::deploy::create_backend("manual", None, None, None).expect("manual backend"),
        );
        let state = Arc::new(
            AppState::new(
                dir.path().to_path_buf(),
                Arc::new(crate::runtime::NoOpRuntime),
                roko_core::config::schema::RokoConfig::default(),
                deploy_backend,
            )
            .expect("AppState::new"),
        );
        let index =
            roko_fs::run_index::run_index_path(&state.layout.events_jsonl_path(), "g1").unwrap();
        write_rows(
            &index,
            &[
                json!({"type":"plan_started","run_id":"g1","plan_id":"p1","tasks_total":1}),
                json!({"type":"agent_heartbeat","run_id":"g1","plan_id":"p1","task_id":"T1","elapsed_ms":5000}),
            ],
        );

        let query = EventQuery {
            types: Some("plan_started,gate_result,run_completed".to_string()),
            ..EventQuery::default()
        };
        let mut body = get_run_events_stream(
            State(Arc::clone(&state)),
            Path("g1".into()),
            HeaderMap::new(),
            Query(query),
        )
        .await
        .expect("open the stream")
        .into_response()
        .into_body();

        let (kind, id) = next_sse_event(&mut body).await.expect("the replayed row");
        assert_eq!(kind, "plan_started");
        assert!(id.is_some(), "a resumable id");

        // Rows the run appends later arrive live; the heartbeat is filtered out.
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&index)
            .unwrap();
        for row in [
            json!({"type":"gate_result","run_id":"g1","plan_id":"p1","task_id":"T1","gate":"verify[0:test]","passed":true}),
            json!({"type":"run_completed","run_id":"g1","outcome":"succeeded","duration_ms":10}),
        ] {
            std::io::Write::write_all(&mut file, format!("{row}\n").as_bytes()).unwrap();
        }
        assert_eq!(
            next_sse_event(&mut body).await.expect("gate_result").0,
            "gate_result"
        );
        assert_eq!(
            next_sse_event(&mut body).await.expect("run_completed").0,
            "run_completed"
        );
        assert_eq!(
            next_sse_event(&mut body).await,
            None,
            "the stream ends after the run's end"
        );
    }

    /// The statuses `GET /runs/{run_id}/tasks` reports for a run's tasks.
    async fn task_statuses(state: &Arc<AppState>, run_id: &str) -> BTreeMap<String, String> {
        let Json(body) = get_run_tasks(State(Arc::clone(state)), Path(run_id.to_string()))
            .await
            .unwrap();
        body["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|task| {
                (
                    task["task_id"].as_str().unwrap().to_string(),
                    task["status"].as_str().unwrap().to_string(),
                )
            })
            .collect()
    }

    fn write_rows(path: &FsPath, rows: &[Value]) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let body: String = rows.iter().map(|row| format!("{row}\n")).collect();
        std::fs::write(path, body).unwrap();
    }

    /// bug-9a5970: a run whose plans the spec gate refused before any
    /// dispatch has, in its index, a refusal entry per finding, the run's
    /// error and its end. Its summary says `failed`, not active, with
    /// `refused: {stage: spec_gate, reason}` naming each task and rule, and
    /// does not call it interrupted. A run that failed at its work has no
    /// `refused`.
    #[tokio::test]
    async fn a_run_refused_before_dispatch_records_why_in_its_events_at_the_spec_gate() {
        let dir = tempfile::tempdir().unwrap();
        let deploy_backend = Arc::from(
            crate::deploy::create_backend("manual", None, None, None).expect("manual backend"),
        );
        let state = Arc::new(
            AppState::new(
                dir.path().to_path_buf(),
                Arc::new(crate::runtime::NoOpRuntime),
                roko_core::config::schema::RokoConfig::default(),
                deploy_backend,
            )
            .expect("AppState::new"),
        );
        let index = |run_id: &str| {
            roko_fs::run_index::run_index_path(&state.layout.events_jsonl_path(), run_id).unwrap()
        };
        let path = "plans/demos/demo-hello/tasks.toml";
        write_rows(
            &index("r-gated"),
            &[
                json!({"type":"plan_set_loaded","run_id":"r-gated","ts_millis":1,"plans":[{"plan_id":"demo-hello","title":"Hello"}]}),
                json!({"type":"event_log_entry","run_id":"r-gated","ts_millis":2,"event_type":"spec.gate.refused","plan_id":path,"task_id":"DEMO-T01","message":"HF3: verify passes on the unchanged base"}),
                json!({"type":"event_log_entry","run_id":"r-gated","ts_millis":2,"event_type":"spec.gate.refused","plan_id":path,"task_id":"DEMO-T02","message":"HF2: step 1: ends in `|| true`"}),
                json!({"type":"error","run_id":"r-gated","ts_millis":2,"message":"the spec gate refused the plan before any agent ran: …"}),
                json!({"type":"run_completed","run_id":"r-gated","ts_millis":3,"outcome":"failed","duration_ms":2}),
            ],
        );
        write_rows(
            &index("r-failed"),
            &[
                json!({"type":"plan_started","run_id":"r-failed","ts_millis":1,"plan_id":"p1"}),
                json!({"type":"task_completed","run_id":"r-failed","ts_millis":2,"plan_id":"p1","task_id":"t1","outcome":"failed"}),
                json!({"type":"run_completed","run_id":"r-failed","ts_millis":3,"outcome":"failed","duration_ms":2}),
            ],
        );

        let Json(summary) = get_run_summary(State(Arc::clone(&state)), Path("r-gated".into()))
            .await
            .unwrap();
        assert_eq!(summary["state"], "failed", "{summary}");
        assert_eq!(summary["active"], false, "{summary}");
        assert_eq!(summary["refused"]["stage"], "spec_gate", "{summary}");
        assert_eq!(
            summary["refused"]["reason"],
            "DEMO-T01: HF3: verify passes on the unchanged base; DEMO-T02: HF2: step 1: ends in \
             `|| true`",
            "{summary}"
        );
        assert_eq!(summary["interrupted"], Value::Null, "{summary}");

        let Json(summary) = get_run_summary(State(Arc::clone(&state)), Path("r-failed".into()))
            .await
            .unwrap();
        assert_eq!(summary["state"], "failed", "{summary}");
        assert_eq!(summary["refused"], Value::Null, "{summary}");
    }

    /// bug-dceafd: a run whose roko process died (a machine restart, a crash,
    /// `kill -9`) left an open manifest and an index with no end. Its summary
    /// reads `interrupted`, not active, with why and its last sign of life,
    /// and its detail and the dashboard's run list agree, the list even for a
    /// run nobody asked about; its cancel answers 200, every time, and leaves
    /// it interrupted. A run whose process still runs keeps reading
    /// `running`, and an id no run has is still 404.
    #[tokio::test]
    async fn a_run_whose_process_died_reads_interrupted() {
        use roko_learn::telemetry::RunProvenanceManifest;
        use roko_learn::telemetry::records::RunInvocation;

        let dir = tempfile::tempdir().unwrap();
        let deploy_backend = Arc::from(
            crate::deploy::create_backend("manual", None, None, None).expect("manual backend"),
        );
        let state = Arc::new(
            AppState::new(
                dir.path().to_path_buf(),
                Arc::new(crate::runtime::NoOpRuntime),
                roko_core::config::schema::RokoConfig::default(),
                deploy_backend,
            )
            .expect("AppState::new"),
        );
        let open_run = |run_id: &str, pid: u32, started_at: &str| {
            write_rows(
                &roko_fs::run_index::run_index_path(&state.layout.events_jsonl_path(), run_id)
                    .unwrap(),
                &[
                    json!({"type":"plan_started","run_id":run_id,"ts_millis":1_792_076_316_479_u64,"plan_id":"p1","tasks_total":2}),
                    json!({"type":"task_completed","run_id":run_id,"ts_millis":1_792_076_328_281_u64,"plan_id":"p1","task_id":"t1","outcome":"passed"}),
                    json!({"type":"task_started","run_id":run_id,"ts_millis":1_792_076_329_000_u64,"plan_id":"p1","task_id":"t2","phase":"graph-executing"}),
                ],
            );
            let mut manifest = RunProvenanceManifest::new(run_id, "plan_run");
            manifest.begin_invocation(RunInvocation {
                started_at: started_at.to_string(),
                pid,
                host: "local".to_string(),
                ..RunInvocation::default()
            });
            manifest.store(&state.layout.run_dir(run_id)).unwrap();
        };
        // A pid no process has any more.
        let mut child = std::process::Command::new("true").spawn().unwrap();
        let gone = child.id();
        child.wait().unwrap();
        open_run("r-crashed", gone, "2026-10-15T14:58:36.254Z");
        // Only ever listed, never asked for by id.
        open_run("r-listed", gone, "2026-10-15T14:58:36.254Z");
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        open_run("r-going", std::process::id(), &now);

        let Json(summary) = get_run_summary(State(Arc::clone(&state)), Path("r-crashed".into()))
            .await
            .unwrap();
        assert_eq!(summary["state"], "interrupted", "{summary}");
        assert_eq!(summary["verdict"], "interrupted", "{summary}");
        assert_eq!(summary["active"], false, "{summary}");
        assert_eq!(
            summary["interrupted"]["reason"],
            crate::interrupted_runs::INTERRUPTED_REASON,
            "{summary}"
        );
        assert!(summary["interrupted"]["at"].is_string(), "{summary}");
        assert_eq!(
            summary["finished_at"], summary["interrupted"]["at"],
            "{summary}"
        );
        assert_eq!(summary["tasks"]["passed"], 1, "{summary}");
        let Json(detail) = get_run_detail(State(Arc::clone(&state)), Path("r-crashed".into()))
            .await
            .unwrap();
        assert_eq!(detail["status"], "interrupted", "{detail}");
        let Json(listed) = get_dashboard_runs(State(Arc::clone(&state))).await.unwrap();
        let row = |run_id: &str| {
            listed["runs"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["run_id"] == run_id)
                .cloned()
                .unwrap_or_else(|| panic!("{run_id} is listed: {listed}"))
        };
        assert_eq!(row("r-crashed")["outcome"], "interrupted", "{listed}");
        assert_eq!(row("r-crashed")["is_complete"], true, "{listed}");
        assert_eq!(row("r-going")["is_complete"], false, "{listed}");
        assert_eq!(row("r-listed")["outcome"], "interrupted", "{listed}");
        assert_eq!(row("r-listed")["is_complete"], true, "{listed}");

        for _ in 0..2 {
            let answer = crate::routes::plans::cancel_plan_run(&state, "r-crashed")
                .await
                .unwrap();
            assert_eq!(answer["cancelled"], true, "{answer}");
            assert_eq!(answer["state"], "interrupted", "{answer}");
            assert_eq!(answer["already_ended"], true, "{answer}");
            assert_eq!(
                answer["reason"],
                crate::interrupted_runs::INTERRUPTED_REASON,
                "{answer}"
            );
        }
        let Json(summary) = get_run_summary(State(Arc::clone(&state)), Path("r-crashed".into()))
            .await
            .unwrap();
        assert_eq!(summary["state"], "interrupted", "{summary}");
        let index = std::fs::read_to_string(
            roko_fs::run_index::run_index_path(&state.layout.events_jsonl_path(), "r-crashed")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(index.matches("run_completed").count(), 1, "{index}");

        let Json(going) = get_run_summary(State(Arc::clone(&state)), Path("r-going".into()))
            .await
            .unwrap();
        assert_eq!(going["state"], "running", "{going}");
        assert_eq!(going["active"], true, "{going}");
        assert_eq!(going["interrupted"], Value::Null, "{going}");
        let refused = crate::routes::plans::cancel_plan_run(&state, "r-going").await;
        assert!(
            refused.is_err(),
            "its process runs, and serve has no handle"
        );
        let unknown = crate::routes::plans::cancel_plan_run(&state, "r-unknown").await;
        assert!(unknown.is_err());
    }

    /// gap-4f6cbe: a run's `delivery` milestone says how its plans'
    /// deliveries ended, from a Graph run's `delivery_finished` or a
    /// `graph.delivery` runtime event. A delivery a stop cut short reads
    /// `cancelled`, not failed, and one that did not complete wins over an
    /// earlier one that did.
    #[tokio::test]
    async fn run_summary_says_how_its_delivery_ended() {
        let dir = tempfile::tempdir().unwrap();
        let deploy_backend = Arc::from(
            crate::deploy::create_backend("manual", None, None, None).expect("manual backend"),
        );
        let state = Arc::new(
            AppState::new(
                dir.path().to_path_buf(),
                Arc::new(crate::runtime::NoOpRuntime),
                roko_core::config::schema::RokoConfig::default(),
                deploy_backend,
            )
            .expect("AppState::new"),
        );
        let finished = |run_id: &str, plan_id: &str, at: u64, state: &str| {
            json!({"type":"delivery_finished","run_id":run_id,"ts_millis":at,"plan_id":plan_id,
                "delivery_id":format!("{run_id}:{plan_id}"),"target":format!("roko/batch/{run_id}"),
                "state":state})
        };
        for (run_id, ends, said) in [
            ("d1", vec![("p1", "delivered")], "delivery completed"),
            ("d2", vec![("p1", "cancelled")], "delivery cancelled"),
            (
                "d3",
                vec![("p1", "delivered"), ("p2", "regression_failed")],
                "delivery failed: the check on its merge failed",
            ),
            (
                "d4",
                vec![("p1", "conflict"), ("p2", "delivered")],
                "delivery failed: its merge conflicted",
            ),
        ] {
            let mut rows = vec![json!({"type":"run.started","run_id":run_id,"timestamp_ms":1})];
            for (at, &(plan_id, end)) in (2..).zip(&ends) {
                rows.push(finished(run_id, plan_id, at, end));
            }
            rows.push(
                json!({"type":"run.completed","run_id":run_id,"timestamp_ms":9,
                "outcome":"failed"}),
            );
            write_rows(
                &roko_fs::run_index::run_index_path(&state.layout.events_jsonl_path(), run_id)
                    .unwrap(),
                &rows,
            );
            let Json(summary) = get_run_summary(State(Arc::clone(&state)), Path(run_id.into()))
                .await
                .unwrap();
            let delivery = summary["milestones"]
                .as_array()
                .unwrap()
                .iter()
                .find(|milestone| milestone["kind"] == "delivery")
                .unwrap_or_else(|| panic!("{run_id}: no delivery milestone: {summary}"));
            assert_eq!(delivery["text"], said, "{run_id}: {summary}");
        }

        let extension = |kind: &str, failure_state: Option<&str>| {
            json!({"event":{"payload":{"kind":"extension","data":{"namespace":"graph.delivery",
                "version":"1","value":{"kind":kind,"failure_state":failure_state}}}}})
        };
        assert_eq!(
            delivery_end(&extension("completed", None)),
            Some("completed")
        );
        assert_eq!(
            delivery_end(&extension("failed", Some("cancelled"))),
            Some("cancelled")
        );
        assert_eq!(
            delivery_end(&extension("failed", Some("regression_failed"))),
            Some("failed")
        );
        assert_eq!(delivery_end(&extension("started", None)), None);
    }

    /// B9 B4: a run's summary gives one state with its verdict and cost, how
    /// its tasks ended and at most five milestones, in the order they
    /// happened. A completed run whose tasks passed is `succeeded`; one with
    /// an unverified task and no failed one is `unverified`, though the run
    /// itself reported failure.
    #[tokio::test]
    async fn run_summary_reports_verdict_cost_and_milestones() {
        let dir = tempfile::tempdir().unwrap();
        let deploy_backend = Arc::from(
            crate::deploy::create_backend("manual", None, None, None).expect("manual backend"),
        );
        let state = Arc::new(
            AppState::new(
                dir.path().to_path_buf(),
                Arc::new(crate::runtime::NoOpRuntime),
                roko_core::config::schema::RokoConfig::default(),
                deploy_backend,
            )
            .expect("AppState::new"),
        );
        let index = |run_id: &str| {
            roko_fs::run_index::run_index_path(&state.layout.events_jsonl_path(), run_id).unwrap()
        };

        write_rows(
            &index("r1"),
            &[
                json!({"type":"run.started","run_id":"r1","timestamp_ms":1,"plan_id":"p1"}),
                json!({"type":"gate.completed","run_id":"r1","timestamp_ms":2,"plan_id":"p1","task_id":"t1","attempt":1,"gate":"test","passed":false}),
                json!({"type":"task_completed","run_id":"r1","timestamp_ms":3,"plan_id":"p1","task_id":"t1","outcome":"passed"}),
                json!({"type":"task_completed","run_id":"r1","timestamp_ms":4,"plan_id":"p1","task_id":"t2","outcome":"passed"}),
                json!({"type":"run.completed","run_id":"r1","timestamp_ms":5,"outcome":"succeeded","total_cost_usd":0.25,"duration_ms":10}),
            ],
        );
        let Json(summary) = get_run_summary(State(Arc::clone(&state)), Path("r1".into()))
            .await
            .unwrap();
        assert_eq!(summary["state"], "succeeded", "{summary}");
        assert_eq!(summary["verdict"], "succeeded", "{summary}");
        assert_eq!(summary["cost_usd"], 0.25, "{summary}");
        assert_eq!(summary["tasks"]["total"], 2, "{summary}");
        assert_eq!(summary["tasks"]["passed"], 2, "{summary}");
        let milestones = summary["milestones"].as_array().unwrap();
        assert!(milestones.len() <= 5, "{summary}");
        let kinds: Vec<&str> = milestones
            .iter()
            .map(|milestone| milestone["kind"].as_str().unwrap())
            .collect();
        assert_eq!(
            kinds,
            [
                "started",
                "first_gate_failure",
                "first_task_accepted",
                "completed"
            ]
        );
        assert_eq!(milestones[1]["text"], "gate test failed on task t1");
        assert_eq!(milestones[3]["text"], "run succeeded");

        write_rows(
            &index("r2"),
            &[
                json!({"type":"run.started","run_id":"r2","timestamp_ms":1}),
                json!({"type":"task_completed","run_id":"r2","timestamp_ms":2,"plan_id":"p1","task_id":"t1","outcome":"passed"}),
                json!({"type":"task_completed","run_id":"r2","timestamp_ms":3,"plan_id":"p1","task_id":"t2","outcome":"unverified"}),
                json!({"type":"run.completed","run_id":"r2","timestamp_ms":4,"outcome":"failed","total_cost_usd":0.05}),
            ],
        );
        let Json(summary) = get_run_summary(State(Arc::clone(&state)), Path("r2".into()))
            .await
            .unwrap();
        assert_eq!(summary["state"], "unverified", "{summary}");
        assert_eq!(summary["verdict"], "unverified", "{summary}");
        assert_eq!(summary["tasks"]["unverified"], 1, "{summary}");
        assert_eq!(summary["tasks"]["failed"], 0, "{summary}");
    }

    /// bug-74164c: a plan that did not finish delivered only its verified tasks, so
    /// its run's summary says the batch is partial and, per plan, which
    /// tasks' work it holds and why the others' is missing. A run whose
    /// plans all delivered whole is not partial.
    #[tokio::test]
    async fn run_summary_says_which_work_is_partial() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().to_path_buf();
        let git = |args: &[&str]| -> String {
            let output = std::process::Command::new("git")
                .current_dir(&repo)
                .args([
                    "-c",
                    "user.name=roko-test",
                    "-c",
                    "user.email=roko@test.invalid",
                    "-c",
                    "commit.gpgsign=false",
                ])
                .args(args)
                .env_remove("GIT_DIR")
                .env_remove("GIT_WORK_TREE")
                .env_remove("GIT_INDEX_FILE")
                .output()
                .expect("run git");
            assert!(
                output.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        };
        git(&["init", "--quiet", "-b", "main"]);
        std::fs::write(repo.join("lib.py"), "def one():\n    return 1\n").unwrap();
        git(&["add", "lib.py"]);
        git(&["commit", "--quiet", "-m", "base"]);
        let base = git(&["rev-parse", "HEAD"]);
        git(&["switch", "--quiet", "-c", "roko/batch/p1"]);
        std::fs::write(repo.join("one.py"), "ONE = 1\n").unwrap();
        git(&["add", "one.py"]);
        git(&["commit", "--quiet", "-m", "roko: pp/T1: one"]);
        git(&["switch", "--quiet", "main"]);
        git(&["branch", "roko/batch/w1", "roko/batch/p1"]);

        let deploy_backend = Arc::from(
            crate::deploy::create_backend("manual", None, None, None).expect("manual backend"),
        );
        let state = Arc::new(
            AppState::new(
                repo.clone(),
                Arc::new(crate::runtime::NoOpRuntime),
                roko_core::config::schema::RokoConfig::default(),
                deploy_backend,
            )
            .expect("AppState::new"),
        );
        let ended = |run_id: &str, plan_id: &str, outcome: &str| {
            let index =
                roko_fs::run_index::run_index_path(&state.layout.events_jsonl_path(), run_id)
                    .unwrap();
            write_rows(
                &index,
                &[
                    json!({"type":"run.started","run_id":run_id,"timestamp_ms":1,"plan_id":plan_id}),
                    json!({"type":"task_completed","run_id":run_id,"timestamp_ms":2,"plan_id":plan_id,"task_id":"T1","outcome":"passed"}),
                    json!({"type":"task_completed","run_id":run_id,"timestamp_ms":3,"plan_id":plan_id,"task_id":"T2","outcome":outcome}),
                    json!({"type":"run.completed","run_id":run_id,"timestamp_ms":4,"outcome":"failed","duration_ms":10}),
                ],
            );
        };
        ended("p1", "pp", "failed");
        ended("w1", "pw", "passed");
        let record = |plan_id: &str, value: Value| {
            let checkpoint = super::super::run_result::graph_plan_state_dir(&repo, plan_id)
                .join("checkpoint.json");
            std::fs::create_dir_all(checkpoint.parent().unwrap()).unwrap();
            std::fs::write(
                &checkpoint,
                json!({"extensions":{"roko.batch@1":{"value": value}}}).to_string(),
            )
            .unwrap();
        };
        record(
            "pp",
            json!({
                "run_id":"p1","branch":"roko/batch/p1","base_oid":base,"plan_id":"pp",
                "plan_branch":"roko/plan/pp","state":"delivered","partial":true,
                "work":{"partial":true,"delivered_tasks":["T1"],
                    "missing_tasks":[{"task_id":"T2","why":"failed"}]}
            }),
        );
        record(
            "pw",
            json!({"run_id":"w1","branch":"roko/batch/w1","base_oid":base,"state":"delivered"}),
        );
        let summary = |run_id: &'static str| {
            let state = Arc::clone(&state);
            async move {
                let Json(summary) = get_run_summary(State(state), Path(run_id.into()))
                    .await
                    .unwrap();
                summary
            }
        };

        let partial = summary("p1").await["result"].clone();
        assert_eq!(partial["partial"], true, "{partial}");
        assert_eq!(partial["files_changed"], json!(["one.py"]), "{partial}");
        assert_eq!(
            partial["merge_command"], "git merge --ff-only roko/batch/p1",
            "the verified part can be taken: {partial}"
        );
        assert_eq!(
            partial["plans"],
            json!([{
                "plan_id": "pp",
                "partial": true,
                "delivered_tasks": ["T1"],
                "missing_tasks": [{"task_id": "T2", "why": "failed"}],
            }]),
            "{partial}"
        );
        let whole = summary("w1").await["result"].clone();
        assert_eq!(whole["partial"], false, "{whole}");
        assert!(whole.get("plans").is_none(), "{whole}");
    }

    /// bug-db89d4: a plan that ended unverified was not delivered, so its
    /// run's batch is still at its base and has nothing to merge. Its batch
    /// record names the work no check ran on, which the summary's
    /// `result.unverified` reports with its plan branch, files and the command
    /// that shows it, and which `diff?work=unverified` returns. Once the plan
    /// branch moves on, the work is still named by its commit.
    #[tokio::test]
    async fn run_summary_names_unverified_work() {
        use axum::http::StatusCode;

        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().to_path_buf();
        let git = |args: &[&str]| -> String {
            let output = std::process::Command::new("git")
                .current_dir(&repo)
                .args([
                    "-c",
                    "user.name=roko-test",
                    "-c",
                    "user.email=roko@test.invalid",
                    "-c",
                    "commit.gpgsign=false",
                ])
                .args(args)
                .env_remove("GIT_DIR")
                .env_remove("GIT_WORK_TREE")
                .env_remove("GIT_INDEX_FILE")
                .output()
                .expect("run git");
            assert!(
                output.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        };
        git(&["init", "--quiet", "-b", "main"]);
        std::fs::write(repo.join("lib.py"), "def one():\n    return 1\n").unwrap();
        git(&["add", "lib.py"]);
        git(&["commit", "--quiet", "-m", "base"]);
        let base = git(&["rev-parse", "HEAD"]);
        git(&["branch", "roko/batch/u1"]);
        git(&["switch", "--quiet", "-c", "roko/plan/pu"]);
        std::fs::write(repo.join("new.py"), "def two():\n    return 2\n").unwrap();
        git(&["add", "new.py"]);
        git(&["commit", "--quiet", "-m", "roko: pu/T1: add two"]);
        let tip = git(&["rev-parse", "HEAD"]);
        git(&["switch", "--quiet", "main"]);

        let deploy_backend = Arc::from(
            crate::deploy::create_backend("manual", None, None, None).expect("manual backend"),
        );
        let state = Arc::new(
            AppState::new(
                repo.clone(),
                Arc::new(crate::runtime::NoOpRuntime),
                roko_core::config::schema::RokoConfig::default(),
                deploy_backend,
            )
            .expect("AppState::new"),
        );
        let index =
            roko_fs::run_index::run_index_path(&state.layout.events_jsonl_path(), "u1").unwrap();
        write_rows(
            &index,
            &[
                json!({"type":"run.started","run_id":"u1","timestamp_ms":1,"plan_id":"pu"}),
                json!({"type":"task_completed","run_id":"u1","timestamp_ms":2,"plan_id":"pu","task_id":"T1","outcome":"unverified"}),
                json!({"type":"run.completed","run_id":"u1","timestamp_ms":3,"outcome":"failed","duration_ms":10}),
            ],
        );
        let checkpoint =
            super::super::run_result::graph_plan_state_dir(&repo, "pu").join("checkpoint.json");
        std::fs::create_dir_all(checkpoint.parent().unwrap()).unwrap();
        std::fs::write(
            &checkpoint,
            json!({"extensions":{"roko.batch@1":{"value":{
                "run_id":"u1","branch":"roko/batch/u1","base_oid":base,"plan_id":"pu",
                "plan_branch":"roko/plan/pu",
                "work":{"unverified":{"commit":tip,"base":base,
                    "tasks":[{"task_id":"T1","verdict":"unverified"}]}}
            }}}})
            .to_string(),
        )
        .unwrap();
        let summary = || {
            let state = Arc::clone(&state);
            async move {
                let Json(summary) = get_run_summary(State(state), Path("u1".into()))
                    .await
                    .unwrap();
                summary
            }
        };
        let diff = |query: DiffQuery| {
            let state = Arc::clone(&state);
            async move { get_run_diff(State(state), Path("u1".into()), Query(query)).await }
        };

        let ended = summary().await;
        assert_eq!(ended["verdict"], "unverified", "{ended}");
        let result = &ended["result"];
        assert_eq!(result["branch"], "roko/batch/u1", "{result}");
        assert_eq!(
            result["commit"],
            base.as_str(),
            "nothing was delivered: {result}"
        );
        assert_eq!(result["merged"], false, "{result}");
        assert!(result.get("merge_command").is_none(), "{result}");
        assert_eq!(
            result["unverified"],
            json!([{
                "plan_id": "pu",
                "branch": "roko/plan/pu",
                "commit": tip,
                "base": base,
                "tasks": [{"task_id": "T1", "verdict": "unverified"}],
                "files_changed": ["new.py"],
                "diffstat": {"files": 1, "insertions": 2, "deletions": 0},
                "review_command": format!("git diff {}..roko/plan/pu", &base[..12]),
            }]),
            "{result}"
        );

        let Json(unverified) = diff(DiffQuery {
            work: Some("unverified".into()),
            ..DiffQuery::default()
        })
        .await
        .unwrap();
        assert_eq!(unverified["work"], "unverified", "{unverified}");
        assert_eq!(unverified["plan_id"], "pu", "{unverified}");
        assert_eq!(unverified["branch"], "roko/plan/pu", "{unverified}");
        assert_eq!(unverified["commit"], tip.as_str(), "{unverified}");
        assert_eq!(unverified["files"][0]["path"], "new.py", "{unverified}");
        assert!(
            unverified["files"][0]["patch"]
                .as_str()
                .unwrap()
                .contains("+def two():"),
            "{unverified}"
        );
        let Json(verified) = diff(DiffQuery::default()).await.unwrap();
        assert_eq!(verified["totals"]["files"], 0, "{verified}");
        let other_plan = diff(DiffQuery {
            work: Some("unverified".into()),
            plan_id: Some("other".into()),
            ..DiffQuery::default()
        })
        .await
        .unwrap_err();
        assert_eq!(
            other_plan.status,
            StatusCode::NOT_FOUND,
            "{}",
            other_plan.message
        );
        let refused = diff(DiffQuery {
            work: Some("everything".into()),
            ..DiffQuery::default()
        })
        .await
        .unwrap_err();
        assert_eq!(
            refused.status,
            StatusCode::BAD_REQUEST,
            "{}",
            refused.message
        );

        // Another run moved the plan branch on: the work is named by its commit.
        git(&["branch", "--force", "roko/plan/pu", "main"]);
        let moved = summary().await["result"]["unverified"][0].clone();
        assert!(moved.get("branch").is_none(), "{moved}");
        assert_eq!(
            moved["review_command"],
            format!("git diff {}..{tip}", &base[..12]),
            "{moved}"
        );
    }

    /// gap-8409d0: an ended run's summary names its batch branch, the branch's
    /// tip and base, and what it changes, with how to take it: a merge command
    /// while the checkout lacks the work, `merged` once it has it, the
    /// promotion once the run was promoted. The base comes from the run's
    /// manifest or a plan's batch record, so it survives the merge. A run
    /// still active, or one with no batch branch, has no result.
    /// gap-0d6d21: an ended run's diff lists every file its batch changes
    /// from its base, with status, line counts, a binary flag and the patch,
    /// within `max_files` and `max_patch_bytes`; 409 while the run is active,
    /// 404 when it has no result, 400 for limits out of range.
    #[tokio::test]
    async fn run_diff_returns_the_runs_patches() {
        use axum::http::StatusCode;

        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().to_path_buf();
        let git = |args: &[&str]| -> String {
            let output = std::process::Command::new("git")
                .current_dir(&repo)
                .args([
                    "-c",
                    "user.name=roko-test",
                    "-c",
                    "user.email=roko@test.invalid",
                    "-c",
                    "commit.gpgsign=false",
                ])
                .args(args)
                .env_remove("GIT_DIR")
                .env_remove("GIT_WORK_TREE")
                .env_remove("GIT_INDEX_FILE")
                .output()
                .expect("run git");
            assert!(
                output.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        };
        git(&["init", "--quiet", "-b", "main"]);
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("src/lib.rs"), "pub fn one() {}\n").unwrap();
        std::fs::write(repo.join("src/old.rs"), "pub fn gone() {}\n").unwrap();
        git(&["add", "src"]);
        git(&["commit", "--quiet", "-m", "base"]);
        let base = git(&["rev-parse", "HEAD"]);
        git(&["switch", "--quiet", "-c", "roko/batch/r9"]);
        std::fs::write(
            repo.join("src/lib.rs"),
            "pub fn one() {}\npub fn two() {}\n",
        )
        .unwrap();
        std::fs::write(repo.join("src/new.rs"), "pub fn three() {}\n").unwrap();
        std::fs::remove_file(repo.join("src/old.rs")).unwrap();
        std::fs::create_dir_all(repo.join("assets")).unwrap();
        std::fs::write(
            repo.join("assets/logo.bin"),
            [0_u8, 159, 146, 150, 0, 255, 10],
        )
        .unwrap();
        let big: String = (1..=3000).map(|n| format!("line {n}\n")).collect();
        std::fs::write(repo.join("big.txt"), big).unwrap();
        git(&["add", "-A", "src", "assets", "big.txt"]);
        git(&["commit", "--quiet", "-m", "the run's work"]);
        let tip = git(&["rev-parse", "HEAD"]);
        git(&["switch", "--quiet", "main"]);
        git(&["update-ref", "refs/heads/roko/batch/r6", &tip]);

        let deploy_backend = Arc::from(
            crate::deploy::create_backend("manual", None, None, None).expect("manual backend"),
        );
        let state = Arc::new(
            AppState::new(
                repo.clone(),
                Arc::new(crate::runtime::NoOpRuntime),
                roko_core::config::schema::RokoConfig::default(),
                deploy_backend,
            )
            .expect("AppState::new"),
        );
        let index = |run_id: &str| {
            roko_fs::run_index::run_index_path(&state.layout.events_jsonl_path(), run_id).unwrap()
        };
        let ended = |run_id: &str| {
            vec![
                json!({"type":"run.started","run_id":run_id,"timestamp_ms":1,"plan_id":run_id}),
                json!({"type":"task_completed","run_id":run_id,"timestamp_ms":2,"plan_id":run_id,"task_id":"t1","outcome":"passed"}),
                json!({"type":"run.completed","run_id":run_id,"timestamp_ms":3,"outcome":"succeeded","duration_ms":10}),
            ]
        };
        write_rows(&index("r9"), &ended("r9"));
        let manifest = roko_fs::RokoLayout::for_project(&repo)
            .run_dir("r9")
            .join("manifest.json");
        std::fs::create_dir_all(manifest.parent().unwrap()).unwrap();
        std::fs::write(
            &manifest,
            json!({"run_id":"r9","plan_ids":["r9"],"workspace":{"base_commit":base}}).to_string(),
        )
        .unwrap();
        // r6 is still running; r5 ended with no batch branch.
        write_rows(
            &index("r6"),
            &[json!({"type":"run.started","run_id":"r6","timestamp_ms":1,"plan_id":"r6"})],
        );
        write_rows(&index("r5"), &ended("r5"));

        let diff = |run_id: &'static str, query: DiffQuery| {
            let state = Arc::clone(&state);
            async move { get_run_diff(State(state), Path(run_id.into()), Query(query)).await }
        };

        let Json(all) = diff("r9", DiffQuery::default()).await.unwrap();
        assert_eq!(all["branch"], "roko/batch/r9", "{all}");
        assert_eq!(all["base"], base.as_str(), "{all}");
        assert_eq!(all["commit"], tip.as_str(), "{all}");
        assert_eq!(all["truncated"], false, "{all}");
        assert_eq!(
            all["totals"],
            json!({"files": 5, "additions": 3002, "deletions": 1}),
        );
        let files = all["files"].as_array().unwrap();
        let summary = files
            .iter()
            .map(|file| {
                (
                    file["path"].as_str().unwrap(),
                    file["status"].as_str().unwrap(),
                    file["binary"].as_bool().unwrap(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            summary,
            [
                ("assets/logo.bin", "added", true),
                ("big.txt", "added", false),
                ("src/lib.rs", "modified", false),
                ("src/new.rs", "added", false),
                ("src/old.rs", "deleted", false),
            ]
        );
        assert_eq!(files[0]["patch"], Value::Null, "a binary file has no patch");
        assert_eq!(files[2]["additions"], 1);
        assert!(
            files[2]["patch"]
                .as_str()
                .unwrap()
                .contains("+pub fn two() {}")
        );
        assert!(
            files[3]["patch"]
                .as_str()
                .unwrap()
                .contains("+pub fn three() {}")
        );
        assert!(
            files[4]["patch"]
                .as_str()
                .unwrap()
                .contains("-pub fn gone() {}")
        );
        assert_eq!(files[1]["truncated"], false);

        // A patch is cut on a line boundary at max_patch_bytes.
        let Json(cut) = diff(
            "r9",
            DiffQuery {
                max_patch_bytes: Some(1_024),
                ..DiffQuery::default()
            },
        )
        .await
        .unwrap();
        let big_patch = cut["files"][1]["patch"].as_str().unwrap();
        assert_eq!(cut["files"][1]["truncated"], true, "{}", cut["files"][1]);
        assert!(big_patch.len() <= 1_024 && big_patch.ends_with('\n'));
        assert_eq!(cut["files"][2]["truncated"], false);

        // Files past max_files are left out; the totals still count them.
        let Json(first_two) = diff(
            "r9",
            DiffQuery {
                max_files: Some(2),
                ..DiffQuery::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(first_two["files"].as_array().unwrap().len(), 2);
        assert_eq!(first_two["truncated"], true);
        assert_eq!(first_two["totals"]["files"], 5);

        let active = diff("r6", DiffQuery::default()).await.unwrap_err();
        assert_eq!(active.status, StatusCode::CONFLICT, "{}", active.message);
        let none = diff("r5", DiffQuery::default()).await.unwrap_err();
        assert_eq!(none.status, StatusCode::NOT_FOUND, "{}", none.message);
        let refused = diff(
            "r9",
            DiffQuery {
                max_files: Some(0),
                ..DiffQuery::default()
            },
        )
        .await
        .unwrap_err();
        assert_eq!(refused.status, StatusCode::BAD_REQUEST);
        let too_much = diff(
            "r9",
            DiffQuery {
                max_patch_bytes: Some(MAX_DIFF_PATCH_BYTES + 1),
                ..DiffQuery::default()
            },
        )
        .await
        .unwrap_err();
        assert_eq!(too_much.status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn run_summary_reports_where_the_verified_work_is() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().to_path_buf();
        let git = |args: &[&str]| -> String {
            let output = std::process::Command::new("git")
                .current_dir(&repo)
                .args([
                    "-c",
                    "user.name=roko-test",
                    "-c",
                    "user.email=roko@test.invalid",
                    "-c",
                    "commit.gpgsign=false",
                    "-c",
                    "tag.gpgsign=false",
                ])
                .args(args)
                .env_remove("GIT_DIR")
                .env_remove("GIT_WORK_TREE")
                .env_remove("GIT_INDEX_FILE")
                .output()
                .expect("run git");
            assert!(
                output.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        };
        git(&["init", "--quiet", "-b", "main"]);
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("src/lib.rs"), "pub fn one() {}\n").unwrap();
        git(&["add", "src/lib.rs"]);
        git(&["commit", "--quiet", "-m", "base"]);
        let base = git(&["rev-parse", "HEAD"]);
        // The run's verified work is on its batch branch; the checkout stays
        // on main.
        git(&["switch", "--quiet", "-c", "roko/batch/r9"]);
        std::fs::write(
            repo.join("src/lib.rs"),
            "pub fn one() {}\npub fn two() {}\n",
        )
        .unwrap();
        std::fs::write(repo.join("src/new.rs"), "pub fn three() {}\n").unwrap();
        git(&["add", "src/lib.rs", "src/new.rs"]);
        git(&["commit", "--quiet", "-m", "the run's work"]);
        let tip = git(&["rev-parse", "HEAD"]);
        git(&["switch", "--quiet", "main"]);
        for run_id in ["r7", "r6"] {
            git(&[
                "update-ref",
                &format!("refs/heads/roko/batch/{run_id}"),
                &tip,
            ]);
        }

        let deploy_backend = Arc::from(
            crate::deploy::create_backend("manual", None, None, None).expect("manual backend"),
        );
        let state = Arc::new(
            AppState::new(
                repo.clone(),
                Arc::new(crate::runtime::NoOpRuntime),
                roko_core::config::schema::RokoConfig::default(),
                deploy_backend,
            )
            .expect("AppState::new"),
        );
        let index = |run_id: &str| {
            roko_fs::run_index::run_index_path(&state.layout.events_jsonl_path(), run_id).unwrap()
        };
        let ended = |run_id: &str, plan_id: &str| {
            vec![
                json!({"type":"run.started","run_id":run_id,"timestamp_ms":1,"plan_id":plan_id}),
                json!({"type":"task_completed","run_id":run_id,"timestamp_ms":2,"plan_id":plan_id,"task_id":"t1","outcome":"passed"}),
                json!({"type":"run.completed","run_id":run_id,"timestamp_ms":3,"outcome":"succeeded","duration_ms":10}),
            ]
        };
        // r9's manifest records its base; r7's plan checkpoint does.
        write_rows(&index("r9"), &ended("r9", "p9"));
        let manifest = roko_fs::RokoLayout::for_project(&repo)
            .run_dir("r9")
            .join("manifest.json");
        std::fs::create_dir_all(manifest.parent().unwrap()).unwrap();
        std::fs::write(
            &manifest,
            json!({"run_id":"r9","plan_ids":["p9"],"workspace":{"base_commit":base}}).to_string(),
        )
        .unwrap();
        write_rows(&index("r7"), &ended("r7", "p7"));
        let checkpoint =
            super::super::run_result::graph_plan_state_dir(&repo, "p7").join("checkpoint.json");
        std::fs::create_dir_all(checkpoint.parent().unwrap()).unwrap();
        std::fs::write(
            &checkpoint,
            json!({"extensions":{"roko.batch@1":{"value":{
                "run_id":"r7","branch":"roko/batch/r7","base_oid":base,"state":"delivered"
            }}}})
            .to_string(),
        )
        .unwrap();
        // r6 is still running; r5 ended with no batch branch.
        write_rows(
            &index("r6"),
            &[json!({"type":"run.started","run_id":"r6","timestamp_ms":1,"plan_id":"p6"})],
        );
        write_rows(&index("r5"), &ended("r5", "p5"));

        let summary = |run_id: &'static str| {
            let state = Arc::clone(&state);
            async move {
                let Json(summary) = get_run_summary(State(state), Path(run_id.into()))
                    .await
                    .unwrap();
                summary
            }
        };

        let result = summary("r9").await["result"].clone();
        assert_eq!(result["branch"], "roko/batch/r9", "{result}");
        assert_eq!(result["commit"], tip.as_str(), "{result}");
        assert_eq!(result["base"], base.as_str(), "{result}");
        assert_eq!(result["files_changed"], json!(["src/lib.rs", "src/new.rs"]));
        assert_eq!(
            result["diffstat"],
            json!({"files": 2, "insertions": 2, "deletions": 0}),
        );
        assert_eq!(result["merged"], false, "{result}");
        assert_eq!(
            result["merge_command"], "git merge --ff-only roko/batch/r9",
            "{result}"
        );
        assert!(result.get("tag").is_none(), "{result}");

        assert!(summary("r6").await.get("result").is_none(), "an active run");
        assert!(
            summary("r5").await.get("result").is_none(),
            "no batch branch"
        );

        // Once the checkout has the work, it is merged, and the recorded base
        // still names what the run changed.
        git(&["merge", "--quiet", "--ff-only", "roko/batch/r9"]);
        let result = summary("r9").await["result"].clone();
        assert_eq!(result["merged"], true, "{result}");
        assert!(result.get("merge_command").is_none(), "{result}");
        assert_eq!(result["diffstat"]["files"], 2, "{result}");
        let result = summary("r7").await["result"].clone();
        assert_eq!(result["base"], base.as_str(), "{result}");
        assert_eq!(result["files_changed"], json!(["src/lib.rs", "src/new.rs"]));
        assert_eq!(result["merged"], true, "{result}");

        // A promoted run names its tag and target.
        git(&[
            "tag",
            "--annotate",
            "--message",
            "roko run r9: roko/batch/r9 promoted into release",
            "roko/run/r9",
            &tip,
        ]);
        let result = summary("r9").await["result"].clone();
        assert_eq!(result["tag"], "roko/run/r9", "{result}");
        assert_eq!(result["promoted_into"], "release", "{result}");
    }

    /// 9117: each task the run dispatched an agent for reports what screens
    /// the text it reads. With `[agent.data_llm]` set, a task whose model
    /// runs roko's own tool loop is screened and one that ran a CLI agent is
    /// not; without it, no task is.
    #[test]
    fn run_summary_reports_each_tasks_untrusted_input_boundary() {
        use roko_core::config::{DataLlmConfig, ModelProfile, ProviderConfig};

        let mut config = RokoConfig::default();
        for (provider, kind) in [
            ("zai", roko_core::ProviderKind::OpenAiCompat),
            ("claude", roko_core::ProviderKind::ClaudeCli),
        ] {
            config.providers.insert(
                provider.to_string(),
                ProviderConfig {
                    kind,
                    ..ProviderConfig::default()
                },
            );
        }
        for (model, provider) in [("api-model", "zai"), ("cli-model", "claude")] {
            config.models.insert(
                model.to_string(),
                ModelProfile {
                    provider: provider.to_string(),
                    slug: model.to_string(),
                    ..ModelProfile::default()
                },
            );
        }
        let events = [
            json!({"type":"agent_spawned","run_id":"r1","plan_id":"p1","task_id":"T1","model":"api-model"}),
            json!({"type":"agent.dispatch.started","run_id":"r1","plan_id":"p1","task_id":"T2","requested_model":"cli-model"}),
        ]
        .map(|value| IndexedEvent { cursor: 0, value });
        let boundaries = |config: &RokoConfig| {
            run_summary("r1", &events, None, config)["untrusted_input_boundary"].clone()
        };

        assert_eq!(
            boundaries(&config),
            json!([
                {"plan_id": "p1", "task_id": "T1", "boundary": "none"},
                {"plan_id": "p1", "task_id": "T2", "boundary": "none"}
            ])
        );
        config.agent.data_llm = Some(DataLlmConfig::default());
        assert_eq!(
            boundaries(&config),
            json!([
                {"plan_id": "p1", "task_id": "T1", "boundary": "data_llm"},
                {"plan_id": "p1", "task_id": "T2", "boundary": "none"}
            ])
        );
    }

    /// bug-54c729: a task that settled as unverified, skipped or already
    /// satisfied is reported as that, not as failed. A runner attempt record
    /// and a runtime event that says only whether the task passed keep their
    /// passed or failed status.
    #[tokio::test]
    async fn runs_route_keeps_unverified_and_skipped_statuses() {
        use roko_core::runtime_event::{RuntimeEvent, RuntimeEventEnvelope};

        let dir = tempfile::tempdir().unwrap();
        let deploy_backend = Arc::from(
            crate::deploy::create_backend("manual", None, None, None).expect("manual backend"),
        );
        let state = Arc::new(
            AppState::new(
                dir.path().to_path_buf(),
                Arc::new(crate::runtime::NoOpRuntime),
                roko_core::config::schema::RokoConfig::default(),
                deploy_backend,
            )
            .expect("AppState::new"),
        );

        // Task records that name their outcome, as dashboard events do.
        let outcomes = [
            ("t-passed", "passed"),
            ("t-unverified", "unverified"),
            ("t-skipped", "skipped"),
            ("t-condition", "condition-skipped"),
            ("t-satisfied", "already_satisfied"),
            ("t-accepted", "accepted_with_failures"),
            ("t-failed", "failed"),
        ];
        let mut rows: Vec<Value> = outcomes
            .iter()
            .map(|(task, outcome)| {
                json!({"type":"task_completed","run_id":"r1","plan_id":"p1","task_id":task,"outcome":outcome})
            })
            .collect();
        rows.push(json!({"type":"task.attempt.completed","run_id":"r1","plan_id":"p1","task_id":"t-attempt","attempt":1,"outcome":"timed_out"}));
        write_rows(
            &roko_fs::run_index::run_index_path(&state.layout.events_jsonl_path(), "r1").unwrap(),
            &rows,
        );
        assert_eq!(
            task_statuses(&state, "r1").await,
            BTreeMap::from(
                [
                    ("t-passed", "passed"),
                    ("t-unverified", "unverified"),
                    ("t-skipped", "skipped"),
                    ("t-condition", "skipped"),
                    ("t-satisfied", "already_satisfied"),
                    ("t-accepted", "accepted_with_failures"),
                    ("t-failed", "failed"),
                    ("t-attempt", "failed"),
                ]
                .map(|(task, status)| (task.to_string(), status.to_string()))
            )
        );

        // Runtime events say whether a task passed, or that it was skipped.
        let events = [
            RuntimeEvent::TaskCompleted {
                run_id: "r2".into(),
                plan_id: "p1".into(),
                task_id: "t-ok".into(),
                passed: true,
                duration_ms: 1,
                outcome: None,
            },
            RuntimeEvent::TaskCompleted {
                run_id: "r2".into(),
                plan_id: "p1".into(),
                task_id: "t-bad".into(),
                passed: false,
                duration_ms: 1,
                outcome: None,
            },
            RuntimeEvent::TaskSkipped {
                task_id: "t-never".into(),
                reason: "its dependency failed".into(),
            },
        ];
        let rows: Vec<Value> = events
            .into_iter()
            .zip(0..)
            .map(|(event, seq)| {
                serde_json::to_value(RuntimeEventEnvelope::new("r2", seq, "test", event)).unwrap()
            })
            .collect();
        write_rows(&state.runtime_event_logger.run_path("r2").unwrap(), &rows);
        assert_eq!(
            task_statuses(&state, "r2").await,
            BTreeMap::from(
                [
                    ("t-ok", "passed"),
                    ("t-bad", "failed"),
                    ("t-never", "skipped")
                ]
                .map(|(task, status)| (task.to_string(), status.to_string()))
            )
        );
    }
}
