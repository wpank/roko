+++
id = "bug-a65fd2"
kind = "bug"
title = "[provider F080] Two divergent supports_thinking() predicates with non-overlapping coverage"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-learn/cascade"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F080"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F080"
anchors = ["crates/roko-learn/src/cascade/helpers.rs", "crates/roko-agent/src/model_call_service.rs", "supports_thinking()"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`cascade/helpers.rs` has a thinking-support heuristic that covers Gemini, Kimi, GLM, o-series but not Claude. `model_call_service.rs` has a heuristic that covers Claude Opus, o1/o3/o4, deepseek-r1 but not Gemini, Kimi, or GLM. Neither is complete.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F080`
- `tmp/archive/provider-audit/18-thinking-reasoning.md`

How to verify: Confirm in crates/roko-learn/src/cascade/helpers.rs, crates/roko-agent/src/model_call_service.rs whether still true: Two divergent `supports_thinking()` predicates with non-overlapping coverage
