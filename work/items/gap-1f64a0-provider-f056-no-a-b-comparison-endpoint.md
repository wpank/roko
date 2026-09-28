+++
id = "gap-1f64a0"
kind = "gap"
title = "[provider F056] No A/B comparison endpoint for same task across models"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-learn"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F056"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F056"
anchors = ["crates/roko-learn/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
There is no mechanism to dispatch identical tasks to two different models and compare outcomes. The learning system records outcomes but cannot generate side-by-side model comparisons for the same task.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F056`
- `tmp/archive/provider-audit/04-health-efficiency.md`

How to verify: Roadmap P2-8 added GET /api/learn/model-scorecard, but a same-task cross-model A/B comparison endpoint/CLI (P4-6, deferred) may still be missing. Confirm in crates/roko-learn/ whether still true: No A/B comparison endpoint for same task across models
