+++
id = "gap-a11465"
kind = "gap"
title = "[provider F084] cost_usd_without_cache always equals cost_usd in event subscriber and feedback service"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-learn/event_subscriber"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F084"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F084"
anchors = ["crates/roko-learn/src/event_subscriber.rs", "crates/roko-learn/src/feedback_service.rs", "cost_usd_without_cache", "cost_usd"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`event_subscriber.rs` and `feedback_service.rs` both set `cost_usd_without_cache = total_cost_usd`. Cache savings are always recorded as zero for the primary plan execution path.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F084`
- `tmp/archive/provider-audit/19-caching.md`

How to verify: Confirm in crates/roko-learn/src/event_subscriber.rs, crates/roko-learn/src/feedback_service.rs whether still true: `cost_usd_without_cache` always equals `cost_usd` in event subscriber and feedback service
