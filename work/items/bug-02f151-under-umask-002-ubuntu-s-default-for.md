+++
id = "bug-02f151"
kind = "bug"
title = "Under umask 002 (Ubuntu's default for a user with their own group) roko creates .roko group-writable with create_dir_all, then its own secure-open checks refuse it: the per-run event index and the runtime event log are never written, so serve's run routes find no runs (roko-serve's lib fails 37 tests under umask 002)"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "M"
subsystem = ["roko-fs"]
created = 2026-11-19
updated = 2026-11-19
last_verified = 2026-11-19
source = "roko-recovery while fixing bug-c6f163 (round v8, 2026-11-19): roko-serve lib under umask 002 on macOS"
discovered_from = "agent:roko-recovery (round v8, 2026-11-19)"
anchors = ["crates/roko-fs/src/layout.rs::ensure_dirs", "crates/roko-fs/src/log_rotation.rs::append_jsonl_line_sync", "crates/roko-runtime/src/jsonl_logger.rs::JsonlLogger", "crates/roko-cli/src/runner/persist.rs::PersistPaths"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-c6f163"], supersedes = [], duplicate_of = "" }
+++

## Problem

roko opens its event logs and per-run indexes without following symlinks, and refuses a directory or file that
another user could write. The checks are in `roko_fs::run_index` ("global log directory", "run index directory",
"run index") and `roko_runtime::jsonl_logger` ("runtime event log directory" and the others). The directories they
judge are `.roko` itself and its index directories.

The index directories are created private (mkdirat 0700). `.roko` is not: whichever code first needs a path under it
creates it with `create_dir_all`, at 0777 masked by the umask:
- `roko init`'s `RokoLayout::ensure_dirs`;
- a plan run's `PersistPaths::from_workdir`;
- `log_rotation::append_jsonl_line_sync` for `.roko/events.jsonl`;
- `JsonlLogger` for `.roko/runtime-events.jsonl`.

Under umask 002, Ubuntu's default for a user with their own group, `.roko` is 0775. roko's own checks then refuse it:
- the per-run event index is never written; its append fails as a non-fatal warning;
- the runtime event log can't open;
- serve's run routes find no runs (404s) or fail with 500s.

Seen on 2026-11-19 while fixing bug-c6f163, on macOS with `umask 002`: roko-serve's lib failed 37 of 1104 tests
(routes::runs: 32 of 38), with "run 'g1' not found" and "run index must be a single-link, user-owned regular file
without group/world write access". Under umask 022 all pass. CI's runner evidently uses 022, since those tests pass on
Linux.

## Why it matters

Goal core. On a common Linux setup, serve's run views, the portal's run pages and the Hermes plugin's run cards would
show no runs. The cause is a umask, and it shows only as a warning in the log.

## Where

- `crates/roko-fs/src/layout.rs::RokoLayout::ensure_dirs`.
- `crates/roko-fs/src/log_rotation.rs::append_jsonl_line_sync` and its relaxed sibling: the parent's creation.
- `crates/roko-runtime/src/jsonl_logger.rs::JsonlLogger::ensure_writer`: the parent's creation.
- `crates/roko-cli/src/runner/persist.rs::PersistPaths::from_workdir`.

## Plan

1. A roko-fs helper creates missing directories with mode 0700, whatever the umask. The places that create `.roko`,
   or the directories the checks judge, use it.
2. Decide what happens to a `.roko` an earlier roko made 0775: tighten it (it is roko's own), or keep refusing it with
   an error that names the path and the fix.
3. Red first: a child test process runs under `umask 002` and writes then reads a run's events through roko's own
   writer and serve's reader. Today it can't.

## Done when

Under umask 002, roko creates `.roko` private, and a run's events reach serve's routes. The `[[verify]]` runs that
under umask 002.
