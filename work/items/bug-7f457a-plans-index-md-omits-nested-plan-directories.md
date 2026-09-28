+++
id = "bug-7f457a"
kind = "bug"
title = "plans/INDEX.md omits nested plan directories"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-cli/plan-index"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/work-management/02-artifact-inventory.md"
discovered_from = "doc:tmp/work-management/02-artifact-inventory.md"
anchors = ["crates/roko-cli/src/index.rs", "plans/INDEX.md"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = 'grep -q "portal-programme" plans/INDEX.md'

[[verify]]
command = 'grep -q "portal-programme" plans/INDEX.md'
+++

The generated `plans/INDEX.md` lists only top-level plan directories; the eight nested `plans/portal-programme/0N-*` plans are absent, while CI's `plan validate` walks `tasks.toml` recursively.
Fix: index plans recursively (grouped by parent directory) and keep `plan index --check` deterministic.
