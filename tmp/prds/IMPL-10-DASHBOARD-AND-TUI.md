# IMPL-10: Dashboard, TUI, and roko stabilization

**Implements:** PRD-10 (Dashboard and TUI unified surfaces)
**Status:** Active
**Date:** 2026-04-21
**Estimated effort:** 10.5 weeks across 6 phases
**Authoritative:** This plan supersedes all previous versions of IMPL-10.

---

## Three workstreams

This plan covers three workstreams that depend on each other:

1. **Roko stabilization** -- fix real gaps so roko can execute IMPL-01 through IMPL-09 via its self-hosting loop. Without these fixes, the orchestrator drops data on restart, polls files at O(N) per frame, and exposes an unauthenticated HTTP surface.

2. **Dashboard complete redesign** -- rebuild the web dashboard from scratch. The current codebase mixes mock data with live API calls, has no router, runs 14+ polling timers, and stores a plaintext password in main.jsx. Every component gets a full rewrite.

3. **TUI enhancements** -- add missing tabs (F8 Marketplace, F9 Atelier), implement the stubbed sub-views (ProviderHealth, ModelComparison, EngramDag, EpisodeReplay, KnowledgeBrowse), fix bugs in existing views, and port bardo-era widgets.

## Ground truth

Every task in this plan was derived from code audits. The appendix at the end lists every audited file with line counts and specific problems found.

---

## Phase 0: Roko stabilization (2 weeks)

Make roko reliable enough to execute the other IMPL plans via the self-hosting loop. Tasks 0.1 through 0.5 are blocking -- nothing in Phases 1-5 starts until these land.

---

### Task 0.1: Incremental file watchers for TUI event parity

**Effort:** 2 days
**Dependencies:** None
**Fixes:** ux-followup items 71, 72, 73, 74, 76

**Files to modify:**
- `crates/roko-cli/src/tui/fs_watch.rs` -- extend to emit per-file change events, not a single `Coalesced` signal
- `crates/roko-cli/src/tui/jsonl_cursor.rs` -- already correct, wire into more consumers
- `crates/roko-cli/src/tui/dashboard.rs` -- switch from full-file re-reads to cursor-based incremental reads

**Files to create:**
- `crates/roko-cli/src/tui/incremental.rs` -- coordinator that maps watched paths to JsonlCursor instances

**What to implement:**

The current `FsWatchHandle` emits a single `FsRefresh::Coalesced` event for any change anywhere in `.roko/`. Every consumer responds by re-reading its entire source file from disk. This is O(N) per refresh where N is total file size, and it happens on every frame when files are actively written.

The fix has two parts.

Part 1: Extend `FsRefresh` to carry the changed path:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsRefresh {
    Coalesced,
    FileChanged(PathBuf),
}
```

Update `RefreshHandler::handle_event` to extract paths from `notify::Event.paths` and emit `FileChanged` for each distinct file. Keep `Coalesced` as a fallback when the debouncer reports errors without paths.

Part 2: Create `IncrementalReader` in `incremental.rs`:

```rust
pub struct IncrementalReader {
    cursors: HashMap<PathBuf, JsonlCursor>,
}

impl IncrementalReader {
    pub fn new() -> Self;
    pub fn register(&mut self, path: PathBuf);
    pub fn read_updates(&mut self, changed: &Path) -> Vec<(PathBuf, Vec<String>)>;
    pub fn read_all_updates(&mut self) -> Vec<(PathBuf, Vec<String>)>;
}
```

Wire `IncrementalReader` into `DashboardData::refresh()`. Replace the five full-file re-reads:

| Data source | Current read | After |
|---|---|---|
| Gate verdicts | `signals.jsonl` full parse | `JsonlCursor` on `signals.jsonl`, filter kind == "gate" |
| Task outputs | `readdir` + read each file | `JsonlCursor` on `state/task-outputs.jsonl` |
| Episodes | `episodes.jsonl` full read | `JsonlCursor` on `episodes.jsonl` |
| Event log | `state/events.json` full parse | `JsonlCursor` on `state/events.jsonl` |
| Learning data | Multiple files re-read | `JsonlCursor` on `learn/efficiency.jsonl` |

**Acceptance criteria:**
- [ ] `FsRefresh::FileChanged` carries the changed path
- [ ] `IncrementalReader` tracks byte offsets per file across refreshes
- [ ] Dashboard refresh reads only new bytes appended since last tick
- [ ] TUI shows correct data after 1000+ appended lines without visible lag
- [ ] Existing `watch_roko_dir_emits_refresh_within_500ms` test still passes
- [ ] New test: `incremental_reader_skips_already_read_bytes`
- [ ] `cargo test -p roko-cli` passes
- [ ] `cargo clippy -p roko-cli --no-deps -- -D warnings` clean
- [ ] `cargo +nightly fmt --all` passes

---

### Task 0.2: Live agent WebSocket streaming in TUI

**Effort:** 2 days
**Dependencies:** None (parallel with 0.1)
**Fixes:** ux-followup item 70

**IMPORTANT:** `TuiState` already has `agent_streams: HashMap<String, AgentStream>` with methods `push_agent_chunk`, `mark_agent_stream_connected/disconnected/done`. The `AgentStreamClient` with `connect_direct` already exists in `ws_client.rs`. Do NOT add duplicate fields. Instead, extend the existing `AgentStreamClient` to auto-connect when an agent is selected in the Agents tab, and ensure chunks flow through the existing `push_agent_chunk` method.

**Files to modify:**
- `crates/roko-cli/src/tui/ws_client.rs` -- extend `AgentStreamClient` to connect to per-agent sidecar `/stream` endpoint
- `crates/roko-cli/src/tui/views/agents_view.rs` -- render live stream chunks instead of stale executor.json snapshots
- `crates/roko-cli/src/tui/state.rs` -- extend existing `agent_streams` field, do not add duplicate

**What to implement:**

The Agents tab (F3) reads agent status from `executor.json` snapshots. These snapshots are written only on autosave (every 5 actions) and on shutdown. Between saves, the UI shows stale data. The fix connects to the agent sidecar's `/stream` WebSocket endpoint for real-time output.

Step 1: Add a connection manager to `TuiState`:

```rust
pub struct AgentStreamState {
    pub client: AgentStreamClient,
    pub buffer: VecDeque<StreamChunk>,
    pub connected: bool,
    pub last_error: Option<String>,
}
```

Step 2: When the user selects an agent in F3, check if we already have a stream for that agent. If not, look up the agent's sidecar endpoint from `DashboardData.discovered_agents` and call `AgentStreamClient::connect_direct(endpoint)`. If the agent has no sidecar, fall back to `AgentStreamClient::connect(agent_id, serve_base_url, auth_token)` which filters the global event bus.

Step 3: In the TUI tick loop, drain `agent_stream.try_recv()` and push chunks into a ring buffer (cap at 500 lines). Render the ring buffer in the output stream panel of F3.

Step 4: Show connection status in the agent list:
- Green dot: connected via direct WS
- Yellow dot: connected via event bus filter
- Gray dot: no stream, reading from snapshot
- Red dot: connection failed, will retry

`AgentStreamClient` already handles reconnection with exponential backoff (1s initial, 30s max). No changes needed there.

**Acceptance criteria:**
- [ ] Selecting an agent in F3 opens a WebSocket to its sidecar
- [ ] Live text/reasoning/tool-call chunks render in the output panel within 100ms
- [ ] Connection status dot updates on connect/disconnect
- [ ] Fallback to event bus filtering works when no direct endpoint exists
- [ ] Fallback to snapshot data works when no event bus is available
- [ ] Deselecting an agent closes the WebSocket
- [ ] `cargo test -p roko-cli` passes
- [ ] `cargo clippy -p roko-cli --no-deps -- -D warnings` clean
- [ ] `cargo +nightly fmt --all` passes

---

### Task 0.3: Generation counter persistence — ALREADY IMPLEMENTED

**Status:** Complete. `DurableDashboardGenerationCounter` already exists at `crates/roko-cli/src/tui/dashboard_gen.rs` with `load()`, `next()` (atomic persist), and tests. It is wired into `DashboardData`. No work needed.

**Verification:** Confirm the counter persists to `.roko/state/dashboard-gen.json` by running the TUI, stopping it, and restarting — generation numbers should continue from where they left off.

---

### Task 0.4: HTTP auth middleware upgrade

**Effort:** 3 days
**Dependencies:** None (parallel with 0.1-0.3)

**Files to modify:**
- `crates/roko-serve/src/routes/middleware.rs` -- add JWT validation, rate limiting, role extraction
- `crates/roko-serve/src/routes/mod.rs` -- apply new middleware layers
- `crates/roko-core/src/config/schema.rs` -- extend `ServeAuthConfig` with JWT fields

**Files to create:**
- `crates/roko-serve/src/auth.rs` -- JWT validation logic, JWKS caching, role types
- `crates/roko-serve/src/routes/auth_routes.rs` -- key CRUD, session info endpoints

**Dependencies to add:** Add `jsonwebtoken = "9"` to `[workspace.dependencies]` in the root `Cargo.toml` and add `jsonwebtoken.workspace = true` to `crates/roko-serve/Cargo.toml`. The `tower` crate with `limit` feature is already available.

**Route registration:** Add `.merge(auth_routes::routes())` to the `build_router` function in `crates/roko-serve/src/routes/mod.rs`, and add `pub mod auth_routes;` at the top with other module declarations.

**What to implement:**

The current auth is a single `X-Api-Key` header check in `require_api_key`. No JWT validation, no role-based access, no rate limiting.

**Part 1: Auth types and JWT validation (`auth.rs`)**

```rust
/// Roles for route-level access control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthRole {
    /// Read-only access to status and metrics.
    Viewer,
    /// Can trigger runs and manage plans.
    Operator,
    /// Full access including config changes and key management.
    Admin,
}

/// Extracted identity from a validated request.
#[derive(Debug, Clone)]
pub struct AuthIdentity {
    pub subject: String,
    pub role: AuthRole,
    pub source: AuthSource,
}

#[derive(Debug, Clone)]
pub enum AuthSource {
    ApiKey,
    Jwt { issuer: String },
}

/// JWKS-backed JWT validator with key caching.
pub struct JwtValidator {
    jwks_url: String,
    /// Cached JWKS, refreshed every 5 minutes.
    cached_keys: RwLock<CachedJwks>,
    required_audience: Option<String>,
    required_issuer: Option<String>,
}

impl JwtValidator {
    pub async fn new(jwks_url: String, audience: Option<String>, issuer: Option<String>) -> Result<Self>;
    pub async fn validate(&self, token: &str) -> Result<AuthIdentity, AuthError>;
    async fn refresh_keys(&self) -> Result<()>;
}

struct CachedJwks {
    keys: Vec<jsonwebtoken::jwk::Jwk>,
    fetched_at: Instant,
    ttl: Duration,
}
```

**Part 2: Config extension**

Add to `ServeAuthConfig`:

```rust
pub struct ServeAuthConfig {
    pub enabled: bool,
    pub api_key: String,
    // New fields:
    pub jwt_enabled: bool,
    pub jwks_url: Option<String>,
    pub jwt_audience: Option<String>,
    pub jwt_issuer: Option<String>,
    pub rate_limit_rpm: Option<u32>,
}
```

**Part 3: Middleware layers**

Replace `require_api_key` with `authenticate`:

```rust
pub async fn authenticate(
    State(auth_state): State<Arc<AuthState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    // 1. Check Authorization: Bearer <jwt> header
    // 2. Fall back to X-Api-Key header
    // 3. If neither, return 401
    // 4. Insert AuthIdentity into request extensions
    // 5. Call next
}
```

Add `require_role` middleware:

```rust
pub async fn require_role(
    role: AuthRole,
    req: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let identity = req.extensions().get::<AuthIdentity>()
        .ok_or(ApiError::unauthorized("no identity"))?;
    if identity.role < role {
        return Err(ApiError::forbidden("insufficient permissions"));
    }
    Ok(next.run(req).await)
}
```

Add rate limiting via `tower::limit::RateLimitLayer` or a custom token-bucket keyed by IP + identity.

**Part 4: Auth routes (`auth_routes.rs`)**

```
GET  /api/auth/me           -- return current identity
POST /api/auth/keys         -- create API key (Admin only)
GET  /api/auth/keys         -- list API keys (Admin only)
DELETE /api/auth/keys/:id   -- revoke API key (Admin only)
```

**Acceptance criteria:**
- [ ] JWT validation works with a test JWKS endpoint
- [ ] API key auth still works as before (backwards compatible)
- [ ] `AuthIdentity` is injected into request extensions for downstream use
- [ ] Rate limiting returns 429 when exceeded
- [ ] Admin-only routes reject Viewer/Operator tokens
- [ ] Auth can be disabled entirely via `serve.auth.enabled = false` (default)
- [ ] `cargo test -p roko-serve` passes
- [ ] `cargo clippy -p roko-serve --no-deps -- -D warnings` clean
- [ ] `cargo +nightly fmt --all` passes

---

### Task 0.5: Server state persistence

**Effort:** 1.5 days
**Dependencies:** None (parallel with 0.1-0.4)

**Files to modify:**
- `crates/roko-serve/src/state.rs` -- add persist/restore methods to `AppState`
- `crates/roko-serve/src/lib.rs` -- call restore on startup, persist on shutdown

**Files to create:**
- `crates/roko-serve/src/persistence.rs` -- serialization/deserialization for ephemeral state

**What to implement:**

`active_plans`, `operations`, `deployments`, `discovered_agents`, and `template_runs` are all in-memory `RwLock<HashMap<...>>`. They vanish on restart. The fix persists them to `.roko/state/server-state.json` on shutdown and a periodic timer, and restores them on startup.

**Type names:** Use `Deployment` from `crate::deploy::Deployment` (not `DeploymentRecord`). Use `HashMap<String, AgentRegistrationRecord>` for `discovered_agents` (not `Vec<DiscoveredAgent>`). `AppState` uses `tokio::sync::RwLock` so all lock operations are `.read().await` / `.write().await`, not `.read().unwrap()`.

```rust
/// Serializable snapshot of server state that survives restarts.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ServerStateSnapshot {
    pub version: u32,
    pub timestamp: u64,
    pub discovered_agents: HashMap<String, AgentRegistrationRecord>,
    pub deployments: Vec<Deployment>,
    pub template_runs: HashMap<String, Vec<TemplateRunRecord>>,
    /// Active plans/runs are NOT persisted -- they use the executor
    /// snapshot mechanism in `.roko/state/executor.json` instead.
}

const SERVER_STATE_PATH: &str = "state/server-state.json";
const PERSIST_INTERVAL: Duration = Duration::from_secs(60);

impl AppState {
    /// Restore ephemeral state from the last persisted snapshot.
    pub async fn restore_state(&self) -> Result<()> {
        let path = self.layout.root().join(SERVER_STATE_PATH);
        let content = match tokio::fs::read_to_string(&path).await {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e.into()),
        };
        let snapshot: ServerStateSnapshot = serde_json::from_str(&content)?;
        // Merge into live state...
        Ok(())
    }

    /// Persist ephemeral state to disk.
    pub async fn persist_state(&self) -> Result<()> {
        let snapshot = ServerStateSnapshot {
            version: 1,
            timestamp: now_unix_secs(),
            discovered_agents: self.discovered_agents.read().await.clone(),
            deployments: self.deployments.read().await.values().cloned().collect(),
            template_runs: self.template_runs.read().await.clone(),
        };
        let path = self.layout.root().join(SERVER_STATE_PATH);
        tokio::fs::create_dir_all(path.parent().unwrap()).await?;
        let json = serde_json::to_string_pretty(&snapshot)?;
        tokio::fs::write(&path, json).await?;
        Ok(())
    }
}
```

Spawn a background task that calls `persist_state()` every 60 seconds and on SIGTERM/graceful shutdown.

**Acceptance criteria:**
- [ ] `discovered_agents` survive server restart
- [ ] `deployments` survive server restart
- [ ] `template_runs` survive server restart
- [ ] Periodic persistence runs every 60 seconds
- [ ] Graceful shutdown persists before exit
- [ ] Corrupt snapshot file logs a warning and starts fresh
- [ ] `cargo test -p roko-serve` passes
- [ ] `cargo +nightly fmt --all` passes

---

### Task 0.6: Jobs backend

**Effort:** 3 days
**Dependencies:** Task 0.4 (auth middleware for role-gated routes)

**Files to create:**
- `crates/roko-core/src/jobs.rs` -- job types, state machine, validation
- `crates/roko-serve/src/routes/jobs.rs` -- HTTP route handlers
- `crates/roko-serve/src/jobs_store.rs` -- file-backed job persistence

**Files to modify:**
- `crates/roko-serve/src/routes/mod.rs` -- register jobs routes
- `crates/roko-serve/src/state.rs` -- add `jobs_store` field
- `crates/roko-serve/src/events.rs` -- add job-related `ServerEvent` variants

**Module wiring:**
1. Add `pub mod jobs;` to `crates/roko-core/src/lib.rs`
2. Add `pub mod jobs;` to `crates/roko-serve/src/routes/mod.rs`
3. In `crates/roko-serve/src/routes/mod.rs` `build_router()`: add `.merge(jobs::routes())` after the last existing `.merge()`
4. Add `jobs_store: Arc<FileJobStore>` to `AppState` struct in `crates/roko-serve/src/state.rs`
5. Initialize in `AppState::new()`: `jobs_store: Arc::new(FileJobStore::new(&roko_dir))`
6. In `FileJobStore::new()`, call `std::fs::create_dir_all(dir.join("jobs"))` to ensure the directory exists
7. `ServerEvent` variants need `#[serde(tag = "type", rename_all = "snake_case")]` — follow the existing pattern

**What to implement:**

**Part 1: Job types (`roko-core/src/jobs.rs`)**

```rust
/// Unique job identifier.
pub type JobId = String;

/// What kind of work a job requires.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    ResearchBrief {
        topic: String,
        depth: ResearchDepth,
        max_sources: Option<usize>,
    },
    CodingTask {
        spec: String,
        language: Option<String>,
        test_required: bool,
    },
    CodeReview {
        repo_url: String,
        pr_number: Option<u64>,
        focus_areas: Vec<String>,
    },
    Custom {
        description: String,
        acceptance_criteria: Vec<String>,
    },
}

/// Research depth level.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchDepth {
    Quick,      // ~5 min, 3-5 sources
    Standard,   // ~15 min, 8-12 sources
    Deep,       // ~30 min, 15+ sources
}

/// Job lifecycle states.
///
/// State machine:
///
///   Open --> Assigned --> InProgress --> Submitted --> Completed
///     |                      |              |
///     +--> Cancelled         +--> Failed    +--> Rejected --> InProgress
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Open,
    Assigned,
    InProgress,
    Submitted,
    Completed,
    Rejected,
    Failed,
    Cancelled,
}

impl JobStatus {
    /// Valid transitions from this status.
    pub fn valid_transitions(self) -> &'static [JobStatus] {
        match self {
            Self::Open => &[Self::Assigned, Self::Cancelled],
            Self::Assigned => &[Self::InProgress, Self::Cancelled],
            Self::InProgress => &[Self::Submitted, Self::Failed],
            Self::Submitted => &[Self::Completed, Self::Rejected],
            Self::Rejected => &[Self::InProgress, Self::Cancelled],
            Self::Failed => &[Self::Open],
            Self::Completed | Self::Cancelled => &[],
        }
    }

    pub fn can_transition_to(self, next: Self) -> bool {
        self.valid_transitions().contains(&next)
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled)
    }
}

/// Full job record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: JobId,
    pub title: String,
    pub kind: JobKind,
    pub status: JobStatus,
    pub creator: String,
    pub assignee: Option<String>,
    pub domain_tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub deadline: Option<chrono::DateTime<chrono::Utc>>,
    pub reward: Option<JobReward>,
    pub submission: Option<JobSubmission>,
    pub evaluation: Option<JobEvaluation>,
    pub timeline: Vec<JobTimelineEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobReward {
    pub amount: f64,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSubmission {
    pub submitted_at: chrono::DateTime<chrono::Utc>,
    pub deliverables: Vec<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobEvaluation {
    pub evaluated_at: chrono::DateTime<chrono::Utc>,
    pub score: f64,
    pub feedback: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobTimelineEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub from_status: Option<JobStatus>,
    pub to_status: JobStatus,
    pub actor: String,
    pub note: Option<String>,
}
```

**Part 2: Job store (`jobs_store.rs`)**

```rust
/// File-backed job persistence in `.roko/jobs/`.
pub struct JobsStore {
    root: PathBuf,  // .roko/jobs/
}

impl JobsStore {
    pub fn new(roko_dir: &Path) -> Self;
    pub async fn create(&self, job: &Job) -> Result<()>;
    pub async fn get(&self, id: &str) -> Result<Option<Job>>;
    pub async fn list(&self, filter: &JobFilter) -> Result<Vec<Job>>;
    pub async fn update(&self, job: &Job) -> Result<()>;
    pub async fn transition(&self, id: &str, to: JobStatus, actor: &str, note: Option<&str>) -> Result<Job>;
}

#[derive(Debug, Default, Deserialize)]
pub struct JobFilter {
    pub status: Option<JobStatus>,
    pub kind: Option<String>,
    pub domain: Option<String>,
    pub assignee: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}
```

Each job is stored as `.roko/jobs/{id}.json`. The store does directory listing + deserialization for list queries (fine for the expected scale of <1000 jobs).

**Part 3: HTTP routes (`routes/jobs.rs`)**

```
POST   /api/jobs                -- create a job (Operator+)
GET    /api/jobs                -- list jobs with filters
GET    /api/jobs/:id            -- get job by id
PUT    /api/jobs/:id/status     -- transition job status (Operator+)
POST   /api/jobs/:id/assign     -- assign job to agent (Operator+)
POST   /api/jobs/:id/submit     -- submit deliverables (Operator+)
POST   /api/jobs/:id/evaluate   -- evaluate submission (Admin)
DELETE /api/jobs/:id            -- cancel job (Admin)
```

Each status transition emits a `ServerEvent`:

```rust
// In events.rs, add:
pub enum ServerEvent {
    // ...existing variants...
    JobCreated { job_id: String, kind: String },
    JobAssigned { job_id: String, assignee: String },
    JobStatusChanged { job_id: String, from: JobStatus, to: JobStatus },
    JobSubmitted { job_id: String },
    JobCompleted { job_id: String, score: f64 },
    JobRejected { job_id: String, feedback: String },
}
```

**Acceptance criteria:**
- [ ] `POST /api/jobs` creates a job and returns it with an id
- [ ] `GET /api/jobs` lists jobs with filter support (status, kind, domain)
- [ ] `PUT /api/jobs/:id/status` validates state machine transitions and rejects invalid ones with 422
- [ ] `POST /api/jobs/:id/submit` records deliverables
- [ ] `POST /api/jobs/:id/evaluate` records score and feedback, transitions to Completed or Rejected
- [ ] Job timeline records every status change with actor and timestamp
- [ ] ServerEvents emit on WebSocket for each transition
- [ ] Jobs persist to `.roko/jobs/{id}.json`
- [ ] Jobs survive server restart
- [ ] Invalid transitions return 422 with a message listing valid transitions
- [ ] `cargo test -p roko-core` and `cargo test -p roko-serve` pass
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` clean
- [ ] `cargo +nightly fmt --all` passes

---

### Task 0.7: Job execution pipeline

**Effort:** 2 days
**Dependencies:** Task 0.6

**Files to create:**
- `crates/roko-cli/src/jobs.rs` -- job executor that bridges jobs to roko's existing pipelines

**Files to modify:**
- `crates/roko-cli/src/orchestrate.rs` -- add job-triggered dispatch mode
- `crates/roko-cli/src/main.rs` -- add `roko job` subcommands

**What to implement:**

When an agent picks up a job, it runs through one of roko's existing pipelines depending on the job kind:

```rust
pub struct JobExecutor {
    workdir: PathBuf,
    config: Config,
    serve_url: Option<String>,
}

impl JobExecutor {
    /// Execute a job, updating status via the serve API as it progresses.
    pub async fn execute(&self, job: &Job) -> Result<JobResult> {
        self.transition_status(&job.id, JobStatus::InProgress).await?;

        let result = match &job.kind {
            JobKind::ResearchBrief { topic, depth, max_sources } => {
                self.execute_research(topic, *depth, *max_sources).await
            }
            JobKind::CodingTask { spec, language, test_required } => {
                self.execute_coding(spec, language.as_deref(), *test_required).await
            }
            JobKind::CodeReview { repo_url, pr_number, focus_areas } => {
                self.execute_review(repo_url, *pr_number, focus_areas).await
            }
            JobKind::Custom { description, acceptance_criteria } => {
                self.execute_custom(description, acceptance_criteria).await
            }
        };

        match &result {
            Ok(r) => {
                self.submit_deliverables(&job.id, &r.deliverables, r.notes.as_deref()).await?;
            }
            Err(e) => {
                self.transition_status_with_note(&job.id, JobStatus::Failed, &e.to_string()).await?;
            }
        }

        result
    }
}
```

The key insight: research jobs use `roko research topic`, coding jobs create a plan from spec and run `plan run`, code reviews use the agent dispatch with a review-focused system prompt. No new LLM integration needed -- the existing dispatch infrastructure handles everything.

**Entry points for existing pipelines:**
- Research: `crate::research::cmd_research_topic(topic, &config, &runtime)` in `crates/roko-cli/src/research.rs`
- Coding: `PlanRunner::from_plans_dir(dir, config, runtime)` in `crates/roko-cli/src/orchestrate.rs`
- Add `pub mod job_runner;` to `crates/roko-cli/src/main.rs` (or `lib.rs` if it exists)
- Add `roko job list/show/poll` subcommands to the `Commands` enum in `main.rs` — read the existing enum to see the pattern

CLI surface:

```
roko job list                     -- list available jobs
roko job show <id>                -- show job details
roko job take <id>                -- assign to self and execute
roko job create <kind> "<title>"  -- create a new job
```

**Acceptance criteria:**
- [ ] `roko job list` shows jobs from roko-serve
- [ ] `roko job take <id>` assigns the job, executes it, and submits results
- [ ] Research jobs produce a report via existing `roko research topic` pipeline
- [ ] Coding jobs create a plan, execute it, and submit changed files
- [ ] Status transitions happen automatically during execution
- [ ] Failed jobs transition to Failed with error details
- [ ] `cargo test -p roko-cli` passes
- [ ] `cargo +nightly fmt --all` passes

---

### Task 0.8: Heartbeat protocol

**Effort:** 2 days
**Dependencies:** None (parallel with 0.1-0.7)

**Files to create:**
- `crates/roko-core/src/heartbeat.rs` -- heartbeat payload wire protocol types
- `crates/roko-serve/src/routes/heartbeat.rs` -- ingestion and aggregation routes

**Files to modify:**
- `crates/roko-cli/src/heartbeat.rs` -- extend existing heartbeat to emit to roko-serve
- `crates/roko-cli/src/orchestrate.rs` -- emit heartbeats from the plan runner loop
- `crates/roko-serve/src/routes/mod.rs` -- register heartbeat routes
- `crates/roko-serve/src/state.rs` -- add `heartbeat_aggregator: HeartbeatAggregator` field, initialize in `AppState::new()`

**Naming note:** Create `crates/roko-core/src/heartbeat.rs`. Add `pub mod heartbeat;` to `crates/roko-core/src/lib.rs`. Note: `crates/roko-cli/src/heartbeat.rs` already exists with a local `HeartbeatClock` type — these are DIFFERENT modules in DIFFERENT crates. The core crate defines the wire protocol types; the CLI crate has the local clock implementation. No naming collision.

**What to implement:**

The CLI already has a `HeartbeatClock` and `HeartbeatSnapshot` for local persistence. This task extends it to POST heartbeats to roko-serve, where they are aggregated for the dashboard and TUI.

**Heartbeat payload (`roko-core/src/heartbeat.rs`)**

```rust
/// Network-visible heartbeat from a running agent or orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatPayload {
    pub agent_id: String,
    pub instance_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub status: AgentHeartbeatStatus,
    pub current_task: Option<HeartbeatTaskInfo>,
    pub metrics: HeartbeatMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentHeartbeatStatus {
    Idle,
    Working,
    Gating,
    Waiting,
    Error,
    ShuttingDown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatTaskInfo {
    pub task_id: String,
    pub plan_id: String,
    pub description: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HeartbeatMetrics {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
    pub turns_completed: u32,
    pub gate_pass_rate: Option<f64>,
    pub uptime_secs: u64,
}
```

**Server-side aggregation:**

```rust
// In state.rs
pub struct HeartbeatAggregator {
    /// Most recent heartbeat per agent, keyed by agent_id.
    heartbeats: RwLock<HashMap<String, HeartbeatPayload>>,
    /// Stale threshold -- agents with no heartbeat for this duration are marked stale.
    stale_threshold: Duration,
}

impl HeartbeatAggregator {
    pub async fn ingest(&self, payload: HeartbeatPayload);
    pub async fn all(&self) -> Vec<HeartbeatPayload>;
    pub async fn get(&self, agent_id: &str) -> Option<HeartbeatPayload>;
    pub async fn stats(&self) -> NetworkStats;
    pub async fn prune_stale(&self) -> Vec<String>;
}
```

**Routes:**

```
POST /api/heartbeats          -- ingest a heartbeat
GET  /api/heartbeats          -- list all recent heartbeats
GET  /api/heartbeats/:id      -- get heartbeat for one agent
GET  /api/network/stats       -- aggregated network stats
```

**Acceptance criteria:**
- [ ] `POST /api/heartbeats` stores the heartbeat and returns 202
- [ ] `GET /api/heartbeats` returns all heartbeats within the stale window
- [ ] `GET /api/network/stats` returns agent count, active count, total tokens, total cost
- [ ] Agents older than `stale_threshold` (default 5 min) are pruned
- [ ] Plan runner emits heartbeats every 30 seconds during execution
- [ ] `cargo test -p roko-core` and `cargo test -p roko-serve` pass
- [ ] `cargo +nightly fmt --all` passes

---

### Task 0.9: CLI auth flow

**Effort:** 1 day
**Dependencies:** Task 0.4

**Files to create:**
- `crates/roko-cli/src/auth.rs` -- login/logout/whoami commands

**Files to modify:**
- `crates/roko-cli/src/main.rs` -- add auth subcommands

**What to implement:**

```
roko login         -- open browser for SSO, store JWT in ~/.roko/credentials.json
roko logout        -- delete stored credentials
roko whoami        -- print current identity from stored JWT or API key
```

Login flow:
1. Start a local HTTP server on a random port
2. Open `{sso_url}?redirect_uri=http://localhost:{port}/callback` in the default browser
3. Wait for the callback with `?token=<jwt>` query parameter
4. Validate the JWT
5. Write `~/.roko/credentials.json`:

```json
{
  "token": "<jwt>",
  "subject": "will@example.com",
  "role": "admin",
  "expires_at": "2026-05-21T00:00:00Z",
  "issuer": "privy"
}
```

All subsequent CLI requests to roko-serve include `Authorization: Bearer <jwt>`.

**Config:** Add `sso_url: Option<String>` to `ServeAuthConfig` in `crates/roko-core/src/config/schema.rs`. Default: `None`. When None, `roko login` prints an error saying SSO is not configured.

**Local HTTP server:** The callback listener needs a temporary HTTP server. Use `axum` (already a dependency of `roko-serve`) or `hyper` (already in workspace). Bind to `127.0.0.1:0` (OS-assigned port), extract the port, use it in the redirect_uri.

**Credentials path:** Use `dirs::home_dir().unwrap().join(".roko").join("credentials.json")`. The `dirs` crate is already in the workspace.

**Acceptance criteria:**
- [ ] `roko login` opens browser and stores JWT on callback
- [ ] `roko logout` deletes credentials file
- [ ] `roko whoami` prints subject and role
- [ ] Stored JWT is used for roko-serve API calls
- [ ] Expired JWT prints a warning suggesting `roko login`
- [ ] `cargo test -p roko-cli` passes
- [ ] `cargo +nightly fmt --all` passes

---

### Task 0.10: PlanRunner constructor dedup

**Effort:** 4 hours
**Dependencies:** None

**Files to modify:**
- `crates/roko-cli/src/orchestrate.rs` -- extract shared init from `from_plans_dir`, `from_snapshot`, `from_snapshots`

**What to implement:**

The three `PlanRunner` constructors share about 200 lines of identical initialization (supervisor setup, conductor setup, gate pipeline config, heartbeat clock, etc.). Extract this into a private `PlanRunnerInit` struct:

```rust
struct PlanRunnerInit {
    supervisor: Arc<ProcessSupervisor>,
    conductor: Conductor,
    gate_pipeline: GatePipeline,
    heartbeat_clock: HeartbeatClock,
    episode_logger: EpisodeLogger,
    efficiency_sink: Option<FsObservabilitySinks>,
    cancel: CancelToken,
    // ... all shared fields
}

impl PlanRunnerInit {
    fn new(workdir: &Path, config: &Config, roko_config: &RokoConfig) -> Result<Self> {
        // All the shared initialization logic, extracted once
    }
}

impl PlanRunner {
    pub fn from_plans_dir(plans_dir: &Path, config: &Config, ...) -> Result<Self> {
        let init = PlanRunnerInit::new(workdir, config, roko_config)?;
        let plans = discover_plans(plans_dir)?;
        // ... plan-specific setup
        Ok(Self::from_init(init, plans, ...))
    }

    pub fn from_snapshot(snapshot: ExecutorSnapshot, ...) -> Result<Self> {
        let init = PlanRunnerInit::new(workdir, config, roko_config)?;
        // ... snapshot-specific setup
        Ok(Self::from_init(init, plans, ...))
    }

    fn from_init(init: PlanRunnerInit, plans: Vec<Plan>, ...) -> Self {
        // Common constructor body
    }
}
```

**Acceptance criteria:**
- [ ] All three constructors share a single initialization path
- [ ] No behavior change -- existing tests pass
- [ ] `cargo test -p roko-cli` passes
- [ ] `cargo clippy -p roko-cli --no-deps -- -D warnings` clean
- [ ] `cargo +nightly fmt --all` passes

---

### Task 0.11: Cascade router force_backend learning

**Effort:** 4 hours
**Dependencies:** None
**Fixes:** UX34

**Files to modify:**
- `crates/roko-cli/src/orchestrate.rs` -- in `dispatch_agent_with`, feed force_backend outcomes to cascade router

**What to implement:**

When a user sets `force_backend` in `roko.toml` or via CLI, the cascade router is bypassed entirely. The router never learns from these forced runs, missing data that would improve future routing decisions.

The actual `CascadeRouter` API uses `record_outcome(model_slug: &str, success: bool)`. In `dispatch_agent_with()`, after the agent result is known, if a `force_backend` was used, call `self.cascade_router.record_outcome(&forced_model, gate_result.passed())`. This ensures forced overrides contribute to the router's learning. No new `CascadeOutcome` struct is needed.

```rust
// In dispatch_agent_with, after agent completes:
if let Some(forced_backend) = task_config.force_backend.as_ref() {
    if let Some(cascade_router) = self.cascade_router.as_ref() {
        cascade_router.record_outcome(forced_backend, gate_result.passed());
        cascade_router.persist(&self.cascade_router_path)?;
    }
}
```

**Acceptance criteria:**
- [ ] Force_backend completions are recorded in cascade router
- [ ] Cascade router JSON file reflects forced outcomes
- [ ] No new `CascadeOutcome` struct introduced
- [ ] No change to non-forced routing behavior
- [ ] `cargo test -p roko-cli` passes
- [ ] `cargo +nightly fmt --all` passes

---

## Phase 1: Nexus relay (1.5 weeks)

The Nexus is a lightweight WebSocket relay that gives both the dashboard and TUI a single connection point for network-wide agent data. Without it, every surface has to poll roko-serve independently, and there is no way to see agents running on other machines.

---

### Task 1.1: Nexus core server

**Effort:** 5 days
**Dependencies:** Phase 0 complete

**Files to create (new crate `crates/roko-nexus/`):**
- `Cargo.toml`
- `src/lib.rs` -- crate root, re-exports
- `src/server.rs` -- tokio + tungstenite WebSocket server
- `src/protocol.rs` -- JSON-RPC 2.0 message types
- `src/room.rs` -- room system (per-atelier, per-domain, global)
- `src/presence.rs` -- agent presence directory
- `src/heartbeat.rs` -- heartbeat aggregation from connected clients
- `src/stats.rs` -- network stats computation
- `src/auth.rs` -- signed challenge or API key auth

**Workspace setup:**
1. Add `"crates/roko-nexus"` to the `members` array in the root `/Users/will/dev/nunchi/roko/roko/Cargo.toml`
2. Add `roko-nexus = { path = "crates/roko-nexus" }` to `[workspace.dependencies]`
3. The `Cargo.toml` for roko-nexus should include: `tokio`, `tokio-tungstenite`, `serde`, `serde_json`, `uuid`, `chrono`, `tracing` — all with `.workspace = true`

**What to implement:**

The Nexus is a standalone WebSocket server. It does not execute agent tasks -- it relays messages between participants and aggregates heartbeats.

**Protocol (`protocol.rs`)**

JSON-RPC 2.0 over WebSocket:

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct RpcMessage {
    pub jsonrpc: String,  // "2.0"
    pub id: Option<u64>,
    pub method: Option<String>,
    pub params: Option<Value>,
    pub result: Option<Value>,
    pub error: Option<RpcError>,
}

/// Methods the Nexus handles:
///
/// Client -> Nexus:
///   "auth.challenge"      -- request a challenge nonce
///   "auth.verify"         -- submit signed challenge
///   "room.join"           -- join a room
///   "room.leave"          -- leave a room
///   "room.broadcast"      -- send message to all room members
///   "heartbeat.send"      -- submit a heartbeat
///   "presence.query"      -- query connected agents
///   "stats.query"         -- query network stats
///
/// Nexus -> Client (notifications, no id):
///   "room.message"        -- message from another room member
///   "presence.update"     -- agent joined/left
///   "heartbeat.aggregate" -- periodic heartbeat summary
```

**Room system (`room.rs`)**

```rust
pub struct Room {
    pub id: RoomId,
    pub kind: RoomKind,
    pub members: HashSet<ClientId>,
    pub created_at: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RoomKind {
    Global,                          // All connected clients
    Atelier { workspace_id: String }, // Per-workspace
    Domain { tag: String },          // Per-domain (e.g., "code", "research")
}
```

Every connected client automatically joins the `Global` room. Clients can join additional rooms via `room.join`.

**Server (`server.rs`)**

```rust
pub struct NexusServer {
    pub config: NexusConfig,
    clients: RwLock<HashMap<ClientId, ClientHandle>>,
    rooms: RwLock<HashMap<RoomId, Room>>,
    presence: PresenceDirectory,
    heartbeats: HeartbeatAggregator,
}

pub struct NexusConfig {
    pub bind: SocketAddr,
    pub max_connections: usize,
    pub heartbeat_interval: Duration,
    pub stale_timeout: Duration,
    pub auth_required: bool,
    pub api_keys: Vec<String>,
}

impl NexusServer {
    pub async fn run(self) -> Result<()>;
}
```

**CLI integration:**

```
roko nexus start                -- start the nexus server
roko nexus start --port 7788   -- start on a specific port
```

**Acceptance criteria:**
- [ ] Nexus accepts WebSocket connections and authenticates clients
- [ ] `room.join` / `room.leave` / `room.broadcast` work correctly
- [ ] `heartbeat.send` stores heartbeats and prunes stale ones
- [ ] `presence.query` returns connected agents
- [ ] `stats.query` returns aggregate network stats
- [ ] Clients receive `presence.update` notifications when agents join/leave
- [ ] Periodic `heartbeat.aggregate` notifications sent to Global room
- [ ] Load test: 50 concurrent clients exchanging messages without deadlock
- [ ] `cargo test -p roko-nexus` passes
- [ ] `cargo clippy -p roko-nexus --no-deps -- -D warnings` clean

---

### Task 1.2: CLI nexus integration

**Effort:** 1.5 days
**Dependencies:** Task 1.1

**Files to create:**
- `crates/roko-cli/src/nexus.rs` -- nexus connect/status commands and background client

**Files to modify:**
- `crates/roko-cli/src/main.rs` -- add nexus subcommands
- `crates/roko-cli/src/orchestrate.rs` -- auto-connect to nexus when `nexus.url` is set in roko.toml
- `crates/roko-core/src/config/schema.rs` -- add `NexusConfig` section
- `crates/roko-cli/Cargo.toml` -- add `roko-nexus.workspace = true`

**What to implement:**

```toml
# roko.toml
[nexus]
url = "ws://localhost:7788"
auto_connect = true
api_key = "..."
```

```
roko nexus connect              -- connect to nexus and print status
roko nexus status               -- show connection state and connected agents
roko nexus disconnect           -- disconnect from nexus
```

When `nexus.auto_connect = true`, the plan runner connects to the Nexus at startup and:
- Sends heartbeats every 30 seconds
- Joins the workspace's atelier room
- Emits agent lifecycle events (started, completed, failed)

The NexusClient is a long-lived background task:

```rust
pub struct NexusClient {
    endpoint: String,
    tx: mpsc::Sender<NexusCommand>,
    rx: mpsc::Receiver<NexusEvent>,
    task: JoinHandle<()>,
}

pub enum NexusCommand {
    SendHeartbeat(HeartbeatPayload),
    JoinRoom(RoomId),
    Broadcast { room: RoomId, message: Value },
    Disconnect,
}

pub enum NexusEvent {
    Connected,
    RoomMessage { room: RoomId, from: ClientId, message: Value },
    PresenceUpdate { agent_id: String, online: bool },
    HeartbeatAggregate(Vec<HeartbeatPayload>),
    Disconnected { reason: String },
}
```

**Acceptance criteria:**
- [ ] `roko nexus connect` connects and prints status
- [ ] `roko nexus status` shows connected agents from nexus
- [ ] Plan runner auto-connects when config is set
- [ ] Heartbeats sent every 30 seconds
- [ ] Reconnects on connection loss with exponential backoff
- [ ] `cargo test -p roko-cli` passes
- [ ] `cargo +nightly fmt --all` passes

---

### Task 1.3: roko-serve nexus bridge

**Effort:** 1.5 days
**Dependencies:** Task 1.1

**Files to create:**
- `crates/roko-serve/src/routes/nexus.rs` -- nexus status and relay routes
- `crates/roko-serve/src/nexus_bridge.rs` -- background nexus client

**Files to modify:**
- `crates/roko-serve/src/routes/mod.rs` -- register nexus routes
- `crates/roko-serve/src/state.rs` -- add nexus client handle
- `crates/roko-serve/Cargo.toml` -- add `roko-nexus.workspace = true`

**What to implement:**

roko-serve connects to the Nexus as a client and bridges its own agents:

```rust
pub struct NexusBridge {
    client: NexusClient,
    /// Forward agent registrations to nexus presence.
    /// Forward heartbeats from /api/heartbeats to nexus.
    /// Relay room messages to SSE/WebSocket subscribers.
}
```

Routes:

```
GET /api/nexus/status        -- nexus connection status
GET /api/nexus/agents        -- all agents visible via nexus (network-wide)
GET /api/nexus/stats         -- network stats from nexus
```

**Acceptance criteria:**
- [ ] roko-serve auto-connects to nexus when config is set
- [ ] Agent registrations are forwarded to nexus presence
- [ ] `/api/nexus/agents` returns network-wide agent list
- [ ] `/api/nexus/stats` returns network stats
- [ ] Nexus messages are relayed to SSE subscribers
- [ ] `cargo test -p roko-serve` passes

---

## Phase 2: Dashboard complete redesign (3 weeks)

Full rewrite. Every component rebuilt from scratch. The current dashboard is at `/Users/will/dev/nunchi/nunchi-dashboard`.

---

### Task 2.1: Project setup and design system

**Effort:** 2 days
**Dependencies:** Phase 0 complete

**Files to create:**
- `package.json` -- add React Router, Zustand, TanStack Query
- `src/design-system/tokens.ts` -- ROSEDUST color tokens, spacing scale, typography
- `src/design-system/primitives.ts` -- base component primitives (Box, Stack, Text, etc.)
- `src/design-system/theme.ts` -- theme provider with dark/light/system support
- `src/stores/auth.ts` -- Zustand auth store (JWT, API key, identity)
- `src/stores/settings.ts` -- Zustand settings store (theme, density, notifications)

**What to implement:**

**Design tokens must match IMPL-10-DEMO.md Task A1.** Use CSS custom properties in `src/index.css` (not TypeScript). The canonical values are:
- `--bg-void: #060608` (from existing index.css)
- `--rose: #AA7088` (from existing index.css)
- `--bone: #C8B890`
- `--rose-bright: #CC90A8`

Keep the existing dark mode palette from `index.css` as the base and extend it.

**Design tokens (`tokens.ts`):**

```typescript
export const colors = {
  // ROSEDUST palette (matches TUI theme and index.css CSS custom properties)
  bg: { primary: '#060608', secondary: '#121218', tertiary: '#1a1a24' },
  fg: { primary: '#e8e0d8', secondary: '#b0a898', muted: '#6b6560' },
  accent: { rose: '#c47070', amber: '#c4a070', jade: '#70c490' },
  status: {
    running: '#70a0c4',
    passed: '#70c490',
    failed: '#c47070',
    pending: '#c4a070',
    stale: '#6b6560',
  },
  border: { default: '#2a2a34', focused: '#c47070' },
} as const;

export const spacing = {
  xs: '4px', sm: '8px', md: '16px', lg: '24px', xl: '32px', xxl: '48px',
} as const;

export const typography = {
  mono: "'JetBrains Mono', 'Fira Code', monospace",
  sans: "'Inter', -apple-system, sans-serif",
  sizes: { xs: '11px', sm: '12px', md: '14px', lg: '16px', xl: '20px', xxl: '24px' },
} as const;

export const breakpoints = {
  mobile: 768,
  tablet: 1024,
  desktop: 1440,
  ultrawide: 1920,
} as const;
```

**Zustand stores:**

Replace all React state + useEffect polling with Zustand stores that manage server state via TanStack Query. Each store owns one concern:

- `auth.ts` -- JWT/API key, identity, login/logout actions
- `settings.ts` -- theme, display density, notification preferences

**Acceptance criteria:**
- [ ] Design tokens are importable throughout the app
- [ ] Theme switching works (dark/light/system)
- [ ] Zustand stores work without react-query (unit testable)
- [ ] No more inline color hex codes in components
- [ ] Typography scale applied globally

---

### Task 2.2: Layout shell and routing

**Effort:** 2 days
**Dependencies:** Task 2.1

**Files to create:**
- `src/App.tsx` -- complete rewrite with React Router
- `src/layouts/AppLayout.tsx` -- three-column responsive layout
- `src/layouts/TopBar.tsx` -- breadcrumbs, auth, network pulse indicator
- `src/layouts/SideNav.tsx` -- collapsible left navigation
- `src/layouts/ContextPanel.tsx` -- collapsible right context panel
- `src/routes/index.ts` -- route definitions

**What to implement:**

Route structure:

```
/                          -- Landing page
/command/chat              -- Chat with agents
/command/research          -- Research interface
/observatory/agents        -- Live agent monitoring
/observatory/plans         -- Plan management
/observatory/learning      -- Learning metrics
/observatory/conductor     -- Conductor status
/observatory/costs         -- Cost tracking
/network/topology          -- Agent network graph
/network/pheromones        -- Pheromone heatmap
/network/knowledge         -- Knowledge graph
/network/swarm             -- Swarm coordination
/marketplace/jobs          -- Job board
/marketplace/create        -- Create job
/marketplace/jobs/:id      -- Job detail
/agent-studio/:id          -- Agent detail
/agent-studio/:id/strategy -- Agent strategy config
/agent-studio/:id/keys     -- Agent API keys
/agent-studio/:id/deploy   -- Agent deployment
/atelier                   -- Workspace dashboard
/atelier/prds              -- PRD browser
/atelier/execution         -- Execution monitor
/settings                  -- Settings
```

The `AppLayout` is a responsive three-column layout:

```
+------------------+-------------------+------------------+
| SideNav          | Main Content      | ContextPanel     |
| (collapsible)    | (scrollable)      | (collapsible)    |
| 240px / 0px      | flex-1            | 320px / 0px      |
+------------------+-------------------+------------------+
```

SideNav collapses to icon-only on tablet, fully hidden on mobile. ContextPanel hidden below desktop.

**Acceptance criteria:**
- [ ] React Router handles all routes with proper code splitting
- [ ] Layout is responsive across all breakpoints
- [ ] SideNav highlights the active section
- [ ] TopBar shows breadcrumbs derived from the current route
- [ ] Network pulse indicator shows connection status
- [ ] No route renders blank content (fallback 404 page)

---

### Task 2.3: API layer rewrite

**Effort:** 2 days
**Dependencies:** Task 2.1

**Files to create:**
- `src/services/api.ts` -- TanStack Query-based API client
- `src/services/ws.ts` -- single WebSocket connection with event routing
- `src/services/nexus.ts` -- Nexus WebSocket client
- `src/hooks/useApi.ts` -- typed query hooks for every API endpoint
- `src/hooks/useWs.ts` -- WebSocket event subscription hooks

**What to implement:**

Replace all manual `fetch` + `setInterval` polling with TanStack Query:

```typescript
// api.ts
import { QueryClient } from '@tanstack/react-query';

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 5_000,
      retry: 3,
      retryDelay: (attempt) => Math.min(1000 * 2 ** attempt, 30_000),
    },
  },
});

export async function apiFetch<T>(path: string, options?: RequestInit): Promise<T> {
  const { token, apiKey } = useAuthStore.getState();
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  };
  if (token) headers['Authorization'] = `Bearer ${token}`;
  else if (apiKey) headers['X-Api-Key'] = apiKey;

  const response = await fetch(`${BASE_URL}${path}`, { ...options, headers: { ...headers, ...options?.headers } });
  if (!response.ok) throw new ApiError(response.status, await response.text());
  return response.json();
}
```

```typescript
// hooks/useApi.ts
export function useJobs(filter?: JobFilter) {
  return useQuery({
    queryKey: ['jobs', filter],
    queryFn: () => apiFetch<Job[]>('/api/jobs', { params: filter }),
  });
}

export function useAgents() {
  return useQuery({
    queryKey: ['agents'],
    queryFn: () => apiFetch<Agent[]>('/api/managed-agents'),
    refetchInterval: 10_000,
  });
}

// ... one hook per API endpoint
```

WebSocket manager:

```typescript
// ws.ts
export class WsManager {
  private ws: WebSocket | null = null;
  private listeners: Map<string, Set<(data: any) => void>> = new Map();
  private reconnectDelay = 1000;

  connect(url: string): void;
  disconnect(): void;
  on(event: string, callback: (data: any) => void): () => void;
  private handleMessage(msg: MessageEvent): void;
  private reconnect(): void;
}
```

The WsManager maintains a single WebSocket connection to roko-serve `/ws` and routes events to subscribers by event type. No more 14 parallel polling timers.

**Zero mock data contamination.** Every API hook returns `undefined` while loading and the actual API response when ready. Components show a loading skeleton, not fake data. If an API endpoint does not exist yet, the hook is marked `enabled: false` with a TODO comment.

**Acceptance criteria:**
- [ ] All API calls go through TanStack Query
- [ ] Request deduplication works (two components requesting the same data make one fetch)
- [ ] Exponential backoff on failures
- [ ] Auth headers injected automatically
- [ ] WebSocket manager handles reconnection
- [ ] No `setInterval` or `setTimeout` polling anywhere
- [ ] No mock data mixed with API responses
- [ ] TypeScript strict mode passes

---

### Task 2.4: Landing page

**Effort:** 2 days
**Dependencies:** Task 2.2

**Files to create:**
- `src/pages/Landing.tsx` -- landing page container
- `src/pages/landing/HeroSection.tsx` -- hero with system status
- `src/pages/landing/ArchitectureExplorer.tsx` -- interactive SVG/Canvas system diagram
- `src/pages/landing/NetworkStats.tsx` -- live stats from roko-serve or Nexus

**What to implement:**

The landing page communicates what roko is and what it is doing right now. Three sections:

1. **Hero**: System name, current state (idle / running N tasks / N agents active), uptime, cost since last reset. Data from `GET /api/status`.

2. **Architecture explorer**: Interactive SVG diagram showing the cognitive loop (query -> score -> route -> compose -> act -> verify -> write -> react). Each step is clickable and expands to show what that step is doing right now (e.g., "act: agent-3 running task fix-clippy-warnings").

3. **Network stats**: Agent count, total tokens processed, gate pass rate, active plans, active jobs. Data from `GET /api/network/stats` or Nexus.

Each section starts collapsed to a one-liner and expands on click for progressive depth.

**Acceptance criteria:**
- [ ] Landing page loads in under 2 seconds
- [ ] Architecture explorer is interactive and shows live system state
- [ ] Network stats update via WebSocket (not polling)
- [ ] Responsive on mobile
- [ ] Accessible (keyboard navigable, screen reader labels)

---

### Task 2.5: Command pages (Chat and Research)

**Effort:** 2 days
**Dependencies:** Task 2.3

**Files to create:**
- `src/pages/command/ChatPage.tsx` -- chat interface
- `src/pages/command/ResearchPage.tsx` -- research interface
- `src/pages/command/components/AgentSelector.tsx` -- dropdown of discovered agents
- `src/pages/command/components/MessageBubble.tsx` -- chat message with citations
- `src/pages/command/components/ResearchReport.tsx` -- rendered research output

**What to implement:**

**Chat page:**
- Agent selector dropdown populated from `GET /api/managed-agents` and discovered agents
- Message input with Shift+Enter for newlines
- Streaming response rendering via WebSocket or SSE from `POST /api/agents/:id/message`
- Citation cards when the agent references sources
- Conversation history (stored client-side in IndexedDB)

**Research page:**
- Topic input with depth selector (Quick/Standard/Deep)
- Submit to `POST /api/research/topic`
- Poll for results via `GET /api/research/:id`
- Render completed report with collapsible sections, source links, confidence scores
- Research history sidebar

Remove all `setTimeout` simulation. Every interaction hits a real API endpoint.

**Acceptance criteria:**
- [ ] Chat sends messages to real agents and renders streaming responses
- [ ] Agent selector shows all discovered agents
- [ ] Research submits to real API and renders real reports
- [ ] No setTimeout or simulated delays
- [ ] Loading states while waiting for responses

---

### Task 2.6: Observatory pages

**Effort:** 4 days
**Dependencies:** Task 2.3, Task 0.8 (heartbeats)

**Files to create:**
- `src/pages/observatory/AgentsPage.tsx` -- live agent monitoring
- `src/pages/observatory/PlansPage.tsx` -- plan management with DAG view
- `src/pages/observatory/LearningPage.tsx` -- C-Factor, experiments, gate rates
- `src/pages/observatory/ConductorPage.tsx` -- circuit breakers, watchers
- `src/pages/observatory/CostsPage.tsx` -- cost tracking per agent/model

**What to implement:**

**Agents page:**
- Card grid of active agents, each card showing: agent name, status (from heartbeat), current task, token burn sparkline, uptime, tier badge
- Click to navigate to `/agent-studio/:id`
- Real-time updates via WebSocket events (`AgentOutput`, `RunStarted`, `RunCompleted`)
- Conductor alert banner when circuit breakers are open

**Plans page:**
- Plan list from `GET /api/plans`
- DAG visualization using a tree layout (SVG). Each node is a task with status color coding.
- Click a plan to see its tasks. Click a task to see details.
- "Execute" button that calls `POST /api/plans/:id/run`
- Progress bar per plan (completed tasks / total tasks)

**Learning page:**
- C-Factor trend chart (from `GET /api/learning/cfactor`)
- Gate pass rates over time (from `GET /api/learning/gate-stats`)
- A/B experiment results table (from `GET /api/learning/experiments`)
- Model routing decisions log (from `GET /api/learning/routing`)
- Cost tier comparison (from `GET /api/models`)

**Conductor page:**
- Circuit breaker status per agent (from `GET /api/diagnosis/circuit-breakers`)
- Watcher status list
- Intervention history timeline
- Error pattern summary

**Costs page:**
- Per-agent cost table with sparklines
- Per-model cost table with sparklines
- Provider health status (from `GET /api/providers/health`)
- Budget usage gauge
- Date range selector for historical data

**Acceptance criteria:**
- [ ] Each page loads data from real API endpoints
- [ ] Agent cards update in real-time via WebSocket
- [ ] Plan DAG renders correctly for plans with 50+ tasks
- [ ] Learning charts render with real data
- [ ] Cost sparklines show last 24 hours of data
- [ ] Each page has a loading skeleton state
- [ ] Each page has an empty state with helpful text

---

### Task 2.7: Network pages

**Effort:** 3 days
**Dependencies:** Task 2.3, Task 1.3 (nexus bridge)

**Files to create:**
- `src/pages/network/TopologyPage.tsx` -- force-directed agent graph
- `src/pages/network/PheromoneFieldPage.tsx` -- pheromone heatmap
- `src/pages/network/KnowledgeGraphPage.tsx` -- interactive knowledge explorer
- `src/pages/network/SwarmPage.tsx` -- swarm coordination view

**What to implement:**

**Topology page:**
- Force-directed graph (use `d3-force` or `react-force-graph-2d`)
- Nodes are agents, edges are communication channels
- Node size proportional to token throughput
- Node color by status (from heartbeat)
- Click node to see agent details in ContextPanel
- Data from `GET /api/nexus/agents` or `GET /api/managed-agents`

**Pheromone field page:**
- Heatmap of pheromone deposits across domains
- X-axis: domain tags, Y-axis: time (last 24h)
- Cell intensity: pheromone strength
- Data from `GET /api/aggregator/pheromones`

**Knowledge graph page:**
- Interactive graph of the InsightStore
- Nodes are engrams/knowledge entries
- Edges are provenance links
- Search bar to filter by keyword or domain
- Click a node to see full content in ContextPanel
- Data from `GET /api/aggregator/insights`

**Swarm page:**
- Timeline view of agent coordination
- Shows which agents are working on which tasks, when they communicate, and how work flows between them
- Data from WebSocket events

**Acceptance criteria:**
- [ ] Topology graph renders 20+ agents without performance issues
- [ ] Pheromone heatmap renders with real data
- [ ] Knowledge graph supports search and filtering
- [ ] All pages handle empty state (no agents, no data)
- [ ] Graph interactions are smooth (60fps pan/zoom)

---

### Task 2.8: Marketplace pages

**Effort:** 2 days
**Dependencies:** Task 2.3, Task 0.6 (jobs backend)

**Files to create:**
- `src/pages/marketplace/JobBoardPage.tsx` -- filterable job list
- `src/pages/marketplace/CreateJobPage.tsx` -- job creation form
- `src/pages/marketplace/JobDetailPage.tsx` -- full lifecycle view

**What to implement:**

**Job board:**
- Filterable list of jobs from `GET /api/jobs`
- Filters: status, kind, domain, bounty range
- Sort by: newest, deadline, bounty amount
- Cards show: title, kind badge, status badge, bounty amount, domain tags, deadline countdown
- Click to navigate to `/marketplace/jobs/:id`

**Create job:**
- Multi-step form:
  1. Select kind (Research / Coding / Review / Custom)
  2. Kind-specific fields (topic for research, spec for coding, etc.)
  3. Domain tags, deadline, bounty amount
  4. Review and submit
- Submits to `POST /api/jobs`
- Quick-create buttons for common patterns (research bounty, coding bounty)

**Job detail:**
- Full job info with timeline
- Status badge with transition history
- Deliverables section (rendered markdown)
- Evaluation section (score, feedback)
- Action buttons: Assign, Submit, Evaluate (role-gated)

**Acceptance criteria:**
- [ ] Job board loads from real API
- [ ] Filters work correctly
- [ ] Create job form validates inputs
- [ ] Job detail shows full lifecycle
- [ ] Action buttons are gated by user role
- [ ] Status transitions update in real-time via WebSocket

---

### Task 2.9: Agent studio pages

**Effort:** 2 days
**Dependencies:** Task 2.3, Task 0.8 (heartbeats)

**Files to create:**
- `src/pages/agent-studio/OverviewPage.tsx` -- agent dashboard
- `src/pages/agent-studio/StrategyPage.tsx` -- strategy config
- `src/pages/agent-studio/KeysPage.tsx` -- API key management
- `src/pages/agent-studio/DeployPage.tsx` -- deployment

**What to implement:**

**Overview:**
- Agent identity card (name, id, status, uptime)
- Heartbeat history sparkline
- Episode timeline (last 20 episodes from `GET /api/agents/:id/episodes`)
- Cognitive trace: last 10 reasoning steps with timing
- Cost summary: tokens, cost, model breakdown

All data from real API endpoints. Works for every agent, not hardcoded to mocks.

**Strategy:**
- Skill/extension toggle grid
- Config values editable and persisted via `PUT /api/agents/:id/config` (or similar)
- Changes save to roko-serve, not lost on refresh

**Keys:**
- List API keys from `GET /api/agents/:id/token`
- Create new key via `POST /api/agents/:id/token`
- Revoke key via `DELETE /api/agents/:id/token`

**Deploy:**
- Deploy target selector (Railway, Manual)
- One-click deploy button
- Deployment status from `GET /api/deployments`
- Generated binding code (copy-to-clipboard)

**Acceptance criteria:**
- [ ] Overview shows real data for any discovered agent
- [ ] Strategy changes persist across page refresh
- [ ] Key CRUD works end-to-end
- [ ] Deploy triggers a real deployment
- [ ] All pages handle the "agent not found" case

---

### Task 2.10: Atelier (workspace) pages

**Effort:** 2 days
**Dependencies:** Task 2.3

**Files to create:**
- `src/pages/atelier/WorkspacePage.tsx` -- workspace dashboard
- `src/pages/atelier/PrdBrowserPage.tsx` -- PRD list, create, view
- `src/pages/atelier/ExecutionMonitorPage.tsx` -- live plan execution

**What to implement:**

**Workspace dashboard:**
- Active plans count and progress from `GET /api/plans`
- Agent pool: which agents are available, their current assignments
- PRD coverage: plans/tasks/done ratio from `GET /api/prds/status`
- Recent activity feed from WebSocket events

**PRD browser:**
- List PRDs from `GET /api/prds`
- Create new PRD via `POST /api/prds`
- View PRD details in ContextPanel
- Promote draft to published via `POST /api/prds/:slug/promote`
- Generate plan from PRD via `POST /api/prds/:slug/plan`

**Execution monitor:**
- Select a running plan from dropdown
- Show live task-by-task progress via WebSocket events
- Each task shows: status, assigned agent, gate result, duration
- Log tail for the selected task

**Acceptance criteria:**
- [ ] Workspace dashboard shows real workspace state
- [ ] PRD list, create, and view work end-to-end
- [ ] Promote and plan generation work from the UI
- [ ] Execution monitor updates in real-time
- [ ] All pages handle empty workspace state

---

### Task 2.11: Settings page

**Effort:** 1 day
**Dependencies:** Task 2.2

**Files to create:**
- `src/pages/settings/SettingsPage.tsx`

**What to implement:**

- Config editor: read config from `GET /api/config`, edit values, save with `PUT /api/config`
- Theme selector (dark / light / system) -- persisted in local storage
- Display density (compact / default / comfortable) -- persisted in local storage
- Notification preferences (which events show toast notifications)
- Connection info: roko-serve URL, Nexus URL, connection status
- About: version, build info

**Acceptance criteria:**
- [ ] Config changes persist to roko.toml via the API
- [ ] Theme switching works immediately
- [ ] Settings survive page refresh (stored in localStorage)

---

### Task 2.12: Shared components

**Effort:** 3 days
**Dependencies:** Task 2.1

**Files to create:**
- `src/components/shared/StatCard.tsx` -- metric card with optional sparkline
- `src/components/shared/GaugeBar.tsx` -- horizontal gauge with thresholds
- `src/components/shared/Sparkline.tsx` -- inline sparkline chart (SVG)
- `src/components/shared/Badge.tsx` -- status badge with color coding
- `src/components/shared/Timeline.tsx` -- vertical timeline with entries
- `src/components/shared/ActivityFeed.tsx` -- scrollable activity feed
- `src/components/shared/Modal.tsx` -- modal dialog
- `src/components/shared/Toast.tsx` -- toast notification
- `src/components/shared/AgentCard.tsx` -- agent summary card
- `src/components/shared/PlanTree.tsx` -- plan task tree
- `src/components/shared/TaskChecklist.tsx` -- task checkbox list
- `src/components/shared/LoadingSkeleton.tsx` -- loading placeholders
- `src/components/shared/EmptyState.tsx` -- empty state with icon and message
- `src/components/shared/ErrorBoundary.tsx` -- error boundary with retry

**What to implement:**

Each component follows the design system tokens. All components are:
- Responsive (work on mobile through ultrawide)
- Accessible (ARIA labels, keyboard navigation)
- Themed (use tokens, not hardcoded colors)
- Typed (full TypeScript interfaces for props)

Example `AgentCard` props:

```typescript
interface AgentCardProps {
  agent: {
    id: string;
    label?: string;
    status: 'idle' | 'working' | 'error' | 'stale';
    currentTask?: string;
    uptime?: number;
    tokensBurned?: number;
    costUsd?: number;
    tier?: string;
    heartbeatAge?: number;
  };
  onClick?: () => void;
  compact?: boolean;
}
```

The `AgentCard` renders:
- Heartbeat dot (green pulsing = active, yellow = idle, red = error, gray = stale)
- Agent name and id
- Current task (truncated with tooltip)
- Token burn mini-sparkline (last 20 data points)
- Tier badge (sonnet / opus / haiku)
- Cost since session start

**Acceptance criteria:**
- [ ] Each component renders correctly in isolation (Storybook or test)
- [ ] All components use design system tokens
- [ ] All interactive components are keyboard navigable
- [ ] AgentCard works with minimal data (only id required)
- [ ] Sparkline handles 0 data points, 1 data point, and 100+ data points
- [ ] Modal traps focus correctly
- [ ] Toast auto-dismisses after configurable timeout

---

## Phase 3: TUI enhancements (2 weeks)

Add missing tabs, implement stubbed sub-views, fix bugs, and port bardo widgets.

---

### Task 3.1: Fix SubView enum alignment

**Effort:** 4 hours
**Dependencies:** None

**Files to modify:**
- `crates/roko-cli/src/tui/views/mod.rs` -- ensure every SubView variant maps to a real rendering path

**What to implement:**

The `SubView` enum declares 20 variants across 7 tabs. Of these, 5 are declared but never render distinct content:

| SubView | Tab | Renders | Fix |
|---|---|---|---|
| `ProviderHealth` | F6 Config | Same as ConfigEditor | Implement in Task 3.2 |
| `ModelComparison` | F6 Config | Same as ConfigEditor | Implement in Task 3.2 |
| `EngramDag` | F7 Inspect | Same as default | Implement in Task 3.3 |
| `EpisodeReplay` | F7 Inspect | Same as default | Implement in Task 3.3 |
| `KnowledgeBrowse` | F7 Inspect | Same as default | Implement in Task 3.3 |

This task adds the dispatch scaffolding so that when Tasks 3.2 and 3.3 implement the content, the routing is already in place.

In `config_view.rs`, add a `match` on the active sub-view:

```rust
pub fn render(frame: &mut Frame<'_>, area: Rect, ...) {
    let sub = view_state.active_sub_view(Tab::Config);
    match sub {
        SubView::ConfigEditor => render_config_editor(frame, area, ...),
        SubView::ProviderHealth => render_provider_health(frame, area, ...),
        SubView::ModelComparison => render_model_comparison(frame, area, ...),
        _ => render_config_editor(frame, area, ...),
    }
}
```

Same pattern in `context_view.rs`:

```rust
pub fn render(frame: &mut Frame<'_>, area: Rect, ...) {
    let sub = view_state.active_sub_view(Tab::Inspect);
    match sub {
        SubView::EngramDag => render_engram_dag(frame, area, ...),
        SubView::EpisodeReplay => render_episode_replay(frame, area, ...),
        SubView::KnowledgeBrowse => render_knowledge_browse(frame, area, ...),
        _ => render_health_and_costs(frame, area, ...),
    }
}
```

For now, stub `render_provider_health` etc. with a centered "Coming soon" message. Tasks 3.2 and 3.3 fill in the real content.

**Acceptance criteria:**
- [ ] Number keys 1/2/3 switch between sub-views in F6 and F7
- [ ] Sub-view bar shows the correct labels
- [ ] No panics on any sub-view selection
- [ ] `cargo test -p roko-cli` passes

---

### Task 3.2: F6 sub-views -- ProviderHealth and ModelComparison

**Effort:** 2 days
**Dependencies:** Task 3.1

**Files to modify:**
- `crates/roko-cli/src/tui/views/config_view.rs` -- implement the two sub-views

**Files to modify (data source):**
- `crates/roko-cli/src/tui/dashboard.rs` -- add provider health and model stats to DashboardData

**What to implement:**

**ProviderHealth sub-view:**

Layout: table with columns: Provider, Status, Latency (p50/p95), Error Rate, Last Check, Cost/1K tokens.

Data sources:
- `ProviderHealthTracker` from roko-learn (already exists, used by roko-serve)
- For TUI, read from `.roko/learn/provider-health.json` or query roko-serve `/api/providers/health`
- `LatencyRegistry` from roko-learn

```rust
fn render_provider_health(frame: &mut Frame<'_>, area: Rect, data: &DashboardData, ...) {
    let headers = ["Provider", "Status", "p50 ms", "p95 ms", "Err %", "Last", "$/1K"];
    // Build rows from data.provider_health entries
    // Color-code status: green = healthy, yellow = degraded, red = down
    // Color-code error rate: green < 1%, yellow < 5%, red >= 5%
}
```

**ModelComparison sub-view:**

Layout: table with columns: Model, Tokens In, Tokens Out, Cost, Gate Pass %, Avg Latency, Tasks.

Data sources:
- Efficiency events from `.roko/learn/efficiency.jsonl`
- Aggregate by model name

```rust
fn render_model_comparison(frame: &mut Frame<'_>, area: Rect, data: &DashboardData, ...) {
    let headers = ["Model", "In", "Out", "Cost", "Pass %", "Lat ms", "Tasks"];
    // Aggregate efficiency events by model
    // Sort by cost descending (biggest spender first)
    // Color-code pass rate: green >= 90%, yellow >= 70%, red < 70%
}
```

**Acceptance criteria:**
- [ ] F6 + key "2" shows ProviderHealth with real data
- [ ] F6 + key "3" shows ModelComparison with real data
- [ ] Tables scroll when content exceeds viewport
- [ ] Empty state shown when no data available
- [ ] `cargo test -p roko-cli` passes

---

### Task 3.3: F7 sub-views -- EngramDag, EpisodeReplay, KnowledgeBrowse

**Effort:** 3 days
**Dependencies:** Task 3.1

**Files to modify:**
- `crates/roko-cli/src/tui/views/context_view.rs` -- implement three sub-views
- `crates/roko-cli/src/tui/dashboard.rs` -- add engram, episode, knowledge data

**What to implement:**

**EngramDag sub-view:**

Renders an ASCII-art directed graph of engrams and their provenance links. Read from `.roko/engrams.jsonl`.

Layout: two-panel. Left 40%: list of engrams (newest first) with kind badge (Signal, Insight, Memory). Right 60%: selected engram detail showing content, provenance chain (what signal triggered it, what it produced), and metadata.

Navigation: Up/Down to select engram, Enter to expand, Tab to switch panels.

```rust
fn render_engram_dag(frame: &mut Frame<'_>, area: Rect, data: &DashboardData, ...) {
    let panels = Layout::horizontal([Pct(40), Pct(60)]).split(area);
    render_engram_list(frame, panels[0], &data.engrams, selected_idx, theme);
    render_engram_detail(frame, panels[1], selected_engram, theme);
}
```

**EpisodeReplay sub-view:**

Step through an episode's turns with timing. Read from `.roko/episodes.jsonl`.

Layout: left 30%: episode list. Right 70%: turn-by-turn replay with:
- Turn number and timestamp
- Input (truncated, expandable)
- Output (truncated, expandable)
- Token usage per turn
- Total elapsed time bar

Navigation: Left/Right to step through turns, Up/Down to select episodes.

```rust
fn render_episode_replay(frame: &mut Frame<'_>, area: Rect, data: &DashboardData, ...) {
    let panels = Layout::horizontal([Pct(30), Pct(70)]).split(area);
    render_episode_list(frame, panels[0], &data.episodes, selected_idx, theme);
    render_episode_turns(frame, panels[1], selected_episode, current_turn, theme);
}
```

**KnowledgeBrowse sub-view:**

Searchable browser for the neuro store. Read from `.roko/memory/` directory.

Layout: top: search input. Below: two-panel. Left 40%: filtered list of knowledge entries. Right 60%: selected entry detail.

Search filters by keyword match against entry content and tags.

```rust
fn render_knowledge_browse(frame: &mut Frame<'_>, area: Rect, data: &DashboardData, ...) {
    let sections = Layout::vertical([Len(3), Min(0)]).split(area);
    render_search_bar(frame, sections[0], &search_query, theme);
    let panels = Layout::horizontal([Pct(40), Pct(60)]).split(sections[1]);
    render_entry_list(frame, panels[0], &filtered_entries, selected_idx, theme);
    render_entry_detail(frame, panels[1], selected_entry, theme);
}
```

**Acceptance criteria:**
- [ ] F7 + key "1" shows EngramDag with real engram data
- [ ] F7 + key "2" shows EpisodeReplay with real episode data
- [ ] F7 + key "3" shows KnowledgeBrowse with real neuro store data
- [ ] Engram list scrolls and updates when new engrams appear
- [ ] Episode replay steps through turns with correct timing
- [ ] Knowledge search filters entries in real-time
- [ ] All sub-views handle empty data gracefully
- [ ] `cargo test -p roko-cli` passes

---

### Task 3.4: F8 Marketplace tab

**Effort:** 2 days
**Dependencies:** Task 0.6 (jobs backend)

**Files to create:**
- `crates/roko-cli/src/tui/views/marketplace_view.rs` -- marketplace tab content

**Files to modify:**
- `crates/roko-cli/src/tui/tabs.rs` -- add `Marketplace` variant, update `ALL`, `fkey`, `from_key`, `label`, `label_with_key`, `index`, `next`, `prev`
- `crates/roko-cli/src/tui/views/mod.rs` -- add `marketplace_view` module, update `render_tab_content`, extend `SubView`

**What to implement:**

Add `Tab::Marketplace` mapped to F8.

SubViews for Marketplace:
- `JobBoard` (default): list of jobs from roko-serve
- `CreateBounty`: create a new research or coding bounty
- `JobDetail`: detail view of selected job

Update `Tab::ALL` to include the new tab:

```rust
pub const ALL: [Tab; 8] = [
    Tab::Dashboard, Tab::Plans, Tab::Agents, Tab::Git,
    Tab::Logs, Tab::Config, Tab::Inspect, Tab::Marketplace,
];
```

Update `next()`/`prev()` to include `Marketplace` in the cycle.

**JobBoard rendering:**

```rust
fn render_job_board(frame: &mut Frame<'_>, area: Rect, data: &DashboardData, ...) {
    // Table with columns: Status, Kind, Title, Bounty, Domain, Deadline
    // Status badges: Open (yellow), Assigned (blue), InProgress (cyan),
    //   Submitted (purple), Completed (green), Failed (red)
    // Kind badges: Research, Coding, Review, Custom
    // Scrollable with Up/Down, Enter to view detail
}
```

Data comes from querying roko-serve `GET /api/jobs` on a 10-second interval (or WebSocket if connected).

**Acceptance criteria:**
- [ ] F8 opens the Marketplace tab
- [ ] Job board shows real jobs from roko-serve
- [ ] Status and kind badges render with correct colors
- [ ] Up/Down scrolls the job list
- [ ] Enter on a job shows the detail sub-view
- [ ] "c" key opens the create bounty sub-view
- [ ] Tab wrapping works: F7 -> next -> F8, F8 -> next -> F1
- [ ] Existing tests in `tabs.rs` updated: `next_prev_cycle` now cycles through 8 tabs, `index_is_sequential` checks all 8 tab indices
- [ ] `cargo test -p roko-cli` passes
- [ ] `cargo +nightly fmt --all` passes

---

### Task 3.5: F9 Atelier tab

**Effort:** 2 days
**Dependencies:** Task 3.4 (both modify `tabs.rs` — must be sequential)

**Files to create:**
- `crates/roko-cli/src/tui/views/atelier_view.rs` -- atelier tab content

**Files to modify:**
- `crates/roko-cli/src/tui/tabs.rs` -- add `Atelier` variant
- `crates/roko-cli/src/tui/views/mod.rs` -- add module, extend SubView, update render dispatch

**What to implement:**

Add `Tab::Atelier` mapped to F9.

SubViews:
- `WorkspaceStatus` (default): PRD count, plan count, task completion ratio, agent pool status
- `PrdList`: scrollable PRD list from `.roko/prd/`
- `ActivePlans`: running plans with per-task progress

**WorkspaceStatus rendering:**

Three-panel layout:
- Top 30%: status gauges (plans total, tasks total, completed ratio, PRD coverage)
- Mid 40%: active agents table (id, status, current task, tokens, cost)
- Bottom 30%: recent activity from event log

Data from local files (`.roko/prd/`, `.roko/state/executor.json`, etc.) or roko-serve if connected.

Update `Tab::ALL`:

```rust
pub const ALL: [Tab; 9] = [
    Tab::Dashboard, Tab::Plans, Tab::Agents, Tab::Git,
    Tab::Logs, Tab::Config, Tab::Inspect, Tab::Marketplace, Tab::Atelier,
];
```

**Acceptance criteria:**
- [ ] F9 opens the Atelier tab
- [ ] Workspace status shows real PRD and plan counts
- [ ] PRD list shows all PRDs in `.roko/prd/`
- [ ] Active plans show task-level progress
- [ ] Tab wrapping works: F8 -> next -> F9, F9 -> next -> F1
- [ ] Existing tests in `tabs.rs` updated: `next_prev_cycle` now cycles through 9 tabs, `index_is_sequential` checks all 9 tab indices
- [ ] `cargo test -p roko-cli` passes
- [ ] `cargo +nightly fmt --all` passes

---

### Task 3.6: Nexus integration for TUI

**Effort:** 2 days
**Dependencies:** Task 1.1, Task 1.2

**Files to create:**
- `crates/roko-cli/src/tui/nexus_client.rs` -- background Nexus connection for TUI

**Files to modify:**
- `crates/roko-cli/src/tui/app.rs` -- connect to Nexus on startup if configured
- `crates/roko-cli/src/tui/views/agents_view.rs` -- show network-wide agents from Nexus
- `crates/roko-cli/src/tui/views/dashboard_view.rs` -- show network stats in header bar

**What to implement:**

The TUI gets its own Nexus connection so it can show network-wide data without roko-serve:

```rust
pub struct TuiNexusClient {
    client: NexusClient,
    /// All agents seen via Nexus (network-wide).
    network_agents: Vec<HeartbeatPayload>,
    /// Aggregate network stats.
    network_stats: NetworkStats,
}
```

On each TUI tick, drain `NexusEvent::PresenceUpdate` and `NexusEvent::HeartbeatAggregate` to update the local cache. Display in:

- F1 Dashboard header: "Network: 12 agents, 3 active" (or "Network: offline" when not connected)
- F3 Agents: extra section below local agents showing network-wide agents from Nexus

**Acceptance criteria:**
- [ ] TUI connects to Nexus when `nexus.url` is set
- [ ] Dashboard header shows network stats
- [ ] Agents tab shows network-wide agents
- [ ] Graceful degradation when Nexus is not available
- [ ] `cargo test -p roko-cli` passes

---

### Task 3.7: Bug fixes for existing views

**Effort:** 2 days
**Dependencies:** None

**Files to modify:**
- `crates/roko-cli/src/tui/widgets/plan_tree.rs` -- fix vfy column, wire wave collapse
- `crates/roko-cli/src/tui/views/logs_view.rs` -- add memoization cache for O(N) rebuild
- `crates/roko-cli/src/tui/views/git_view.rs` -- replace fragile --graph parser with structured format
- `crates/roko-cli/src/tui/widgets/parallel_pool.rs` -- fix "progress" column label, add scroll

**What to implement:**

**plan_tree.rs -- vfy column (currently stubbed)**

The "vfy" (verify) column in the plan tree always shows "---". Wire it to the actual gate verification status:

```rust
fn vfy_cell(task: &TaskEntry, gate_results: &HashMap<String, GateResult>) -> Cell<'_> {
    match gate_results.get(&task.id) {
        Some(GateResult { passed: true, .. }) => Cell::from("PASS").style(style_pass),
        Some(GateResult { passed: false, .. }) => Cell::from("FAIL").style(style_fail),
        None if task.status == TaskStatus::Running => Cell::from("...").style(style_pending),
        None => Cell::from("---").style(style_none),
    }
}
```

**plan_tree.rs -- wave collapse/expand**

The `Wave` struct has an `expanded` field but the keyboard handler doesn't toggle it. Wire the Enter key on a wave header to toggle `expanded`:

```rust
KeyCode::Enter => {
    if let Some(wave) = state.waves.get_mut(state.selected_wave_idx) {
        wave.expanded = !wave.expanded;
    }
}
```

**logs_view.rs -- O(N) rebuild**

Every frame rebuilds the entire log list from scratch. Add a generation-based cache:

```rust
struct LogViewCache {
    generation: u64,
    rendered_lines: Vec<Line<'static>>,
    scroll_offset: u16,
}
```

Only rebuild when `data.generation()` changes.

**git_view.rs -- fragile --graph parser**

Replace the regex-based `--graph` parser with a structured format using NUL byte as the field separator (safe for branch names containing any printable characters):

```rust
// Instead of: git log --graph --oneline --all
// Use: git log --format='%H%x00%h%x00%P%x00%D%x00%s' --all
// Parse fields by splitting on '\0' (NUL byte)
// Build tree structure from parent hashes
```

**parallel_pool.rs -- mislabeled column + no scroll**

Rename "progress" column to "status" (it shows the status, not progress). Add scroll support using a new dedicated field to avoid semantic confusion with `secondary_selected`:

```rust
// Use a new field ViewState.pool_scroll: usize instead of reusing secondary_selected
// Handle Up/Down when pool is focused
```

**Acceptance criteria:**
- [ ] Plan tree vfy column shows actual gate results
- [ ] Enter key toggles wave expand/collapse
- [ ] Logs view does not rebuild when data hasn't changed
- [ ] Git view works with branches containing special characters and slashes
- [ ] Parallel pool scrolls and shows correct column labels
- [ ] `ViewState.pool_scroll` added and used (not `secondary_selected`)
- [ ] All changes tested: `cargo test -p roko-cli` passes
- [ ] `cargo +nightly fmt --all` passes

---

### Task 3.8: Bardo-inspired widget ports

**Effort:** 3 days
**Dependencies:** None

**Files to create:**
- `crates/roko-cli/src/tui/widgets/cognitive_frequency_bar.rs` -- gamma/theta/delta indicators
- `crates/roko-cli/src/tui/widgets/mortality_gauge.rs` -- eroding bar for agent health
- `crates/roko-cli/src/tui/widgets/decision_ring.rs` -- 9-step pipeline ring
- `crates/roko-cli/src/tui/widgets/phosphor_log.rs` -- scrolling log with phosphor decay
- `crates/roko-cli/src/tui/widgets/unit_array.rs` -- data wall display

**What to implement:**

**CognitiveFrequencyBar:**

Renders three frequency indicators for the agent's operating mode:

```
[gamma ||||||||    ] [theta |||         ] [delta |||||||||||| ]
```

Gamma = high-frequency reactive mode (responding to events). Theta = medium-frequency reflective mode (reviewing work). Delta = low-frequency consolidation mode (synthesizing knowledge).

Width proportional to the current operating frequency from `OperatingFrequency::Gamma/Theta/Delta`. Color: gamma = cyan, theta = amber, delta = magenta.

```rust
pub struct CognitiveFrequencyBar {
    gamma: f32,  // 0.0 - 1.0
    theta: f32,
    delta: f32,
}

impl Widget for CognitiveFrequencyBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Three horizontal bars stacked, each with a label and fill
    }
}
```

**MortalityGauge:**

A horizontal bar that erodes from right to left as the agent's health degrades. Full bar = healthy. Eroded sections render in a dimmer color with unicode decay characters.

```rust
pub struct MortalityGauge {
    health: f32,     // 0.0 - 1.0
    threshold: f32,  // Below this, show warning color
}

impl Widget for MortalityGauge {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Full blocks for healthy portion
        // Fractional block at the boundary
        // Light shade blocks for eroded portion
        // Warning color when below threshold
    }
}
```

**DecisionRing:**

Renders the 9-step cognitive pipeline as a horizontal sequence of labeled boxes, with the current step highlighted:

```
[query] -> [score] -> [route] -> [compose] -> [act] -> [verify] -> [write] -> [react]
                                   ^^^^
                              (current step)
```

```rust
pub struct DecisionRing {
    current_step: usize,  // 0-7
    step_labels: [&'static str; 8],
}
```

**PhosphorLog:**

A scrolling log where older entries fade. The most recent entry is full brightness. Entries decay in brightness over time (phosphor persistence effect).

```rust
pub struct PhosphorLog<'a> {
    entries: &'a [LogEntry],
    decay_rate: f32,      // brightness lost per entry (0.1 = 10% per line)
    max_brightness: u8,   // starting fg color value
}
```

**UnitArray:**

A dense data wall showing numeric values in a grid. Each cell is a single metric with a tiny label. Inspired by the Eva-style instrument panels in bardo.

```rust
pub struct UnitArray<'a> {
    cells: &'a [UnitCell],
    columns: u16,
}

pub struct UnitCell {
    pub label: String,
    pub value: String,
    pub status: CellStatus,  // Normal, Warning, Critical
}
```

**Module wiring:** Add all 5 widget file names (`cognitive_frequency_bar`, `mortality_gauge`, `decision_ring`, `phosphor_log`, `unit_array`) as `pub mod <name>;` declarations to `crates/roko-cli/src/tui/widgets/mod.rs`.

**Acceptance criteria:**
- [ ] Each widget renders correctly in a test frame
- [ ] CognitiveFrequencyBar shows three labeled bars with correct fill
- [ ] MortalityGauge erodes smoothly and changes color at threshold
- [ ] DecisionRing highlights the current step
- [ ] PhosphorLog fades older entries
- [ ] UnitArray lays out cells in a grid
- [ ] All widgets handle zero-width and zero-height gracefully
- [ ] `cargo test -p roko-cli` passes
- [ ] `cargo +nightly fmt --all` passes

---

### Task 3.9: Adaptive density mode

**Effort:** 1.5 days
**Dependencies:** Task 3.8

**Files to create:**
- `crates/roko-cli/src/tui/density.rs` -- density mode logic

**Files to modify:**
- `crates/roko-cli/src/tui/state.rs` -- add `density: DensityMode` field
- `crates/roko-cli/src/tui/views/*.rs` -- adjust layouts based on density mode
- `crates/roko-cli/src/tui/mod.rs` -- add `pub mod density;`

**What to implement:**

Before implementing `DensityMode::auto_detect`, add these helper methods to `DashboardData` in `crates/roko-cli/src/tui/dashboard.rs`:

```rust
pub fn gate_failures_last_5min(&self) -> usize {
    // count gate failures from gate_results_page where timestamp > now - 5min
}
pub fn open_circuit_breakers(&self) -> usize {
    // count conductor_alerts with severity >= Warning
}
```

Three display modes that control information density:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DensityMode {
    /// Normal operation. Standard spacing, all panels visible.
    Cruise,
    /// High activity. Compact spacing, priority data emphasized.
    Volatile,
    /// Active failures. Minimal chrome, alerts and errors dominate.
    Crisis,
}

impl DensityMode {
    /// Auto-detect from system state.
    pub fn auto_detect(data: &DashboardData) -> Self {
        let active_failures = data.gate_failures_last_5min();
        let circuit_breakers_open = data.open_circuit_breakers();

        if circuit_breakers_open > 0 || active_failures > 3 {
            Self::Crisis
        } else if data.active_agents() > 5 || data.active_tasks() > 10 {
            Self::Volatile
        } else {
            Self::Cruise
        }
    }
}
```

Each view adjusts its layout based on the density mode:
- **Cruise:** Full spacing, all sub-panels visible, decorative borders
- **Volatile:** Compact spacing, secondary panels collapsed, data-dense tables
- **Crisis:** Minimal chrome, error-focused, alert banner visible

Users can override with Ctrl+1 (Cruise), Ctrl+2 (Volatile), Ctrl+3 (Crisis), Ctrl+0 (auto).

**Acceptance criteria:**
- [ ] Auto-detection switches modes based on system state
- [ ] Ctrl+0/1/2/3 override the mode
- [ ] Dashboard view adjusts layout per mode
- [ ] Plans view adjusts layout per mode
- [ ] Mode indicator visible in the status bar
- [ ] `cargo test -p roko-cli` passes
- [ ] `cargo +nightly fmt --all` passes

---

### Task 3.10: Command palette

**Effort:** 1.5 days
**Dependencies:** None

**Files to create:**
- `crates/roko-cli/src/tui/modals/command_palette.rs` -- fuzzy search command palette

**Files to modify:**
- `crates/roko-cli/src/tui/input.rs` -- handle Ctrl+P and `:` to open palette
- `crates/roko-cli/src/tui/app.rs` -- overlay palette modal on active view
- `crates/roko-cli/src/tui/modals/mod.rs` -- add `pub mod command_palette;`

**What to implement:**

Ctrl+P opens a centered overlay with a text input and a filtered list of commands:

```rust
pub struct CommandPalette {
    query: String,
    commands: Vec<PaletteCommand>,
    filtered: Vec<usize>,
    selected: usize,
}

pub struct PaletteCommand {
    pub label: String,
    pub shortcut: Option<String>,
    pub action: PaletteAction,
}

pub enum PaletteAction {
    SwitchTab(Tab),
    SwitchSubView(Tab, usize),
    ToggleDensity(DensityMode),
    OpenModal(ModalKind),
    RunCommand(String),
}
```

Built-in commands:
- "Go to Dashboard" (F1)
- "Go to Plans" (F2)
- ...all tabs...
- "Switch to Cruise mode" (Ctrl+1)
- "Switch to Volatile mode" (Ctrl+2)
- "Switch to Crisis mode" (Ctrl+3)
- "Show provider health" (F6 + 2)
- "Show model comparison" (F6 + 3)
- "Show engram DAG" (F7 + 1)
- "Show episode replay" (F7 + 2)
- "Show knowledge browser" (F7 + 3)

Fuzzy matching: score by substring match position (earlier = better) and character coverage.

Vim-style `:` also opens the palette with the `:` prefix for command mode:
- `:q` -- quit
- `:set density cruise` -- set density mode
- `:connect nexus` -- connect to nexus

**Acceptance criteria:**
- [ ] Ctrl+P opens the command palette
- [ ] Typing filters the command list
- [ ] Enter executes the selected command
- [ ] Escape closes the palette
- [ ] Fuzzy matching ranks "Plans" above "Provider Health" when typing "pla"
- [ ] `:` mode works for command entry
- [ ] Palette renders centered with 60% width, max 20 visible items
- [ ] `cargo test -p roko-cli` passes
- [ ] `cargo +nightly fmt --all` passes

---

## Phase 4: Integration and polish (1 week)

---

### Task 4.1: End-to-end job flow test

**Effort:** 2 days
**Dependencies:** Tasks 0.6, 0.7, 2.8, 3.4

**Files to create:**
- `tests/integration/job_flow.rs` -- integration test for the full job lifecycle

**What to implement:**

Test the complete flow:
1. Create a research job via `POST /api/jobs`
2. List jobs, verify it appears
3. Assign to an agent
4. Agent executes (via `roko job take`)
5. Deliverables submitted
6. Job evaluated and completed
7. Verify all status transitions in the timeline
8. Verify WebSocket events emitted at each transition
9. Verify TUI marketplace tab shows the job
10. Verify dashboard job board shows the job

Run with `cargo test -p roko-cli --test job_flow`.

**Acceptance criteria:**
- [ ] Full research job lifecycle completes end-to-end
- [ ] Full coding job lifecycle completes end-to-end
- [ ] Invalid transitions are rejected
- [ ] WebSocket events verified for each transition
- [ ] Test is deterministic (no flaky timing)

---

### Task 4.2: Dashboard to Nexus integration test

**Effort:** 1 day
**Dependencies:** Tasks 1.3, 2.3, 2.7

**What to implement:**

Verify that:
- Dashboard connects to Nexus via the WS manager
- Network topology page shows agents from Nexus
- Agent presence updates propagate within 2 seconds
- Network stats page shows aggregate data from Nexus
- Graceful fallback when Nexus is unavailable

Manual test procedure documented in a test plan file.

**Acceptance criteria:**
- [ ] Dashboard renders Nexus data when connected
- [ ] Dashboard falls back to roko-serve-only data when Nexus is down
- [ ] No console errors during fallback

---

### Task 4.3: TUI to Nexus integration test

**Effort:** 1 day
**Dependencies:** Tasks 1.2, 3.6

**What to implement:**

Verify that:
- TUI connects to Nexus on startup when configured
- Network stats appear in F1 header
- Network agents appear in F3
- Reconnection works after Nexus restart

**Acceptance criteria:**
- [ ] TUI renders Nexus data when connected
- [ ] TUI recovers from Nexus disconnect within 30 seconds
- [ ] No panics during connection lifecycle

---

### Task 4.4: Performance audit

**Effort:** 1 day
**Dependencies:** All Phase 2 and 3 tasks

**Files to modify:**
- Various across dashboard and TUI

**What to implement:**

Performance sweep:

**Dashboard:**
- Remove any remaining `setInterval` or `setTimeout` polling
- Verify TanStack Query deduplication (dev tools: no duplicate requests)
- Measure largest contentful paint (target: < 2s)
- Verify WebSocket reconnection (disconnect wifi, reconnect, verify)
- Bundle size audit (target: < 500KB gzipped)

**TUI:**
- Profile frame time (target: < 16ms per frame on 50+ tasks)
- Verify `IncrementalReader` actually skips read bytes (add trace logging)
- Verify `LogViewCache` avoids rebuilds (add hit/miss counter)
- Check for leaked `AgentStreamClient` handles on tab switch
- Verify `notify::Watcher` does not spin under load

Document findings and fix any regressions.

**Acceptance criteria:**
- [ ] No polling timers in dashboard code
- [ ] TUI frame time < 16ms with 100 tasks active
- [ ] Dashboard LCP < 2 seconds
- [ ] Dashboard bundle < 500KB gzipped
- [ ] No resource leaks in 1-hour soak test

---

## Phase 5: Demo polish (3 days)

---

### Task 5.1: Landing page final polish

**Effort:** 1 day
**Dependencies:** Task 2.4

**What to implement:**

- Smooth animations on architecture explorer transitions
- Responsive typography scaling
- Loading skeleton matches final layout exactly
- Favicon and page title set correctly
- OpenGraph meta tags for link previews

**Acceptance criteria:**
- [ ] Landing page is visually polished
- [ ] No layout shift during loading
- [ ] Works on mobile Safari and Chrome

---

### Task 5.2: Research bounty demo flow

**Effort:** 1 day
**Dependencies:** Tasks 0.6, 0.7, 2.8

**What to implement:**

A scripted end-to-end demo:

1. Open dashboard marketplace
2. Create a research bounty: "Compare Rust async runtimes: tokio vs smol vs async-std"
3. Watch the job appear in the job board
4. An agent picks it up (auto-assign or manual)
5. Watch the execution in the observatory
6. View the completed research report
7. Evaluate and complete the job

Create realistic seed data for the demo if real execution takes too long. Tag all demo data with `"demo": true` in the JSON.

**Acceptance criteria:**
- [ ] Demo runs end-to-end in under 3 minutes
- [ ] Each step is visually clear in the dashboard
- [ ] Demo data is clearly tagged and removable

---

### Task 5.3: Coding bounty demo flow

**Effort:** 1 day
**Dependencies:** Tasks 0.6, 0.7, 2.8

**What to implement:**

Similar to 5.2 but for a coding task:

1. Create a coding bounty: "Add a /healthz endpoint to roko-serve"
2. Watch an agent create a plan from the spec
3. Watch the plan execute with gates
4. View the diff produced
5. Evaluate and complete

**Acceptance criteria:**
- [ ] Demo runs end-to-end
- [ ] Gates visibly pass/fail during execution
- [ ] Diff is rendered in the job deliverables

---

## Appendix: Code audit results

These are the audited files and specific problems found, referenced throughout the plan.

### TUI files audited

| File | Lines | Key problems |
|---|---|---|
| `tui/fs_watch.rs` | 256 | Single `Coalesced` event, no per-file granularity |
| `tui/jsonl_cursor.rs` | 237 | Correct implementation, but only used by one consumer |
| `tui/views/mod.rs` | 243 | SubView enum declares variants never rendered |
| `tui/views/config_view.rs` | ~500 | No sub-view dispatch -- always renders config editor |
| `tui/views/context_view.rs` | ~770 | No sub-view dispatch -- always renders health/costs |
| `tui/views/plans_view.rs` | ~800 | No pagination on DAG, ETA always None |
| `tui/views/agents_view.rs` | ~700 | Reads from snapshot, not live WS |
| `tui/views/git_view.rs` | ~765 | Fragile --graph parser |
| `tui/views/logs_view.rs` | ~467 | O(N) rebuild per frame |
| `tui/widgets/plan_tree.rs` | ~1016 | vfy column stubbed, wave collapse not wired |
| `tui/widgets/parallel_pool.rs` | ~172 | Mislabeled column, no scroll |
| `tui/tabs.rs` | 183 | Only 7 tabs, needs 9 |
| `tui/ws_client.rs` | ~200 | Has connection logic, not wired to Agents tab |

### roko-serve files audited

| File | Lines | Key problems |
|---|---|---|
| `routes/middleware.rs` | 289 | Only API key auth, no JWT, no RBAC, no rate limit |
| `routes/mod.rs` | 102 | No jobs routes, no auth routes, no nexus routes |
| `state.rs` | ~400 | In-memory maps lost on restart |
| `routes/agents.rs` | 677 | Works, but no heartbeat ingestion |

### Orchestrator

| File | Lines | Key problems |
|---|---|---|
| `orchestrate.rs` | ~5000+ | Three near-identical PlanRunner constructors, UX34 not wired |

---

## Dependency graph

```
Phase 0 (parallel tasks)
  0.1  Incremental watchers      ─┐
  0.2  Live agent WS             │
  0.3  Gen counter persistence   │
  0.4  HTTP auth                 ├──> Phase 1 (Nexus)
  0.5  Server state persistence  │      1.1 Nexus core    ─┐
  0.8  Heartbeat protocol        │      1.2 CLI integration│──> Phase 2 + Phase 3
  0.10 Constructor dedup         │      1.3 Serve bridge   ─┘
  0.11 Cascade router UX34       ─┘
  0.6  Jobs backend              ──> 0.7 Job execution ──> Phase 4 + Phase 5
  0.9  CLI auth                  ──> (depends on 0.4)
```

Tasks within each phase can run in parallel unless explicitly noted as dependent. Phase 2 and Phase 3 can run in parallel with each other once Phase 0 and Phase 1 complete.
