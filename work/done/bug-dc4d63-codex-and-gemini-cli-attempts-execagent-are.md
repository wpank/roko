+++
id = "bug-dc4d63"
kind = "bug"
title = "Codex and Gemini CLI attempts (ExecAgent) are recorded at $0 when they time out or fail"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "M"
subsystem = ["roko-agent/codex-cli", "roko-agent/providers"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:fix/dispatch-timeouts-cost e0673e3e0"
anchors = ["crates/roko-agent/src/exec.rs::failure_signal", "crates/roko-agent/src/exec.rs:651", "crates/roko-agent/src/provider/claude_cli.rs::CodexCliAdapter", "crates/roko-agent/src/provider/gemini_cli.rs", "crates/roko-agent/src/provider/codex_cli/stream.rs::parse_stream_line_with_model"]
links = { depends_on = [], blocks = [], related = ["bug-690dc6", "q-1faa0c", "gap-552af4", "gap-288e38"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_timed_out_exec_run_reports_estimated_usage' crates/roko-agent/src && cargo test -p roko-agent --lib a_timed_out_exec_run_reports_estimated_usage && grep -rqw 'fn codex_exec_reports_turn_completed_usage' crates/roko-agent/src && cargo test -p roko-agent --lib codex_exec_reports_turn_completed_usage"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:24Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "M"
claimed_at = "2026-10-01T16:34:20Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

`e0673e3e0` made a Claude CLI attempt that is killed at its timeout report the usage it streamed, marked
`UsageSource::Estimated`. The Codex CLI and Gemini CLI providers run through `ExecAgent`, which got no such change.
Every failure path of `ExecAgent::run` returns `failure_signal`, whose usage is `Usage { wall_ms, ..Default }`: zero
tokens and $0. The paths are:

- timeout: `kill_tree`, then `"timed out after N ms"`;
- wait failure;
- non-zero exit;
- the Codex policy violation;
- the safety block.

On timeout the stdout reader is not even awaited. Graph dispatch then settles $0 in the plan ledger, per-task spend and
`.roko/learn/costs.jsonl`, and the provider's most expensive failures look free, which is what `bug-690dc6` fixed for
Claude.

Codex reports usage only on its terminal `turn.completed` event, so a killed `codex exec` usually has no reported usage.
The fix must estimate it from what the run consumed and streamed, not fall back to zero. The success path ignores
Codex's reported usage as well: it returns `prompt.len()/4` and `stdout.len()/4`, where stdout is only the extracted
agent text, with no source marker. `dispatch_v2::fill_cost_from_profile` then prices that guess.

## Why it matters

Goal `core`. Cost reports, budgets (per-task caps, the plan ledger) and routing learn from these numbers. Plans that
route cheap tasks to Codex or Gemini are exactly the runs the golden path wants to measure, and their failed attempts
are recorded as free.

## Where

- `crates/roko-agent/src/exec.rs`:
  - `ExecAgent::run` failure returns: wait failure :643-650, timeout :651-664, non-zero exit :710-719;
  - the success usage heuristic :747-755;
  - `failure_signal` :773-784.
- `crates/roko-agent/src/provider/claude_cli.rs::CodexCliAdapter` (:127-262): builds `ExecAgent` with
  `with_extract_codex_jsonl(true)`.
- `crates/roko-agent/src/provider/gemini_cli.rs` (:100): Gemini CLI on `ExecAgent`. The no-provider fallback
  (`provider/mod.rs:293`) uses it too.
- `crates/roko-agent/src/provider/codex_cli/stream.rs::parse_stream_line_with_model` (:161, `turn.completed` at
  :213-238, pricing `estimate_codex_cost` :137): an existing parser that reads Codex usage and prices it. Its only
  caller is the legacy `runner/agent_stream.rs:433`.
- For comparison, `crates/roko-agent/src/claude_cli_agent.rs` (:1056-1086) has the kill path to copy:
  `drain_killed_output` plus the `StreamUsage` estimate.

## Current state

Checked statically at `33e107da1`.

- The timeout path does not read the output stream.
- Nothing in `ExecAgent` parses Codex JSONL usage.
- `AgentResult.usage_obs` is never set, so the source is unknown.
- `q-1faa0c` (row 1 and plan step 1) names "the same in exec.rs" as part of the `bug-690dc6` fix. `e0673e3e0` covered
  Claude only.

## Plan

1. On the timeout and wait-failure paths, drain stdout and stderr the way `claude_cli_agent.rs` does (a bounded
   `drain_killed_output`).
2. When `extract_codex_jsonl` is set, feed each stdout line to `parse_stream_line_with_model`:
   - if a `turn.completed` was seen, report its tokens and cost as `UsageSource::ProviderReported`;
   - otherwise estimate the prompt tokens plus the streamed item text, and mark the result `Estimated` through
     `with_usage_obs`.
3. Apply the same parsing to the non-zero-exit and success paths, so a successful Codex run reports Codex's own numbers
   instead of the chars/4 guess.
4. For Gemini CLI, and any `ExecAgent` without a parser, return the chars/4 estimate as `Estimated` on every path
   instead of zero.
5. Add the tests `a_timed_out_exec_run_reports_estimated_usage` and `codex_exec_reports_turn_completed_usage` in
   `roko-agent` (lib).

## Done when

- A timed-out or failed Codex or Gemini CLI attempt reports non-zero, estimated usage.
- A successful Codex run reports its `turn.completed` usage.
- The `[[verify]]` command passes.

## Notes

- `exec.rs` is shared by every exec-based provider and holds the Codex policy broker (`gap-baab0a` plans changes there).
  Do not run both at once.
- `gap-552af4` (parked, unverified) is an older, broader claim that several CLI providers report zero tokens.
- 2026-10-01 (wk-model-truth): implemented on work/bug-3aa61f; cargo verification deferred to the batch check.
  Every `ExecAgent` path that ran its subprocess now carries a usage observation (`run_usage`): a Codex run's summed
  `turn.completed` counts as provider-reported (input without its cached tokens, reasoning apart), else a chars/4
  estimate of the stdin and the streamed text, marked `Estimated`. The timeout and wait-failure paths drain the
  killed run's stdout first (`claude_cli_agent::drain_killed_output`, now `pub(crate)`). The success path's
  unmarked chars/4 guess became the same observation. Cost is left to dispatch's pricing (profile or registry), as
  for other estimated usage. Tests: `a_timed_out_exec_run_reports_estimated_usage`,
  `codex_exec_reports_turn_completed_usage`.
- 2026-10-01 (wk-model-truth): after gate 6b brought in the Codex policy broker's live stop (gap-baab0a), a run
  it stops reports its usage the same way (`run_usage` after draining stdout) instead of none. Test:
  `a_run_the_policy_broker_stops_reports_estimated_usage`.
