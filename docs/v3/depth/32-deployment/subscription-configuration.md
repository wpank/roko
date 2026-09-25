# 32-deployment/10 -- Subscription Configuration

> Daemon subscription format: cron, file watch, and webhook triggers,
> per-repo overrides, debouncing, and subscription lifecycle.

**Parent:** [32-DEPLOYMENT](../../32-DEPLOYMENT.md)

**Source:** `crates/roko-cli/src/commands/daemon.rs`,
`crates/roko-core/src/config/schema.rs`

---

## 1. Overview

A subscription is a binding between a repository and a trigger. When the
trigger fires, the daemon executes plans in that repository. Subscriptions
are defined in the global `~/.config/roko/config.toml` and can be
overridden by the per-repo `.roko/config.toml`.

| Trigger | When it fires | Use case |
|---------|--------------|----------|
| **Cron** | On a time schedule | Periodic builds, nightly consolidation |
| **Watch** | When files change (fsnotify) | Reactive to PRD edits |
| **Webhook** | HTTP POST arrives | GitHub push events, CI triggers |

Multiple triggers can be combined for a single repository.

---

## 2. Configuration Format

### Global Config

```toml
# ~/.config/roko/config.toml

[daemon]
socket = "/tmp/roko-daemon.sock"
log_level = "info"
max_concurrent_runs = 4

[daemon.defaults]
model = "claude-sonnet-4-6"
max_agents = 4

[[subscriptions]]
repo = "/Users/will/dev/project-a"

[subscriptions.cron]
schedule = "*/30 * * * *"
plan_dirs = ["plans/"]
changed_paths = [".roko/prd/**/*.md", "plans/**/*.toml"]

[[subscriptions]]
repo = "/Users/will/dev/project-b"

[subscriptions.watch]
paths = [".roko/prd/"]
debounce_ms = 5000

plan_dirs = ["plans/"]

[[subscriptions]]
repo = "/Users/will/dev/project-c"

[subscriptions.webhook]
path = "/hook/project-c"
secret = "${ROKO_WEBHOOK_SECRET_C}"

plan_dirs = ["plans/"]
```

### Per-Repo Override

Each repository can override its subscription settings:

```toml
# .roko/config.toml

[agent]
model = "claude-opus-4-6"
max_agents = 8

[subscription.cron]
schedule = "*/15 * * * *"

[subscription.gates]
compile = true
test = true
clippy = true
```

### Config Merge Order

```
1. Daemon defaults       ([daemon.defaults])
2. Global subscription   ([[subscriptions]] entry)
3. Per-repo config       (.roko/config.toml)
4. Environment vars      (ROKO_* prefix)
```

---

## 3. Cron Trigger

Standard 5-field cron expressions:

```toml
schedule = "*/30 * * * *"    # Every 30 minutes
schedule = "0 2 * * *"       # Nightly at 2 AM
schedule = "0 9 * * 1-5"     # Weekdays at 9 AM
```

The daemon uses the `cron` crate to parse expressions and calculate
next-fire times. The scheduler sleeps in a Tokio task until the next
scheduled time.

### Changed-Path Filtering

When `changed_paths` is specified, the cron trigger only executes if
matching files changed since the last successful run:

```toml
[subscriptions.cron]
schedule = "*/30 * * * *"
changed_paths = [".roko/prd/**/*.md"]
```

The daemon tracks the last successful run timestamp per subscription in
`~/.local/state/roko/subscriptions.json`. On each cron tick, it checks
file modification times against the last run timestamp.

---

## 4. File Watch Trigger

Uses the `notify` crate (workspace dependency) for real-time filesystem
event monitoring:

```toml
[subscriptions.watch]
paths = [".roko/prd/", "src/"]
debounce_ms = 5000
recursive = true
ignore = ["*.swp", "*~", ".git/"]
```

### Debouncing

File events arrive in bursts. The debounce timer waits for the burst to
settle:

```
Event 1 (file saved)   -> Start 5s timer
Event 2 (100ms later)  -> Reset timer
Event 3 (200ms later)  -> Reset timer
... (no more events)
Timer expires (5s)      -> Trigger plan run
```

Implemented with a Tokio delay that resets on each new event.

---

## 5. Webhook Trigger

An embedded Axum HTTP server (default port 9090) routes incoming POST
requests to the appropriate subscription:

```toml
[subscriptions.webhook]
path = "/hook/project-c"
secret = "${ROKO_WEBHOOK_SECRET_C}"
```

### HMAC Verification

If a secret is configured, the daemon verifies the `x-hub-signature-256`
header using HMAC-SHA256.

### GitHub Integration

The webhook format is compatible with GitHub payloads:

1. In repo settings, add webhook URL:
   `https://your-host:9090/hook/project-c`
2. Set content type to `application/json`
3. Set secret to match `ROKO_WEBHOOK_SECRET_C`
4. Select events: Push, Pull Request

For local development, use a tunnel:

```bash
cloudflared tunnel --url http://localhost:9090
```

---

## 6. Environment Variable Interpolation

Config values support `${VAR}` syntax:

```toml
[subscriptions.webhook]
secret = "${ROKO_WEBHOOK_SECRET}"

[[subscriptions]]
repo = "${HOME}/dev/project-a"
```

Rules:
- `${VAR}` resolves from the environment
- `${VAR:-default}` falls back to a default
- `$$` escapes a literal dollar sign
- Unresolved variables log a warning and remain literal

---

## 7. Subscription Lifecycle

### Adding

```bash
# Via config file
roko config edit

# Via CLI
roko daemon subscribe --repo ~/dev/project-a --cron "*/30 * * * *"

# Via IPC (takes effect immediately)
roko daemon send subscribe --repo ~/dev/project-a --cron "*/30 * * * *"
```

### Removing

```bash
roko daemon unsubscribe --repo ~/dev/project-a
```

### Listing

```bash
$ roko daemon send list-subscriptions

Subscriptions:
  /Users/will/dev/project-a
    Triggers: cron (*/30 * * * *)
    Last run: 2h ago (success)
    Next run: in 12 minutes

  /Users/will/dev/project-b
    Triggers: watch (.roko/prd/)
    Last run: 15m ago (running)
```

### Pausing and Resuming

```bash
roko daemon send pause                        # All subscriptions
roko daemon send pause --repo ~/dev/project-a # Specific
roko daemon send resume
```

---

## 8. State Persistence

Subscription state persists to
`~/.local/state/roko/subscriptions.json`:

```json
{
  "subscriptions": [
    {
      "repo": "/Users/will/dev/project-a",
      "last_run_at": "2026-04-12T08:30:00Z",
      "last_run_status": "success",
      "last_run_tasks": 3,
      "next_run_at": "2026-04-12T09:00:00Z",
      "paused": false
    }
  ]
}
```

Updated after each run, loaded on daemon startup to resume scheduling.

---

## 9. Implementation Status

> **Implementation status:** The TOML schema is designed. The `notify`
> crate (file watching) and `cron` crate (scheduling) are workspace
> dependencies. The `roko config subscriptions list/add/remove` CLI
> commands exist. Implementation depends on the daemon infrastructure
> being wired (IPC, event loop, subscription runner).
