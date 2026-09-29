+++
id = "q-46a488"
kind = "question"
title = "WF-00/01: Remove the PRD idea/draft/publish pipeline in favor of plan-first?"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/prd"]
created = 2026-09-25
updated = 2026-09-28
source = "tmp/workflow-audit/00-INDEX.md#Design Decisions (from user)"
discovered_from = "audit:tmp/workflow-audit/00-INDEX.md#Design Decisions (from user)"
anchors = ["CLAUDE.md#Self-hosting workflow", "crates/roko-cli/src/main.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Workflow audit (2026-09-23) proposes deleting the PRD pipeline (~29 files/~8.2K LOC, 8 routes) and collapsing run/do/develop into `roko run`; by 09-25 it was ~80% executed in an uncommitted 227-file tree while CLAUDE.md still documents `roko prd` as wired.

Imported without verification from:
- `tmp/workflow-audit/00-INDEX.md#Design Decisions (from user)`
- `tmp/workflow-audit/01-PRD-REMOVAL.md#Scope`
- `tmp/workflow-audit/11-FINAL-STATUS.md#Headline Numbers`

How to verify: Check whether PRD removal is committed on main and CLAUDE.md/docs reflect it.
