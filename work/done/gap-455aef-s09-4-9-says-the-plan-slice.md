+++
id = "gap-455aef"
kind = "gap"
title = "S09 §4.9 says the plan-slice arms share no visible checks, but both get the same base repo and its tests"
status = "done"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["cybernetic-harness/specs"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "cf66f3dfd"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:47, wk-bench-slice's report on gap-89f393)"
anchors = ["tmp/cybernetic-harness/specs/S09-experiments.md", "benchmarks/viabilitybench/families/plan_slice/README.md"]
lane = "paper"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-419298", "gap-89f393", "gap-1cd676", "gap-204848"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'since the arms share no visible checks' tmp/cybernetic-harness/specs/S09-experiments.md && grep -q 'v1.3' tmp/cybernetic-harness/specs/S09-experiments.md"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "S09 v1.3 (tmp, wk-bench-slice): §4.9's VF also requires the base's visible tests, restored from pristine, to pass at the final commit, with the reason and census fields; changelog and status line updated. The tracked part (census vf includes the visible tests; verified = vf with no canary hit; new test for a base regression) merged with gap-204848 (248a925d6). Verify passes. Paper follow-up filed as gap for D.12 and §5.4."
+++

## Problem

S09 v1.2 §4.9 defines the plan-level slice's verified feature (VF) and says: "There is no visible-check condition, since the arms share no visible checks." The fixtures from gap-89f393 give both arms, `roko_plan` and `fd_claude`, the same base repo with its passing tests and a visible smoke check (`families/plan_slice/README.md`, "The verdict"). The census already computes `visible`, and a stricter `verified` that also requires the visible tests to pass.

## Why it matters

Pilot benchmark (epic spec-567e52): VF is the slice's main outcome. With no stated rule, a run that passes the hidden suite but breaks the base's own tests counts as a verified feature, and the reason the spec gives for that is false.

## Where

- `tmp/cybernetic-harness/specs/S09-experiments.md`, §4.9 (around line 592).
- `benchmarks/viabilitybench/families/plan_slice/README.md`, "The verdict" (`vf` against `verified`).

## Current state

S09 is at v1.2 (gap-419298, wk-specs). The slice README points out the mismatch.

## Plan

Choose one, and write it into S09 v1.3 with a changelog line:

- **Option A (recommended): add the visible condition.** VF also requires the base's visible tests, restored from the pristine base as the census does, to pass at the final commit. This matches the census's `verified`.
- **Option B: keep VF on the hidden suite only,** and say that the shared base tests are ignored, and why.

Then make the census's `vf` field match the chosen definition, and update the paper sections that quote §4.9 (Appendix A.7 and §5.3).

## Done when

- [ ] S09 v1.3 §4.9 says how the shared visible tests count, with a true reason.
- [ ] The `[[verify]]` command passes.

## Notes

- `tmp/cybernetic-harness/` is untracked: edit S09 in place in the main checkout.
