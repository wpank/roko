+++
id = "gap-7b065c"
kind = "gap"
title = "ProcessSupervisor Fire-and-Forget Spawned Tasks Not Tracked"
status = "done"
triage = "verified"
severity = "p2"
goal = "tooling"
subsystem = ["roko-runtime"]
created = 2026-09-07
updated = 2026-10-08
last_verified = 2026-10-08
last_verified_rev = "ad7a3a337"
source = "tmp/backlog/archive/94-process-supervisor-untracked-tasks.md#94 — ProcessSupervisor Fire-and-Forget Spawned Tasks Not Tracked"
discovered_from = "audit:tmp/backlog/archive/94-process-supervisor-untracked-tasks.md#94 — ProcessSupervisor Fire-and-Forget Spawned Tasks Not Tracked"
anchors = ["crates/roko-runtime/", "src/process.rs", "crates/roko-runtime/src/process.rs", "ProcessSupervisor", "ProcessHandle", "HashMap", "ProcessSupervisor::new", "SpawnConfig::default()"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn cancellation_watchers_are_tracked_pruned_and_stopped' crates/roko-runtime/src/process.rs && cargo test -p roko-runtime --lib cancellation_watchers_are_tracked_pruned_and_stopped"

[closed]
at = 2026-10-08
at_ts = "2026-10-08T13:21:49Z"
commit = "e6304ffa7"
forced = false
evidence = "gate 24 (2026-10-08): its verify passes; check, nightly fmt, clippy -D warnings, CI feature checks, nextest --workspace --lib (15223 passed), touched crates' full tests, roko-acp integration, roko-cli bin (457) and CI canaries all pass"
+++
Reliability: untracked cancellation-watcher tasks hold Arc references that can delay cleanup and cannot be awaited during shutdown. `ProcessSupervisor` manages long-running subprocesses (agents, sidecars) and provides `shutdown_all()` and `Drop` paths to clean them up. When a process is spawned…

Imported without verification from:
- `tmp/backlog/archive/94-process-supervisor-untracked-tasks.md#94 — ProcessSupervisor Fire-and-Forget Spawned Tasks Not Tracked`

How to verify: Check: `ProcessSupervisor` has a `watcher_tasks: Mutex<Vec<JoinHandle<()>>>` field (or equivalent).; Every `tokio::spawn(...)` call for a cancellation watcher stores its `JoinHandle` in `watcher_tasks`.; `shutdown_all()` aborts (or awaits) all… [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): S | 2 |]
