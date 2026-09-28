+++
id = "find-1f8640"
kind = "finding"
title = "[status-quo P1-C] Provider health degradation and missing API keys"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-learn/provider_health"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/archive/status-quo-audit-2026-09-21/06-remaining-work.md#6. Provider health degradation"
discovered_from = "audit:tmp/archive/status-quo-audit-2026-09-21/06-remaining-work.md#6. Provider health degradation"
anchors = ["roko-learn/src/provider_health.rs", "roko config providers health"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
providers health showed openai 87.8% errors, zai 89.1% HALF-OPEN, anthropic 100%; 3 of 7 providers missing keys. Tracker treats it as operator action (code wired), but openai/zai error patterns were never investigated.

Imported without verification from:
- `tmp/archive/status-quo-audit-2026-09-21/06-remaining-work.md#6. Provider health degradation`
- `tmp/archive/status-quo-audit-2026-09-21/09-completion-tracking.md#P1 items: 5/7 done`

How to verify: Run `roko config providers health`; inspect error classes for openai/zai.
