+++
id = "bug-6c0fd7"
kind = "bug"
title = "`roko config set --global` silently ignored"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/cli"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/39-ROADMAP.md#3.4 CLI Surface Fixes"
discovered_from = "audit:docs/v3/39-ROADMAP.md#3.4 CLI Surface Fixes"
anchors = ["roko config set"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Roadmap §3.4: config set --global exists but is silently ignored (writes project config or nothing).

Imported without verification from:
- `docs/v3/39-ROADMAP.md#3.4 CLI Surface Fixes`

How to verify: Run roko config set --global x y and inspect ~/.config vs ./roko.toml.
