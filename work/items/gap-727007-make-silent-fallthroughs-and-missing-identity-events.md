+++
id = "gap-727007"
kind = "gap"
title = "Make Silent Fallthroughs and Missing-Identity Events Observable"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/backlog/archive/343-silent-fallthrough-and-missing-event-observability.md#343 — Make Silent Fallthroughs and Missing-Identity Events Observable"
discovered_from = "audit:tmp/backlog/archive/343-silent-fallthrough-and-missing-event-observability.md#343 — Make Silent Fallthroughs and Missing-Identity Events Observable"
anchors = ["crates/roko-cli/src/dispatch_v2.rs", "crates/roko-learn/src/feedback_service.rs", "efficiency.jsonl", "feedback_service.rs", "FeedbackEvent::ModelCall", "DemoEvent", "UnsupportedCliProvider"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
[blocked] Blocked on #208 and #283 — unexpected provider and feedback failures currently disappear without evidence. Two production fallthroughs erase useful evidence:

Imported without verification from:
- `tmp/backlog/archive/343-silent-fallthrough-and-missing-event-observability.md#343 — Make Silent Fallthroughs and Missing-Identity Events Observable`

How to verify: Check: No production `Err(_) => {}` or `ModelCall { .. } => {}` arm remains at the audited sites.; Unsupported CLI provider fallback still succeeds when the HTTP path is valid.; Unexpected CLI detection errors are visible in structured tracing (and… [evidence: own status: Blocked on #208 and #283]
