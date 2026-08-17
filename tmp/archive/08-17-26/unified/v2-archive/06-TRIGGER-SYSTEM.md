# 06 — Trigger System

> Event-driven Graph firing. Triggers listen for events, evaluate filters, and start Flows. Every trigger event is a Pulse on Bus. Triggers are persistent, declarative, and composable.

**Subsumes**: Cron jobs, webhooks, file watchers, event subscriptions, manual triggers, chain event listeners.

---

## 1. Trigger Protocol

The Trigger protocol ([doc-02](02-CELL.md) S2.9) defines three operations: arm, disarm, and poll. A Trigger Cell listens for events matching a binding and, when fired, starts a Graph as a new Flow.

```rust
#[async_trait]
pub trait TriggerProtocol: Cell {
    /// Arm the trigger. Begins listening for matching events.
    /// Returns a handle for disarming and polling.
    async fn arm(&self, binding: &TriggerBinding) -> Result<TriggerHandle>;

    /// Disarm the trigger. Stops listening, cleans up resources.
    async fn disarm(&self, handle: TriggerHandle) -> Result<()>;

    /// Poll for pending trigger events (non-blocking).
    /// Returns events that have fired since the last poll.
    /// The engine calls this; Trigger Cells do not self-dispatch.
    async fn poll(&self, handle: &TriggerHandle) -> Result<Vec<TriggerEvent>>;
}

/// Handle to an armed trigger.
pub struct TriggerHandle {
    pub id: TriggerId,
    pub binding: TriggerBinding,
    pub armed_at: DateTime<Utc>,
    pub state: TriggerState,
}

pub enum TriggerState {
    Armed,
    Firing,
    Cooldown { until: DateTime<Utc> },
    Disarmed,
    Failed { error: String },
}

/// An event that fired from a trigger.
pub struct TriggerEvent {
    pub trigger_id: TriggerId,
    pub fired_at: DateTime<Utc>,
    pub payload: Value,
    pub source: TriggerSource,
    /// Trace ID for correlating the trigger event with the resulting Flow.
    pub trace_id: TraceId,
}

pub enum TriggerSource {
    Cron { expression: String },
    Webhook { method: String, path: String, headers: BTreeMap<String, String> },
    FileWatch { path: PathBuf, event: FileWatchEvent },
    Bus { topic: Topic, pulse_seq: u64 },
    ChainEvent { chain_id: u64, block_number: u64, tx_hash: String },
    Manual { user: Author },
    SignalPattern { matched_signals: Vec<SignalRef> },
}
```

---

## 2. TriggerBinding

A TriggerBinding is the persistent, TOML-defined configuration that connects an event source to a Graph. Bindings survive process restarts — they are stored in `.roko/triggers/` and re-armed on startup.

```rust
pub struct TriggerBinding {
    /// Unique name for this binding.
    pub name: String,

    /// The trigger kind and its configuration.
    pub kind: TriggerKind,

    /// The Graph to fire when the trigger activates.
    pub graph: GraphRef,

    /// Input mapping: how to transform the trigger event payload
    /// into input Signals for the Graph.
    pub input_mapping: Option<TriggerInputMapping>,

    /// Concurrency policy: what to do if the trigger fires while
    /// a previous Flow from this trigger is still running.
    pub concurrency: ConcurrencyPolicy,

    /// Event filter: conditions that must be met for the trigger
    /// to actually fire (beyond the kind-specific matching).
    pub filter: Option<TriggerFilter>,

    /// Whether this binding is currently enabled.
    pub enabled: bool,

    /// Space this trigger runs in (capability scoping).
    pub space: Option<SpaceId>,

    /// Auth configuration for the trigger source.
    pub auth: Option<TriggerAuth>,
}

pub struct TriggerInputMapping {
    /// Transform the trigger event payload into Graph input Signals.
    pub mappings: Vec<InputFieldMapping>,
}

pub struct InputFieldMapping {
    /// JSONPath expression selecting from the trigger event payload.
    pub from: String,
    /// Target field in the input Signal.
    pub to: String,
    /// Optional transformation.
    pub transform: Option<Expr>,
}
```

### Persistence

```
.roko/triggers/
├── on-pr-opened.toml
├── nightly-consolidation.toml
├── file-watcher.toml
└── bus-gate-failure.toml
```

On startup, the engine reads all `.toml` files in `.roko/triggers/` and arms each enabled binding. On shutdown, trigger state (e.g., last cron execution time) is written back.

---

## 3. Seven Built-in Trigger Kinds

### 3.1 Cron

Time-based triggers using cron expressions. Supports second-resolution cron syntax (6 fields) and timezone awareness.

```rust
pub struct CronTrigger {
    /// Cron expression (6 fields: sec min hour day month weekday).
    pub expression: String,

    /// Timezone for evaluation (default: UTC).
    pub timezone: String,

    /// Last execution time (for preventing double-fire on restart).
    pub last_fired: Option<DateTime<Utc>>,
}
```

```toml
[[triggers]]
name = "nightly-consolidation"
kind = "cron"
expression = "0 0 3 * * *"   # 3:00 AM daily
timezone = "America/New_York"
graph = "plans/dream-consolidation.toml"
```

### 3.2 Webhook

HTTP endpoint triggers. The engine registers routes on the HTTP control plane (`:6677`) for each webhook trigger.

```rust
pub struct WebhookTrigger {
    /// HTTP path to listen on (relative to /webhooks/).
    pub path: String,

    /// Allowed HTTP methods.
    pub methods: Vec<String>,

    /// Secret for signature verification (HMAC-SHA256).
    pub secret: Option<SecretRef>,

    /// IP allowlist (empty = allow all).
    pub ip_allowlist: Vec<String>,
}
```

```toml
[[triggers]]
name = "github-pr-opened"
kind = "webhook"
path = "/github/pr"
methods = ["POST"]
secret = { env = "GITHUB_WEBHOOK_SECRET" }
graph = "plans/code-review.toml"

[triggers.filter]
where = "payload.action == 'opened'"
```

### 3.3 FileWatch

File system event triggers using `notify::RecommendedWatcher`. Watches files or directories for changes.

```rust
pub struct FileWatchTrigger {
    /// Path to watch (file or directory).
    pub path: PathBuf,

    /// Whether to watch recursively (for directories).
    pub recursive: bool,

    /// File events to trigger on.
    pub events: Vec<FileWatchEvent>,

    /// Glob pattern for file name filtering.
    pub glob: Option<String>,
}

pub enum FileWatchEvent {
    Created,
    Modified,
    Deleted,
    Renamed,
    Any,
}
```

```toml
[[triggers]]
name = "plan-file-changed"
kind = "file_watch"
path = "plans/"
recursive = true
glob = "*.toml"
events = ["modified", "created"]
graph = "plans/validate-plan.toml"
concurrency = "skip"
```

### 3.4 Bus

Trigger on Pulses matching a topic filter. This is the primary mechanism for trigger chaining: one Flow publishes a Pulse, which triggers another Flow.

```rust
pub struct BusTrigger {
    /// Topic filter to match.
    pub filter: TopicFilter,

    /// Optional body predicate (Expr evaluated against Pulse body).
    pub body_predicate: Option<Expr>,
}
```

```toml
[[triggers]]
name = "on-gate-failure"
kind = "bus"
filter = "gate.verdict.emitted"
graph = "plans/gate-failure-replan.toml"
concurrency = "queue"

[triggers.filter]
where = "payload.hard_pass == false"
```

### 3.5 ChainEvent

Trigger on on-chain events (ERC-8004 identity updates, contract events, token transfers). Phase 2+ — requires a chain indexer connection.

```rust
pub struct ChainEventTrigger {
    /// Chain ID (e.g., 1 for Ethereum mainnet, 8453 for Base).
    pub chain_id: u64,

    /// Contract address to watch.
    pub contract: String,

    /// Event signature to match.
    pub event_signature: String,

    /// ABI for decoding event data.
    pub abi: Option<Value>,

    /// Finality requirement.
    pub finality: FinalityRequirement,
}

pub enum FinalityRequirement {
    /// Wait for finality oracle to confirm.
    Final,
    /// Accept quasi-finalized (high confidence but not proven).
    QuasiFinalized,
    /// Accept immediately (may be reorged).
    Reversible,
}
```

```toml
[[triggers]]
name = "identity-updated"
kind = "chain_event"
chain_id = 8453
contract = "0x..."
event_signature = "IdentityUpdated(address,bytes32)"
finality = "quasi_finalized"
graph = "plans/update-agent-identity.toml"
```

### 3.6 Manual

Triggers that fire only when explicitly invoked via CLI, API, or TUI. Used for on-demand operations.

```rust
pub struct ManualTrigger {
    /// Description shown in CLI/TUI.
    pub description: String,

    /// Input schema for the manual payload.
    pub input_schema: Option<TypeSchema>,

    /// Whether to prompt for confirmation before firing.
    pub confirm: bool,
}
```

```toml
[[triggers]]
name = "manual-deploy"
kind = "manual"
description = "Deploy the current build to staging"
confirm = true
graph = "plans/deploy-staging.toml"
```

### 3.7 SignalPattern

Trigger when a pattern of Signals appears in Store. Unlike Bus triggers (which match individual Pulses), SignalPattern triggers match aggregate conditions over stored Signals.

```rust
pub struct SignalPatternTrigger {
    /// Query to run against Store.
    pub query: StoreQuery,

    /// Minimum number of matching Signals required.
    pub min_matches: usize,

    /// Time window for the pattern (matches must occur within this window).
    pub window: Duration,

    /// Poll interval (how often to check Store).
    pub poll_interval: Duration,
}
```

```toml
[[triggers]]
name = "failure-cluster"
kind = "signal_pattern"
graph = "plans/investigate-failures.toml"

[triggers.query]
kind = "Finding"
severity = "high"
min_matches = 3
window_seconds = 300
poll_interval_seconds = 30
```

---

## 4. Concurrency Policies

When a trigger fires while a previous Flow from the same trigger is still running, the concurrency policy determines behavior.

```rust
pub enum ConcurrencyPolicy {
    /// Queue the new firing. Execute after the current Flow completes.
    /// Maximum queue depth is configurable (default: 10).
    Queue { max_depth: Option<usize> },

    /// Skip the new firing. Log the skip event.
    Skip,

    /// Cancel the currently running Flow and start a new one.
    CancelRunning,

    /// Run in parallel. Multiple Flows from the same trigger coexist.
    /// Maximum concurrent is configurable (default: unlimited).
    Parallel { max_concurrent: Option<usize> },
}
```

| Policy | Use Case |
|---|---|
| Queue | Webhook events that must all be processed (e.g., PR comments) |
| Skip | File watch debounce (only care about latest state) |
| CancelRunning | Deploy triggers (new deploy supersedes in-progress) |
| Parallel | Independent event processing (each event is self-contained) |

```toml
# Queue policy with max depth
[[triggers]]
name = "pr-comment"
kind = "webhook"
concurrency = { kind = "queue", max_depth = 50 }

# Skip policy (debounce)
[[triggers]]
name = "file-changed"
kind = "file_watch"
concurrency = "skip"

# CancelRunning (latest wins)
[[triggers]]
name = "deploy"
kind = "manual"
concurrency = "cancel_running"

# Parallel with limit
[[triggers]]
name = "job-submitted"
kind = "bus"
concurrency = { kind = "parallel", max_concurrent = 5 }
```

---

## 5. Trigger Chaining via Bus

Triggers compose through Bus: the output of one Flow publishes Pulses that trigger another Flow. This creates event-driven pipelines without explicit wiring between Graphs.

```
Flow A completes
    │
    ├─► Publishes Pulse on "flow.{run_a}.completed"
    │
    ├─► Bus trigger B matches "flow.*.completed"
    │   └─► Starts Flow B
    │
    └─► Bus trigger C matches "flow.*.completed"
        └─► Starts Flow C (parallel)
```

### Example: PR review pipeline via chaining

```toml
# Trigger 1: PR opened -> fetch diff
[[triggers]]
name = "pr-opened"
kind = "webhook"
path = "/github/pr"
graph = "plans/fetch-diff.toml"

[triggers.filter]
where = "payload.action == 'opened'"

# Trigger 2: diff fetched -> run review
[[triggers]]
name = "diff-ready"
kind = "bus"
filter = "flow.*.completed"
graph = "plans/code-review.toml"

[triggers.filter]
where = "payload.graph_name == 'fetch-diff'"

# Trigger 3: review completed -> post comment
[[triggers]]
name = "review-ready"
kind = "bus"
filter = "flow.*.completed"
graph = "plans/post-pr-comment.toml"

[triggers.filter]
where = "payload.graph_name == 'code-review'"
```

---

## 6. Filtering

Triggers support layered filtering to control when they actually fire.

```rust
pub struct TriggerFilter {
    /// Filter by event kind (for Bus and webhook triggers).
    pub event_kind: Option<Vec<Kind>>,

    /// Predicate expression evaluated against the event payload.
    /// Must return true for the trigger to fire.
    pub where_clause: Option<Expr>,

    /// Pattern matching against payload fields.
    pub matches: Option<BTreeMap<String, Value>>,

    /// Debounce: suppress repeated firings within this window.
    /// Only the last event in the window fires.
    pub debounce: Option<Duration>,

    /// Rate limit: maximum firings per time window.
    pub rate_limit: Option<RateLimit>,

    /// Custom filter Cell: evaluate via a user-defined Cell.
    pub custom_filter: Option<CellRef>,
}

pub struct RateLimit {
    /// Maximum number of firings.
    pub max_fires: u32,

    /// Time window for the rate limit.
    pub window: Duration,

    /// What to do when rate-limited.
    pub on_limit: RateLimitAction,
}

pub enum RateLimitAction {
    /// Drop the event silently.
    Drop,
    /// Queue the event for processing after the window.
    Queue,
    /// Log a warning and drop.
    Warn,
}
```

```toml
[[triggers]]
name = "important-signals"
kind = "bus"
filter = "gate.verdict.emitted"
graph = "plans/process-verdict.toml"

[triggers.filter]
event_kind = ["Verdict"]
where = "payload.reward > 0.8"
debounce_ms = 5000
rate_limit = { max_fires = 10, window_seconds = 60, on_limit = "warn" }
```

### Filter evaluation order

Filters are evaluated in order from cheapest to most expensive:

1. **event_kind**: O(1) check against Pulse Kind
2. **matches**: O(n) field comparison against payload
3. **where_clause**: Expr evaluation (bounded by 100ms timeout)
4. **debounce**: time check against last fire timestamp
5. **rate_limit**: counter check against window
6. **custom_filter**: Cell execution (most expensive, skipped if earlier filters reject)

---

## 7. Events as Pulses on Bus

All trigger system events are published as Pulses on Bus. This makes the trigger system observable via the same Lens and telemetry infrastructure as the rest of the runtime.

| Event | Topic | Graduates? |
|---|---|---|
| Trigger armed | `trigger.{name}.armed` | Yes |
| Trigger fired | `trigger.{name}.fired` | Yes |
| Trigger filtered (event rejected) | `trigger.{name}.filtered` | No (noise) |
| Trigger skipped (concurrency) | `trigger.{name}.skipped` | Yes |
| Trigger queued (concurrency) | `trigger.{name}.queued` | No |
| Trigger rate-limited | `trigger.{name}.rate_limited` | Yes |
| Trigger error | `trigger.{name}.error` | Yes |
| Trigger disarmed | `trigger.{name}.disarmed` | Yes |
| Flow started by trigger | `trigger.{name}.flow.started` | Yes |
| Flow completed by trigger | `trigger.{name}.flow.completed` | Yes |

### Graduation policy

Trigger events that affect system state (armed, fired, error, disarmed) graduate to Signals for audit. Routine events (filtered, queued) do not. Rate-limited events graduate because they indicate capacity issues.

---

## 8. Auth and Secrets

Trigger authentication uses the workspace secret store. Secrets are never stored in TOML — only references.

```rust
pub enum TriggerAuth {
    /// No authentication.
    None,

    /// HMAC-SHA256 signature verification (webhooks).
    HmacSha256 {
        /// Reference to secret in the workspace secret store.
        secret: SecretRef,
        /// HTTP header containing the signature.
        header: String,
    },

    /// Bearer token (API keys).
    BearerToken {
        secret: SecretRef,
    },

    /// Mutual TLS.
    MutualTls {
        cert: PathBuf,
        key: SecretRef,
    },
}

pub enum SecretRef {
    /// Read from environment variable.
    Env(String),

    /// Read from the workspace secret store.
    Store { key: String },

    /// Read from a file.
    File(PathBuf),
}
```

```toml
# Webhook with HMAC-SHA256 verification
[[triggers]]
name = "github-webhook"
kind = "webhook"
path = "/github"

[triggers.auth]
kind = "hmac_sha256"
secret = { env = "GITHUB_WEBHOOK_SECRET" }
header = "X-Hub-Signature-256"
```

---

## 9. CLI Surface

The trigger system is fully manageable via CLI.

### Trigger management

| Command | What it does |
|---|---|
| `roko trigger list` | List all trigger bindings with status (armed/disarmed) |
| `roko trigger show <name>` | Show trigger details, recent firings, concurrency state |
| `roko trigger create <file>` | Create a trigger binding from TOML |
| `roko trigger enable <name>` | Enable a disabled trigger |
| `roko trigger disable <name>` | Disable an active trigger (disarm) |
| `roko trigger fire <name> [payload]` | Manually fire a trigger with optional payload |
| `roko trigger delete <name>` | Remove a trigger binding |
| `roko trigger history <name>` | Show firing history with Flow references |
| `roko trigger test <name> <payload>` | Dry-run: evaluate filters without starting a Flow |

### Examples

```bash
# List all triggers
roko trigger list

# Show details of a specific trigger
roko trigger show github-pr-opened

# Create a new trigger from TOML
roko trigger create triggers/my-trigger.toml

# Manually fire a trigger with payload
roko trigger fire manual-deploy '{"branch": "main", "env": "staging"}'

# Test filter evaluation without firing
roko trigger test on-gate-failure '{"hard_pass": false, "reward": 0.2}'

# View firing history
roko trigger history nightly-consolidation --limit 10
```

### API surface

| Method | Path | What |
|---|---|---|
| `GET` | `/triggers` | List all trigger bindings |
| `GET` | `/triggers/{name}` | Get trigger details |
| `POST` | `/triggers` | Create trigger binding |
| `PUT` | `/triggers/{name}` | Update trigger binding |
| `DELETE` | `/triggers/{name}` | Delete trigger binding |
| `POST` | `/triggers/{name}/fire` | Manually fire |
| `POST` | `/triggers/{name}/enable` | Enable trigger |
| `POST` | `/triggers/{name}/disable` | Disable trigger |
| `GET` | `/triggers/{name}/history` | Firing history |
| `POST` | `/webhooks/{path}` | Webhook endpoint (dynamic per binding) |

---

## 10. Trigger Lifecycle

```
Defined (TOML binding created)
    │
    ├─► arm() ──► Armed ──► Listening for events
    │                │
    │                ├─► Event matches ──► Filter passes ──► fire()
    │                │                          │
    │                │                          ├─► ConcurrencyPolicy check
    │                │                          │       │
    │                │                          │       ├─► Queue/Skip/Cancel/Parallel
    │                │                          │       │
    │                │                          │       └─► Start Flow
    │                │                          │
    │                │                          └─► Filter rejects ──► log + continue
    │                │
    │                ├─► Error ──► retry (3x) ──► Failed
    │                │
    │                └─► disarm() ──► Disarmed
    │
    └─► Disabled (never armed)
```

---

## 11. Acceptance Criteria

| Criterion | Verification |
|---|---|
| `TriggerProtocol` trait compiles with `arm`, `disarm`, `poll` | Compile check |
| `TriggerBinding` persists to `.roko/triggers/` and survives restart | Integration test: create binding, restart, verify armed |
| Cron trigger fires at the scheduled time | Integration test with mock clock |
| Cron trigger does not double-fire on restart | Integration test: fire, restart within same minute, verify single fire |
| Webhook trigger registers HTTP route and receives events | Integration test: POST to webhook endpoint, verify trigger fires |
| Webhook HMAC-SHA256 verification rejects invalid signatures | Unit test |
| FileWatch trigger fires on file modification | Integration test: modify file, verify trigger fires |
| FileWatch glob filter excludes non-matching files | Unit test |
| Bus trigger fires on matching Pulse | Integration test: publish Pulse, verify trigger fires |
| Bus trigger body predicate filters correctly | Unit test: matching and non-matching payloads |
| ChainEvent trigger fires on matching on-chain event | Integration test (mocked chain indexer) |
| Manual trigger fires via CLI and API | Integration test |
| SignalPattern trigger fires when pattern threshold met | Integration test: add Signals, verify fires at min_matches |
| ConcurrencyPolicy Queue: second fire queues behind first | Integration test |
| ConcurrencyPolicy Skip: second fire is dropped | Integration test |
| ConcurrencyPolicy CancelRunning: first Flow cancelled on second fire | Integration test |
| ConcurrencyPolicy Parallel: both Flows run concurrently | Integration test |
| Trigger chaining: Flow A completes -> Bus trigger B fires -> Flow B starts | Integration test (full chain) |
| Filter debounce: rapid fires produce single execution | Integration test: 10 events in 1s, debounce=2s, verify single fire |
| Filter rate_limit: max_fires respected within window | Integration test |
| Filter evaluation order: cheapest first, custom_filter last | Unit test with instrumented filters |
| All trigger events published as Pulses on correct topics | Integration test |
| Graduation policy: `trigger.*.fired` graduates, `trigger.*.filtered` does not | Integration test |
| SecretRef.Env reads from environment | Unit test |
| CLI `roko trigger list` shows all bindings with status | Integration test |
| CLI `roko trigger fire` starts a Flow | Integration test |
| CLI `roko trigger test` evaluates filters without starting Flow | Integration test |
| API endpoints return correct responses | Integration test per endpoint |
| Input mapping transforms trigger payload into Graph input Signals | Unit test |
| Disabled trigger is not armed on startup | Integration test |
