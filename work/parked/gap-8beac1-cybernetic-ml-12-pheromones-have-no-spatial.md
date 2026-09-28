+++
id = "gap-8beac1"
kind = "gap"
title = "[cybernetic ML-12] Pheromones have no spatial propagation/diffusion"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-core/groups"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/cybernetic-audit/28-feedback-loops.md#ML-12: Pheromone Spatial Propagation / Diffusion"
discovered_from = "audit:tmp/archive/cybernetic-audit/28-feedback-loops.md#ML-12: Pheromone Spatial Propagation / Diffusion"
anchors = ["pheromone position_hint"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Pheromones are flat per-group; position_hint field has no spatial semantics. Should diffuse to adjacent groups or file-space positions to form gradient fields.

Imported without verification from:
- `tmp/archive/cybernetic-audit/28-feedback-loops.md#ML-12: Pheromone Spatial Propagation / Diffusion`

How to verify: grep position_hint usage and any diffusion logic.
