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
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/work-management/03-roko-native-capabilities.md"
discovered_from = "doc:tmp/work-management/03-roko-native-capabilities.md"
anchors = ["crates/roko-cli/src/main.rs::finish_with_index_rebuild", "crates/roko-cli/src/main.rs:3829", "crates/roko-cli/src/index.rs::rebuild_all"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! sed -n '/Command::Prd { cmd } => {/,/^        }/p' crates/roko-cli/src/main.rs | grep -q 'finish_with_index_rebuild(result, &wd, true)'"
+++

Every `roko prd ...` invocation, including read-only `list` and `status`, ends in `finish_with_index_rebuild(result, &wd, true)` (`main.rs:3802`), which rebuilds all indexes and rewrites `.roko/prd/INDEX.md`, the git-tracked `plans/INDEX.md` and others.
Inspecting PRDs therefore dirties the working tree.
Fix: rebuild only after mutating subcommands (as other groups do via `should_rebuild`) and test that `prd list` leaves files untouched.

Rechecked 2026-09-29 at d9e79e9d8: still open. The unconditional rebuild call has moved to main.rs:3829. The Plan group's gating (PlanCmd::should_rebuild_indexes, main.rs:2302) is the pattern to copy for PrdCmd.
