+++
id = "bug-bfd4ca"
kind = "bug"
title = "Knowledge endpoint URL mismatch (/neuro/ vs /knowledge/)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-serve"]
created = 2026-08-13
updated = 2026-09-28
source = "tmp/archive/08-15-26/MASTER-TASKS.md#2. Runtime Bugs (#12)"
discovered_from = "audit:tmp/archive/08-15-26/MASTER-TASKS.md#2. Runtime Bugs (#12)"
anchors = ["/neuro/", "/knowledge/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
A client uses the `/neuro/` HTTP path while roko-serve exposes `/knowledge/` routes (runtime bug #12 from dogfood).

Imported without verification from:
- `tmp/archive/08-15-26/MASTER-TASKS.md#2. Runtime Bugs (#12)`

Warning: every file this item cites is gone (`/knowledge/`, `/neuro/`) — likely obsolete or moved.

How to verify: grep for "/neuro/" in crates/ and demo/ clients; confirm roko-serve knowledge routes match callers.
