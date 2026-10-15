+++
id = "reg-c7db5d"
kind = "regression"
title = "The effects ledger and the run's arm-set file append each JSONL row in one write, through roko_core::io::append_jsonl_line"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph-task-dispatch", "roko-cli/effects"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-2"
anchors = ["crates/roko-cli/src/effects_apply.rs::append_record", "crates/roko-cli/src/graph_task_dispatch/attempt.rs::append_line", "crates/roko-core/src/io.rs::append_jsonl_line"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-8417d9", "bug-779ae7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rqF --include='*.rs' 'writeln!(file, \"{line}\")' crates/ && grep -rqw 'fn concurrent_jsonl_appends_never_interleave' crates/roko-core/src/ && cargo test -p roko-core --lib concurrent_jsonl_appends_never_interleave"

[[verify]]
command = "grep -q 'append_jsonl' crates/roko-cli/src/effects_apply.rs && grep -q 'append_jsonl' crates/roko-cli/src/graph_task_dispatch/attempt.rs"
+++

## Problem

bug-8417d9 (done 2026-10-01, merged `e3df791b8`) moved every JSONL appender to one write per row, because
`writeln!(file, "{line}")` on a bare `File` issues two `write` calls (the row, then the newline), and two appenders
can interleave between them. Two appenders added after it closed went back to the two-write form:

- `crates/roko-cli/src/effects_apply.rs::append_record` (`:360-371`, added in `28b772f6c`, 2026-10-03) appends to
  `.roko/state/effects.jsonl`, the staged-effect ledger `roko effects` reads.
- `crates/roko-cli/src/graph_task_dispatch/attempt.rs::append_line` (`:229-240`, added in `4e463cb42`, 2026-10-04)
  appends `KeptArmSet` rows to the run's `arm-sets.jsonl` (`ARM_SETS_FILE`, `:57`) from `RunAttempts::keep`, once per
  chain's first arm draw.

bug-8417d9's own verify, `! grep -rqF --include='*.rs' 'writeln!(file, "{line}")' crates/ && …`, fails at HEAD on
exactly these two lines.

## Why it matters

Goal `truth`: one intact record per event. Parallel tasks of one run draw their chains' arm sets concurrently, so
two `keep` calls can append at once; `read_arm_sets` (`attempt.rs:220-226`) silently skips a line that does not
parse, so a resumed run redraws that chain's arms and its loop-audit arm assignment changes mid-run. The effects
ledger is rarely written concurrently (an approve and an apply at once), but a torn row there loses an approval
record.

## Where

- `crates/roko-cli/src/effects_apply.rs::append_record`: opens the ledger with `append(true)`, then `writeln!`.
- `crates/roko-cli/src/graph_task_dispatch/attempt.rs::append_line`: the same shape; its only caller is
  `RunAttempts::keep`.
- `crates/roko-core/src/io.rs::append_jsonl_line` (`:110`): the single-write helper (creates the parent directory,
  opens in append mode, one `write_all` of row plus newline); `append_jsonl` (`:128`) serializes and appends.

## Current state

The helper and its test `concurrent_jsonl_appends_never_interleave` (`crates/roko-core/src/io.rs:266`) exist. No CI
step or test stops a new two-write appender: bug-8417d9's grep runs only when someone runs its verify, and it
matches one spelling only (`writeln!(file, "{line}")`; `writeln!(f, "{json}")` would pass it).

## Plan

1. `effects_apply.rs::append_record`: replace the body with `roko_core::io::append_jsonl(&effect_records_path(workdir), record)`
   (or `append_jsonl_line` after `serde_json::to_string`), keeping its `std::io::Result<()>` signature.
2. `attempt.rs`: delete `append_line` and call `roko_core::io::append_jsonl_line(path, &line)` from `keep`.
3. Optional guard against the next one: a CI step running bug-8417d9's grep, or a small check that flags any
   `writeln!`/`write!` on a `File` opened with `.append(true)` in the same function. The grep is one line but catches
   one spelling; the check is broader but needs its own test. Either is a `.github/workflows/ci.yml` change.

Out of scope: `crates/roko-agent-server/src/state.rs::append_log_line_sync` (a plain-text sidecar log, not JSONL)
and `crates/roko-runtime/src/state_hub.rs:133` (a `BufWriter` flushed once per row; only a row over 8 KiB is split).

## Done when

Both `[[verify]]` commands pass: no `writeln!(file, "{line}")` appender is left in `crates/`, the roko-core
interleaving test passes, and both files call the shared helper.

## Notes

Reopens bug-8417d9 as a regression (decision 7, 2026-10-15). `graph_task_dispatch/` is a hot module (one agent per
file). Evidence: `tmp/status-quo/2026-10-15/backlog/B-top-level-and-regressions.md:163` and `:197-199`; re-checked at
b42578bab by agent F1.
