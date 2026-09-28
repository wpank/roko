+++
id = "gap-6bbf69"
kind = "gap"
title = "[cli-audit E34] Integrate witness/custody logging into the plan execution loop"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/safety"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/SUMMARY.md"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/SUMMARY.md"
anchors = ["roko-agent/src/safety/witness.rs WitnessLogger", "roko-agent/src/safety/provenance.rs CustodyLogger", "crates/roko-cli/src/graph_execution/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
TaintTracker, CustodyLogger, WitnessDag exist with tests, but full integration of witness/custody logging into plan execution is called out as an E34 product residual (runner wording predates Graph cutover).

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/SUMMARY.md`
- `tmp/archive/cli-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md`

How to verify: grep graph_execution/ and roko-graph for WitnessLogger/CustodyLogger usage.
