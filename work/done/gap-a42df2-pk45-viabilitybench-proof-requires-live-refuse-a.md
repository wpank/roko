+++
id = "gap-a42df2"
kind = "gap"
title = "PK45 ViabilityBench proof: requires_live: refuse a live manifest whose loops are not LIVE at its harness_sha (+1 more)"
status = "done"
triage = "verified"
severity = "p1"
goal = "proof"
rank = 45
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-02
updated = 2026-10-04
last_verified = 2026-10-04
last_verified_rev = "343bc053f"
source = "tmp/backlog/2026-10-02-complete-and-wire PK45"
anchors = ["benchmarks/viabilitybench/driver", "benchmarks/viabilitybench/experiments"]
lane = "bench"
parent = "spec-c6e21b"
links = { depends_on = ["gap-f61823", "gap-daeaa9", "gap-5ddf9b", "gap-1f4bec", "gap-85d176"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_live_manifest_refuses_loops_that_are_not_live' benchmarks/viabilitybench/driver/test_campaign.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_campaign.py -k test_live_manifest_refuses_loops_that_are_not_live -q"

[[verify]]
command = "grep -qw 'def test_live_manifests_name_lines_caps_and_required_loops' benchmarks/viabilitybench/experiments/test_live.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/experiments/test_live.py -k test_live_manifests_name_lines_caps_and_required_loops -q"

[closed]
at = 2026-10-04
at_ts = "2026-10-03T22:14:17Z"
commit = "343bc053f"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-03T20:55:02Z"
forced = false
evidence = "Gate 10a (work/backlog-batch-10a, merged into main as 343bc053f): cargo check --workspace --tests, nightly fmt, cargo clippy --workspace -D warnings, nextest --lib 4,234 tests over roko-cli and roko-gate, roko-cli bin 438 passed and the golden-path canaries pass incl. the audit canary drill (plan_validate: only bug-2a31bc's two known alias tests fail), the bench driver and experiments suites 182 passed; every [[verify]] passes. PK45 2/2: campaign.py refuses a live manifest before dispatch when a requires_live loop isn't LIVE (a NOT RUN stub names the loops and harness sha); h5/h6/h7_live.toml manifests validate by dry run."
+++

## Problem

This package delivers 2 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK45, slice 33xx, phase 5), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3359 | S | p1 | requires_live: refuse a live manifest whose loops are not LIVE at its harness_sha | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3359-requires-live-refuses-loops-not-live.md` |
| 2 | 3361 | S | p2 | Live manifests for H5, H6 and H7 | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3361-live-manifests-h5-h6-h7.md` |

## Why it matters

Phase 5: M2 loop-liveness audit (S03). The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3300-the-proof-viabilitybench-pilots-log1-and-head-to-head.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/driver/campaign.py`, `benchmarks/viabilitybench/driver/test_campaign.py`, `benchmarks/viabilitybench/experiments/h5_live.toml`, `benchmarks/viabilitybench/experiments/h6_live.toml`, `benchmarks/viabilitybench/experiments/h7_live.toml`, `benchmarks/viabilitybench/experiments/test_live.py`.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: PK10 (gap-f61823), PK25 (gap-daeaa9), PK27 (gap-5ddf9b), PK40 (gap-1f4bec), PK44 (gap-85d176).
- Suggested model: sonnet.

## Progress

- 3359: implemented at 2bb61611b. `check()` runs the loop census (new `census_report`, injectable) and refuses
  before dispatch when a `requires_live` loop's audit state is not `live`; `cmd_campaign` writes a NOT RUN stub
  (`vb.not_run/1`, not a forced `vb.metric_record/1`) naming the loops and the harness sha before raising.
  `test_campaign.py::test_live_manifest_refuses_loops_that_are_not_live` fakes the census with monkeypatch (dormant
  refuses and writes the stub; all-LIVE then runs normally). Verify passes.
- 3361: implemented at fd2d12aab. New `h5_live.toml` (BL4), `h6_live.toml` (BL3), `h7_live.toml` (BL5), each a
  valid `vb.experiment/1` manifest on 3331's compiled streams (s1_learncurve, s3_disturbance +
  s3_disturbance_hooks.toml, s5_holdout), with `requires_live` from S09 §5's own example (H5/H6) and the prereg
  sketch's `closure_4` census check (H7: L-M1, L-audit, L-route-trust). `test_live.py::test_live_manifests_name_lines_caps_and_required_loops`
  validates all three via `vb campaign --dry-run`, faking the census LIVE (3359) and the pre-registration lock
  clean (3341/3345, held for Will as gap-394f28, not this task's job) with monkeypatch. Verify passes. Two
  judgment calls, documented in the manifests' own comments: the H5-A0/A1/A3 and H6 9-seed/arm split (no separate
  arm files exist for these labels, so each uses `roko_fixed` differentiated by seed/disturbance rather than a
  new arm), and H7's stream (budget.toml's BL3 note mentions "the S1 stream and the harmful stream" for H7, but
  only s5_holdout, 3331's harmful-loop stream, is used here; nothing read for this task names a second file).
