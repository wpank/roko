+++
id = "gap-9eebcb"
kind = "gap"
title = "Integration test C7: the watchdog kills a silent agent, and a low-disk run refuses to start"
status = "open"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/tests"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "a4e175c9c"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e10"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W8-roko-as-executor.md (gate G7, canary C7)"
anchors = ["crates/roko-cli/tests/supervision_canary.rs"]
lane = "rust-cold"
parent = "spec-edda86"
links = { depends_on = ["spec-a0403b", "reg-7cf6f9", "gap-a791b4", "gap-5a6e01"], blocks = [], related = ["gap-3aa9cb"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn supervision_canary' crates/roko-cli/tests/ && cargo test -p roko-cli --test supervision_canary"
+++

## Problem

Nothing tests, through the built binary, that a plan run survives a hung agent or refuses to start on a nearly full
disk. Today only the attempt timeout (600 s, rising by half on each retry) bounds the first, and nothing checks the
second.

## Why it matters

This is the exit check for epic spec-edda86 and canary C7 for W8's gate G7: a silent agent is killed after the
watchdog window, not after 600 s × 3, then classified and reported. It joins the golden-path suite (E11.1,
gap-3aa9cb).

## Where

- **New file:** `crates/roko-cli/tests/supervision_canary.rs`.
- **Pattern to copy:** `crates/roko-cli/tests/graph_budget_resume.rs`.
- **Surfaces to assert:** exit status and duration, the Graph checkpoint, the `--log-file` JSONL
  (`dashboard.diagnosis` lines) and the fake provider's call log.

## Current state

No such test at `41c7ffbd6`. The watchdog (spec-a0403b) and the disk preflight (reg-7cf6f9) do not exist yet.

## Plan

1. **Silent agent.** The fake provider prints one event, writes its pid to a file and sleeps 60 s. Config:
   `[conductor] task_stall_secs = 2`, task `timeout_secs = 120`, `max_retries = 1`. Assert:
   - the run ends within about 20 s, not after 2 × 120 s;
   - the checkpoint classifies both attempts as stalled (or timed out, whichever spec-a0403b settles on);
   - the `--log-file` JSONL holds a `dashboard.diagnosis` line naming the task;
   - the provider process is gone afterwards (check the pid file, not `ps` output).
2. **Low disk.** Set the threshold reg-7cf6f9 wires (today's `[resources] min_free_disk_mb`) far above any real disk,
   for example 1,000,000,000 MB. Assert that `roko plan run` exits non-zero before any dispatch, that the message names
   the free space and the threshold, and that the provider's call log is empty.
3. **If gap-5a6e01 has landed:** in shadow mode, the plan-start log lists each tier's configured and suggested limits.

## Done when

- [ ] The test exists and passes in under a minute, with the fake provider only.
- [ ] Without the watchdog it fails on the duration check. Check this once by hand and say so in the closing evidence.
- [ ] The `[[verify]]` command passes.

## Notes

- The test edits no hot file; it merges last.
- If reg-7cf6f9 adds a new threshold key instead of honouring `min_free_disk_mb`, use that key.
- 2026-10-01 (wk-tiers), reg-7cf6f9 (notes here, since its item file is dirty in MAIN): implemented on `work/gap-9eebcb` at `aa68eeff7` in `graph_execution/disk_admission.rs`.
  - `check_free_disk` refuses a run below `[resources] min_free_disk_mb` before any dispatch, and the message names the free space and the threshold. `plan run --force` now skips that check on the Graph engine; before, the engine ignored `--force`.
  - With `--worktree-per-task`, `DiskAdmission` reserves `WORKTREE_GROWTH_ESTIMATE_MB` (3 GB) per attempt until the attempt ends. Under disk pressure, attempts wait for running ones to end and then run one at a time. Each admission publishes `worktree_count` and `disk_budget_remaining` on the conductor ring.
  - Tests: `disk_admission_blocks_under_low_budget` (its verify), `disk_admission_serialises_only_under_pressure`, `a_run_refuses_to_start_below_the_free_disk_threshold`, `an_attempt_waits_for_disk_headroom`.
  - Not restored: Runner-v2's pre-plan log rotation, stale-target cleanup and GC; the opt-in post-gate `cargo clean` (`ROKO_EXPLICIT_CARGO_CLEAN`); `per_plan_disk_budget_mb`; growth measured from the worktrees (the estimate is fixed). The Graph conductor evaluates only attempt-tagged signals, so nothing acts on the two metrics yet.
- 2026-10-01 (wk-tiers): C7 implemented on `work/gap-9eebcb` at `7d4169c48`, with the 300 s sleep at `d399a6526`. `supervision_canary` and `supervision_canary_refuses_a_low_disk_run` pass in the worktree in about 10 s.
  - With `task_stall_secs = 0` set by hand, `supervision_canary` fails on the duration check: the run took 307 s.
  - The stalled attempts' per-attempt record is the dashboard diagnosis (`intervention_taken = "cancelled stalled attempt"`, one per attempt key). Their verdicts say `provider_error` with infra blame: the `RokoError::Timeout` text lacks the `timed out after` marker that `provider_failure_outcome` looks for. So a stall is neither a timeout nor agent-blamed, and the checkpoint records only that T1 failed.
  - The watchdog counts silence only after an `assistant` message: an agent that printed only `content_block_delta` lines and then slept was never cancelled. It then exited 0 with no `result` line, and its attempt passed.
  - Plan step 3, the shadow tier-limit log, is not asserted. Workspace verification is deferred to the batch check.
