+++
id = "bug-9fc3fc"
kind = "bug"
title = "roko backlog audit reads the removed engine's state and ignores Graph checkpoints"
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
anchors = ["crates/roko-cli/src/commands/backlog.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

`backlog audit` compares plans on disk with the legacy `state-snapshot.json` (`commands/backlog.rs`), which the Graph engine no longer writes; it never reads `.roko/state/graph/<plan>/checkpoint.json`.
It also scans one directory level (nested `tasks.toml` files are invisible) and silently skips plans without `[meta] plan`; a local run reported 0 plans in the executor and 0 drift for 13 plans.
Fix: read Graph checkpoints, recurse like `plan validate`, and report skipped plans.
