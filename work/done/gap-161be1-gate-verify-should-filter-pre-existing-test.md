+++
id = "gap-161be1"
kind = "gap"
title = "Gate Verify Should Filter Pre-Existing Test Failures"
status = "done"
triage = "verified"
severity = "p1"
size = "M"
goal = "core"
subsystem = ["roko-cli/runner"]
created = 2026-09-07
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "aec267cac"
source = "tmp/backlog/archive/166-gate-verify-preexisting-filter.md#166 — Gate Verify Should Filter Pre-Existing Test Failures"
discovered_from = "audit:tmp/backlog/archive/166-gate-verify-preexisting-filter.md#166 — Gate Verify Should Filter Pre-Existing Test Failures"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs::GraphTaskDispatcher::settle_task_verification", "crates/roko-cli/src/graph_task_dispatch/baseline_verify.rs::GraphTaskDispatcher::judge_against_baseline", "crates/roko-cli/src/graph_task_dispatch/sibling_settle.rs::InFlightTasks::settle_failed_step", "crates/roko-cli/src/runner/gate_dispatch.rs::run_focused_baseline_verify", "crates/roko-cli/src/runner/gate_report.rs::filter_preexisting_failures", "crates/roko-core/src/config/gates.rs"]
links = { depends_on = [], blocks = [], related = ["gap-a534e4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn preexisting_verify_failure_is_filtered' crates/roko-cli/ && cargo test -p roko-cli preexisting_verify_failure_is_filtered && grep -rqw 'fn new_failure_on_top_of_preexisting_still_rejects' crates/roko-cli/ && cargo test -p roko-cli new_failure_on_top_of_preexisting_still_rejects"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T13:44:22Z"
by = "coordinator (session 7622b882)"
size = "M"
claimed_at = "2026-10-01T08:25:50Z"
forced = false
evidence = "Batch 20e gate on 1a8aad603, re-checked with the compile fixes (cf722c1be, bed29287d), tiers' rustfmt (8a6c932ce) and the run-index scrub fix (d972959bd) on 32fe02384; MAIN aec267cac has the same crates: check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-acp/agent/cli/core/dreams/fs/gateway/graph/learn/neuro/serve/std; lib tests roko-cli 3305, roko-agent 2294, roko-core 1962, roko-learn 1213, roko-serve 991, roko-graph 478, roko-fs 260, roko-neuro 239, roko-std 229, roko-acp 199, roko-dreams 100, roko-gateway 42 all pass; extras: golden_path_suite 2/2, all eight canaries pass (secret_canary 11/11 and C2 2/2 after the scrub fix), worktree_task_diff 2/2, plan_run_config_flag 1/1, default_engine 1, bin 429, routing crash loop 10/10, bench driver 18, including the baseline-verify dispatch and unit tests (preexisting_verify_failure_is_filtered and new_failure_on_top_of_preexisting_still_fails). Merged 096ed268a (work/gap-161be1 4bc969541)."
+++

## Problem

A plan task's authored verify step usually runs a whole crate's tests, for example `cargo test -p roko-serve`.
If a test in that crate was already failing before the agent started, the step fails and the Graph engine
rejects the task. It then retries until `max_retries` and fails the task, although the agent did not cause the
failure and was never asked to fix it.

Expected:
- a failure that also happens on the base revision, unchanged, does not reject the task;
- a new failure (a test that passed before and fails now, or a different failure in the same step) still
  rejects it;
- the gate output and logs say which failures were filtered as pre-existing.

Seen live:
- dogfood 2026-08-23: task `gate-compile-fail-closed` T02 ran `cargo test -p roko-gate && cargo clippy …`,
  which failed on the pre-existing broken test `tautology_filter_discards_preexisting_passing_tests`. The
  agent spent about $1.13 over two retries investigating it before it was killed.
- portal-programme run 2026-09-25 (finding N-8): `cargo test -p roko-serve` inherited three `job_lifecycle`
  tests that were already failing, because `POST /api/jobs/{id}/cancel` returned 500. They had to be marked
  `#[ignore]` so unrelated tasks could pass.

## Why it matters

Goal `core`. Pre-existing failures waste tokens and retries, fail plans that did the right thing, and push
operators to `#[ignore]` tests or narrow verify commands until they check nothing. This is "Proof Case 3:
baseline gate rejection filtering" in the 2026-09-23 master action plan: a verify command fails identically on
base and candidate, the baseline failure is reported, and the candidate is not rejected.

Related:
- `gap-a534e4` (parked, DA-11): baseline classification unproven; `BenchmarkRegressionGate` always passes.
- Backlog #170 (adaptive verify scoping) was the companion idea.

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::settle_task_verification` (:1807): the
  Graph path. It runs each `[[task.verify]]` step as `ShellGate("bash -o pipefail -c <command>")` in the
  attempt's `effective_workdir`, fail-fast. A failed step goes through sibling settlement and is re-run once.
  Any remaining failure returns `RokoError::Verify`, which means retry and then fail. This is where the filter
  belongs.
- `crates/roko-cli/src/graph_task_dispatch/sibling_settle.rs`: `InFlightTasks::settle_failed_step`. It waits
  for sibling tasks in the same tree, re-runs, and tags `blocked_by_sibling`. Added in `3049b7fcf`.
- `crates/roko-cli/src/runner/gate_dispatch.rs`: `run_gate_once` (:876) and `spawn_gate` (:324), the old
  runner gate path. No production caller; only tests and doc references reach it. Its baseline logic sits at
  :1283-1305. When gate mode is `Focused` and a `task-verify:<task>:` step fails, it calls
  `run_focused_baseline_verify` (:2101), which:
  - makes a detached `git worktree add --detach <tmp> HEAD`;
  - re-runs only the `cargo test` verify steps there (chosen by `cargo_command_fingerprint(..).action == "test"`);
  - returns `Vec<GateVerdictSummary>`, and removes the worktree.
- `crates/roko-cli/src/runner/gate_report.rs::filter_preexisting_failures` (:52). For each failing verdict it
  looks for a failing baseline verdict with the same raw gate name and the same
  `normalized_failure_fingerprint(error_digest)` (the digest JSON minus `duration_ms`). On a match it flips the
  verdict to passed, renames the gate `pre-existing-filtered:<gate>` and logs "filtered unchanged pre-existing
  gate failure".
- `crates/roko-gate/src/shell.rs` and `crates/roko-gate/src/compile_errors.rs::GateFailureClassification`:
  `ShellGate` puts a JSON classification (gate, primary class, summary, classes, compile errors, counts) in
  `Verdict.error_digest`.
- `crates/roko-core/src/config/gates.rs`: `[gates]` config (`sibling_settle_secs`, `compile_concurrency`, …),
  where a new knob would go.

Entry point: `roko plan run <dir>` → Graph engine → `GraphTaskDispatcher` (batch and streaming paths) →
`settle_task_verification`.

## Current state

- The Graph path has no baseline. Checked at `a17d9d766`: no mention of `preexisting`,
  `filter_preexisting_failures` or `run_focused_baseline_verify` under `graph_task_dispatch*`.
- The baseline filter exists only in the unreachable `runner/gate_dispatch.rs` path. It survived the Runner-v2
  deletion (`6b5da8616`, 2026-09-06), and the event loop the original spec wanted to change
  (`runner/event_loop.rs`) is gone.
- `3049b7fcf` added sibling-aware settlement. That covers failures caused by concurrent sibling tasks only.
  Failures present before the task started still reject it.
- Unknown: whether `GateFailureClassification.summary` names the failing tests for a `cargo test` failure. If
  it does not, two different test failures in one step give the same fingerprint, and the old filter would hide
  a real regression.

## Plan

1. Add a baseline helper under `graph_task_dispatch/` (for example `baseline_verify.rs`), reusing the runner
   code (move `run_focused_baseline_verify`'s worktree handling and `filter_preexisting_failures` into it
   rather than copying them):
   - run lazily: only when a step still fails after sibling settlement;
   - only for test-like steps (`cargo test`, `cargo nextest`; reuse `cargo_command_fingerprint`);
   - run the same command in a detached temporary worktree at the base revision and compare.
2. Choose the base revision. Options:
   - (a) `HEAD`, as the old code does. Simple, but it misses uncommitted work in a shared tree.
   - (b) The plan run's start commit, recorded once per run.
   - (c) For per-task worktrees (`gap-4ec59f`), the attempt's base commit.

   Recommend (b), falling back to `HEAD`, and cache the baseline result per `(base sha, command)` for the
   whole run so ten tasks with the same failing step pay for one baseline.
3. Compare at test-name level for test steps. Parse the libtest output (`test <name> ... FAILED` lines, or the
   `failures:` list) into a set on both sides:
   - new failures = candidate − baseline;
   - if that is empty and the candidate failed only because of tests in the baseline set, treat the step as
     passed-with-pre-existing-failures;
   - otherwise fail, and name only the new failures in the retry prompt.

   Keep the digest-fingerprint comparison as the fallback for non-test output.
4. Report it:
   - record the filtered test names on the verdict (a gate name like `pre-existing-filtered:<step>` plus the
     list);
   - log at info level;
   - forward to the TUI through `tui.gate_result_with_output`;
   - write the list into the attempt's gate output so it lands in the checkpoint and the activity log.
5. Add a `[gates]` switch (for example `baseline_filter = true`, on by default) and skip it in FAST mode if
   that mode forbids extra cargo runs.
6. Tests (fake shell commands are fine; no real cargo needed):
   - `preexisting_verify_failure_is_filtered`: a step fails identically on base and candidate → the task
     passes and the filtered test is reported;
   - `new_failure_on_top_of_preexisting_still_rejects`: base fails test A, candidate fails A and B → rejected,
     B named.

## Done when

- In a scratch repo whose `HEAD` has a failing test, a Graph task with an unrelated change and a verify step
  running that crate's tests is accepted. The run's gate output and log list the failing test as
  pre-existing.
- A task that breaks another test in the same crate is still rejected, and only the new failure is reported to
  the retry.
- No `#[ignore]` workaround is needed for known-broken tests to let unrelated tasks pass.
- `cargo test -p roko-cli preexisting_verify_failure_is_filtered` and
  `cargo test -p roko-cli new_failure_on_top_of_preexisting_still_rejects` pass (see the suggested verify),
  and so does the existing `[[verify]]` grep.

## Notes

- Never turn a genuine regression into a pass. When in doubt (output cannot be parsed, the baseline did not run
  or timed out, the worktree could not be created), fall back to failing, as today.
- The baseline worktree builds from scratch unless it shares a target dir. The runner code passed
  `main_target_dir`. Sharing `CARGO_TARGET_DIR` avoids a cold build but can invalidate incremental caches for
  the main tree. Measure this, and respect `compile_concurrency` / `verify_compile_permit`.
- Do not change `sibling_settle.rs` semantics. The baseline check runs after sibling settlement.
- Parallel safety: this edits `graph_task_dispatch.rs`, a hot file (task dispatch, worktree release).
  Coordinate with items touching the same file (for example `gap-4ec59f`, `gap-5d3b82`).
- Options B (auto-scoped test commands) and C (`known_failures = [...]` per verify step) from the original
  spec remain possible follow-ups. C is a cheap escape hatch but needs manual upkeep.
- Implemented on `work/gap-161be1` at `a8c93a596`; cargo verification deferred to the batch check. A failing
  `cargo test` step is judged in `graph_task_dispatch/baseline_verify.rs` after sibling settlement: its tests
  run again without fail-fast on the attempt's tree and on the run's start commit (manifest `base_commit`,
  else `HEAD`) in a detached worktree with its own target dir, cached per (commit, command) for the run.
  Libtest output is paired with cargo's stderr target headers and compared by test name. All failures
  pre-existing: the gate becomes `pre-existing-filtered:<step>`, the output names them, and the attempt
  settles as the new `passed_with_preexisting_failures` verdict (dashboard outcome of the same name, counted
  as passed). New failures, or tests the task names or declares a file for, still fail, and the retry output
  names them apart from the old. Anything not comparable by name fails as before. Deliberately no
  whole-output fingerprint fallback: an unmet check of the task's own work fails identically on the base.
  Read-only checks before the tests are dropped from both runs, and the parts after the tests must still
  pass. `[gates] baseline_filter` (default on); FAST mode skips it. Not judged yet: commands that run tests
  in more than one part, and `cargo nextest`.

## Original notes

agents waste tokens and retries on failures they didn't cause. Task verify commands in plans (e.g., `cargo test -p roko-gate`) run the entire crate's test suite, including pre-existing broken tests that the agent was never assigned to fix. When a pre-existing test fails, the gate rejects the…

Imported without verification from:
- `tmp/backlog/archive/166-gate-verify-preexisting-filter.md#166 — Gate Verify Should Filter Pre-Existing Test Failures`
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#5.3 Proof Case 3: Baseline gate rejection filtering`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`.

How to verify: Check: A pre-existing test failure does not cause a gate rejection for an unrelated task.; A genuine regression (test that passed before the agent's changes and now fails); The gate logs which failures were filtered as pre-existing. [evidence: 00-STATUS-SUMMARY 3. Open / P1 -- High (Open): S | 3 |]

Verified 2026-09-28: The baseline filter exists only in crates/roko-cli/src/runner/gate_dispatch.rs::run_gate_once (filter_preexisting_failures, run_focused_baseline_verify), reachable through spawn_gate from tests only. The Graph path's graph_task_dispatch.rs::settle_task_verification runs authored [[task.verify]] steps with no baseline. Live confirmation: tmp/dogfood/2026-09-25-portal-programme-run.md N-8 (whole-crate gates inherit pre-existing failures; the tests had to be #[ignore]d).

Checked 2026-09-29 at d9e79e9d8: still open. 3049b7fcf added sibling-aware settlement (crates/roko-cli/src/graph_task_dispatch/sibling_settle.rs): a verify step that fails while sibling tasks are mid-attempt in the same working tree is re-run once after they settle, and a persistent failure located in a sibling's files is tagged blocked_by_sibling. That handles failures caused by concurrent siblings only; failures that already existed before the task started still reject it, and nothing logs filtered pre-existing failures.
