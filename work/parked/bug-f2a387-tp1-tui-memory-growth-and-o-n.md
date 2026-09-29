+++
id = "bug-f2a387"
kind = "bug"
title = "TP1: TUI memory growth and O(n) history buffers"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/tui-parity/00-INDEX.md#3. New items from the UX audit not in this tracker"
discovered_from = "audit:tmp/tui-parity/00-INDEX.md#3. New items from the UX audit not in this tracker"
anchors = ["IncrementalTailer", "TuiState"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
IncrementalTailer::items and several TuiState vectors grow unbounded in 24/7 monitoring, and history buffers use Vec::remove(0) instead of VecDeque.

Imported without verification from:
- `tmp/tui-parity/00-INDEX.md#3. New items from the UX audit not in this tracker`

How to verify: grep remove(0) and unbounded pushes in tui/.
