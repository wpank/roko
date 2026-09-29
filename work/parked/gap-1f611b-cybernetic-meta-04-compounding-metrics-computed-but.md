+++
id = "gap-1f611b"
kind = "gap"
title = "[cybernetic META-04] Compounding metrics computed but never used to adjust learning strategy"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-learn"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/cybernetic-audit/28-feedback-loops.md#META-04: Compounding Metrics (Autocatalytic Self-Assessment)"
discovered_from = "audit:tmp/archive/cybernetic-audit/28-feedback-loops.md#META-04: Compounding Metrics (Autocatalytic Self-Assessment)"
anchors = ["LearningRuntime::record_completed_run", "roko-core/src/cfactor.rs", ".roko/learn/compounding.jsonl"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
compounding.jsonl, CFactorSummary, Variance Inequality and autocatalytic metrics are live but advisory; nothing adjusts learning strategy when learning stops accelerating.

Imported without verification from:
- `tmp/archive/cybernetic-audit/28-feedback-loops.md#META-04: Compounding Metrics (Autocatalytic Self-Assessment)`

How to verify: Find consumers of compounding metrics that change router/threshold/playbook parameters.
