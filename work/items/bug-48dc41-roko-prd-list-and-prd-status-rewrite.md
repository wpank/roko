+++
id = "bug-48dc41"
kind = "bug"
title = "roko prd list and prd status rewrite tracked index files"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/prd"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/work-management/03-roko-native-capabilities.md"
discovered_from = "doc:tmp/work-management/03-roko-native-capabilities.md"
anchors = ["crates/roko-cli/src/main.rs::finish_with_index_rebuild", "crates/roko-cli/src/index.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

Every `roko prd ...` invocation, including read-only `list` and `status`, ends in `finish_with_index_rebuild(result, &wd, true)` (`main.rs:3802`), which rebuilds all indexes and rewrites `.roko/prd/INDEX.md`, the git-tracked `plans/INDEX.md` and others.
Inspecting PRDs therefore dirties the working tree.
Fix: rebuild only after mutating subcommands (as other groups do via `should_rebuild`) and test that `prd list` leaves files untouched.
