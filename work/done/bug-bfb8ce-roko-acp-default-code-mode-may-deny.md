+++
id = "bug-bfb8ce"
kind = "bug"
title = "roko acp default code mode may deny every builtin tool"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
subsystem = ["roko-acp/tools"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-acp/src/bridge_events/tools.rs:455", "crates/roko-acp/src/bridge_events/mod.rs:607", "crates/roko-acp/src/bridge_events/cost.rs::acp_role_for_mode", "crates/roko-agent/src/safety/contract.rs::load_for_role_with_mode"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn builtin_tool_permitted_in_default_code_mode' crates/roko-acp/ && cargo test -p roko-acp builtin_tool_permitted_in_default_code_mode"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:22Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T17:36:42Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

A local static trace found that the builtin-tool contract gate looks up the session mode string `"code"` as a role; no bundled contract exists for it, so it falls back to a restricted contract with an empty allow-list and every builtin tool is denied in the default mode.
No test exercises role `"code"`.
Confirm with a conformance test that a builtin tool executes in `code` mode; fix by mapping modes to roles explicitly (e.g. `code` -> implementer).

Verified 2026-09-28 (static check against 3d0ee4d02): The gate lives at crates/roko-acp/src/bridge_events/tools.rs:455-478: it loads AgentContract::load_for_role_with_mode(role, RestrictedFallback) and denies tools the contract does not permit, with unknown roles falling back to a deny-everything restricted contract. The role is the session mode (session_agent_role = agent_mode, bridge_events/mod.rs:607; SafetyLayer .with_role(agent_mode) at :624, :913), default "code" (session.rs:179, :296); bundled contracts cover roles such as implementer/auto-fixer (crates/roko-agent/src/safety/contract.rs:46-51) but no "code" role, and no code->implementer mapping exists in roko-acp. Not run; a conformance test would confirm.

Re-verified 2026-09-29 at d9e79e9d8 (static): unchanged. The anchor crates/roko-acp/src/tools.rs:458 no longer exists; the gate is in bridge_events/tools.rs:455-478. bridge_events/cost.rs::acp_role_for_mode already maps code -> Implementer for cost and capability derivation; the fix can reuse it for the contract lookup.

## Notes

- 2026-10-01 (wk-specq): implemented on work/bug-8dbffd; cargo verification deferred to the batch check.
  New `cost.rs::acp_contract_role_for_mode` maps `code`/`plan`/`research` to the implementer, strategist and
  researcher contracts (reusing `acp_role_for_mode`); any other mode keeps its name and still gets the deny-all
  fallback. The prompt handler passes it as the builtin handlers' role. The pre/post-dispatch `SafetyLayer` still
  takes the raw mode; the role-scoped layer in gap-55eada is where that changes.
