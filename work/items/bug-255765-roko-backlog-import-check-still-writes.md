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
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/work-management/03-roko-native-capabilities.md"
discovered_from = "doc:tmp/work-management/03-roko-native-capabilities.md"
anchors = ["crates/roko-cli/src/commands/backlog.rs:28"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

`--check` is declared as a dry run without side effects (`main.rs:1731-1733`), but the dispatcher destructures the import arguments with `..` (`commands/backlog.rs:28`) and never passes the flag on, so `backlog import --check` performs the import.
Fix: thread `check` into the importer, print the planned changes without writing, and test that no files change.
