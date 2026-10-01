+++
id = "bug-730243"
kind = "bug"
title = "verified_outcome_drives_output_verdict_and_feedback flakes under load; concurrent JSONL appends can interleave"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/graph-dispatch", "roko-cli/tests"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:fix/dispatch-timeouts-cost e0673e3e0"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs::verified_outcome_drives_output_verdict_and_feedback", "crates/roko-cli/src/graph_task_dispatch/verification.rs::efficiency_records", "crates/roko-cli/src/graph_task_dispatch/tui_forward.rs::append_jsonl_line_async", "crates/roko-cli/src/graph_task_dispatch/verification.rs::GraphTaskDispatcher::settle_task_verification"]
links = { depends_on = [], blocks = [], related = ["bug-c34782", "bug-0b668a", "bug-ea9959"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn rows_appended_at_once_stay_whole' crates/roko-cli/src && cargo test -p roko-cli --lib rows_appended_at_once_stay_whole && cargo test -p roko-cli --lib verified_outcome_drives_output_verdict_and_feedback"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:58Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:12:09Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Already fixed at BASE ebdc0f5d5; the item's 2026-10-01 note gives the evidence."
+++

## Problem

`graph_task_dispatch::tests::verified_outcome_drives_output_verdict_and_feedback` fails intermittently in loaded full
runs (seen by the fix/dispatch-timeouts-cost worker, 2026-09-29) and passes alone. The test dispatches three attempts
and then waits for four efficiency records: `a0` success, `a1` success, `a1/gate-pass`, and `a2` failure. The wait uses
`efficiency_records`, which polls `efficiency.jsonl` for up to 60 s (600 × 100 ms), drops lines that do not parse, and
panics "efficiency records were not written".

A likely cause, from reading the code (not yet reproduced): every record is appended by a fire-and-forget
`tokio::spawn`, which calls `append_jsonl_line_async` and then `spawn_blocking`. That opens the file with `O_APPEND`
and calls `writeln!(file, "{line}")`, which makes two `write` calls: the line, then `"\n"`. Attempt `a1` spawns its
success record and its gate-pass record back to back, so two blocking threads append at once. If they interleave (A's
line, B's line, A's newline, B's newline), the result is one unparseable merged line and one empty line. The helper then
waits the full 60 s for a fourth record that never arrives. Load makes preemption between the two syscalls more likely.

## Why it matters

- The test is in the mandatory `cargo test --workspace` run, and `bug-c34782`'s verify command runs it too.
- If the cause is confirmed, production loses data the same way. The helper writes `.roko/learn/efficiency.jsonl`
  (:1884), `costs.jsonl` (:1935) and gate-failure records (:2710, :2823), and parallel tasks append to the same files.
  A torn line is silently dropped by every reader, so costs and learning signals are undercounted.

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs`:
  - the test `verified_outcome_drives_output_verdict_and_feedback` (:6142);
  - the helper `efficiency_records` (:5780);
  - `append_jsonl_line_async` (:3274, `writeln!` at :3284) and the sync `append_jsonl_line` (:3252, :3264);
  - the gate-pass record spawn (:3008-3022) and the `emit_feedback` efficiency spawn (:1878-1892).

## Current state

Checked statically at `33e107da1`. The 60 s deadline is already "generous" per its comment. A deadline alone cannot
explain a 60 s miss for a local append, and a torn line can. Nothing serializes appends to a path.

## Plan

1. Confirm the cause: make `efficiency_records` include the file's contents in its panic message, then run the test in
   a loop under load, for example beside `cargo build`.
2. Make each append a single write: format `"{line}\n"` into one buffer and call `write_all` once. That is atomic for
   `O_APPEND` on local files at these sizes. Alternatively, serialize appends per path through one writer task.
3. Make the test wait for the writes instead of polling. For example, have the dispatcher expose a flush or join of its
   pending feedback writes, or have the test drain them.
4. Add `concurrent_jsonl_appends_never_interleave`: many concurrent `append_jsonl_line_async` calls to one file must
   yield exactly that many parseable lines.

## Done when

- The new test passes.
- `verified_outcome_drives_output_verdict_and_feedback` passes repeatedly under load.
- The `[[verify]]` command passes.

## Notes

- The same `writeln!(file, "{line}")` append pattern exists elsewhere and may need the same treatment:
  - `roko-learn`: `run_metrics.rs:59`, `tool_metrics_store.rs:109`;
  - `roko-neuro`: `lifecycle.rs:1279`, `admission.rs:1095`;
  - `roko-agent`: `safety/witness.rs:270`, `safety/provenance.rs:289`;
  - `roko-core`: `forensic.rs:374`.

  Fix the Graph helper first; file the others if confirmed.
- `bug-0b668a` (closed) was an earlier flake of this kind: an unflushed `tokio::fs` write.
- 2026-10-01 (wk-honestbench): already fixed at BASE `ebdc0f5d5`, by `2c61e9053` (2026-09-30, for bug-779ae7).
  `append_jsonl_line_async` (`crates/roko-cli/src/graph_task_dispatch/tui_forward.rs:204`) writes each row and
  its newline in one `write_all` (:219), and `rows_appended_at_once_stay_whole` (:236) appends 64 rows at once
  and parses every line. All Graph feedback rows go through `background_writes::spawn`
  (`crates/roko-cli/src/background_writes.rs:30`), and `efficiency_records`
  (`graph_task_dispatch/verification.rs:1499`) awaits `background_writes::settled` (:1500) before it reads, so
  the test waits on the writes rather than a deadline. The `[[verify]]` now names that test instead of the
  planned `concurrent_jsonl_appends_never_interleave`. The sibling files listed above no longer contain a
  `writeln!` append.
