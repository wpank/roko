+++
id = "gap-21b451"
kind = "gap"
title = "Notification Batching Orchestrator"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/417-notification-batching-orchestrator.md#417 — Notification Batching Orchestrator"
discovered_from = "audit:tmp/backlog/archive/417-notification-batching-orchestrator.md#417 — Notification Batching Orchestrator"
anchors = ["crates/roko-core/src/dashboard_snapshot.rs", "crates/roko-runtime/src/state_hub.rs", "crates/roko-runtime/src/event_bus.rs", "crates/roko-core/src/pulse.rs", "crates/roko-core/src/notification.rs", "crates/roko-runtime/src/notification_batcher.rs", "roko.toml", "crates/roko-core/src/config/schema.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
platform notifications fire individually per event; no aggregation, no per-user preferences, no platform-specific formatting. When a plan completes, a gate fails, or an agent stalls, roko should notify the operator. With platforms wired (#224) and ChatBridge live (#414), the raw delivery path…

Imported without verification from:
- `tmp/backlog/archive/417-notification-batching-orchestrator.md#417 — Notification Batching Orchestrator`

Some cited files are gone: `crates/roko-core/src/notification.rs`.

How to verify: Check: `NotificationUrgency` enum: Immediate, Batched, Digest; `UrgencyClassifier` maps `DashboardEvent` variants to urgency levels; `NotificationBatcher` accumulates Batched items within time windows [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4]
