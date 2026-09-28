+++
id = "bug-bfb8ce"
kind = "bug"
title = "roko acp default code mode may deny every builtin tool"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
subsystem = ["roko-acp/tools"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-acp/src/tools.rs:458", "crates/roko-acp/src/bridge_events/tools.rs:455"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

A local static trace found that the builtin-tool contract gate looks up the session mode string `"code"` as a role; no bundled contract exists for it, so it falls back to a restricted contract with an empty allow-list and every builtin tool is denied in the default mode.
No test exercises role `"code"`.
Confirm with a conformance test that a builtin tool executes in `code` mode; fix by mapping modes to roles explicitly (e.g. `code` -> implementer).

Verified 2026-09-28 (static check against 3d0ee4d02): The gate lives at crates/roko-acp/src/bridge_events/tools.rs:455-478: it loads AgentContract::load_for_role_with_mode(role, RestrictedFallback) and denies tools the contract does not permit, with unknown roles falling back to a deny-everything restricted contract. The role is the session mode (session_agent_role = agent_mode, bridge_events/mod.rs:607; SafetyLayer .with_role(agent_mode) at :624, :913), default "code" (session.rs:179, :296); bundled contracts cover roles such as implementer/auto-fixer (crates/roko-agent/src/safety/contract.rs:46-51) but no "code" role, and no code->implementer mapping exists in roko-acp. Not run; a conformance test would confirm.
