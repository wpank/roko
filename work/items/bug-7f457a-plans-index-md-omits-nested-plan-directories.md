+++
id = "bug-7f457a"
kind = "bug"
title = "plans/INDEX.md omits nested plan directories"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/plan-index"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/work-management/02-artifact-inventory.md"
discovered_from = "doc:tmp/work-management/02-artifact-inventory.md"
anchors = ["crates/roko-cli/src/index.rs", "plans/INDEX.md", "crates/roko-cli/src/index.rs::collect_plan_index_entries"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = 'grep -q "portal-programme" plans/INDEX.md'

[[verify]]
command = 'grep -q "portal-programme" plans/INDEX.md'
+++

The generated `plans/INDEX.md` lists only top-level plan directories; the eight nested `plans/portal-programme/0N-*` plans are absent, while CI's `plan validate` walks `tasks.toml` recursively.
Fix: index plans recursively (grouped by parent directory) and keep `plan index --check` deterministic.

Verified 2026-09-28 (static check against 3d0ee4d02): collect_plan_index_entries (crates/roko-cli/src/index.rs:345-376) does a single std::fs::read_dir of plans/ and keeps only direct children containing tasks.toml, so the 12 nested plans/portal-programme/*/tasks.toml plans (01..09 incl. 03b/03c/04b, not 8) are never indexed; `grep portal-programme plans/INDEX.md` has no match, so the item's verify command still fails. index.rs last changed in 91b4745f8; no uncommitted edits.
