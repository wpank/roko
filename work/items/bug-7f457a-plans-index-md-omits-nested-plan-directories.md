+++
id = "bug-7f457a"
kind = "bug"
title = "plans/INDEX.md omits nested plan directories"
status = "superseded"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/plan-index"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/work-management/02-artifact-inventory.md"
discovered_from = "doc:tmp/work-management/02-artifact-inventory.md"
anchors = ["crates/roko-cli/src/index.rs::collect_plan_index_entries", "plans/INDEX.md"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "bug-5e7de4" }

[[repro]]
command = 'grep -q "portal-programme" plans/INDEX.md'

[[verify]]
command = 'grep -q "portal-programme" plans/INDEX.md'

[closed]
at = 2026-09-29
by = "work sweep 2026-09-29 (static check against HEAD; cargo verify not re-run while the portal plan run held the build lock)"
evidence = "Duplicate of bug-5e7de4 (older item; keep that ID). Still true: crates/roko-cli/src/index.rs:345-376 collect_plan_index_entries does a single read_dir of plans/ and keeps only direct children with tasks.toml, and plans/INDEX.md has no portal-programme entry. The same defect ('omits nested portal-programme/01-08') is one of the three defects in bug-5e7de4, which is older (created 2026-09-25, vs 2026-09-28). The one fix task, 03-tooling-fixes T02, closes both ids."
+++

The generated `plans/INDEX.md` lists only top-level plan directories; the eight nested `plans/portal-programme/0N-*` plans are absent, while CI's `plan validate` walks `tasks.toml` recursively.
Fix: index plans recursively (grouped by parent directory) and keep `plan index --check` deterministic.

Verified 2026-09-28 (static check against 3d0ee4d02): collect_plan_index_entries (crates/roko-cli/src/index.rs:345-376) does a single std::fs::read_dir of plans/ and keeps only direct children containing tasks.toml, so the 12 nested plans/portal-programme/*/tasks.toml plans (01..09 incl. 03b/03c/04b, not 8) are never indexed; `grep portal-programme plans/INDEX.md` has no match, so the item's verify command still fails. index.rs last changed in 91b4745f8; no uncommitted edits.
