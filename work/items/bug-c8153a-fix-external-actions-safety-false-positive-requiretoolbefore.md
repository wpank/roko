+++
id = "bug-c8153a"
kind = "bug"
title = "Fix external_actions Safety False Positive (RequireToolBeforeEdit)"
status = "open"
triage = "verified"
severity = "p1"
goal = "core"
subsystem = ["roko-agent/safety"]
created = 2026-09-21
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/392-external-actions-tracking-safety-false-positive.md#392 — Fix external_actions Safety False Positive (RequireToolBeforeEdit)"
discovered_from = "audit:tmp/backlog/archive/392-external-actions-tracking-safety-false-positive.md#392 — Fix external_actions Safety False Positive (RequireToolBeforeEdit)"
anchors = ["crates/roko-agent/src/safety/contract.rs::has_prior_tool", "crates/roko-agent/src/safety/contract.rs:755", "crates/roko-core/src/tool/handler.rs::ToolContext::record_external_action", "crates/roko-agent/src/safety/contracts/implementer.yaml:10"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
safety rule always fires false positive. Dev-audit found that the RequireToolBeforeEdit safety rule always fires a false positive because the read_file handler never records actions in the external_actions tracker. The rule checks whether the agent read a file before editing it, but file reads are…

Imported without verification from:
- `tmp/backlog/archive/392-external-actions-tracking-safety-false-positive.md#392 — Fix external_actions Safety False Positive (RequireToolBeforeEdit)`

How to verify: Check whether the gap described in tmp/backlog/archive/392-external-actions-tracking-safety-false-positive.md still exists at the anchored paths. [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Audit Sweep Items (#376-#395)]

Verified 2026-09-28: still true. `RequireToolBeforeEdit` (crates/roko-agent/src/safety/contract.rs:755-781) rejects edits to existing files unless `has_prior_tool` (:972-978) finds the required tool in `ctx.external_actions`, but no production code records actions there. `ToolContext::record_external_action` (crates/roko-core/src/tool/handler.rs:538) is called only from its own tests (:683), and nothing in the dispatcher or tool loop writes `external_actions`. The rule is enabled for implementer, auto-fixer and architect (e.g. safety/contracts/implementer.yaml:10), so edits made through roko's own `ToolDispatcher` (API-provider tool loops) are denied even after `read_file`. contract.rs has uncommitted changes from a concurrent session, but they do not touch this rule.
