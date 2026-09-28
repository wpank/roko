+++
id = "gap-ebd656"
kind = "gap"
title = "Conductor Supervisor Loop (Live Intervention)"
status = "open"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/runner"]
created = 2026-09-07
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/178-conductor-supervisor-loop.md#178 — Conductor Supervisor Loop (Live Intervention)"
discovered_from = "audit:tmp/backlog/archive/178-conductor-supervisor-loop.md#178 — Conductor Supervisor Loop (Live Intervention)"
anchors = ["crates/roko-cli/src/runner/conductor_adapter.rs", "crates/roko-cli/src/graph_execution/feedback.rs::ConductorSink", "crates/roko-conductor/src/conductor.rs"]
links = { depends_on = ["spec-a0403b"], blocks = [], related = ["gap-fab31c"], supersedes = [], duplicate_of = "" }
+++
without a live intervention loop, stuck agents burn tokens indefinitely; the conductor ring buffer and 12 watchers exist but nothing reads or acts on their signals during plan execution. Mori's conductor ran a live supervision loop: every 2 seconds, the conductor evaluated signal data from…

Imported without verification from:
- `tmp/backlog/archive/178-conductor-supervisor-loop.md#178 — Conductor Supervisor Loop (Live Intervention)`
- `tmp/backlog/_archive/_mori-old-gaps.md#MO-08`
- `tmp/archive/CONSOLIDATED-BACKLOG-2026-09-23.md#UXP-10 (UX/TUI Parity: Partial Items (13 items f)`
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#4.9 Parity items requiring runner/infras PX.5`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`.

How to verify: Check: Conductor supervision tick reads signals from the ring buffer every interval and produces typed interventions beyond Restart/Fail.; A stalled agent (no output for `silence_timeout_secs`) triggers a Nudge or Restart depending on severity.; A… [evidence: CONSOLIDATED UXP-10: Tick+thresholds wired; actions only log; 00-STATUS-SUMMARY 3. Open / P1 -- High (Open): M | 3 |]

Verified 2026-09-28: still true, worse than recorded - the Runner-v2 supervision tick (whose actions only logged, gap-fab31c) was deleted and the Graph engine has no conductor tick at all (see spec-a0403b); the only Graph-side conductor hook is the post-settlement ConductorSink (graph_execution/feedback.rs:602).
