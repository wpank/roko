+++
id = "bug-619253"
kind = "bug"
title = "Graph engine `--approval` fails before workspace lock (no interactive approval channel)"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/39-ROADMAP.md#7.10 Additional Deferred Items"
discovered_from = "audit:docs/v3/39-ROADMAP.md#7.10 Additional Deferred Items"
anchors = ["roko plan run --approval"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Roadmap §7.10: `--approval` fails before the workspace lock under the Graph engine; an interactive approval protocol for Graph runs is missing.

Imported without verification from:
- `docs/v3/39-ROADMAP.md#7.10 Additional Deferred Items`

How to verify: Run roko plan run <dir> --approval on a fixture plan.
