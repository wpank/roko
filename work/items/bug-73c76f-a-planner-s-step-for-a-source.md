+++
id = "bug-73c76f"
kind = "bug"
title = "A planner's step for a source's regression check keeps the fail_on_base it was written with: 7.2's first rewrite labelled P7.2.13 (tools/spec.sh 7 ship) fail_on_base against the spec's \"A regression check\" and expect = pass_on_base, and plan validate --strict passed it"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/plan_generate"]
created = 2026-11-13
updated = 2026-11-13
last_verified = 2026-11-13
last_verified_rev = "48fb93ca2"
source = "round v8: the lead's note and hermes-pilot-v8's findings, phase 7.2 rewrite 1 (op ee2a9e7d, plans/p7-2-the-window-as-designed-2, Undertow 1414699)"
discovered_from = "agent:hermes-pilot-v8 (round v8)"
anchors = ["crates/roko-cli/src/plan_generate/source_checks.rs::assign_runs", "crates/roko-cli/src/plan_generate/source_checks.rs::is_regression", "crates/roko-cli/src/plan_validate.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-59782c", "gap-29c1e8"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_regression_check_labelled_fail_on_base_is_warned_about' crates/roko-cli/src/ && cargo test -p roko-cli --lib -- a_kept_step_of_a_regression_check a_regression_check_labelled_fail_on_base"
+++

## Problem

A step the planner wrote for a source's regression check keeps the `expect` label it was written with, even when that
contradicts the source, and `plan validate --strict` passes it.

- **The spec.** Round v8's phase 7.2 (Undertow `docs/specs/phase-7.2-window-as-designed.md` at 1414699) calls P7.2.13
  (`tools/spec.sh 7 ship`) a regression check twice:
  - in its check table: "A regression check: it passes on the base";
  - in its notes for the planner: "Checks: the regression checks P7.2.13 and P7.2.14 (`expect = "pass_on_base"`)".
- **The plan.** The first rewrite (op ee2a9e7d, `plans/p7-2-the-window-as-designed-2`, untracked) labelled T5's step
  `tools/spec.sh 7 ship` as `fail_on_base`. Its other step, P7.2.14, it labelled `pass_on_base`.
- **The two checks.** `plan validate --strict --json` passed it. Undertow's plancheck flagged it, and `--dynamic` gave
  T5 grade B with HF3, "verify passes on the unchanged base". roko's run-start gate would likely have refused T5 mid
  run. Rewrite 2 (op 369096a3) wrote it right.

There are two defects:
1. **The writer dropped the spec's label.** bug-59782c's repair sets `pass_on_base` only on a step that `assign_runs`
   adds or extends to run a source check's command. A step the planner already wrote with the check's exact command
   counts as kept, and its `expect` is never compared with the check's `regression` flag (`is_regression`).
2. **Strict validate doesn't compare the labels.** No `plan validate` rule compares a step's `expect` with the
   regression labels of the `[meta] sources` checks it runs.

## Why it matters

- A regression check labelled `fail_on_base` fails roko's red-on-base check at run start, so the run stops for the
  person. Without `--dynamic`, that happens in front of the person, mid run.
- The spec stated the label twice, and the generated plan dropped it.
- Goal: core.

## Where

- `crates/roko-cli/src/plan_generate/source_checks.rs`: `assign_runs`, the bug-59782c repair; `is_regression`.
- `crates/roko-cli/src/plan_validate.rs`: the plan-level warnings that read `[meta] sources`.

## Plan

1. After the source checks are placed, a step that runs only the source's regression checks gets `expect =
   "pass_on_base"`, whether the planner wrote it or roko did. A step that runs a check the source does not call a
   regression check loses a `pass_on_base` the source gives no reason for, as `assign_runs` already does for the
   steps it touches.
2. `plan validate` warns when a step's `expect` contradicts the regression label of a source check it runs, naming
   the step, the check and the label.
3. Add the tests `a_kept_step_of_a_regression_check_passes_on_the_base` and
   `a_regression_check_labelled_fail_on_base_is_warned_about`, on phase 7.2's notes for the planner and rewrite 1's T5.

## Done when

- The `[[verify]]` command passes.

## Notes

- From the lead's note (round v8), with the pilot's findings ("The spec fix … Rewrite 1"). Evidence is read-only:
  the untracked plan directory in Undertow and the spec at 1414699.
- Related: bug-59782c (regression checks pass on the base), gap-29c1e8 (a plan-level warning from `[meta] sources`).
