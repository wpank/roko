+++
id = "gap-9c804c"
kind = "gap"
title = "[provider F066] Cache affinity feature (dimension 17) always 0.0"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-learn/cascade_router"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F066"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F066"
anchors = ["crates/roko-learn/src/cascade_router.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The LinUCB context vector dimension 17 represents `cache_affinity` — whether the task benefits from using the same model as the previous task (for prompt caching). In runner-v2 dispatch, `previous_model` is always `None`, so this dimension is always 0.0.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F066`
- `tmp/archive/provider-audit/08-cascade-router.md`

How to verify: Confirm in crates/roko-learn/src/cascade_router.rs whether still true: Cache affinity feature (dimension 17) always 0.0
