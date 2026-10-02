+++
id = "gap-6af7e2"
kind = "gap"
title = "ACP tool policy leftovers: remote MCP tool names and the session safety layer's mode"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-acp"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "39feebc07"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-55eada"
anchors = ["crates/roko-acp/src/bridge_events/mod.rs", "crates/roko-acp/src/bridge_events/dispatch.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-55eada"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-acp --lib remote_mcp_tools_respect_forbidden_tools"

[closed]
at = 2026-10-02
at_ts = "2026-10-01T23:45:22Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T18:50:43Z"
forced = false
evidence = "Gate 6d on 9eacfde5f plus its fixes, re-checked at c9e78d12d and merged as 39feebc07 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean; lib tests pass (roko-cli 3407, roko-agent 2276, roko-core 1980, roko-learn 1228, roko-serve 1012, roko-acp 219, roko-compose 562, roko-execution 192, roko-gateway 43); all eight canaries with the new default-isolation canary, golden_path_suite, secret_canary, C2, graph_plan_callers, cost_dedup, phase0_wiring, run_serve_share, graph_timeout_matrix (6), plan_prepare_full and the gateway pipeline contract pass; bin tests pass; scripts/test_run_evidence_graph.py 9/9 against the gate binary; Cargo.lock unchanged; route snapshot matches. Implemented in this round; the item's notes name the change and its test."
+++

## Problem

gap-55eada applied the AgentContract tool policy to ACP tool dispatch. Two parts are left: Plan step 5(b), checking remote MCP tool names against ForbiddenTools; and the session pre/post-dispatch SafetyLayer in `bridge_events/mod.rs`, which still takes the raw mode, so "code" gets the restricted contract there.

## Plan

Check remote MCP tool names against ForbiddenTools, and map the mode to its contract role in the session layer. Add a test named `remote_mcp_tools_respect_forbidden_tools`.

## Done when

- The test passes.

## Notes

- Reported on 2026-10-01 by wk-specq, working on gap-55eada, during the evening close-out round.
- 2026-10-01 (wk-specq): implemented on work/bug-8dbffd; cargo verification deferred to the batch check.
  `setup_session_mcp_tools` takes the contract role, and `AcpMcpToolHandler` refuses a tool whose remote name is in
  the role's `ForbiddenTools` (`tools.rs::contract_forbids_tool`). The pre/post-dispatch layer comes from
  `mod.rs::session_safety_layer`, which uses `acp_contract_role_for_mode`. Behaviour change to watch: in `plan` and
  `research` modes, the post-dispatch check now blocks a turn that changed files (strategist and researcher forbid
  writes). Edits the user makes in the workspace during that turn also count.
