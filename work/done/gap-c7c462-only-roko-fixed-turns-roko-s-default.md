+++
id = "gap-c7c462"
kind = "gap"
title = "Only roko_fixed turns Roko's default audits off: roko_fixed_glm/_kimi/_mini, fr_claude, roko_ladder and roko_plan still audit, and an audit dispatches models outside the arm"
status = "done"
triage = "verified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/arms"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
last_verified_rev = "ccc19e6aa"
source = "gap-327242 worker (claude-agent), 2026-10-20: preparing Pilot B"
discovered_from = "gap-327242"
anchors = ["benchmarks/viabilitybench/driver/run_roko.py::run_switches", "benchmarks/viabilitybench/driver/run_roko_plan.py", "benchmarks/viabilitybench/arms/roko_ladder.toml"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-327242", "gap-d76b1b", "bug-082810", "gap-aa60f2", "gap-e9218e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'def test_every_roko_arm_but_roko_full_turns_audits_off' benchmarks/viabilitybench/driver/test_run_roko.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_roko.py -q -p no:cacheprovider -k 'every_roko_arm_but_roko_full_turns_audits_off'"

[closed]
at = 2026-10-20
at_ts = "2026-10-20T16:05:57Z"
commit = "ccc19e6aa"
forced = false
evidence = "f5c2d6d9d and ccc19e6aa: every Roko arm but roko_full sets [roko] audit = false, so plan run gets --no-audit (run_roko.run_switches): roko_fixed, roko_fixed_glm/_kimi/_mini, fr_claude, roko_ladder and roko_plan; run_roko_plan.py adds run_switches to its plan run argv. roko_full keeps audits through its overlay audit_floor. Before the fix test_shakedown D7 failed once in two runs: an audit B3 review called gpt-5.4-mini (cost row role audit) after the ladder glm-4.7 pass, a model mismatch. Verify: test_run_roko.py -k every_roko_arm_but_roko_full_turns_audits_off passed; test_run_roko_plan (asserts --no-audit in the run argv), test_log1, test_shakedown, test_fr_claude, test_run_roko_routed, test_pilot_c passed."
+++

## Problem

Since 22ac6f06b (gap-d76b1b, 2026-10-18) Roko audits a share of its passes by default (`[audit] enabled = true`, ρ 0.10),
and an audit dispatches models besides the task's own: B1's suite author is a model of another family, and B3's
reviewer a cheap model (bug-082810). The ViabilityBench Roko runners hand Roko only the agent environment, so an
operator's `ROKO__AUDIT__ENABLED=false` never reaches it.

Pilot B (gap-327242) turned audits off for `roko_fixed` alone: its arm file says `[roko] audit = false`, and
`run_roko.run_switches` then adds `plan run --no-audit` (f5c2d6d9d). Every other Roko arm still runs Roko's default:

| arm | runner | where it runs | audits today |
|---|---|---|---|
| `roko_fixed_glm`, `roko_fixed_kimi`, `roko_fixed_mini` | `run_roko` (pinned) | LOG1 block A (gap-e9218e) | on |
| `fr_claude` | `run_roko` (pinned, ClaudeCli) | the H3 probe | on |
| `roko_ladder` | `run_roko` (ladder mode) | Pilot C (gap-aa60f2) | on |
| `roko_plan` | `run_roko_plan` (its own `plan run` command) | the plan-level slice | on |
| `roko_full` | `run_roko`, with an `[overlay]` whose `audit_floor` turns audits on | P1 live | on, by design (S05) |

## Why it matters

- A pinned arm must run only its model (S08 §4.9, W10 rec 5). An audit's other model is refused by the metering proxy
  (`models_allow`), and the census flags a task with a refused call (place `web`), which makes the run
  `leak_suspected`. An audit that reached a model would also bill calls outside the arm's cell.
- LOG1 block A and Pilot C are the next paid Roko runs.

## Where

- `benchmarks/viabilitybench/arms/*.toml`: each Roko arm's `[roko]` table.
- `benchmarks/viabilitybench/driver/run_roko.py::run_switches`, and `run_roko_plan.py`'s own `plan run` command line,
  which does not call it.

## Current state

Fixed on `work/gap-327242`. Every Roko arm but `roko_full` sets `[roko] audit = false`: `roko_fixed` (f5c2d6d9d),
then `roko_fixed_glm`, `roko_fixed_kimi`, `roko_fixed_mini`, `fr_claude`, `roko_ladder` and `roko_plan`.
`run_roko_plan.py` adds `run_roko.run_switches(arm)` to its `plan run` argv. `experiments/test_log1.py` holds the
per-model variants' `[roko]` tables equal to `roko_fixed`'s, so they had to follow it.

Seen before the fix: `test_shakedown_d7_auth_failure_keeps_healthy_circuits_closed` failed once in two runs on
2026-10-20. Its `roko_ladder` run passed on glm-4.7, then an audit's B3 review called gpt-5.4-mini (a cost row with
`role = "audit"`), which the runner flags as a model mismatch. The test was flaky at Roko's audit rate since
22ac6f06b.

## Plan

1. Decide per arm whether S08/S09 want audits: every arm but `roko_full` reads as "no S05 audits" (S08 §4.9's table
   gives audits to `roko_full` alone).
2. Add `audit = false` to those arms' `[roko]` tables, and have `run_roko_plan.py` add `run_switches(arm)` to its
   `plan run` argv.
3. A test that every Roko arm file but `roko_full` yields `--no-audit`.

## Done when

- [x] Every Roko arm but `roko_full` runs `plan run --no-audit`: the `[[verify]]` test.

## Notes

- Found while preparing Pilot B (gap-327242), 2026-10-20.
- Roko's S01 run manifest records `ablation_flags = ["learning_frozen"]` for a `--no-audit` run, with no audit flag;
  the bench's own record carries the switch through the arm file in `config_hash` and the transcript's `run` argv.
