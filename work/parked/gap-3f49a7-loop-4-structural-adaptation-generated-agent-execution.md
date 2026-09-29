+++
id = "gap-3f49a7"
kind = "gap"
title = "Loop 4 structural adaptation, generated-agent execution and continuous recursive-safety wrapping are not implemented"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-serve/meta-agents", "roko-graph/corrigibility"]
created = 2026-09-28
updated = 2026-09-28
source = "gaps-md#recently-closed-epic-status/r04"
anchors = ["crates/roko-serve/src/routes/meta.rs", "crates/roko-graph/src/cells/corrigibility.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++

R04 delivered an owner-scoped, restart-durable meta-agent lifecycle: proposal, validation, activation, morph, rollback and deactivation. Authority can never widen, and activation requires the five-head safety Graph plus single-use evaluation evidence. R04 explicitly does not implement:
- Loop 4 structural adaptation;
- ADAS/HGM-style autonomous generation;
- autonomous execution of generated agents;
- continuous recursive-safety monitoring around every Flow (backlog #09 is archived without a status).

Fix: roadmap. Start with continuous recursive-safety wrapping, because the other three depend on it.
