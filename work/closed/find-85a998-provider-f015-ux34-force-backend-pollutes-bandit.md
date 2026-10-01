+++
id = "find-85a998"
kind = "finding"
title = "[provider F015] UX34: force_backend pollutes bandit signal with full weight"
status = "superseded"
triage = "verified"
severity = "p1"
subsystem = ["roko-cli/runner"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F015"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F015"
anchors = ["crates/roko-cli/src/runtime_feedback/routing.rs:88"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "bug-57ae52" }

[closed]
at = 2026-09-28
evidence = "duplicate of bug-57ae52 (UX34 override-learning isolation, backlog #90), itself done at HEAD 91b4745f8: runtime_feedback/routing.rs routes ModelChoiceSource::Override through the dampened record_override_outcome instead of the full-weight update."
+++
When `force_backend` overrides the cascade router's selection, the resulting observation is recorded as if the override was a natural routing outcome. The LinUCB matrices receive a reward signal for a model selection that was forced, not learned. This biases the bandit toward whatever models oper...

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F015`
- `tmp/archive/provider-audit/08-cascade-router.md`

A source claims this was fixed; confirm against current code before closing.

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`.

How to verify: Roadmap P4-9 claims force_backend labeling implemented (commits b482064e5/c3b8263de), but CLAUDE.md item 17 still lists UX34 override learning as open; check cascade learning of forced dispatches on the Graph path. Confirm in crates/roko-cli/src/runner/event_loop.rs, crates/roko-learn/src/cascade_router.rs whether still true: UX34: `force_backend` pollutes bandit signal with full weight

Verified 2026-09-28: closed as duplicate; see [closed].evidence.
