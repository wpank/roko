+++
id = "gap-645469"
kind = "gap"
title = "[provider F101] EMA alpha=0.1 hardcoded — too slow for volatile providers"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-learn/provider_health"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F101"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F101"
anchors = ["crates/roko-learn/src/provider_health.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The exponential moving average for provider latency uses a hardcoded `alpha=0.1`, meaning it takes approximately 22 observations to reflect a step change in latency at 63% accuracy. For volatile providers with burst latency, this response is too slow.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F101`
- `tmp/archive/provider-audit/04-health-efficiency.md`

How to verify: Confirm in crates/roko-learn/src/provider_health.rs whether still true: EMA alpha=0.1 hardcoded — too slow for volatile providers
