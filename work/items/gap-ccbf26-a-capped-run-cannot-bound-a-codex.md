+++
id = "gap-ccbf26"
kind = "gap"
title = "A capped run cannot bound a Codex, Gemini or Cursor CLI attempt: none streams its usage or takes a spend limit, and Codex's and Gemini's turn caps are advisory, so only the attempt timeout ends one"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-cli/budget"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "tmp/hermes-roko-v5/findings/roko-caps.md (roko-caps, 2026-10-16, at the lead's request)"
discovered_from = "gap-ca1c92 and gap-e14344 cover the Claude CLI only"
anchors = ["crates/roko-agent/src/provider/codex_cli/stream.rs", "crates/roko-agent/src/provider/gemini_cli.rs", "crates/roko-agent/src/provider/cursor_cli.rs", "crates/roko-agent/src/provider/mod.rs::TurnCapEnforcement", "crates/roko-cli/src/graph_task_dispatch/watchdog.rs::run_watched"]
links = { depends_on = [], blocks = [], related = ["gap-ca1c92", "gap-e14344", "gap-8548e3", "bug-4fabd0"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_capped_cli_attempt_without_streamed_usage_is_bounded_by_its_budget' crates/ && cargo test -p roko-cli --lib a_capped_cli_attempt_without_streamed_usage_is_bounded_by_its_budget"
+++

## Problem

A run's cap now holds inside an attempt for roko's own tool loop (bug-4fabd0) and for the Claude CLI: roko prices
what the Claude CLI streams (gap-ca1c92), and the CLI also gets its own `--max-budget-usd` (gap-e14344). The other
agents that run their own loop give roko nothing to hold them with. While such an attempt runs, roko can't see what
it spends, and nothing stops it on spend.

- **Codex CLI** (0.152.0, run as `codex exec --json`; roko reads its JSONL live for the operation broker). Usage
  arrives only in `turn.completed`, at the end of the run. It has no spend flag and no turn flag. roko's turn cap is
  advisory (`TurnCapEnforcement::Advisory`), so only the attempt timeout ends a long run.
- **Gemini CLI** (0.41.2, `gemini -m … -p … --output-format stream-json`). Usage arrives only in the final
  `result.stats`, and it has no spend flag. The turn cap is advisory too. The CLI does have a `model.maxSessionTurns`
  setting (default -1, unlimited) that roko does not set.
- **Cursor CLI** (`agent` 2026.03.30, run over ACP as `agent --force acp`). ACP reports no usage or cost; Cursor
  bills by plan. roko bounds turns by counting tool calls (`TurnCapEnforcement::ToolCalls`). Under a cap, gap-8548e3
  already passes over a Cursor model with no price, but one priced in roko.toml still runs with its spend unknown.

## Why it matters

The Hermes plugin promises that a run stays under the cap the person sets. On a Codex or Gemini rung, a capped
attempt can spend as long as its timeout allows.

## Where

- `crates/roko-agent/src/provider/codex_cli/stream.rs`, `crates/roko-agent/src/exec.rs::codex_stream_broker`:
  Codex's live stream.
- `crates/roko-agent/src/provider/gemini_cli.rs`, `crates/roko-agent/src/provider/gemini_cli/stream.rs`: Gemini's
  arguments and stream.
- `crates/roko-agent/src/provider/cursor_cli.rs`: Cursor over ACP.
- `crates/roko-agent/src/provider/mod.rs::TurnCapEnforcement`: how each provider's turn cap holds.
- `crates/roko-cli/src/graph_task_dispatch/watchdog.rs::run_watched`: where gap-ca1c92's watch would count streamed
  usage.

## Plan

What each CLI could offer, best first:

1. **Streamed usage**, so gap-ca1c92's watch counts it as it does the Claude CLI's:
   - Codex: check whether its app-server protocol, which roko does not use yet, notifies token usage per model
     response. `exec --json` does not.
   - Gemini and Cursor: none known.
2. **A native limit:**
   - Gemini: set `model.maxSessionTurns` through a run-scoped settings file, so roko's turn cap holds natively.
   - None of the three has a spend flag.
3. **A turn or call cap derived from the remaining budget**, for runs with neither:
   - Cap = the attempt's spend limit (`CallAllowance::spend_limit_usd`) divided by an estimated cost per call: the
     context so far at the model's price, plus a typical output.
   - Enforce it by counting the live stream's items or tool calls, as the Cursor adapter counts tool calls.
   - Settle the stop as `budget_exhausted`.
4. Decide whether a capped run should refuse a provider whose spend it can never see (Cursor), as gap-8548e3 refuses
   an unpriced model.

Splitting this item per CLI is fine.

## Done when

`a_capped_cli_attempt_without_streamed_usage_is_bounded_by_its_budget` passes (the `[[verify]]` command): a scripted
Codex or Gemini CLI that never reports usage is stopped by a budget-derived cap, not by its timeout.

## Notes

Filed by roko-caps at the lead's request, after gap-ca1c92 and gap-e14344 covered the Claude CLI. CLI versions and
options were checked on this machine on 2026-10-16 (`--help`; Gemini's settings schema in its bundle).
