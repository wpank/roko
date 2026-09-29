+++
id = "gap-2af461"
kind = "gap"
title = "Webhook Dashboard Metrics"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-serve"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/432-webhook-dashboard-metrics.md#432 — Webhook Dashboard Metrics"
discovered_from = "audit:tmp/backlog/archive/432-webhook-dashboard-metrics.md#432 — Webhook Dashboard Metrics"
anchors = ["trigger_runtime.rs", "state_hub.rs", "crates/roko-serve/src/trigger_runtime.rs", "crates/roko-serve/src/routes/triggers.rs", "crates/roko-serve/src/routes/webhooks.rs", "crates/roko-runtime/src/state_hub.rs", "crates/roko-serve/src/state.rs", "crates/roko-serve/src/webhook_metrics.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
operators have no visibility into webhook delivery health; failures are silent unless the downstream effect (missed trigger, stale data) is noticed manually. The trigger runtime in `crates/roko-serve/src/trigger_runtime.rs` coordinates long-lived trigger bindings: cron, file watchers, webhook…

Imported without verification from:
- `tmp/backlog/archive/432-webhook-dashboard-metrics.md#432 — Webhook Dashboard Metrics`

Some cited files are gone: `crates/roko-serve/src/webhook_metrics.rs`.

How to verify: Check: `WebhookMetricsCollector` tracks received/processed/failed/rejected per binding; Latency percentiles (p50, p95, p99) computed from bounded circular buffer; Time-series buckets at 1min/5min/1hr with automatic rotation and retention [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4]
