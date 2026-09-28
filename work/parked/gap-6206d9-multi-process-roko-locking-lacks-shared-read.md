+++
id = "gap-6206d9"
kind = "gap"
title = "Multi-process .roko/ locking lacks shared/read-only locks"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/workspace_lock"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/39-ROADMAP.md#3.3 Half-Implemented Features"
discovered_from = "audit:docs/v3/39-ROADMAP.md#3.3 Half-Implemented Features"
anchors = ["crates/roko-cli/src/workspace_lock.rs", "workspace lock"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Multi-Process Locking lacks shared/read-only lock modes for concurrent .roko/ writers/readers (backlog #37). Listed under roadmap §3.3 half-implemented features.

Imported without verification from:
- `docs/v3/39-ROADMAP.md#3.3 Half-Implemented Features`
- `tmp/docs-audit/06-POTENTIAL-BACKLOG.md#C5. Multi-Process Shared Locks`
- `tmp/dogfood/2026-09-19-session.md#Dogfood Round 2`
- `tmp/dogfood/2026-09-18-session.md#Dogfood Round 4`
- `tmp/dogfood/2026-09-25-portal-programme-run.md#N-1. The workspace lock makes in-plan live-server verification impossible`

How to verify: Locate the scaffolding named in the roadmap row and confirm no runtime caller. / Inspect lock implementation for shared mode.

Merged 2 mined candidates: m5-075, m4-044.
