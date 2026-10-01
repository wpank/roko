+++
id = "dec-50192e"
kind = "decision"
title = "Should the plan-validate CI job run roko plan validate --strict?"
status = "open"
triage = "unverified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = [".github/workflows"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (gap-ba4d01 plan step 4, left as a separate decision by wk-runstate)"
anchors = [".github/workflows/plan-validate.yml"]
lane = "docs"
parent = "spec-e57870"
links = { depends_on = [], blocks = [], related = ["gap-ba4d01", "gap-d14a43", "find-8cc7ac"], supersedes = [], duplicate_of = "" }
+++

## Question

`.github/workflows/plan-validate.yml` (about line 60) runs `roko plan validate "$plan_dir"` without `--strict`, so warnings such as PLAN_038 (a hand-copied acceptance test) and the tier-size warnings never fail CI. Should the job run `--strict`?

## Context

- gap-ba4d01 made the six portal plans PLAN_038-free; other tracked plans may still carry warnings, and `--strict` would fail CI on them.
- CI on `main` is not green yet (find-8cc7ac), and the CI workflows are shared with the other session's work.

## Recommendation

Not yet. First count the warnings `--strict` would reject across the tracked plans (excluding `fixtures/` and `plans/archive/`, as the job does). Make the active plans warning-free, then switch the job to `--strict` in the same change that fixes the last warning.

## Done when

- [ ] Decided, and the decision is recorded in this item.
