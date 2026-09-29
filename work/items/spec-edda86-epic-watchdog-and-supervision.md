+++
id = "spec-edda86"
kind = "spec"
title = "Epic: watchdog and supervision"
status = "open"
triage = "unverified"
severity = "p1"
goal = "golden-path"
size = "L"
subsystem = ["roko-cli/graph_task_dispatch", "roko-cli/graph_execution", "roko-agent/providers"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e10"
discovered_from = "tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md (P1 #14); evidence/field/CASES.md (CASE-004)"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::task_turn_limit", "crates/roko-cli/src/graph_task_dispatch.rs::base_attempt_timeout_ms", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan"]
doc = "tmp/cybernetic-harness/workstreams/PLAN.md"
lane = "rust-hot"
links = { depends_on = ["spec-a0403b", "reg-7cf6f9", "gap-a791b4", "gap-5a6e01", "gap-9eebcb"], blocks = [], related = ["gap-ebd656", "find-43768e", "gap-c89b40"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn supervision_canary' crates/roko-cli/tests/ && cargo test -p roko-cli --test supervision_canary"
+++

## Problem

A Graph plan run has hard limits but no supervision:

- **No stall detection.** A silent agent holds its slot until the attempt timeout, 600 s by default. Since
  `d4be4e872` each timed-out retry gets 1.5× more time, so a hung provider can hold one task for
  600 + 900 + 1,350 + 2,025 s, about 81 minutes, over four attempts.
- **No disk check.** `[resources] min_free_disk_mb` is documented as refusing a run below its threshold, but no Graph
  code reads it, and `roko plan run --force` is ignored with a warning that the Graph engine does no disk-space
  pre-check (`commands/plan.rs:2478`).
- **Limits set by guesswork** (CASE-004). Turn caps of 10/10/20/30 failed good attempts on 09-28 and were raised by
  hand to 40/60/90/120; operators raised timeouts by hand.
- **Turn caps reach only the Claude CLI.** The HTTP tool loop, which every cheap model on the D11 ladder uses, stops
  at `max_tool_iterations` (default 50) whatever the tier.

## Why it matters

Unattended runs on cheap models need a safety net: cheap models stall and loop more often, and the benchmark's cost
figures are wrong if a hung attempt can run for over an hour. This is W8's gate G7 and tldr/05 P1 #14.

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs`: the dispatch select loops and the 5 s heartbeat
  (`AGENT_HEARTBEAT_INTERVAL`, :67, used at :3875); `task_turn_limit` (:712), `base_attempt_timeout_ms` (:760) and
  `raised_attempt_timeout_ms` (:772).
- `crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan`: host-side preflight and supervision.
- `crates/roko-core/src/config/schema.rs`: `[conductor] silence_timeout_secs` and `task_stall_secs` (:1721, :1727), and
  `[resources] min_free_disk_mb` (:2255). `roko-execution`'s `check_disk` (fixed 2 GB) is never called by `plan run`.
- `crates/roko-agent`: the provider adapters and `tool_loop/`.

## Current state

Checked at `41c7ffbd6`.
- **spec-a0403b (p0) is open.** The heartbeat only feeds the TUI's elapsed-time counter, and only the TUI's conductor
  panel reads the conductor thresholds.
- **reg-7cf6f9 is open.** Nothing in `crates/` admits by disk space or publishes `disk_budget_remaining`.
- **gap-a791b4 is open.** `codex_agent.rs`, `cursor_cli_agent.rs` and `tool_loop/` never mention `max_turns`.
- **Recent help, not a fix.** `d4be4e872` charges a timed-out attempt its partial usage and resumes it with a raised
  timeout; a turn-cap stop is classified and retried with a raised cap.

## Plan

This is the implementation plan.

1. **Stall watchdog** (spec-a0403b part A, M). Cancel an attempt that makes no progress for `task_stall_secs`,
   classify it, retry it under `max_retries` and publish a diagnosis. First: C7 and every unattended run need it.
2. **Disk preflight and admission** (reg-7cf6f9, S). Refuse to start below `min_free_disk_mb`, and reserve headroom
   before creating attempt worktrees. Different files from step 1, so it can run beside it.
3. **Turn caps on every provider** (gap-a791b4, M). Map the tier's cap onto each adapter's limit or the shared tool
   loop. After gap-8c0a20 (the tier enum); roko-agent only, so it can run beside steps 1 and 2.
4. **Limits from history** (gap-5a6e01, M): per-tier caps and timeouts from the p95 of passed attempts. After E4.2
   (gap-96f7ed) and step 3.
5. **Conductor ticker** (spec-a0403b part B, M): semantic watchers on a ticker, with Restart or Fail. C7 does not
   need it, so it can follow.
6. **Exit check C7** (gap-9eebcb, S), after steps 1–4.

## Done when

- [ ] spec-a0403b: Graph Engine Watchdog Integration (existing item)
- [ ] reg-7cf6f9: Disk-aware worktree admission and the disk_budget_remaining metric were lost with Runner-v2
      (existing item)
- [ ] gap-a791b4: Per-tier turn caps reach only Claude CLI; other providers ignore AgentOptions.max_turns (existing
      item)
- [ ] gap-5a6e01: Turn caps and timeouts set from each tier's p95 over successful tasks
- [ ] gap-9eebcb: Integration test C7: the watchdog kills a silent agent, and a low-disk run refuses to start
- [ ] The epic's `[[verify]]` command (test C7) passes on the merged branch.

## Notes

- **Proposed split of spec-a0403b (p0, L)**, along the parts its own plan already names: (A) the per-attempt stall
  watchdog in the dispatcher, which satisfies its verify `graph_watchdog_intervenes_on_stalled_task` (M); (B) the
  conductor ring and supervision ticker at the host (M). gap-ebd656 builds on B.
- **Existing children keep their goal and severity:** spec-a0403b (`core`, p0), reg-7cf6f9 (`core`, p2) and gap-a791b4
  (`core`, p2). PLAN.md §5 proposes moving them to `golden-path`.
- **For epic spec-98f76d:** until gap-a791b4 lands, tier turn caps bind none of the cheap rungs.
- **Related:** find-43768e (the 600 s default is too low for build-heavy tasks) and gap-c89b40 (waits on cargo's build
  lock count against verify timeouts).
- **Hot files:** `graph_task_dispatch.rs` (a split is planned, E15.4) and `plan_runner.rs`. Steps 1 and 5 wait for the
  split; step 3 touches roko-agent only.
