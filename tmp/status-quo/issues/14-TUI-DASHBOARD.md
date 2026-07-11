# TUI Dashboard Issues

## Critical

### Unbounded `output_lines` growth
- `state.rs:2186-2194`: Each snapshot tick extends `output_lines` by cloning previous + extending with new. Ring buffer source is capped at 50, but `AgentRow::output_lines` has no cap. After N ticks: N×50 lines.

### Background thread `expect()` without terminal cleanup
- `app.rs:3363`: `tokio::runtime::Builder::new_current_thread().build().expect("mini-rt...")`. Panic on this thread skips `TerminalCleanupGuard` (main thread only), leaving terminal in raw mode.

## High

### Dual data path race condition
- `app.rs:808`: `drain_snapshot_channel()` called unconditionally even when `replay_disk_snapshots=true`. Push path (StateHub) and pull path (filesystem refresh) can race.

### `async run()` missing critical drains
- `app.rs:498-526`: Standalone approval TUI never drains `sys_rx`, `fs_watch`, `git_watch`. Notifications never expire. Shutdown signals not processed.

### Git data refresh blocks the event loop
- `app.rs:2963`: `collect_git_bg_data()` runs multiple blocking `git` subprocesses synchronously on the main loop thread.

## Medium

### `task_output_tails` HashMap grows without cleanup
- `state.rs:2390-2392`: Entries only inserted, never removed in push path.

### Notifications unbounded in `async run()` path
- `app.rs:1230-3469`: `expire_notifications()` only called on `Event::Tick` path.

### Terminal cleanup — background thread panics skip hook
- `app.rs:731-734`: Panic hook only covers main thread.

### Recursive `.git/` polling every 500ms in fallback
- `git_watch.rs:345-380`: `fingerprint_roots()` with `recursive=true` on `.git_dir` in poll fallback.

## Low

### `cycle_field_value` potential division by zero
- `app.rs:3295-3299`: `% opts.len()` with no guard on empty `opts`.

### Signal handler writes to redirected stdout
- `app.rs:451-460`: If stdout is redirected, escape codes corrupt log file.
