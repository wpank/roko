+++
id = "gap-5495d4"
kind = "gap"
title = "Kernel protocol convergence: two Cell contracts, Bus not a kernel trait, no distributed Bus backends"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core/protocol"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v2/02-CELL.md:7"
discovered_from = "audit:docs/v2/02-CELL.md:7"
anchors = ["roko_core Cell trait", "roko_graph Cell trait"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
v2 docs: roko-core and roko-graph keep distinct Cell execution contracts/registries; Phase 1 kernel-wide Pulse/Bus convergence incomplete (Bus not a kernel trait); distributed Bus backends and some economic/algebraic mechanisms later.

Imported without verification from:
- `docs/v2/02-CELL.md:7`
- `docs/v2/28-ROADMAP.md:62`
- `docs/v2/01-SIGNAL.md:4`
- `docs/v3/01-SIGNAL.md:7`

How to verify: Compare roko-core vs roko-graph Cell traits; check v3 02-CELL for current stance.
