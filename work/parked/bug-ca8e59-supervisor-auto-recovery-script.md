+++
id = "bug-ca8e59"
kind = "bug"
title = "Supervisor Auto-Recovery Script"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/198-supervisor-auto-recovery.md#198 — Supervisor Auto-Recovery Script"
discovered_from = "audit:tmp/backlog/archive/198-supervisor-auto-recovery.md#198 — Supervisor Auto-Recovery Script"
anchors = ["roko plan run plans/ --engine runner-v2", "roko-supervisor.sh", ".roko/state/supervisor.log", "roko run \"<prompt>\"", "WorkflowEngine", "roko plan run ... --resume-plan", "roko plan run"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
convenience for unattended long runs; operators must currently restart manually after crashes. Mori had `mori-supervisor.sh`: a self-healing wrapper that caught panics, read the crash report, fed it to Claude for auto-fix, rebuilt, and restarted. It had a circuit breaker (stop after 10 total…

Imported without verification from:
- `tmp/backlog/archive/198-supervisor-auto-recovery.md#198 — Supervisor Auto-Recovery Script`
- `tmp/backlog/_archive/_mori-old-gaps.md#MO-31`

Warning: every file this item cites is gone (`.roko/state/supervisor.log`) — likely obsolete or moved.

How to verify: Check: `roko-supervisor.sh` exists at the repo root and is executable (`chmod +x`); Running `./roko-supervisor.sh plans/` starts `roko plan run` and monitors its exit code; On non-zero exit, reads `.roko/state/crash-report.json` and feeds context… [evidence: 00-STATUS-SUMMARY 3. Open / P3 -- Low (Open): S | 7 |]
