+++
id = "bug-c6f163"
kind = "bug"
title = "roko-serve's run index tests read an index straight from a temp dir, so the check judges the system temp dir as the global log's directory and refuses it on Linux (/tmp is root-owned, 1777): 9 routes::runs tests fail with 'global log directory must be user-owned and not group/world writable'"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-serve/routes"]
created = 2026-11-19
updated = 2026-11-19
last_verified = 2026-11-19
source = "round v8 lead: release PR #89's first --no-fail-fast run on Linux, 44611946697 (roko-serve lib)"
discovered_from = "agent:roko-recovery (round v8, 2026-11-19)"
anchors = ["crates/roko-serve/src/routes/runs.rs", "crates/roko-fs/src/run_index.rs::open_existing_run_index"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "TMPDIR=/tmp cargo test -p roko-serve --lib -- routes::runs::tests"
+++

## Problem

roko-serve's run index tests put a run's index file directly in a fresh temp directory (`tempdir()/run.jsonl`) and
read it with `read_index_page`. Production keeps an index at `<workspace>/.roko/events-by-run/<digest>.jsonl`, beside
the global log. The reader opens it without following symlinks, and checks two directories: the index's own, and the
global log's, which is its parent. Both must be user-owned and not writable by group or world.

In the tests, the global log's directory is therefore the system temp directory itself. On Linux CI that is `/tmp`,
root-owned and mode 1777, so the check refuses it, as it should, and the read is a 500: "global log directory must be
user-owned and not group/world writable". On macOS TMPDIR is a private 0700 directory, so the tests pass.

Seen on 2026-11-19 in release PR #89's Test + Clippy on Linux (run 44611946697), the only failing binary of the first
`--no-fail-fast` run: roko-serve's lib, 9 tests in routes::runs::tests, while 1,095 others passed.
- a_running_try_s_spend_reaches_the_run_summary
- a_settling_try_counts_once
- a_side_call_event_names_its_try_and_the_summary_keeps_its_cost
- cursor_pages_are_bounded_and_resume_at_line_boundary
- filtered_pagination_resumes_after_last_emitted_match
- mismatched_and_malformed_records_are_quarantined
- partial_tail_does_not_advance_the_external_cursor
- summary_reports_tasks_gates_and_terminal_metrics
- type_filter_and_secret_scrubbing_apply_before_response

The same 9 fail on macOS with `TMPDIR=/private/tmp` (root-owned, 1777), with the same error.

## Why it matters

Goal release. It is the last failing binary of release PR #89 on Linux. The check is right, and the tests build their
index in a place production never uses.

## Where

- `crates/roko-serve/src/routes/runs.rs`, the tests module: the 9 tests' `dir.path().join("run.jsonl")`.

## Plan

1. A test helper gives a run its index as production lays it out: `.roko/events-by-run/<digest>.jsonl`, at
   `roko_fs::run_index::run_index_path`. It creates both directories private (0700), whatever the umask.
2. The 9 tests use it. The check doesn't change.

## Done when

The 9 tests pass with TMPDIR at a shared, sticky, root-owned directory, and the `[[verify]]` passes.
