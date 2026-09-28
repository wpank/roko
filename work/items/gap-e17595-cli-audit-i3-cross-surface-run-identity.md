+++
id = "gap-e17595"
kind = "gap"
title = "[cli-audit I3] Cross-surface run identity/terminal status agreement and reliable event delivery"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#Wave 3 integration node I3"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#Wave 3 integration node I3"
anchors = ["RuntimeEventEnvelope", "RunId (roko-core/src/run_id.rs)", "roko-serve/src/routes/runs.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Unverified: CLI text/JSON/TUI/HTTP/JSONL/snapshot projections agree on run/session/event identity, terminal status and usage; terminal/control/usage/custody events are not dropped under slow consumers or restart.

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#Wave 3 integration node I3`
- `tmp/archive/cli-audit-2026-09-21/IMPLEMENTATION-CHECKLIST.md#Final release checklist`

How to verify: Run a plan; compare run id/status/usage across plan status --json, TUI, /runs HTTP, JSONL log, snapshot.
