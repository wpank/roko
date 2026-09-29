+++
id = "find-e161e4"
kind = "finding"
title = "[provider F182] Version-locked kimi-k2 heuristics conflict with family-level kimi- checks; future Kimi versions degrade silently"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-learn/cascade"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F182"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F182"
anchors = ["crates/roko-learn/src/cascade/helpers.rs:355", "crates/roko-learn/src/cascade/helpers.rs:486", "crates/roko-agent/src/token_estimator.rs:246", "crates/roko-agent/src/provider/openai_compat.rs:109", "crates/roko-compose/src/token_counter.rs:54", "kimi-k2"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Kimi support is split between family-level matchers — `slug.starts_with("kimi-")` for thinking-parameter injection (openai_compat.rs:109) and HF tokenizer selection (token_counter.rs:54) — and version-locked `kimi-k2` matchers in the cascade thinking filter (helpers.rs:355), cascade `slug_family`...

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F182`
- `tmp/archive/provider-audit/08-cascade-router.md`
- `tmp/archive/provider-audit/09-model-registry.md`
- `tmp/archive/provider-audit/18-thinking-reasoning.md`

How to verify: SH-2 widened kimi-k2 arms (done); SH-1 central ModelFamily classifier deferred. Check whether a single classifier exists. Confirm in crates/roko-learn/src/cascade/helpers.rs:355, crates/roko-learn/src/cascade/helpers.rs:486, crates/roko-agent/src/token_estimator.rs:246 whether still true: Version-locked `kimi-k2` heuristics conflict with family-level `kimi-` checks; future Kimi versions degrade si
