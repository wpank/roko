+++
id = "gap-9eebcb"
kind = "gap"
title = "Integration test C7: the watchdog kills a silent agent, and a low-disk run refuses to start"
status = "open"
triage = "unverified"
severity = "p1"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/tests"]
created = 2026-09-29
updated = 2026-09-29
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
