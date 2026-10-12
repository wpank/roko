+++
id = "bug-1c75cc"
kind = "bug"
title = "Plan generation discards a whole plan when a read_files line range ends one line past EOF (PLAN_CONTEXT_RANGE is not repaired like PLAN_CONTEXT_MISSING)"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-09
updated = 2026-10-12
last_verified = 2026-10-12
last_verified_rev = "24fafc69e"
source = "live hermes-plugin-roko e2e, 2026-10-09 (Hermes session 20261009_145635_15ac62)"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs", "crates/roko-cli/src/plan_policy.rs::validate_plan_context"]
links = { depends_on = [], blocks = [], related = ["find-8cc7ac"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn generated_plan_context_range_past_eof_is_clamped' crates/roko-cli/ && cargo test -p roko-cli generated_plan_context_range_past_eof_is_clamped"

[closed]
at = 2026-10-12
at_ts = "2026-10-12T12:13:50Z"
commit = "24fafc69e"
by = "commit trailer"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-12T12:00:09Z"
forced = false
evidence = "24fafc69e: the generation pipeline clamps a read_files range that starts inside its file and ends past it to start-<line count> (clamp_context_ranges_past_eof) before the context re-check, beside the PLAN_CONTEXT_MISSING repair; a range that starts past the end still fails; validate_plan_context stays strict for hand-written plans. Verify passes in the worktree: generated_plan_context_range_past_eof_is_clamped (1-17 on a 16-line file becomes 1-16, 20-25 stays refused); the plan-context tests pass; clippy -p roko-cli --no-deps -D warnings clean."
+++

## Problem

`plan generate` (and `plan_generate` over `/mcp`) throws away a whole generated plan when one task's
`[task.context] read_files` entry names a line range that ends past the end of the file. Observed live on
2026-10-09: a Hermes session asked roko's planner (claude-haiku via the Claude CLI) to add two functions to a
16-line `src/lib.rs`; the plan came back with `lines = "1-17"` and generation failed after its retries:

```
plan generation failed: no valid tasks.toml was produced.
Last error: generated plan violates execution context policy:
  - PLAN_CONTEXT_RANGE [T1]: line range `1-17` is outside `src/lib.rs` (16 lines)
```

The file ends with a newline, so a model that counts the empty line after it says 17. The second retry's
output also lacked a fenced toml block, so the whole operation failed. The host retried with a reworded
prompt and the next generation passed.

## Why it matters

Plan generation is the paid, first step of every plan a chat host (Hermes, OpenClaw) or `roko run --plan`
starts. An off-by-one in an optional context hint should not cost a whole generation. Related: the
generator already repairs `PLAN_CONTEXT_MISSING` (it drops the missing file) in the same function;
`find-8cc7ac` lists committed plans whose ranges run past EOF.

## Where

- `crates/roko-cli/src/plan_generate/pipeline.rs`: the context check after parsing. It drops
  `PLAN_CONTEXT_MISSING` read_files and re-checks, but returns any other violation as a hard error.
- `crates/roko-cli/src/plan_policy.rs::validate_plan_context`: raises `PLAN_CONTEXT_RANGE` when
  `resolved_end > line_count` (`content.lines().count()`).

## Current state

Unfixed at `856ed7570`. Reproduced only with a live planner; the check itself is deterministic.

## Plan

1. In the generation pipeline's repair step, also repair `PLAN_CONTEXT_RANGE` whose start is within the
   file: clamp the end to the file's line count (or drop `lines`, so the whole file is read when it is
   short). Leave a range that starts past EOF as an error.
2. Keep `validate_plan_context` strict for hand-written plans; the repair belongs to generation only.
3. Unit test: a generated plan whose read_file range ends one line past EOF is written with the range
   clamped.

## Done when

`generated_plan_context_range_past_eof_is_clamped` passes (the `[[verify]]` command), and a range that
starts past EOF still fails generation.

## Notes

Found through hermes-plugin-roko's live end-to-end test (Hermes 0.21.0 → roko serve `/mcp`).
