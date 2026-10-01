+++
id = "bug-1cc498"
kind = "bug"
title = "The portal shows a task with no run record as passed when tasks.toml marks it done"
status = "done"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["apps/portal"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "3f022e70d"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-runstate's report on bug-54c729, branch work/bug-4e5a59 at ebf6d4313)"
anchors = ["apps/portal/src/lib/taskRows.ts", "crates/roko-cli/src/serve_runtime.rs"]
lane = "frontend"
parent = "spec-e9d7ec"
links = { depends_on = [], blocks = [], related = ["bug-4e5a59", "gap-cd3529"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'done without a run record' apps/portal/src/lib/taskRows.test.ts"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T08:35:40Z"
commit = "3f022e70d"
by = "commit trailer"
executor = "unknown"
size = "S"
claimed_at = "2026-10-01T08:23:09Z"
forced = false
evidence = "taskRows.ts gives a task that tasks.toml marks done, with no live run record, its own status (marked_done) and glyph state (marked: a faint ☑ labelled 'done without a run record') instead of passed; its dependants still do not wait on it, and a live record takes precedence. The item's verify passes; vitest for the whole portal 789/789, including the 'done without a run record' case, and tsc --noEmit passes."
+++

## Problem

`taskRows.ts` (about line 105) shows a task with no live run record as passed when its wire `completed` flag is set. That flag only means `tasks.toml` says `status = "done"` (`serve_runtime.rs` `task_to_dto`), which can be set without a verified pass (wk-runstate).

## Why it matters

Honest verdicts (epic spec-e9d7ec): only a verified pass is a pass.

## Plan

1. Show such a task as done without a run record (its own label and glyph), not as passed.
2. Add a `taskRows.test.ts` case named "done without a run record".

## Done when

- [ ] A task marked done in tasks.toml with no run record is not shown as passed.
- [ ] The `[[verify]]` command passes.

## Notes

- Fixed on `work/bug-4e5a59` at `3f022e70d`. The glyph reuses the faint `--state-pending` token, so no new CSS
  variable or contrast check was needed. It is neither the green of a verified pass nor the amber of a run that
  judged the task.
- The plan list has the same pattern one level up: `planRows.ts` `diskGlyphState` shows a plan whose disk summary
  says `completed` as `done` when there is no live state. Not changed here.
