+++
id = "gap-c4f364"
kind = "gap"
title = "ViabilityBench Claude Code arm with an isolated config (S08.T12)"
status = "open"
triage = "unverified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e12"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W10-benchmarks-proof.md (the unclean baseline, rec 4); specs/S08-benchmark-suite.md (§4.9, §6 T12)"
anchors = ["benchmarks/viabilitybench/driver/run_cli.py", "benchmarks/viabilitybench/driver/test_run_cli.py", "benchmarks/viabilitybench/arms/fd_claude.toml"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = ["gap-28ebea", "dec-b78874"], blocks = [], related = ["gap-ad0d39"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_claude_arm_command_is_isolated_and_pinned' benchmarks/viabilitybench/driver/test_run_cli.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_cli.py -k test_claude_arm_command_is_isolated_and_pinned -q"

[[verify]]
command = "grep -qw 'def test_result_event_is_priced_as_u_prime_and_r' benchmarks/viabilitybench/driver/test_run_cli.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_cli.py -k test_result_event_is_priced_as_u_prime_and_r -q"
+++

## Problem

The frontier baseline is Claude Code on `claude-opus-5-5` (D2), run non-interactively. Launched from the author's
machine, `claude -p` also loads the author's global `CLAUDE.md`, memory, hooks, plugins and MCP servers, so the
baseline would measure the author's setup rather than Claude Code. No spec isolates it (W10).

Copying Roko's flags as they are adds a second problem: `ClaudeCliAgent::build_command` passes `--fallback-model`
whenever one is configured, and a fallback silently switches the model.

## Why it matters

`fd_claude` is the arm Roko has to match to show "cheaper at equal quality", and H1's bar (D3) is measured against
it. An unclean baseline is neither fair nor reproducible.

## Where

- **New:** `benchmarks/viabilitybench/driver/run_cli.py`, `driver/test_run_cli.py` and `arms/fd_claude.toml`. The
  paths follow D4.
- **Reference:** the flags in `crates/roko-agent/src/claude_cli_agent.rs::ClaudeCliAgent::build_command`
  (`--print --verbose --output-format stream-json --model --effort --max-turns`), without Roko's system prompt.

## Current state

Checked at `41c7ffbd6`: nothing exists. W10 rec 4 leaves the isolation mechanism unverified, and D42 is open
(dec-b78874).

## Plan

1. **A fresh config directory per run** (for example `CLAUDE_CONFIG_DIR` under `$VB_WORK/<run>/`) holding
   credentials only: no user `CLAUDE.md`, memory, hooks or plugins; `--strict-mcp-config` with an empty MCP
   config; and no `CLAUDE.md` in any parent of the workdir.
2. **Probe first.** One throwaway run's `system`/`init` stream event must show the pinned model and no MCP servers
   or extra tools. The subscription credentials may not reach a fresh config directory (for example if they live
   in the macOS keychain); record a mechanism that works before building on it.
3. **Pin the settings:** `--model claude-opus-5-5` and never `--fallback-model`; `--effort` set explicitly, since
   Claude Code's default effort differs from the API's; `--max-turns 50`; a 20-minute limit; and a kill once a task
   passes $5 of API-equivalent cost (S08 §4.10).
4. **Parse the `result` event.** U′ = each model's `modelUsage` × snapshot, the headline (S09 §4.1); R =
   `total_cost_usd`. Store both with |U′ − R|/R. Record the reported models and exclude runs whose model changed.
5. **`config_hash`** covers every flag and a hash of the config directory's contents.

## Done when

- [ ] A unit test shows that the command and environment the arm builds are isolated and pinned.
- [ ] A recorded `result` fixture yields both U′ and R.
- [ ] The probe's `init` event is saved as closing evidence.
- [ ] Both `[[verify]]` commands pass.

## Notes

- **Waits for D42.** No scripted run on the subscription until D42 is answered.
- **Out of scope:** the Codex runner and mini-swe-agent from S08.T12.
- **Subscription contention.** `fd_claude` shares the author's subscription with the development agents, so run it
  off-hours.
- **Small models in the bill.** Claude Code may bill background turns to `claude-haiku-4-5`, which then shows up in
  `modelUsage`. The snapshot in S08 §5.6 has a row for it.
