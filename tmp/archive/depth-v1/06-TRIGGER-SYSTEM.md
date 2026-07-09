# 06 — Trigger System

> Event-driven Graph firing. Triggers decide *when* a Graph runs.

**Subsumes**: Cron jobs, webhooks, file watchers, event-bus subscriptions, chain event listeners, manual invocations.

**Source**: Refactored from `tmp/workflow/03-trigger-system.md` with unified vocabulary.

---

## 1. Overview

A **Trigger** is a Block implementing the Trigger protocol (see [doc-02 §3.9](02-BLOCK.md)). Triggers are the system's event ingress — they listen for events from the outside world and fire Graphs in response.

The Graph does not know how it was fired. The same `doc-ingest` Graph runs identically whether triggered by `roko run doc-ingest <dir>`, by a watcher on `<dir>` detecting a new file, or by a GitHub PR comment containing `/ingest`.

### Key properties

- **First-class**: Triggers are Blocks with identity, version, capabilities, and lifecycle
- **Decoupled**: Triggers bind to Graphs via `TriggerBinding`, not hardcoded references
- **Varied**: A single Graph can be fired by many different Triggers simultaneously
- **Composable**: Graph completion can fire another Trigger (chaining)
- **Persistent**: Bindings are TOML files that survive daemon restarts

---

## 2. The Trigger Protocol

Defined in [doc-02 §3.9](02-BLOCK.md):

```rust
pub trait Trigger: Block {
    /// Start listening for events.
    async fn arm(&mut self, binding: &TriggerBinding) -> Result<()>;

    /// Stop listening.
    async fn disarm(&mut self) -> Result<()>;

    /// Check if trigger condition is met (for poll-based Triggers).
    async fn poll(&self) -> Result<Option<TriggerEvent>>;
}
```

Trigger Blocks also implement the standard Block trait, which means they have:
- `name()`, `version()`, `description()`, `tags()`
- `capabilities()` — what system resources the Trigger needs (net, fs, secrets)
- `protocols()` — includes `Protocol::Trigger`

### Trigger hosts

Triggers run in different host processes depending on their nature:

| Host | Process | Trigger kinds |
|---|---|---|
| `InProcess` | CLI process | Manual, one-shot Schedule |
| `Daemon` | `roko daemon` | Cron, FileWatch, Bus, ChainEvent, SignalPattern |
| `HttpServer` | `roko serve` | Webhook, GitHub, Slack |
| `External` | User-provided | Cloud functions, external cron, third-party integrations |

---

## 3. TriggerBinding

A **TriggerBinding** is the connection between a Trigger and a Graph. Bindings are persistable TOML, live in `.roko/triggers/<name>.toml` (per-Space) or `~/.roko/triggers/<name>.toml` (user-global).

```rust
pub struct TriggerBinding {
    pub name: String,
    pub description: String,
    pub enabled: bool,

    // ── Source ───────────────────────────────────────────
    pub trigger: TriggerRef,            // which Trigger Block
    pub trigger_config: Value,          // kind-specific config (schedule, path, etc.)

    // ── Target ──────────────────────────────────────────
    pub graph: GraphRef,                // which Graph to fire
    pub version_req: String,            // semver requirement (e.g., "^1")

    // ── Input mapping ───────────────────────────────────
    pub input_map: Vec<Mapping>,        // map trigger event → Graph input
    pub macro_overrides: MacroBindings, // Macro values applied to every run

    // ── Filtering ───────────────────────────────────────
    pub filter: Option<Expr>,           // optional filter on trigger events

    // ── Concurrency ─────────────────────────────────────
    pub concurrency: ConcurrencyPolicy,
}
```

### TOML authoring

```toml
[trigger]
name        = "ingest-on-new-doc"
description = "Run doc-ingest whenever a new markdown file lands in tmp/docs"
enabled     = true

[trigger.source]
kind        = "file-watch"
path        = "tmp/docs"
recursive   = true
patterns    = ["**/*.md"]
ignore      = ["**/.git/**", "**/node_modules/**"]

[trigger.filter]
event_kind  = ["created", "modified"]
debounce_ms = 5000
min_size    = 100
max_size    = 1048576

[trigger.binding]
graph       = "doc-ingest"
version_req = "^1"

# Map trigger event payload onto Graph input. Templating uses minijinja.
[trigger.binding.input]
source_dir       = "{{ trigger.event.path | dirname }}"
new_files        = ["{{ trigger.event.path }}"]
incremental      = true

# Macro overrides applied to every run from this trigger.
[trigger.binding.macros]
enable_audit         = true
enable_web_research  = false
max_refine_iterations = 2

# Concurrency policy.
[trigger.policy]
concurrency  = "queue"
queue_depth  = 16
deduplicate  = true
on_failure   = "log"
```

---

## 4. Built-In Trigger Kinds

### 4.1 Cron

Standard 5-or-6-field cron syntax with timezone support:

```toml
[trigger.source]
kind     = "cron"
schedule = "0 3 * * *"                 # 3am daily
timezone = "America/New_York"
catch_up = "skip"                      # "skip" | "run-once" | "run-all" if missed
jitter_s = 60                          # randomize start to spread load
```

Hosted in the `roko daemon`. Use cases: nightly research sweeps, weekly doc audits, quarterly dependency updates, hourly knowledge-decay GC.

### 4.2 Webhook

HTTP endpoint hosted by `roko serve`:

```toml
[trigger.source]
kind          = "webhook"
path          = "/webhooks/deploy-trigger"
methods       = ["POST"]
auth          = { kind = "hmac", header = "X-Signature", secret_ref = "deploy_webhook_secret" }
content_types = ["application/json"]
max_body_size = 65536
```

The binding's `input_map` transforms the inbound body into Graph input via minijinja templates. Idempotency keys (header `Idempotency-Key`) deduplicate retries.

**Auth is required by default.** A Webhook Trigger without an `auth` block fails to register.

### 4.3 FileWatch

Filesystem-event-driven via `notify` crate:

```toml
[trigger.source]
kind        = "file-watch"
path        = "src/"
recursive   = true
patterns    = ["**/*.rs"]
ignore      = ["target/**"]
events      = ["created", "modified", "removed"]
debounce_ms = 2000
batch       = true                     # collapse rapid changes into a single event
```

Use cases: run tests on file change, regenerate docs on code change, retrigger doc-ingest on source-doc change.

### 4.4 Bus

Subscribe to internal Signal events on the Bus (see [doc-01 §9](01-SIGNAL.md)):

```toml
[trigger.source]
kind     = "bus"
topic    = "graph:*:events"
events   = ["GraphCompleted"]
filter   = "tags includes 'produces-prd'"
```

Use cases: audit pipeline fires on every Graph completion; observability pipeline fires on budget exceeded; retrospective pipeline fires on Flow failure.

### 4.5 ChainEvent

Subscribe to smart contract events on-chain:

```toml
[trigger.source]
kind     = "chain-event"
network  = "ethereum"
contract = "0x1234..."
event    = "InsightStored(bytes32,address)"
from_block = "latest"
```

Requires `Capability::Chain { read: true }`. Hosted in daemon with reconnection logic.

### 4.6 Manual

The default. Fired by:
- `roko run <graph-name> [-- input...]`
- Dashboard "Run" button
- TUI hotkey
- HTTP `POST /graphs/{name}/run`

Carries no scheduling. Always permitted. Hosted in-process.

### 4.7 SignalPattern

Fire when a Signal appears on the Bus whose HDC fingerprint is similar to a reference pattern above a threshold:

```toml
[trigger.source]
kind      = "signal-pattern"
pattern   = { kind = "Insight", tags = ["security", "vulnerability"] }
threshold = 0.75
topic     = "signal:Insight"
```

Use cases: security review fires when a vulnerability insight appears; dream consolidation fires when enough related knowledge accumulates.

---

## 5. Concurrency Policies

When a Trigger fires while a previous Flow from the same binding is still executing:

```rust
pub enum ConcurrencyPolicy {
    /// Queue up to `queue_depth`; drop oldest beyond capacity.
    Queue { queue_depth: u32, deduplicate: bool },

    /// Ignore the new event entirely.
    Skip,

    /// Cancel the in-flight Flow; start the new one.
    CancelRunning,

    /// Run concurrently, bounded by max.
    Parallel { max: u32 },
}
```

| Policy | Behavior | Best for |
|---|---|---|
| `Queue` (default) | Queue up to depth; drop oldest beyond capacity | File watchers, event streams |
| `Skip` | Ignore if already running | Idempotent periodic tasks |
| `CancelRunning` | Cancel in-flight, start new | Latest-wins scenarios (deploy) |
| `Parallel` | Run concurrently up to max | Independent work items |

### Deduplication

When `deduplicate = true` (available on `Queue`), the engine collapses identical pending events by content hash. A file watcher on a 1000-file rename doesn't queue 1000 Flows — identical events in the queue are merged.

---

## 6. Trigger Chaining

Graph completion can fire another Trigger via the Bus trigger kind:

```toml
# Trigger: when doc-ingest completes, fire prd-enrich
[trigger]
name = "enrich-after-ingest"

[trigger.source]
kind     = "bus"
topic    = "graph:doc-ingest:events"
events   = ["GraphCompleted"]

[trigger.filter]
where = "output.status == 'success'"

[trigger.binding]
graph = "prd-enrich"

[trigger.binding.input]
prds_to_enrich = "{{ trigger.event.output.created_prds }}"
```

Chaining is how higher-order pipelines are built without one mega-Graph. Each chain link is independently observable, restartable, and overridable.

### Chain depth

There is no explicit chain depth limit, but budget enforcement on each Flow prevents infinite chains. Each chained Flow consumes its own budget allocation.

---

## 7. Filtering

Every Trigger source may carry a filter. Filtering happens **before dispatch** — filtered-out events are logged but do not consume queue slots or dispatch counters.

### Filter operators

```toml
[trigger.filter]
# Regex match on payload field
matches = { field = "body", pattern = "^/run\\s" }

# Numeric / string comparisons
where = "payload.size > 100 AND payload.size < 1048576"

# Event-kind whitelist
event_kind = ["created", "modified"]

# Debouncing (collapse rapid fires)
debounce_ms = 5000

# Rate limiting
rate_limit_per_min = 10

# Require specific fields in payload
require_fields = ["payload.path", "payload.author"]

# External matcher (a Block that returns bool)
custom_filter = { block = "my-org.event-classifier", version = "^1" }
```

### Filter evaluation order

1. `event_kind` — fast whitelist check
2. `require_fields` — presence check
3. `where` — Expr evaluation
4. `matches` — regex on specific fields
5. `custom_filter` — Block invocation (most expensive, last)
6. `debounce_ms` — temporal dedup (applied after content filters)
7. `rate_limit_per_min` — hard rate cap

---

## 8. Observability

Every Trigger fire produces a `TriggerEvent` Signal:

```rust
pub struct TriggerEvent {
    pub id: TriggerEventId,
    pub trigger: String,
    pub space: SpaceRef,
    pub fired_at: DateTime<Utc>,
    pub source_kind: TriggerKind,
    pub payload: Value,
    pub filter_pass: bool,
    pub dispatch_id: Option<RunId>,      // None if filtered out
    pub deduped: bool,
    pub error: Option<TriggerError>,
}
```

`TriggerEvent` Signals persist to `.roko/trigger-events.jsonl` and stream to the Bus on topic `trigger:{name}:events`. Per-Trigger health is computed:

| Metric | What |
|---|---|
| Fire rate | Events per minute |
| Filter pass rate | % of events that pass filters |
| Dispatch success rate | % of dispatched Flows that complete |
| Last error | Most recent error, if any |
| Queue depth | Current pending events (for Queue policy) |

The dashboard's Trigger tab renders these metrics in real time.

---

## 9. Authentication and Secrets

Webhook, GitHub, and Slack Triggers require credentials. Secrets are stored per-Space via the secret backend and referenced by name in Trigger TOML using `_ref = "secret_name"`:

```toml
auth = { kind = "hmac", header = "X-Signature", secret_ref = "deploy_webhook_secret" }
```

Secrets are never inlined in TOML. The Trigger system resolves `_ref` at arm time. If a secret is missing, the Trigger fails to arm with a clear error — it does not start silently without authentication.

---

## 10. CLI Surface

```
roko trigger list [--space <name>] [--kind <kind>]
roko trigger show <name>
roko trigger create <name> --kind <kind> [--graph <name>] [--config <toml>]
roko trigger edit <name>                 # opens $EDITOR on the trigger TOML
roko trigger enable <name>
roko trigger disable <name>
roko trigger remove <name>
roko trigger test <name> [--payload <json>]   # synthetic-fire without external events
roko trigger logs <name> [--tail <n>]
roko trigger status                      # daemon's current trigger registry
```

`roko trigger test` is the development workhorse: dispatches the Trigger with a hand-crafted payload, useful for iterating on filter and binding mappings without waiting for real events.

---

## 11. Security Considerations

| Concern | Mitigation |
|---|---|
| Webhook auth | Required by default. Unauthenticated Webhook Triggers fail to register. |
| Capability disclosure | Marketplace-installed Triggers disclose their capability set on install. |
| Idempotency | Every external event carries an idempotency key (or one is derived from payload hash). |
| Payload limits | Default 64KB max body for Webhook; overridable up to 4MB per Trigger. |
| Egress | Triggers that POST to remote services require explicit `Capability::Net { domains }`. |
| Secret exposure | Secrets resolved at arm time, never serialized into events or logs. |

---

## 12. TOML Configuration Reference

Complete Trigger binding schema:

```toml
[trigger]
name        = "string"           # unique within Space, kebab-case
description = "string"           # human-readable
enabled     = true               # default: true

[trigger.source]
kind        = "string"           # cron | webhook | file-watch | bus | chain-event | manual | signal-pattern
# ... kind-specific fields (see §4)

[trigger.filter]
event_kind         = ["string"]  # whitelist of event types
where              = "string"    # Expr filter
matches            = { field = "string", pattern = "string" }  # regex
debounce_ms        = 0           # temporal dedup (ms)
rate_limit_per_min = 0           # hard rate cap (0 = unlimited)
require_fields     = ["string"]  # required payload fields
custom_filter      = { block = "string", version = "string" }  # Block filter

[trigger.binding]
graph       = "string"           # target Graph name
version_req = "string"           # semver requirement

[trigger.binding.input]
# minijinja template mapping trigger event → Graph input
# key = "{{ trigger.event.field }}"

[trigger.binding.macros]
# Macro overrides for every run from this Trigger
# key = value

[trigger.policy]
concurrency   = "queue"          # queue | skip | cancel-running | parallel
queue_depth   = 16               # max pending events (queue policy only)
max_parallel  = 4                # max concurrent Flows (parallel policy only)
deduplicate   = false            # collapse identical pending events
on_failure    = "log"            # log | alert | stop
```

---

## 13. Acceptance Criteria

| # | Criterion | Verification |
|---|---|---|
| 1 | Cron Trigger registers and daemon picks it up | `roko trigger list` shows it; daemon logs schedule |
| 2 | FileWatch Trigger fires within 2s of matching file change | Integration test with `notify` + file touch |
| 3 | Webhook Trigger validates HMAC; mismatched signatures rejected with 401 | `curl` test with valid + invalid signatures |
| 4 | Bus Trigger fires on matching Graph completion event | Two-Graph chain integration test |
| 5 | Concurrency `Queue` with `queue_depth = 2` drops the third concurrent fire | Synthetic test with 4 rapid fires |
| 6 | Concurrency `Skip` ignores new event while Flow is running | Test: fire twice rapidly, second ignored |
| 7 | Concurrency `CancelRunning` cancels in-flight Flow | Test: fire twice, first cancelled |
| 8 | `roko trigger test` dispatches without external infrastructure | CI test exercise |
| 9 | Disabled Trigger (`enabled = false`) does not fire | Daemon log check |
| 10 | TriggerEvent Signals persist to `trigger-events.jsonl` | File check after fire |
| 11 | Filter `debounce_ms` collapses rapid fires | 10 fires in 1s with 5s debounce → 1 dispatch |
| 12 | Filter `rate_limit_per_min` caps dispatch rate | 100 fires/min with limit=10 → 10 dispatches |
| 13 | Secret resolution fails clearly when secret missing | Test: missing secret → error, not silent |
| 14 | SignalPattern Trigger fires on HDC-similar Signal | Test: publish similar Signal, verify fire |
| 15 | Trigger chaining: Graph A → Trigger → Graph B observable in timeline | End-to-end chain test |

---

## 14. Open Questions

- **Distributed coordination**: Should multi-machine daemons coordinate via leader election to prevent duplicate fires? Out of scope for v1; revisit with fleet scaling.
- **Synchronous webhook responses**: Should Webhook Triggers return a response derived from the dispatched Flow? Needed for some Slack patterns; defer until the shape is clear.
- **Burst aggregation**: The `batch = true` option on FileWatch collapses rapid changes into one event. The exact schema for batch events needs specification in v1.
