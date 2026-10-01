+++
id = "bug-6f7f72"
kind = "bug"
title = "gate-failures.jsonl cuts verify failure summaries at 200 characters, so a long verify command hides the failure message"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-gate", "roko-cli/graph-dispatch"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:fix/diagnose-graph-runs 05f8854ce"
anchors = ["crates/roko-gate/src/compile_errors.rs::GateFailureRecord::from_classification", "crates/roko-cli/src/graph_task_dispatch/verification.rs::settle_task_verification", "crates/roko-cli/src/commands/diagnose.rs::classify_recorded_failure"]
links = { depends_on = [], blocks = [], related = ["bug-165b22"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_long_verify_command_keeps_its_failure_message' crates/roko-cli/src && cargo test -p roko-cli --lib a_long_verify_command_keeps_its_failure_message"
+++

## Problem

Each Graph verify failure appends a `GateFailureRecord` to `.roko/learn/gate-failures.jsonl`. Its `summary` is the
first 200 characters of the failure text, and that text starts with the step label and the whole command:
``verify[1:test] (`<command>`): <fail message>\n<last 30 output lines>``. A verify command of 180 characters or
more (common: `grep -q … && cargo test -p … <filter>`) leaves no room, so the record keeps the label and part of
the command, and loses the failure message and every output line.

## Why it matters

`roko diagnose` (05f8854ce) reads these records to say why a task failed. It recovers the full command from
`tasks.toml`, but not the error, which is the part a person needs. The writer's comment also names triage and
threshold learning (#218), but today `roko diagnose` is the only reader in the workspace.

## Where

- `crates/roko-gate/src/compile_errors.rs::GateFailureRecord::from_classification` (:833-865): when the
  classification's `summary` is empty, the summary is `raw_excerpt.chars().take(200)`. `classify_gate_failure`
  (:491) always returns an empty `summary` (:590).
- `crates/roko-cli/src/graph_task_dispatch.rs::settle_task_verification`: failure strings are built at :2280
  (``"{step_label} (`{cmd}`): {fail_msg}\n{detail_snippet}"``), and the record is written at :2809-2830 with no
  `with_summary`.
- `crates/roko-cli/src/commands/diagnose.rs`: `GateFailureInfo.summary` is documented as "cut at 200 characters"
  (:305), `verify_step` parses the leading `verify[<i>:<phase>]` label (:912), and `classify_recorded_failure`
  truncates to 200 again (:945).
- The Runner-v2-era writer in `crates/roko-cli/src/runner/gate_dispatch.rs` (:1586-1608) has the same fallback.

## Current state

Checked at 33e107da1 by reading the code above. No test covers a long command.

## Plan

1. In the Graph writer, give the classification a summary that keeps the step label first (diagnose parses it),
   then the failure message and the output tail, and leaves the command out, since diagnose reads it from
   `tasks.toml`. Bound it with the `head_and_tail` helper that episodes use (`graph_task_dispatch.rs:836`,
   2 KiB), not a 200-character prefix.
2. Raise the `from_classification` fallback bound the same way, so other writers stop losing the message.
3. Update diagnose: the `summary` doc comment, and the 200-character truncate in `classify_recorded_failure`.
4. Add `a_long_verify_command_keeps_its_failure_message` (roko-cli lib). It fails a step whose command is over
   250 characters with a distinctive message, then asserts that the written record's summary starts with the
   step label and contains the message.

## Done when

- A long verify command's record keeps its failure message, and `roko diagnose` shows it.
- The `[[verify]]` command passes.

## Notes

- 2026-10-01 (wk-honestbench): implemented on work/bug-730243; cargo verification deferred to the batch check.
  The Graph writer (`graph_task_dispatch/verification.rs`, `settle_task_verification`) sets the record's summary
  with `failed_step_summary`: the failed step's label, its `fail_msg` or how it ended, then its output, without
  the command, kept to 2 KiB by `turn_policy::head_and_tail`. `GateFailureRecord::from_classification` falls back to
  the whole `raw_excerpt` (up to 2,000 characters) instead of 200 characters. In diagnose,
  `classify_recorded_failure` reads the summary's first line, and the `summary` doc comment describes the new form.
  Test: `a_long_verify_command_keeps_its_failure_message`.
