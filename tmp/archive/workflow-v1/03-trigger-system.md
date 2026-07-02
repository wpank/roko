# PRD-03 — Trigger System

**Status**: Draft
**Author**: Will (architect) + Claude (synthesis)
**Date**: 2026-04-25
**Crate**: `roko-trigger` (new) + integrations into `roko-daemon`, `roko-serve`, `roko-cli`
**Prerequisites**: PRD-00, PRD-01, PRD-02

---

## 0. Scope

This document defines the Trigger primitive — the system that decides *when* a Workflow runs. Triggers are first-class, decoupled from Workflows, and varied: a single Workflow can be fired by manual CLI invocation, a cron schedule, a file-system watcher, an inbound webhook, a GitHub PR event, a Slack message, an artifact-change notification, or another Workflow's completion.

The Workflow does not know how it was fired. The same `doc-ingest` Workflow runs identically whether triggered by `roko run doc-ingest <dir>`, by a watcher on `<dir>` detecting a new file, or by a GitHub PR comment containing `/ingest`.

---

## 1. The Trigger Trait

```rust
/// A first-class primitive that fires Workflows.
///
/// Triggers are persistable, configurable, capability-gated, and host-agnostic.
/// The roko daemon hosts long-running triggers; the CLI hosts one-shot
/// invocations; the dashboard hosts UI-driven manual triggers; the serve
/// process hosts webhook receivers.
#[async_trait]
pub trait Trigger: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &Version;
    fn description(&self) -> &str;
    fn kind(&self) -> TriggerKind;

    /// Capabilities the trigger needs to operate (typically `net`, `fs.read`,
    /// `secrets`).
    fn capabilities(&self) -> &[Capability];

    /// Where this trigger needs to live: in-process, in the daemon, in the
    /// HTTP server, or as a separate worker.
    fn host(&self) -> TriggerHost;

    /// Start the trigger. The trigger calls `dispatch` whenever it fires.
    /// Returns when the trigger is stopped (e.g. daemon shutdown).
    async fn run(
        &self,
        config: TriggerConfig,
        dispatch: TriggerDispatchHandle,
        ctx: &TriggerContext,
    ) -> Result<(), TriggerError>;
}

pub enum TriggerHost {
    InProcess,        // for one-shot CLI triggers (manual)
    Daemon,           // long-running watch/cron/event-bus
    HttpServer,       // webhooks (need port)
    External,         // user-provided process / cloud function
}

pub enum TriggerKind {
    Manual,
    Cron,
    FileWatch,
    FolderWatch,
    Webhook,
    GitHub,
    Slack,
    EventBus,
    ArtifactChange,
    WorkflowCompletion,
    Schedule,         // one-time absolute timestamp
    Custom(String),
}
```

---

## 2. Trigger → Workflow Binding

A binding is the connection between a Trigger and a Workflow. Bindings are persistable TOML, live in `<workspace>/.roko/triggers/<name>.toml` (workspace) or `~/.roko/triggers/<name>.toml` (user).

```toml
[trigger]
name        = "ingest-on-new-doc"
description = "Run doc-ingest whenever a new markdown file lands in tmp/docs"
enabled     = true

[trigger.source]
kind        = "folder-watch"
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
workflow    = "doc-ingest"
version_req = "^1"

# Map trigger event payload onto workflow input. Templating uses minijinja.
[trigger.binding.input]
source_dir       = "{{ trigger.event.path | dirname }}"
new_files        = ["{{ trigger.event.path }}"]
incremental      = true

# Macro overrides applied to every run from this trigger.
[trigger.binding.macros]
enable_audit         = true
enable_web_research  = false
max_refine_iterations = 2

# Concurrency policy: drop, queue, or cancel-running.
[trigger.policy]
concurrency  = "queue"
queue_depth  = 16
deduplicate  = true                      # collapse identical events while queued
on_failure   = "log"                     # "log" | "alert" | "stop"
on_workspace_locked = "queue"            # if workspace is in maintenance mode
```

---

## 3. Built-In Trigger Kinds

### 3.1 Manual

The default. Fired by `roko run <workflow-name> [-- input...]`, dashboard "Run" button, TUI hotkey, or HTTP POST to `/workflows/{name}/run`. Carries no scheduling. Always permitted.

### 3.2 Cron

Standard 5-or-6-field cron syntax with timezone:

```toml
[trigger.source]
kind     = "cron"
schedule = "0 3 * * *"                 # 3am daily
timezone = "America/New_York"
catch_up = "skip"                      # "skip" | "run-once" | "run-all" if missed
jitter_s = 60                          # randomize start time to spread load
```

Hosted in the roko daemon. Multiple cron triggers per workspace allowed. Use cases: nightly research sweeps, weekly doc audits, quarterly dependency updates, hourly knowledge-decay GC.

### 3.3 FileWatch / FolderWatch

Filesystem-event-driven via `notify` crate. Already wired in TUI for the file watcher; here it's exposed as a trigger.

```toml
[trigger.source]
kind        = "folder-watch"
path        = "src/"
recursive   = true
patterns    = ["**/*.rs"]
ignore      = ["target/**"]
events      = ["created", "modified", "removed"]
debounce_ms = 2000
batch       = true                     # collapse rapid changes into a single event
```

Use cases: run tests on file change, regenerate docs on code change, refresh design tokens on token-file change, retrigger doc-ingest on source-doc change.

### 3.4 Webhook (Generic)

HTTP endpoint hosted by `roko-serve`:

```toml
[trigger.source]
kind          = "webhook"
path          = "/webhooks/deploy-trigger"
methods       = ["POST"]
auth          = { kind = "hmac", header = "X-Signature", secret_ref = "deploy_webhook_secret" }
content_types = ["application/json"]
max_body_size = 65536
```

The trigger binding may transform the inbound body into Workflow input via a minijinja template. Idempotency keys (header `Idempotency-Key`) deduplicate retries.

### 3.5 GitHub

Specialization of webhook with built-in event-shape parsing.

```toml
[trigger.source]
kind        = "github"
events      = ["pull_request.opened", "issue_comment.created", "workflow_dispatch"]
repo        = "wpank/nunchi-dashboard"      # restrict; multiple repos allowed
auth        = { kind = "github-app", app_id_ref = "gh_app_id", private_key_ref = "gh_app_pk" }

[trigger.filter]
# Run only when comment body contains `/ingest <path>` mentioning roko.
matches     = ["^/ingest\\s+(?P<path>.+)$"]
require_label = ["roko"]
require_perm  = "write"                      # commenter must have repo write
```

Use cases: a PR comment `/review` fires the code-review Workflow; an issue label `needs-research` fires research-sweep; a `workflow_dispatch` fires deploy.

### 3.6 Slack

Specialization for Slack events: messages, slash commands, message-shortcuts, app-mentions.

```toml
[trigger.source]
kind          = "slack"
events        = ["message.app_mention", "slash_command:/roko"]
workspace_id  = "T01ABCDEF"
auth          = { kind = "bot-token", token_ref = "slack_bot_token" }

[trigger.filter]
channels      = ["C0123456"]
matches       = ["^research\\s+(?P<topic>.+)$"]
```

Slash commands and message-shortcuts post the trigger event onto the workspace bus and synchronously respond with an acknowledgement; the Workflow runs async and reports back via Slack reply.

### 3.7 EventBus

Subscribe to internal Workflow events from PRD-02 §8. Use cases:
- Audit pipeline fires on every `RunCompleted` for any Workflow tagged `produces-prd`.
- Observability pipeline fires on every `BudgetExceeded`.
- Retrospective pipeline fires on every `RunFailed`.

```toml
[trigger.source]
kind     = "event-bus"
events   = ["RunCompleted"]
filter   = "tags includes 'produces-prd'"
```

### 3.8 ArtifactChange

Fires when an artifact in the workspace store changes (new artifact appears, lineage updated, tag added). Use cases:
- A new research artifact triggers PRD-enrich.
- A new deploy-config artifact triggers deploy-validate.

```toml
[trigger.source]
kind       = "artifact-change"
kinds      = ["markdown"]
tags       = ["research-output"]
on         = ["created"]
```

### 3.9 WorkflowCompletion

Chain Workflows: when A finishes, fire B with A's output as B's input.

```toml
[trigger.source]
kind         = "workflow-completion"
workflow     = "doc-ingest"
version_req  = "^1"
on           = ["success"]                # "success" | "failure" | "any"

[trigger.binding]
workflow     = "prd-enrich"

[trigger.binding.input]
prds_to_enrich = "{{ trigger.run.output.created_prds }}"
```

Workflow chaining is how higher-order pipelines are built without one mega-Workflow. Each chain link is independently observable, restartable, and overridable.

### 3.10 Schedule (one-shot)

A specific absolute timestamp.

```toml
[trigger.source]
kind     = "schedule"
at       = "2026-05-01T09:00:00-04:00"
```

Auto-disables after firing.

### 3.11 Custom (extension point)

User-provided trigger Modules. Implementing the `Trigger` trait in a plugin crate, WASM module, or script. Capabilities are declared and granted as for any Module.

---

## 4. Trigger Hosts

### 4.1 In-Process

The CLI process hosts manual triggers and one-shot schedules. `roko run <workflow>` is in-process.

### 4.2 Daemon

`roko daemon` hosts long-running triggers: cron, watches, event-bus subscriptions, artifact-change subscriptions, workflow-completion subscriptions, custom long-running triggers. The daemon is a single process serving all workspaces registered on the machine. Triggers are isolated per workspace via tokio task groups; one workspace's panic cannot affect another.

### 4.3 HTTP Server

`roko serve` hosts webhook, GitHub, and Slack triggers. The server validates auth, parses event shape, and forwards to the daemon (or runs in-process if daemon is co-located) for actual Workflow execution.

### 4.4 External

A trigger may live entirely outside roko (e.g. a Cloudflare Worker, a Vercel cron) and POST to a webhook trigger. This is the lowest-coupling way to integrate roko with existing infrastructure.

---

## 5. Concurrency Policies

When a trigger fires while a previous run from the same trigger is still executing:

| Policy | Behavior |
|---|---|
| `queue` (default) | Queue up to `queue_depth`; drop oldest beyond capacity. |
| `drop` | Ignore the new event entirely. |
| `cancel-running` | Cancel the in-flight run; start the new one. |
| `wait` | Block the trigger until in-flight run completes; useful for cron with no overlap. |
| `parallel` | Run concurrently; bounded by `max_parallelism`. |

Deduplication (`deduplicate = true`) collapses identical pending events while queued, so a watcher on a 1000-file rename doesn't queue 1000 runs.

---

## 6. Filtering

Every trigger source may carry a `[trigger.filter]` table. Filter operators:

```toml
[trigger.filter]
# regex on payload field
matches            = { field = "body", pattern = "^/run\\s" }

# numeric comparisons
where              = "payload.size > 100 AND payload.size < 1048576"

# event-kind whitelist
event_kind         = ["created", "modified"]

# debouncing
debounce_ms        = 5000

# rate limiting
rate_limit_per_min = 10

# require fields
require_fields     = ["payload.path", "payload.author"]

# external matcher (a Module that returns bool)
custom_filter      = { module = "my-org.event-classifier", version = "^1" }
```

Filtering happens before dispatch. A filtered-out event is logged but does not consume queue slots or dispatch counters.

---

## 7. CLI Surface

```
roko trigger list [--workspace <name>] [--kind <kind>]
roko trigger show <name>
roko trigger create <name> --kind <kind> [--workflow <name>] [--config <toml>]
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

## 8. Authentication & Secrets

Webhook / GitHub / Slack triggers require credentials. Secrets are stored per-workspace via the workspace's secret backend (PRD-01 §3) and referenced by name in trigger TOML using `_ref = "secret_name"`.

```toml
auth = { kind = "hmac", header = "X-Signature", secret_ref = "deploy_webhook_secret" }
```

Secrets are never inlined in TOML. The trigger system resolves `_ref` at start time; if the secret is missing the trigger fails to start with a clear error, not silently.

---

## 9. Observability

Every trigger fire produces a `TriggerEvent`:

```rust
pub struct TriggerEvent {
    pub id:           TriggerEventId,
    pub trigger:      String,
    pub workspace:    WorkspaceRef,
    pub fired_at:     DateTime<Utc>,
    pub source_kind:  TriggerKind,
    pub payload:      Value,
    pub filter_pass:  bool,
    pub dispatch_id:  Option<RunId>,           // None if filtered out
    pub deduped:      bool,
    pub error:        Option<TriggerError>,
}
```

`TriggerEvent`s persist to `<workspace>/.roko/trigger-events.jsonl` and stream to the dashboard. Per-trigger health is computed: fire rate, filter pass rate, dispatch success rate, last error. The dashboard's Trigger Manager (PRD-10) renders this.

---

## 10. Security Considerations

- **Webhook auth is required by default**. A webhook trigger without an `auth` block fails to register.
- **Capability disclosure**. Marketplace-installed triggers disclose their capability set on install (PRD-12). Installing a trigger with `capabilities = ["chain.write"]` requires explicit user grant.
- **Idempotency**. Every external trigger event carries an idempotency key (or one is derived from payload hash). Replays are coalesced.
- **Payload limits**. Default 64KB max body for webhook; overridable per trigger up to 4MB. Above that, the trigger should accept a URL pointing to the payload and fetch it inside the Workflow.
- **Egress on chain triggers**. Triggers that POST to remote services (Slack reply, GitHub comment) require explicit `net.domains` capability.

---

## 11. Acceptance Criteria

| Criterion | Verification |
|---|---|
| `roko trigger create --kind cron --workflow X --schedule "0 * * * *"` registers the trigger and the daemon picks it up. | `roko trigger list` shows it; daemon logs the schedule. |
| File-watch trigger fires within 2s of a matching file change in the watched dir. | Integration test with `notify` and a touch. |
| Webhook trigger validates HMAC; mismatched signatures rejected with 401. | `curl` test with valid + invalid signatures. |
| GitHub trigger parses `pull_request.opened` and runs the bound Workflow with PR metadata as input. | Replay a captured GitHub event; verify Workflow input. |
| Workflow-completion trigger chains Workflows; A→B sequencing observable in run timeline. | Two-step chain integration test. |
| Concurrency `queue` policy with `queue_depth = 2` drops the third concurrent fire. | Synthetic test with 4 rapid fires. |
| `roko trigger test` dispatches without requiring external infrastructure. | Test exercise in CI. |
| Disabled trigger (`enabled = false`) does not fire even if event arrives. | Daemon log check. |
| Trigger events persist to `trigger-events.jsonl` and dashboard streams them. | Dashboard component test against fixture event. |

---

## 12. Open Questions

- Should the trigger system support distributed coordination (multi-machine daemons coordinating via a leader-election so the same trigger isn't fired twice)? Out of scope for v1; revisit when fleet scaling becomes real.
- Should webhook triggers support response shaping (the trigger returns a synchronous reply derived from the dispatched Workflow)? Probably yes for some Slack patterns; defer until shape is clear.
- Should there be a "burst" event aggregation (e.g., 50 file changes in 5s become one synthetic "batch" event with 50 paths)? Yes — `batch = true` already declared in §3.3; specify the schema in v1.
