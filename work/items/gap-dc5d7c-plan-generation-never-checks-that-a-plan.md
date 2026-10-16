+++
id = "gap-dc5d7c"
kind = "gap"
title = "Plan generation never checks that a plan keeps its source's acceptance checks: spec quality rated plans that kept 0 of 8 and 4 of 14 of them at 90.6 and 100"
status = "open"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/plan_generate"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "plans-research, round v6 (tmp/hermes-roko-v6/findings/plans-research.md §3.2 and §5 item 5)"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs::spec_scores", "crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan", "crates/roko-gate/src/spec_quality.rs::lint_files"]
links = { depends_on = [], blocks = [], related = ["bug-f0d60c", "gap-dde99e", "spec-e57870"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn from_file_generation_reports_dropped_source_checks' crates/roko-cli/src && cargo test -p roko-cli --lib -- from_file_generation_reports_dropped_source_checks"
+++

## Problem

Plan generation never checks that a plan keeps the checks its source states: Verification bullets, invariant
rows, `Testing Checkpoint` commands and `CHECKPOINT:` lines. Spec quality scores the form of a plan (`goal`,
`covers`, `expect`, `non_goals`), not whether it is faithful to the source. In plans-research's runs (§3.2), it
rated plans that kept 0 of 8 and 4 of 14 of the source's checks at 90.6 and 100.

## Why it matters

Without a coverage check, a plan can silently drop the criteria the person wrote. Nobody sees it until the work
fails review. The pitch is that a PRD's acceptance criteria become the checks the work must pass (goal
`golden-path`).

## Where

- `crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan` (prompt, regeneration, report) and
  `::spec_scores`.
- `crates/roko-gate/src/spec_quality.rs::lint_files`: the score, which says nothing about the source.
- `crates/roko-cli/src/task_parser.rs::TaskMeta`: where a plan says which checks it drops.

## Plan

1. Extract the source's checks deterministically and number them `SC1`, `SC2`, …. A check is:
   - a list item, a table row, an invariant block or a shell command under a check heading (Verification,
     Acceptance (Criteria), Invariants, Testing Checkpoint, Checklist, Done when);
   - or a `CHECKPOINT:` line anywhere.
2. List them for the planner. Each check either goes into a task's `acceptance` as `ACn: SCk: …`, with a verify step
   whose `covers` names `ACn`, or goes into `[meta] dropped_checks` as `"SCk: <why>"`.
3. After generation, compute the coverage: kept, dropped with a reason, or missing. Ask the planner once more if
   any check is missing, and keep the plan that leaves fewer. A regeneration for another reason never trades away
   a kept check.
4. Report the coverage:
   - `roko plan generate` prints it;
   - `GenerationOutcome.source_checks` records it;
   - serve's operation result carries it as `source_checks`.

## Done when

`[[verify]]` passes: generating from a file reports each source check as kept (with its task and criterion) or
dropped (with the reason).
