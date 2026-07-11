# Dashboard refresh runs synchronous full Git diffs

- Severity: high
- Status: strongly indicated by code and process sampling
- Area: TUI responsiveness

## Observation

Every live process sample caught a transient direct `(git)` child under Roko, while the parent consumed substantial CPU. `DashboardData::refresh` calls `load_dashboard_git_diff` (`crates/roko-cli/src/tui/dashboard.rs:696`), which synchronously executes `git diff --cached` and `git diff HEAD` (`dashboard.rs:941-965`). The async approval loop polls every 250ms (`tui/app.rs:503-520`).

On this large, dirty repository, repeatedly materializing complete diffs can block producer or UI work and competes with several Cargo/Codex children.

Mori keeps blocking sysinfo and file work off the event loop and refreshes domain state directly from events (`apps/mori/src/app/parallel.rs:9261-9268`, `9557-9594`).

## Expected

Git diff computation should be watcher-triggered, debounced, performed off the UI/event thread, cached, and bounded. The main dashboard should use cheap status counters until the Diff tab is opened.

