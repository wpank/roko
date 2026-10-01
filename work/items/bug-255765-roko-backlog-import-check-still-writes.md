+++
id = "bug-255765"
kind = "bug"
title = "roko backlog import --check still writes"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
subsystem = ["roko-cli/backlog"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d9e79e9d8"
source = "tmp/work-management/03-roko-native-capabilities.md"
discovered_from = "doc:tmp/work-management/03-roko-native-capabilities.md"
anchors = ["crates/roko-cli/src/commands/backlog.rs::cmd_backlog", "crates/roko-cli/src/commands/backlog.rs::cmd_backlog_import", "crates/roko-cli/src/main.rs:1739"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn backlog_import_check_writes_nothing' crates/roko-cli/ && cargo test -p roko-cli --bin roko backlog_import_check_writes_nothing"
+++

`--check` is declared as a dry run without side effects (`main.rs:1731-1733`), but the dispatcher destructures the import arguments with `..` (`commands/backlog.rs:28`) and never passes the flag on, so `backlog import --check` performs the import.
Fix: thread `check` into the importer, print the planned changes without writing, and test that no files change.

Re-verified 2026-09-29: unchanged. The --check flag declaration moved to main.rs:1739-1741.

## Notes

- 2026-10-01 (wk-taskdef): implemented on work/bug-e3df7d; cargo verification deferred to the batch check.
- The dispatcher passes `check` to `cmd_backlog_import`, which then prints each idea it would record (marking specs
  already imported) and returns before writing. `backlog_import_check_writes_nothing` snapshots the workspace before
  and after.
