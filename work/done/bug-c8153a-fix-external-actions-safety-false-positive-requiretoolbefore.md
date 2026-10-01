+++
id = "bug-c8153a"
kind = "bug"
title = "Fix external_actions Safety False Positive (RequireToolBeforeEdit)"
status = "done"
triage = "verified"
severity = "p1"
goal = "core"
subsystem = ["roko-agent/safety"]
created = 2026-09-21
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/backlog/archive/392-external-actions-tracking-safety-false-positive.md#392 — Fix external_actions Safety False Positive (RequireToolBeforeEdit)"
discovered_from = "audit:tmp/backlog/archive/392-external-actions-tracking-safety-false-positive.md#392 — Fix external_actions Safety False Positive (RequireToolBeforeEdit)"
anchors = ["crates/roko-agent/src/safety/contract.rs::has_prior_tool", "crates/roko-agent/src/dispatcher/mod.rs::dispatch_with_result_limit", "crates/roko-agent/src/safety/contract.rs::TOOL_HISTORY_SERVICE"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'TOOL_HISTORY_SERVICE' crates/roko-agent/src/dispatcher/mod.rs && cargo test -p roko-agent --lib safety::contract::tests::tool_history_satisfies_read_before_edit_without_counting_as_actions"

[closed]
at = 2026-09-29
commit = "725f21e05"
by = "work sweep 2026-09-29 (static check against HEAD; cargo verify not re-run while the portal plan run held the build lock)"
evidence = "Since 725f21e05, ToolDispatcher::dispatch_with_result_limit (crates/roko-agent/src/dispatcher/mod.rs:563-574) records a roko.tool_history ExternalAction (action_type and metadata.tool = tool name) after every successful call, and both dispatch and dispatch_batch go through it. has_prior_tool (crates/roko-agent/src/safety/contract.rs:988-994) matches that entry, and ToolLoopAgent builds one shared ToolContext per run (tool_loop/agent_wrapper.rs:341). So RequireToolBeforeEdit passes after a successful read_file in the same run. Unit test tool_history_satisfies_read_before_edit_without_counting_as_actions (contract.rs:1388) covers the rule side."
+++
safety rule always fires false positive. Dev-audit found that the RequireToolBeforeEdit safety rule always fires a false positive because the read_file handler never records actions in the external_actions tracker. The rule checks whether the agent read a file before editing it, but file reads are…

Imported without verification from:
- `tmp/backlog/archive/392-external-actions-tracking-safety-false-positive.md#392 — Fix external_actions Safety False Positive (RequireToolBeforeEdit)`

How to verify: Check whether the gap described in tmp/backlog/archive/392-external-actions-tracking-safety-false-positive.md still exists at the anchored paths. [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Audit Sweep Items (#376-#395)]

Verified 2026-09-28: still true. `RequireToolBeforeEdit` (crates/roko-agent/src/safety/contract.rs:755-781) rejects edits to existing files unless `has_prior_tool` (:972-978) finds the required tool in `ctx.external_actions`, but no production code records actions there. `ToolContext::record_external_action` (crates/roko-core/src/tool/handler.rs:538) is called only from its own tests (:683), and nothing in the dispatcher or tool loop writes `external_actions`. The rule is enabled for implementer, auto-fixer and architect (e.g. safety/contracts/implementer.yaml:10), so edits made through roko's own `ToolDispatcher` (API-provider tool loops) are denied even after `read_file`. contract.rs has uncommitted changes from a concurrent session, but they do not touch this rule.
