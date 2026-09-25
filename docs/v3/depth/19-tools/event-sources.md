# Event Sources

> Depth file for [19-TOOLS-PLUGINS](../../19-TOOLS-PLUGINS.md) -- how external
> signals (cron, file watch, webhooks) trigger agent execution via the
> declarative trigger runtime (E31).

---

## 1. Overview

Event sources convert external signals into internal events that the dispatch
loop routes to agent templates via subscription matching. The declarative trigger
runtime (E31 8/8) provides seven source types with root-Cell payload Signals,
Space/capability enforcement, shared Pulse delivery, durable history, IANA/DST
cron, and CA-verified mTLS.

Event sources operate at Layer 0 (Runtime) -- they produce events that flow
upward through the architecture.

---

## 2. Seven Source Types

| Source | Implementation | Configuration |
|---|---|---|
| **Cron** | `tokio-cron-scheduler`, IANA/DST aware | `schedule = "0 9 * * MON-FRI"` |
| **File watch** | `notify::RecommendedWatcher` + debouncer | `watch_path`, `watch_glob` |
| **GitHub webhook** | HTTP POST to `roko-serve` | `pattern = "webhook.github.*"` |
| **Slack event** | Socket Mode or HTTP webhook | `pattern = "webhook.slack.*"` |
| **Generic webhook** | HTTP POST with custom payload | `pattern = "webhook.custom"` |
| **Chain event** | Watcher-to-raw-EVM ABI/finality/reorg | Feature-gated |
| **Trigger binding** | Declarative Cell-composed triggers | `roko trigger create` |

---

## 3. Subscription Configuration

Subscriptions are declared in `roko.toml` or `.roko/subscriptions.toml`:

```toml
[[subscription]]
pattern = "webhook.github.push"
agent_template = "auto-plan-agent"
filter = { ref = "refs/heads/main" }
path_filter = ".roko/prd/**"
max_concurrent = 1
cooldown_secs = 300
enabled = true
```

### 3.1 Filter Matching

Filters match against event body fields with AND logic. All conditions must
match for the subscription to trigger:

- `filter = { ref = "refs/heads/main" }` -- exact string match
- `filter = { action = ["opened", "synchronize"] }` -- array (any match)

### 3.2 Path Filter

Glob patterns applied to changed file paths. Only events affecting matching
files trigger the subscription.

### 3.3 Concurrency and Cooldown

- `max_concurrent` -- maximum simultaneous agent instances from this subscription
- `cooldown_secs` -- minimum seconds between triggers

---

## 4. Dispatch Loop

The dispatch loop connects event sources to agent templates. It is
source-agnostic -- all events flow through the same routing logic regardless
of origin:

1. Event arrives from any source
2. Match `event.kind` against subscription `pattern`
3. Apply `filter` conditions (AND logic)
4. Apply `path_filter` (glob match against changed files)
5. Check concurrency limits
6. Check cooldown timer
7. Spawn agent with matched template

---

## 5. Cron Scheduling

Cron expressions follow standard POSIX format with IANA timezone support:

```toml
[[subscription]]
pattern = "scheduler.cron"
agent_template = "pm-health-agent"
schedule = "0 9 * * MON-FRI"
```

The scheduler is DST-aware: transitions are handled correctly without skipping
or duplicating ticks.

---

## 6. File System Watching

File watchers use `notify::RecommendedWatcher` with debouncing to collapse
rapid changes:

```toml
[[subscription]]
pattern = "watcher.fs_change"
agent_template = "doc-lifecycle-agent"
watch_path = "/path/to/docs"
watch_glob = "**/*.md"
cooldown_secs = 300
```

The TUI also uses `notify` for live filesystem watching in
`crates/roko-cli/src/tui/fs_watch.rs`.

---

## 7. GitHub Webhook Events

| Event Kind | GitHub Event | Typical Agent |
|---|---|---|
| `webhook.github.push` | Push to branch | auto-plan, doc-lifecycle |
| `webhook.github.pull_request` | PR opened/updated/closed | pr-review, triage |
| `webhook.github.pull_request_review` | Review submitted | review-response |
| `webhook.github.issues` | Issue opened/closed/labeled | triage |

---

## 8. Plugin Event Sources

The `roko-plugin` manifest supports trigger declarations:

```toml
[[triggers]]
kind = "cron"
expression = "0 */5 * * * *"
description = "Run every 5 minutes"

[[triggers]]
kind = "file_watch"
paths = ["src/", "tests/"]
include = ["*.rs"]
```

Third-party plugins can register custom event sources via the trigger protocol
in `crates/roko-plugin/src/trigger_protocol.rs`.

---

## 9. CLI Commands

```bash
roko trigger list          # List all trigger bindings
roko trigger show <id>     # Show trigger details
roko trigger create        # Create a trigger binding
roko trigger fire <id>     # Manually fire a trigger
roko config events         # List configured event sources
roko config subscriptions list/add/remove  # Manage subscriptions
```

---

## 10. Source Locations

| Component | Path |
|---|---|
| TUI file watcher | `crates/roko-cli/src/tui/fs_watch.rs` |
| TUI git watcher | `crates/roko-cli/src/tui/git_watch.rs` |
| Trigger protocol | `crates/roko-plugin/src/trigger_protocol.rs` |
| Serve runtime | `crates/roko-cli/src/serve_runtime.rs` |
| Graph trigger Cells | `crates/roko-graph/src/` |

---

*Derived from: v1/18-tools/15-event-sources.md. Chain block event source added
per E31 completion. Declarative trigger runtime details from E31 8/8 manifest.*
