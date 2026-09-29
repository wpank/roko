+++
id = "gap-700314"
kind = "gap"
title = "Enrichment pipeline: learned step selection, parallel steps, per-step cost tracking"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-compose/enrichment"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/depth/06-composition/enrichment-pipeline-13-step.md:210"
discovered_from = "audit:docs/v3/depth/06-composition/enrichment-pipeline-13-step.md:210"
anchors = ["roko_compose::EnrichmentPipeline"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
The 13-step enrichment pipeline ships, but adaptive (learned) step selection, parallel step execution and per-step cost tracking are marked 'Not yet'.

Imported without verification from:
- `docs/v3/depth/06-composition/enrichment-pipeline-13-step.md:210`

How to verify: Inspect EnrichmentPipeline run loop for sequential execution and cost attribution.
