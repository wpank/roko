+++
id = "find-2b34c8"
kind = "finding"
title = "`roko run` / `roko do` silent delegation cleanup"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#6.9 `roko run` / `roko do` silent delegation cleanup"
discovered_from = "audit:tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#6.9 `roko run` / `roko do` silent delegation cleanup"
anchors = ["roko run", "roko do"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
`roko run` silently delegates to `roko do` based on flags. `roko do` classifies prompt intent via keyword matching. Single-word prompts match plan slugs on disk. Make routing explicit or move to graph templates.

Imported without verification from:
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#6.9 `roko run` / `roko do` silent delegation cleanup`

How to verify: Source: Engine audit report 04 (18 instances of silent delegation). Check the described code path for: `roko run` silently delegates to `roko do` based on flags. `roko do` classifies prompt intent via keyword matching. Single-word prompts match plan slugs on…
