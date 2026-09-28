+++
id = "gap-e845b1"
kind = "gap"
title = "Named surfaces are only partly rendered and have no live Inbox, pending-human or autonomy sources"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-serve/surfaces", "roko-cli/tui"]
created = 2026-09-28
updated = 2026-09-28
source = "gaps-md#recently-closed-epic-status/e37"
anchors = ["crates/roko-serve/src/routes/projections.rs", "crates/roko-serve/src/projection_contract.rs", "crates/roko-cli/src/tui/tabs.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

E37's shared contract and backend are complete, but product rendering is only partial:
- The TUI does not render every named surface, and SurfaceEvents do not enter a command path.
- There is no production Inbox publisher or action consumer.
- `pending_human` has no live source, and autonomy config has no live store.
- Canvas uses dashboard plan IDs. Minimap uses a labelled deterministic layout instead of HDC coordinates.
- The five OpenAPI paths use generic JSON schemas.
- Replaying an unresolved Inbox receive event recomputes its receipt timestamp.

Fix: split this per surface when it is scheduled. Start with the Inbox publisher and consumer and typed OpenAPI schemas.
