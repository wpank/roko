+++
id = "gap-5e6611"
kind = "gap"
title = "TUI CreateJob form is a placeholder and PRD view has no inline editing"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-09-21
updated = 2026-09-28
source = "crates/roko-cli/src/surface_inventory.rs:871"
discovered_from = "audit:crates/roko-cli/src/surface_inventory.rs:871"
anchors = ["crates/roko-cli/src/surface_inventory.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
surface_inventory marks the CreateJob subview as Stub ('backend submission is not wired yet') and the PRD surface as read-only with inline PRD editing not wired; several other surfaces are Partial.

Imported without verification from:
- `crates/roko-cli/src/surface_inventory.rs:871`
- `crates/roko-cli/src/surface_inventory.rs:1354`
- `crates/roko-cli/src/surface_inventory.rs:717`

How to verify: Open the Jobs and PRD tabs in `roko dashboard`; list all SurfaceStatus::Stub/Partial entries.
