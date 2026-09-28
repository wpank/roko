+++
id = "gap-553e12"
kind = "gap"
title = "Superseded: Graph Engine Runner-v2 Parity"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-graph"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/54-graph-engine-runner-parity.md#54 — Superseded: Graph Engine Runner-v2 Parity"
discovered_from = "audit:tmp/backlog/archive/54-graph-engine-runner-parity.md#54 — Superseded: Graph Engine Runner-v2 Parity"
anchors = ["crates/roko-graph/", "crates/roko-cli/", "crates/roko-graph/src/engine.rs", "crates/roko-cli/src/runner/gate_dispatch.rs", "crates/roko-cli/src/runner/event_loop.rs", "event_loop.rs", "crates/roko-cli/src/runner/types.rs", "crates/roko-cli/src/commands/plan.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Graph engine works for provider dispatch but lacks gate, replan, worktree isolation, and merge queue lifecycle, making it unsuitable for production plan execution. Roko has two plan execution engines: Runner-v2 (`--engine runner-v2`, the default) and the Graph Engine (`--engine graph`). Runner-v2…

Imported without verification from:
- `tmp/backlog/archive/54-graph-engine-runner-parity.md#54 — Superseded: Graph Engine Runner-v2 Parity`
- `tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#P2-GE-1 (Subsystem: Graph Engine)`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`.

How to verify: Check: After each Activity completes, `run_gate_once()` is called and the result is recorded in `GraphSnapshot`. A gate failure marks the node `Failed`.; A gate failure with `NeedsReplan` triggers `build_gate_failure_plan_revision()`, and the node… [evidence: own status: Superseded by the dependency-ordered #242-#285 program; its CLI-from-graph dependency direction is unsafe; CONSOLIDATED P2-GE-1: open; 00-STATUS-SUMMARY 1…]
