+++
id = "gap-b954ad"
kind = "gap"
title = "Integration test C5: a tampering attempt is flagged and an empty diff is rejected"
status = "open"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/tests"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e9"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W8-roko-as-executor.md (canary C5)"
anchors = ["crates/roko-cli/tests/attempt_diff_canary.rs"]
lane = "rust-cold"
parent = "spec-9230a9"
links = { depends_on = ["gap-abbd22", "gap-b72761"], blocks = [], related = ["gap-3aa9cb", "gap-d14a43"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn c5_tampering_attempt_is_flagged' crates/roko-cli/tests/ && grep -rqw 'fn c5_empty_diff_is_rejected_before_verify' crates/roko-cli/tests/ && cargo test -p roko-cli --test attempt_diff_canary"
+++

## Problem

gap-abbd22 and gap-b72761 get unit tests. Nothing runs a real plan with a misbehaving agent and checks that the
attempt is stopped before its verify steps, and that the checkpoint and `roko plan status` report it the same way.

## Why it matters

This is canary C5 of assessment W8 (gate G5) and the exit check for epic spec-9230a9. It joins the golden-path suite
(gap-3aa9cb).

## Where

- **New file:** `crates/roko-cli/tests/attempt_diff_canary.rs`.
- **Pattern to copy:** `crates/roko-cli/tests/graph_budget_resume.rs`. It configures a scripted fake provider
  through `roko.toml` and runs the built binary with `assert_cmd`. `tests/common/mod.rs` has the shared helpers.

## Current state

Checked at `41c7ffbd6`: no test covers tampering or empty diffs on a Graph run.

## Plan

1. Seed a git repo with a small crate, a pinned `[task.accept]` test (gap-d14a43) and a three-task plan whose verify
   steps would all pass on their own. Each verify step also writes a marker file, so the test can tell whether it
   ran.
2. `c5_tampering_attempt_is_flagged`:
   - T1's fake agent weakens the pinned test (edits its `accept/` source) and adds `#[ignore]` to an existing test.
     Assert that T1's attempt fails with a tamper finding and that its verify marker is absent.
   - T2's fake agent edits a file outside its `files`. Assert that the scope finding is recorded, and that the
     attempt fails when `diff_scope = "enforce"`.
3. `c5_empty_diff_is_rejected_before_verify`: T3's fake agent changes nothing. Assert that the attempt fails as
   "no changes" and that its verify marker is absent.
4. Assert that the checkpoint verdicts and `roko plan status` agree.

## Done when

- [ ] Both tests exist and pass with the fake provider only, in under a minute.
- [ ] Removing the wiring of either check makes a test fail. Check this once by hand and say so in the closing
      evidence.
- [ ] The `[[verify]]` command passes.

## Notes

- The test edits no hot file. It can be written while the fixes land, and it merges last.
- Use gap-3aa9cb's shared scripted provider if it exists.
- **wk-tamper (2026-09-29):** Implemented on `work/gap-b72761` at `58f3c89a8`; cargo verification deferred to the
  batch check. `cargo test -p roko-cli --test attempt_diff_canary` passed locally: 2 tests, 4.5 s, fake provider
  only. Each scenario is a one-task plan run on its own, not a three-task plan, so the scripted agent never has to
  tell tasks apart. Checkpoint status, its failed set and `roko plan status` are compared per plan.
  Checked by hand once: with the screen's diff checks unwired, both tests fail at their first assertion. The
  tampering T1 and the empty T3 both pass their verify steps (the pinned acceptance test included).
