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
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "905fa313d"
source = "tmp/dev-audit/09-additional-live-run-findings.md#Codex tool policy was advisory, not binding"
discovered_from = "audit:tmp/dev-audit/09-additional-live-run-findings.md#Codex tool policy was advisory, not binding"
anchors = ["crates/roko-agent/src/provider/claude_cli.rs::CodexCliAdapter", "crates/roko-agent/src/exec.rs::CodexOperationPolicy", "crates/roko-agent/src/exec.rs::check_codex_output_against_policy", "crates/roko-cli/src/graph_task_dispatch/routing_context.rs::effective_agent_contract", "crates/roko-cli/src/dispatch_v2.rs::validate_contract_support", "crates/roko-cli/src/commands/diagnose.rs::recorded_tool_policies"]
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

Dead path (the item's original anchor; deleted in Plan step 5 on 2026-10-02):
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
- 2026-10-01 (wk-guard2): PARTIAL on work/bug-a70def; cargo verification deferred to the batch check. Keep the item open: its verify command passes after this step, but most of Done-when does not.
- Landed Plan step 1 and the live half of step 2, all in `crates/roko-agent/src/exec.rs`. Baseline tests cover `CodexOperationPolicy::from_contract`, `permits` and the JSONL scan. `ExecAgent` now checks Codex's JSONL as it arrives (`CodexStreamBroker`, one line at a time) and kills the process tree at the first denied operation, so a denied `item.started` stops Codex as the operation begins; the post-exit scan stays for a final line without a newline. Test `codex_restrictive_contract_is_enforced`: a fake `codex` prints a denied `command_execution` and sleeps 30 s; the run fails with the violation within 5 s and the process is gone.
- Still open: the `web_search` and `mcp_tool_call` item types and the `file_change` worktree-root check (confirm Codex's item names and fields against a captured run first); the web-search and sandbox-network pins in `CodexCliAdapter::create_agent` (confirm the `--config` keys against the installed Codex); step 3, the typed fail-closed error for task allowlists plus failover; step 4, policy and denials as run events; step 5, deleting the dead `build_codex_invocation`; step 6, the live check. A denial still lands as the operation starts, so this bounds the damage but does not prevent it; only Plan (C) would.
- 2026-10-02 (wk-guard2): step 2 finished on work/bug-7f15df; cargo verification deferred to the batch check. I checked the item names and config keys against the installed codex-cli 0.152.0. The exec event strings in its binary are `item.started`/`item.completed` with item types `agent_message`, `reasoning`, `command_execution`, `file_change`, `mcp_tool_call`, `web_search` and `todo_list`; `mcp_tool_call` carries `server`/`tool`/`arguments`, `web_search` carries `query`/`action`, and `file_change` carries `changes[].path`. `codex features list` accepted `-c web_search="disabled"` and `-c sandbox_workspace_write.network_access=false`, and rejected bad values for both, so they are real keys. (`web_search` takes `disabled`, `cached`, `indexed` or `live`; the old `tools.web_search` key no longer appears in the binary.)
- The broker now covers `web_search` (denied unless the contract permits the network: new `AgentContract::permits_network`, which is false under `NoNetworkAccess` or with no network tool allowed) and `mcp_tool_call` (any `mcp__` name stands for all of them, so a forbidden MCP tool denies Codex's MCP calls, fail-closed). A permitted `file_change` must stay inside the agent's working directory (`safety::path::is_within_worktree`, relative or absolute, through symlinks); this check runs whenever the broker does, which is for runs with a constraining contract. `CodexCliAdapter` pins web search and sandbox networking off when a contract keeps the role off the network. Tests: `codex_policy_covers_web_search_and_mcp_calls`, `codex_file_change_outside_the_worktree_is_denied`, `exec_agents_other_than_codex_are_not_policed` (a non-Codex exec agent with a deny-all policy and Codex-looking output still succeeds), and `codex_network_is_pinned_off_unless_the_contract_permits_it`.
- 2026-10-02 (wk-guard2): step 3 on work/bug-7f15df; cargo verification deferred to the batch check. A contract with any `allowed_tools` allowlist (a task allowlist, or the restricted fallback's empty one) now refuses Codex in three places. `CodexCliAdapter::create_agent` returns the new `AgentCreationError::ToolAllowlistUnsupported` before it looks for the binary, so every caller fails closed. roko-cli's `validate_contract_support` (now `pub(crate)`) returns `DispatchV2Error::ContractUnsupported` for Codex. Graph failover's `blocked_provider` uses that same check, so the planned Codex model is refused before any call (class `contract_unsupported`, definitive) and the next candidate runs; an explicit `--model` pin to Codex fails the attempt instead. Role contracts with only `ForbiddenTools` still run on Codex under the broker. No advisory opt-out was added; failing closed is the only mode. Tests: `codex_adapter_refuses_a_tool_allowlist` (roko-agent), `codex_cannot_take_a_contract_with_a_tool_allowlist` (dispatch_v2), and `codex_with_a_tool_allowlist_fails_over_before_any_call` (failover.rs: a task hinted at a Codex model with `allowed_tools` runs on claude_cli, the fake codex is never called, and the verdict records the failover chain and reason).
- 2026-10-02 (wk-guard2): step 4 on work/bug-7f15df; cargo verification deferred to the batch check. Each attempt on Codex under a contract records `executed.tool_policy` in its verdict row (`.roko/runs/<run>/attempts.jsonl`, roko-learn's new `ToolPolicyRecord`). Requested: the contract's `allowed_tools` and `forbidden_tools`. Effective: `enforcement = "broker"`, the operations the broker denies (`CodexOperationPolicy::denied_operations`), and `network_off` when the `-c` pins applied. Denial: the operation the broker stopped the run at, read from the new `codex_policy_denial` tag (`CODEX_POLICY_DENIAL_TAG`) that `ExecAgent` puts on the output when it stops a run. dispatch_v2 builds the record (`tool_policy_record`) on `AgentResultDispatch.tool_policy`; the watchdog's cancelled-call dispatch carries none. Other providers record nothing. Tests: `codex_attempts_record_their_tool_policy` (dispatch_v2), the tag check added to `codex_restrictive_contract_is_enforced`, and `a_denied_codex_operation_is_recorded_with_the_tool_policy` (failover.rs: an implementer task on a fake Codex that starts a `web_search` fails, and its verdict carries the record and the denial). `roko diagnose` shows the denial through the failed attempt's failure reason but does not read verdict rows, so the policy itself is visible only in `attempts.jsonl`; showing it in diagnose is a follow-up.
- 2026-10-02 (wk-guard2): step 5, and `roko diagnose` for step 4, on work/bug-7f15df; cargo verification deferred to the batch check. `CodexCliAdapter::create_agent` is now the only Codex argument builder. roko-cli's `build_codex_invocation` is deleted: it was reachable only through `DispatchFacade::spawn_streaming_cli_agent(_controlled)`, which have no callers. Its helpers `codex_plugin_mcp_args` and `env_flag_enabled`, the `ROKO_REQUIRE_BINDING_TOOL_POLICY` and `ROKO_REQUIRE_NATIVE_TURN_LIMIT` switches, and the `DispatchV2Error::ToolPolicyUnsupported`/`TurnLimitUnsupported` variants they returned went with it. `CliDispatchProvider::build_invocation` now refuses a Codex provider (`UnsupportedCliProvider`), so a revived streaming path cannot run Codex outside the broker. `codex_shared_target_dir`, whose only remaining use is the `CARGO_INCREMENTAL` choice in `CliInvocation::new`, is renamed `shared_target_dir`. Tests: `codex_has_no_cli_invocation` replaces the three Codex invocation tests, and `shared_target_is_limited_to_canonical_repo_target` replaces `codex_add_dir_is_limited_to_canonical_repo_target`, checking the shared target through a Claude invocation's `CARGO_INCREMENTAL`.
- `roko diagnose` now reads the run's verdict rows (`.roko/runs/<run>/attempts.jsonl`, for the checkpoint's run id) and lists each attempt's `executed.tool_policy` under its task, in the JSON (`tasks[].tool_policies`) and in the text: the attempt and its model, the enforcement, the allowlist and forbidden tools asked for, the denied operations and the network pin, and the operation the broker stopped it at. Test: `graph_report_lists_the_tool_policy_of_each_attempt`.
- Done-when, checked against steps 1 to 5: all four bullets are met in code and unit tests. The policy and denial are kept in the run's attempt log, not as separate Graph activity-log events. Limits that stay outside this item: a denied operation is stopped as it starts, not prevented (only Plan (C) would prevent it); if Codex reports a `file_change` only when it completes, the broker sees it after the edit; and with `dangerously_skip_permissions` Codex runs without its sandbox, so the network pin then covers only web search.
- Remaining: Plan step 6, the live check, needs a live Codex provider and spend, so it is out of this round (the coordinator files it as a follow-up for Will). Run a small plan on a Codex model with a restrictive task `allowed_tools`, and confirm the task fails over before any Codex call. Then run a role whose contract keeps it off the network, and confirm a web search stops Codex, with the denial in `attempts.jsonl` and in `roko diagnose`.

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
