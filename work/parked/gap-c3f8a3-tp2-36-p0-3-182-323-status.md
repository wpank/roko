+++
id = "gap-c3f8a3"
kind = "gap"
title = "TP2-36 P0.3 / #182/#323: status.json lacks PID and staleness semantics"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/status"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/tui-parity2/36-OPEN-GAPS.md#P0 — operator truth and control"
discovered_from = "audit:tmp/tui-parity2/36-OPEN-GAPS.md#P0 — operator truth and control"
anchors = [".roko/state/status.json", "runner/status_file.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Terminal status sources were unified, but status.json still lacks the requested PID, uses phase instead of current_phase, and kill/stale-process behavior has no post-change live fixture.

Imported without verification from:
- `tmp/tui-parity2/36-OPEN-GAPS.md#P0 — operator truth and control`
- `tmp/tui-parity2/34-BACKLOG-CROSSWALK.md#Partial`

How to verify: Inspect status.json schema after a Graph run and after SIGKILL.
