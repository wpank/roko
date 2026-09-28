+++
id = "bug-15b4f9"
kind = "bug"
title = "[provider F053] ProviderHealthRegistry persistence worker swallows save errors silently"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-learn/provider_health"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F053"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F053"
anchors = ["crates/roko-learn/src/provider_health.rs", "ProviderHealthRegistry"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The background worker that persists `provider-health.json` swallows `io::Error` without logging or alerting. A disk full or permission error causes health state to stop persisting without any indication.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F053`
- `tmp/archive/provider-audit/04-health-efficiency.md`

How to verify: Confirm in crates/roko-learn/src/provider_health.rs whether still true: `ProviderHealthRegistry` persistence worker swallows save errors silently
