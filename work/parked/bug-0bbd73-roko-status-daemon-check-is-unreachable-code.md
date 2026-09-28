+++
id = "bug-0bbd73"
kind = "bug"
title = "`roko status` daemon check is unreachable code"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/cli"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/39-ROADMAP.md#3.4 CLI Surface Fixes"
discovered_from = "audit:docs/v3/39-ROADMAP.md#3.4 CLI Surface Fixes"
anchors = ["roko status"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Roadmap §3.4: the status command's daemon check exists but is unreachable.

Imported without verification from:
- `docs/v3/39-ROADMAP.md#3.4 CLI Surface Fixes`

How to verify: Inspect status command control flow for the daemon branch.
