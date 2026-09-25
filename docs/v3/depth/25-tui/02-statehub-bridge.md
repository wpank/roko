# 25.02 -- StateHub Bridge

> Depth file for [25-TUI.md](../../25-TUI.md) section 3.

---

## Push-Based Architecture

The TUI never polls for data. It consumes a push-based event stream:

```
Runtime (runner / serve / acp)
    |
    v
DashboardEvent (typed enum variants)
    |
    v
tokio::sync::watch::Sender<DashboardState>
    |
    v
TUI render loop (watch::Receiver)
    |
    v
Immediate-mode ratatui::Frame draw
```

## DashboardEvent Variants

The `DashboardEvent` enum covers all state changes the TUI reflects:

- Plan lifecycle: started, completed, failed, paused, resumed
- Task progress: completion percentage, gate results, cost deltas
- Agent lifecycle: spawned, active, idle, stopped
- Provider health: circuit open/close, latency spikes, error rates
- Cost accounting: per-model, per-task, session totals
- Knowledge events: tier promotions, new entries, garbage collection
- Git state: branch changes, commits, worktree modifications
- File system: config reload, plan discovery, state file updates

## TuiBridge

The `TuiBridge` provides convenience methods for publishing events from any async
context. It wraps the `watch::Sender` and is `Clone + Send + Sync`. Publishers never
block -- if the TUI is not consuming, updates are coalesced.

## File System Watcher

`fs_watch.rs` uses `notify::RecommendedWatcher` with debouncing to monitor:

- `.roko/` directory for state changes
- `roko.toml` for configuration changes
- Plan directories for task file updates

File events are converted to `DashboardEvent` variants that trigger re-reads.

## Git Watcher

`git_watch.rs` monitors the git index and HEAD reference for changes, feeding
branch state, commit history, and worktree status into the TUI state model. It
runs as a background task and publishes updates through the same
`watch::Sender` channel.
