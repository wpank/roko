+++
id = "spec-007f1b"
kind = "spec"
title = "Pheromone target design + permissioned subnets (Subnet scope, partitioning, publishing gate)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-core/pheromones"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/depth/16-coordination/04-digital-pheromones.md:379"
discovered_from = "audit:docs/v3/depth/16-coordination/04-digital-pheromones.md:379"
anchors = ["roko_core::PheromoneScope"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Pheromone Substrate/Bus is wired, but bucketed decay, SINR-adjusted sensing and the interference matrix remain target design; PheromoneScope::Subnet, per-subnet morphogenetic partitioning and publishing-gate enforcement are specified but not wired.

Imported without verification from:
- `docs/v3/depth/16-coordination/04-digital-pheromones.md:379`
- `docs/v3/depth/16-coordination/09-permissioned-subnets.md:305`

How to verify: Check PheromoneScope variants and decay implementation.
