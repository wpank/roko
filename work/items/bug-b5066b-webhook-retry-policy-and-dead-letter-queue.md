+++
id = "bug-b5066b"
kind = "bug"
title = "Webhook Retry Policy and Dead-Letter Queue"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/418-webhook-retry-dead-letter-queue.md#418 — Webhook Retry Policy and Dead-Letter Queue"
discovered_from = "audit:tmp/backlog/archive/418-webhook-retry-dead-letter-queue.md#418 — Webhook Retry Policy and Dead-Letter Queue"
anchors = ["crates/roko-core/src/trigger.rs", "crates/roko-serve/src/trigger_runtime.rs", "crates/roko-cli/src/commands/trigger.rs", "trigger_runtime.rs", ".roko/triggers/events/", ".roko/triggers/dlq/", ".roko/triggers/dlq/{id}.json", "crates/roko-serve/src/routes/triggers.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
webhook-triggered flows silently disappear on failure. The trigger runtime (`TriggerCoordinator` in `trigger_runtime.rs`) dispatches flows through `apply_concurrency()` and tracks them via `RunningFlow` handles in `ActiveBinding`. When a flow completes, `Command::FlowDone` carries a `FlowOutcome`…

Imported without verification from:
- `tmp/backlog/archive/418-webhook-retry-dead-letter-queue.md#418 — Webhook Retry Policy and Dead-Letter Queue`

Some cited files are gone: `.roko/triggers/dlq/`, `.roko/triggers/dlq/{id}.json`, `.roko/triggers/events/`.

How to verify: Check: `TriggerBinding` accepts an optional `retry_policy` in TOML/JSON config.; Failed flows are retried up to `max_retries` times with exponential backoff.; Flows that exhaust retries are persisted to `.roko/triggers/dlq/`. [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4]
