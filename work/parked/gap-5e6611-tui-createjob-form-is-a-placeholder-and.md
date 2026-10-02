+++
id = "gap-5e6611"
kind = "gap"
title = "TUI CreateJob form is a placeholder and PRD view has no inline editing"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-09-21
updated = 2026-10-02
source = "crates/roko-cli/src/surface_inventory.rs:871"
discovered_from = "audit:crates/roko-cli/src/surface_inventory.rs:871"
anchors = ["crates/roko-cli/src/surface_inventory.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
surface_inventory marks the CreateJob subview as Stub ('backend submission is not wired yet') and the PRD surface as read-only with inline PRD editing not wired; several other surfaces are Partial.

Imported without verification from:
- `crates/roko-cli/src/surface_inventory.rs:871`
- `crates/roko-cli/src/surface_inventory.rs:1354`
- `crates/roko-cli/src/surface_inventory.rs:717`

How to verify: Open the Jobs and PRD tabs in `roko dashboard`; list all SurfaceStatus::Stub/Partial entries.

2026-10-02 (roko-7d): The PRD half is moot: the TUI's Atelier tab (the PRD view) went with the PRD pipeline (merge bfd36512f). The CreateJob-form half still stands; still parked.
