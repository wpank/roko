+++
id = "bug-59782c"
kind = "bug"
title = "The source-check coverage repair adds a missing check's step with no expect, so a check the source calls a regression check loses pass_on_base (2.1's P2.1.09 in T3)"
status = "done"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-20
updated = 2026-10-21
last_verified = 2026-10-21
last_verified_rev = "a12d2a60f"
source = "round-v7-hermes-pilot#K-p2-1-plan"
discovered_from = "agent:hermes-pilot (round v7 phase 2.1 plan, op 742f8afd)"
anchors = ["crates/roko-cli/src/plan_generate/source_checks.rs::SourceCheck"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_repaired_regression_check_step_passes_on_base' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_repaired_regression_check_step_passes_on_base"

[closed]
at = 2026-10-21
at_ts = "2026-10-20T23:26:30Z"
commit = "18aaf6257"
by = "roko-planner (round v7)"
executor = "claude-agent"
via = "manual"
forced = false
evidence = "Merged be7c29f0b (work/bug-7c6f1b: 18aaf6257). SourceCheck.regression from the check's own text ('regression check', pass_on_base, 'passes on the base too') or a sentence calling its label one ('P2.1.08 and P2.1.09 are regression checks'; the appositive 'P2.1.04, and P2.1.09, a regression check of ...' marks neither). assign_runs: a step it adds for regression checks only has expect = pass_on_base and its repair line says so; an extended step takes it when it then runs only regression checks and loses it when it runs another. Post-merge verify at be7c29f0b: grep OK; a_repaired_regression_check_step_passes_on_base passed. $0 replay of 2.1's draft: T3's added P2.1.09 step has expect = pass_on_base (before: none)."
+++

## Problem

Round v7, Undertow phase 2.1, plan `p2-1-undertow-voice-fixes` (roko d7d1fbb8b, op `742f8afd`, `spec_path =
docs/specs/phase-2.1-voice-fixes.md`). The spec has two regression checks:

- the table rows say so: "P2.1.08 … A regression check: it passes on the base too", and the same for P2.1.09;
- the notes say "P2.1.08 and P2.1.09 are regression checks (`expect = "pass_on_base"`)".

The planner gave P2.1.08 its `expect = "pass_on_base"`, but left P2.1.09 out. roko's coverage repair then added it to
the right task, T3 (`engine.cpp`), as a new step:

```toml
[[task.verify]]
phase = "test"
command = "tools/spec.sh 2_1 engine 'P2.1.09*'"
fail_msg = "the source's checks SC9"
timeout_ms = 900000
```

with no `expect`, so the plan file no longer says the step passes on the base. Here it does no harm: the spec gate
stops at T3's first step (P2.1.04, red on base), and a run only needs the step to pass. But a task whose repaired
step is its first, or its only, step would be scored as if it had to fail on the base. And the plan misstates the
spec it came from.

## Where

- `crates/roko-cli/src/plan_generate/source_checks.rs` around line 1384: the repair pushes
  `VerifyStep { …, expect: None }`.
- `SourceCheck` (same file, line 53) has `id`, `text`, `run` and `plan_level`, but no regression flag, so the repair
  can't know.

## Plan

- When lifting the source's checks, mark a check as a regression check when its own row says "A regression check" or
  "passes on the base", or the source's notes name it with `expect = "pass_on_base"` or as a regression check.
- The repair then writes `expect = "pass_on_base"` on the step it adds for such a check, and on a step it extends
  when every check that step runs is a regression check.

## Done when

A source whose table row calls a check "A regression check: it passes on the base too", planned without it, gets a
repaired step for that check with `expect = "pass_on_base"`. Checked by the test in `[[verify]]`.
