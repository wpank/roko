+++
id = "gap-fab31c"
kind = "gap"
title = "Conductor supervisor ticks and thresholds are wired but its actions only log"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-conductor"]
created = 2026-08-31
updated = 2026-09-28
source = "gaps-md#partial-10/178"
anchors = ["crates/roko-conductor/src/adapter.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++

UX #178: the conductor supervisor tick and the watcher thresholds run, but diagnosed actions such as restart, escalate and pause are only logged, never executed. Backlogs #178 and #388 (self-healing conductor wiring) are archived without a status.

Fix: route conductor actions to the Graph execution control adapter, with a test for each action.
