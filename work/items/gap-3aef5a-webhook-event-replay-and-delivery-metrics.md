+++
id = "gap-3aef5a"
kind = "gap"
title = "Webhook Event Replay and Delivery Metrics"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-serve/routes"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/420-webhook-replay-and-event-log.md#420 — Webhook Event Replay and Delivery Metrics"
discovered_from = "audit:tmp/backlog/archive/420-webhook-replay-and-event-log.md#420 — Webhook Event Replay and Delivery Metrics"
anchors = ["crates/roko-serve/src/routes/triggers.rs", "crates/roko-serve/src/trigger_runtime.rs", "crates/roko-cli/src/commands/trigger.rs", "trigger.rs", "trigger_runtime.rs", "commands/trigger.rs", "crates/roko-core/src/trigger.rs", "TriggerLifecycleEvent"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
improves observability; not blocking any critical path. The trigger runtime already persists lifecycle events to `.roko/triggers/events/` via `emit_lifecycle()` in `TriggerCoordinator`. The `load_trigger_history()` function in `crates/roko-core/src/trigger.rs` (line 175) reads these back as…

Imported without verification from:
- `tmp/backlog/archive/420-webhook-replay-and-event-log.md#420 — Webhook Event Replay and Delivery Metrics`

How to verify: Check: `roko trigger replay <name> --since <ISO-8601>` re-fires matching historical events.; `POST /api/triggers/{name}/replay?since=<ts>` replays events via HTTP.; Replayed events respect the binding's concurrency policy and rate limits. [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4]
