+++
id = "gap-27ea74"
kind = "gap"
title = "Superseded: Decompose event_loop.rs into Focused Modules"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/20-event-loop-decomposition.md#20 — Superseded: Decompose event_loop.rs into Focused Modules"
discovered_from = "audit:tmp/backlog/archive/20-event-loop-decomposition.md#20 — Superseded: Decompose event_loop.rs into Focused Modules"
anchors = ["crates/roko-cli/", "event_loop.rs", "agent_events.rs", "agent_stream.rs", "attempt_ownership.rs", "branch_cleanup.rs", "conductor_adapter.rs", "deadlines.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
maintainability; not blocking correctness. The runner module has 27 source files and most concerns have at least partial representation in dedicated files: `gate_dispatch.rs` owns gate invocation, `persist.rs` owns disk writes, `merge.rs` owns the merge queue, `state.rs` owns the in-memory…

Imported without verification from:
- `tmp/backlog/archive/20-event-loop-decomposition.md#20 — Superseded: Decompose event_loop.rs into Focused Modules`
- `tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#P1-RUN-1 (Subsystem: Runner)`

How to verify: Check: After all 6 targets, `event_loop.rs` is under 5,000 lines. It retains `pub async fn run(...)`; `cargo test -p roko-cli` passes with zero failures after each individual extraction commit.; The public API of the `runner` module is unchanged… [evidence: own status: Superseded by #246-#261; perform only the file moves required by those scoped items; CONSOLIDATED P1-RUN-1: open; 00-STATUS-SUMMARY 1. Impleme / Superseded…]
