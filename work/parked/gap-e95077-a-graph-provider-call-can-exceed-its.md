+++
id = "gap-e95077"
kind = "gap"
title = "A Graph provider call can exceed its budget reservation (no pre-call max-cost API)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-graph/cost"]
created = 2026-09-28
updated = 2026-09-28
source = "gaps-md#graph-engine-incomplete----partial"
anchors = ["crates/roko-graph/src/workspace.rs", "crates/roko-graph/src/engine.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++

The Graph plan-cost ledger reserves spend atomically before each call. The provider bridge has no enforceable pre-call maximum-cost API, so a single call can report more than its reservation once it completes. When `max_turn_usd` is unset, each call reserves all remaining capacity, so calls run one at a time and a budgeted plan loses its parallelism.

Fix: pass a per-call cost ceiling to providers that support one (max tokens derived from the remaining reservation). When `max_turn_usd` is unset, reserve a bounded per-call estimate instead of the full remainder.
