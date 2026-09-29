+++
id = "gap-baab0a"
kind = "gap"
title = "Codex tool policy is advisory; Roko-owned operation-level broker missing"
status = "open"
triage = "verified"
severity = "p1"
size = "L"
goal = "core"
subsystem = ["roko-agent/codex-cli"]
created = 2026-09-14
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "a17d9d766"
source = "tmp/dev-audit/09-additional-live-run-findings.md#Codex tool policy was advisory, not binding"
discovered_from = "audit:tmp/dev-audit/09-additional-live-run-findings.md#Codex tool policy was advisory, not binding"
anchors = ["crates/roko-agent/src/provider/claude_cli.rs::CodexCliAdapter", "crates/roko-agent/src/exec.rs::CodexOperationPolicy", "crates/roko-agent/src/exec.rs::check_codex_output_against_policy", "crates/roko-cli/src/graph_task_dispatch/routing_context.rs::effective_agent_contract", "crates/roko-cli/src/dispatch_v2.rs::build_codex_invocation"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn codex_restrictive_contract_is_enforced' crates/roko-agent/src/ && cargo test -p roko-agent --lib codex_restrictive_contract_is_enforced"
+++

## Problem

When a plan task runs on the Codex CLI provider, roko cannot stop Codex's built-in operations (shell
commands, file edits, web search) from going outside the task's safety contract. Codex has no binding flag for
a tool allowlist. roko passes only `--sandbox workspace-write`, then checks Codex's JSONL output after the
process has exited.

Observed in a live dogfood run (dev-audit "Codex tool policy was advisory, not binding"): the contract allowed
only `grep`, `read_file` and `write_file` and disallowed `web_fetch`/`web_search`; the dispatcher logged
`codex CLI cannot enforce tool policy; proceeding without enforcement`. Task T2 then made 44 Codex tool calls
(70 `agent.tool_call` events): it searched other worktrees and Codex session history under `$HOME`, ran
`git fsck`, and issued a web search.

Expected: a restrictive contract never silently degrades to prompt-only guidance. Either the Codex dispatch is
refused as unsupported, or every Codex operation passes through a roko-owned broker that enforces tool names,
read/write roots, network, commands and per-tool budgets, and records requested vs effective policy and every
denial in the run bundle.

## Why it matters

- Goal `core` (plan runs work reliably). This is a safety and correctness gap, and also a cost gap: the
  out-of-scope exploration used most of that turn's tokens.
- Any role that forbids edits (architect, reviewers) or network (implementer forbids `web_fetch` and
  `web_search`) is only partly enforced when routed to Codex.
- Related: `gap-a791b4` (per-tier turn caps reach only Claude CLI; Codex ignores `max_turns`), which is the same
  "opaque subprocess provider" limitation.

## Where

Live path (Graph engine):
- `crates/roko-cli/src/graph_task_dispatch.rs::effective_agent_contract` (`:3024-3032`): role contract
  (`RestrictedFallback`) plus task `allowed_tools`/`denied_tools` via `with_tool_restrictions`. It is passed as
  `agent_contract: Some(contract)` at `:3657` and `:4296`.
- `graph_task_dispatch.rs::run_bridge_with_failover` (`:4696`) -> `DispatchFactory::run_shared_agent_bridge`
  (`crates/roko-cli/src/dispatch/factory.rs:455-500`) ->
  `crates/roko-cli/src/dispatch_v2.rs::run_agent_result_bridge_with_tools_and_cli_mcp` (`:1756`; streaming
  variant `run_agent_streaming`, `:1626`) -> `create_agent_for_model`. `validate_contract_support` (`:1891`)
  accepts `CodexCli` with a contract.
- `crates/roko-agent/src/provider/claude_cli.rs::CodexCliAdapter::create_agent` (`:127-260`): builds
  `codex exec --json --cd <dir> --skip-git-repo-check --color never`, then `--sandbox workspace-write` (or
  `--dangerously-bypass-approvals-and-sandbox` when `dangerously_skip_permissions`), then `--model`. It derives
  `CodexOperationPolicy::from_contract` and wraps everything in `ExecAgent`.
- `crates/roko-agent/src/exec.rs`: `CodexOperationType` (`:50-90`, only `CommandExecution` and `FileChange`;
  `matches_tool_name` maps `bash`/`shell`/... and `write_file`/`edit_file`/`apply_patch`/...),
  `CodexOperationPolicy` (`:93-190`), `check_codex_output_against_policy` (`:210`), and the call site in
  `ExecAgent`'s run (`:661-683`). That call site scans the whole stdout after the process has exited and returns a
  failure signal on the first violation.

Dead path (the item's original anchor):
- `crates/roko-cli/src/dispatch_v2.rs::build_codex_invocation` (`:586-677`): logs the "cannot enforce tool
  policy" warning (`:607`), fails closed only when `ROKO_REQUIRE_BINDING_TOOL_POLICY` is set (`:596-602`), and pins
  `tools.web_search=false`, `sandbox_workspace_write.network_access=false` and more under `ROKO_FAST_MODE`
  (`:626-641`). It is reached only via `CliProviderConfig::build_invocation` (`:506`) <-
  `runner/agent_stream.rs::spawn_agent` (`:463`) <- `DispatchFacade::spawn_streaming_cli_agent(_controlled)`
  (`crates/roko-cli/src/dispatch/mod.rs:323-340`). Those two methods have no callers since Runner-v2 was deleted
  (2026-09-06). Only tests in `dispatch_v2.rs` (`:2580-2940`) still exercise it.

## Current state

- Partly addressed in `244f564e1` (2026-09-21, "RG-2"): a roko-owned operation broker exists
  (`CodexOperationPolicy` plus `check_codex_output_against_policy`). What it does not do:
  1. Prevent anything. It runs after Codex exits, so denied commands have already run and denied edits are
     already in the worktree. It only fails the turn.
  2. Cover all item types. Only `command_execution` and `file_change` are checked; `web_search`,
     `mcp_tool_call` and any other Codex item types pass.
  3. Check paths. `file_change` paths are not checked against the worktree, and reads are not policed at all:
     Codex reads through shell commands, and for roles that allow commands every command passes.
  4. Apply command policy or per-tool budgets.
  5. Record requested vs effective policy, or the denials, in the run bundle.
  6. Have tests. Nothing in the repo tests `CodexOperationPolicy` or `check_codex_output_against_policy`.
- The live adapter pins no network or web-search config. Whether Codex web search is on depends on the user's
  own Codex config (unknown per machine). `workspace-write` limits writes to the workdir; reads are not limited.
- `agent_options` in `dispatch_v2.rs` (`~:1837-1860`) passes `agent_contract` but no `safety_layer`, so the
  adapter falls back to the thread-local layer or `SafetyLayer::with_defaults()` (`claude_cli.rs:185-200`).

## Plan

Design choice (the dev-audit accepted either of the first two):
- (A) Fail closed for contracts Codex cannot honour. In `CodexCliAdapter::create_agent`, or earlier in
  `validate_contract_support`, return an unsupported-policy error when the effective contract has an
  `allowed_tools` allowlist (a task-level allowlist such as "only grep/read_file/write_file" cannot be expressed:
  Codex reads through its shell). Failover (`run_bridge_with_failover`) then picks a provider with binding
  policy (Claude CLI `--tools`/`--disallowed-tools`, Gemini settings). Cheap and honest, but Codex becomes
  unusable for allowlisted tasks. Role `ForbiddenTools` alone must not trigger it: every bundled role has some,
  and most are enforceable (below).
- (B) Make the broker live and wider. Check each JSONL line as `ExecAgent` reads stdout, and kill the process
  group on the first denied `item.started`. Add `web_search` and `mcp_tool_call` item types. Check
  `file_change` paths against the worktree. Pin `--config tools.web_search=false` and
  `--config sandbox_workspace_write.network_access=false` unless the contract permits network. This bounds
  damage and cost, but it is still not pre-execution: `item.started` arrives as the operation begins.
- (C) A true pre-execution broker: disable Codex's built-in shell and patch tools and expose only roko's
  contract-scoped MCP tools (the per-call local MCP bridge already used for CLI providers,
  `target_supports_per_call_local_mcp`), so every operation goes through `ToolDispatcher` and `SafetyLayer`.
  Whether the installed Codex CLI can disable its built-ins is unknown; check first.

Recommended: (A) for allowlists plus (B), then (C) as a follow-up if Codex supports it. Steps:
1. Add unit tests for the existing broker first (`from_contract`, `permits`, the JSONL scan), so later changes
   have a baseline.
2. Implement (B) in `crates/roko-agent/src/exec.rs`: streaming check and kill on violation (reuse the existing
   kill and PID-unregister path in `ExecAgent`), new item types, `file_change` root check. Add the config pins in
   `CodexCliAdapter::create_agent`. Test `codex_restrictive_contract_is_enforced`: a fake `codex` shell script
   prints an `item.started` `command_execution` line, then sleeps; with a contract forbidding `bash`, the agent
   returns a policy-violation failure well before the sleep ends and the child is gone.
3. Implement (A) for contracts with `allowed_tools`: return a typed error from the adapter (a new
   `AgentCreationError` variant, mirroring `DispatchV2Error::ToolPolicyUnsupported`), and make sure Graph
   failover treats it as "try another provider", not a task failure. If an escape hatch is wanted, make advisory
   mode an explicit opt-in (env or config) that records the degradation in the run bundle; failing closed is the
   default.
4. Record the requested and effective policy and each denial as run events (the Graph activity log /
   `dispatch_events_from_result`), so `roko diagnose` shows them.
5. Delete the dead `build_codex_invocation` path, or make both builders share one function, so there is a single
   Codex argument builder. Move or drop its tests.
6. Live check: run a small plan with a Codex model and a restrictive task `allowed_tools`, and confirm the run
   either fails over or kills Codex on the first denied operation, with the denial in the run bundle.

## Done when

- A Codex dispatch with a task-level `allowed_tools` allowlist never runs in advisory mode: it fails closed with
  a typed error and Graph failover moves to a binding provider.
- For role contracts, a denied `command_execution`, `file_change`, `web_search` or `mcp_tool_call` stops the
  Codex process when the item starts and fails the turn; `file_change` paths outside the worktree are denied.
- Web search and sandbox network are pinned off unless the contract permits network.
- Requested policy, effective policy and denials appear in the run's events.
- Verify:
  `grep -rqw 'fn codex_restrictive_contract_is_enforced' crates/roko-agent/src/ && cargo test -p roko-agent --lib codex_restrictive_contract_is_enforced`

## Notes

- Safety-sensitive and on the core dispatch path. Keep every change fail-closed, and do not weaken
  `SafetyLayer` or the role contracts to make Codex fit.
- The old verify command was unsound: the warning it greps for lives in dead code, and its `roko-cli` test
  filter was unguarded (no such test exists, so cargo passes with zero tests). The command above targets the
  live adapter and guards the test name.
- Decision already recorded in the dev-audit: a restrictive contract must never silently degrade to prompt-only
  guidance, and search scope should be the attempt worktree plus explicit read-only artifacts.
- Codex's JSONL event and item names (`item.started`, `command_execution`, `web_search`, `mcp_tool_call`, ...)
  depend on the installed Codex version. Confirm them against a captured run first.
- Touches `crates/roko-agent/src/exec.rs`, which every exec-based CLI provider uses. Avoid running in parallel
  with other work on `exec.rs` or `provider/claude_cli.rs`.

## Original notes

Codex CLI has no binding allowlist for built-ins; runs logged 'codex CLI cannot enforce tool policy; proceeding without enforcement' and one attempt made 70 tool calls incl. web search and cross-worktree reads. Strict requests now fail closed, but nothing enforces read/write roots, network or com...

Imported without verification from:
- `tmp/dev-audit/09-additional-live-run-findings.md#Codex tool policy was advisory, not binding`
- `tmp/dev-audit/11-implementation-status.md#Explicit residuals`
- `tmp/dev-audit/08-decisions-needed.md#6. P0 implementation order`
- `tmp/dogfood/2026-09-20-final-session.md#Items Completed This Session`
- `tmp/archive/dogfood-2026-08-22/DOGFOOD-DEBRIEF.md#Fix 4: Tool policy enforcement softened`

A source claims this was fixed; confirm against current code before closing.

How to verify: grep 'cannot enforce tool policy'; confirm build_codex_invocation fails closed for restrictive contracts and whether any broker exists.

Verified 2026-09-28: still true. `build_codex_invocation` (crates/roko-cli/src/dispatch_v2.rs:586) fails closed on allowed/disallowed tools only when `ROKO_REQUIRE_BINDING_TOOL_POLICY` is set (:596-602). Otherwise it logs 'codex CLI cannot enforce tool policy; proceeding without enforcement' (:607) and relies on codex `--sandbox` (:648). No Roko-owned operation-level broker exists for read/write roots, network or commands.
