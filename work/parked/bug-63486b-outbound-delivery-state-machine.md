+++
id = "bug-63486b"
kind = "bug"
title = "Outbound Delivery State Machine"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-runtime"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/415-outbound-delivery-state-machine.md#415 — Outbound Delivery State Machine"
discovered_from = "audit:tmp/backlog/archive/415-outbound-delivery-state-machine.md#415 — Outbound Delivery State Machine"
anchors = ["crates/roko-runtime/src/event_bus.rs", "crates/roko-runtime/src/state_hub.rs", "crates/roko-core/src/dashboard_snapshot.rs", "crates/roko-core/src/pulse.rs", "crates/roko-core/src/delivery.rs", "roko.toml", "crates/roko-runtime/src/delivery_manager.rs", ".roko/channels/dlq.jsonl"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
messages sent through ChatBridge have no delivery tracking; failures are silently dropped. When an agent sends a message through a `ChatBridge`, the current code is fire-and-forget: if `send()` returns `Ok`, the message is assumed delivered. If it returns `Err`, the caller logs and moves on. There…

Imported without verification from:
- `tmp/backlog/archive/415-outbound-delivery-state-machine.md#415 — Outbound Delivery State Machine`

Some cited files are gone: `.roko/channels/dlq.jsonl`, `crates/roko-core/src/delivery.rs`, `crates/roko-runtime/src/delivery_manager.rs`.

How to verify: Check: `DeliveryState` enum with six states and enforced transition ordering; `DeliveryRecord` tracks attempts, errors, receipts, and timestamps; `RetryPolicy` configurable per platform in `roko.toml` [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4]
