+++
id = "gap-bca779"
kind = "gap"
title = "Agent temperament only partially propagates (gate strictness, tool selection, review depth)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/temperament"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/depth/05-agent/10-temperament-profiling.md:3"
discovered_from = "audit:docs/v3/depth/05-agent/10-temperament-profiling.md:3"
anchors = ["roko_core::AgentConfig temperament"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Temperament enum/config exist in AgentConfig and influence CascadeRouter thresholds/exploration; propagation to gate strictness, tool selection and review depth is 'partially wired'.

Imported without verification from:
- `docs/v3/depth/05-agent/10-temperament-profiling.md:3`
- `docs/v3/depth/05-agent/temperament-profiling.md:3`

How to verify: grep temperament uses across roko-gate/roko-agent tool selection.
