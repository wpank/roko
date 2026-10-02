+++
id = "q-46a488"
kind = "question"
title = "WF-00/01: Remove the PRD idea/draft/publish pipeline in favor of plan-first?"
status = "done"
triage = "verified"
severity = "p2"
subsystem = ["roko-cli/prd"]
created = 2026-09-25
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "bfd36512f"
source = "tmp/workflow-audit/00-INDEX.md#Design Decisions (from user)"
discovered_from = "audit:tmp/workflow-audit/00-INDEX.md#Design Decisions (from user)"
anchors = ["CLAUDE.md#Self-hosting workflow", "crates/roko-cli/src/main.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-10-02
at_ts = "2026-10-02T20:40:14Z"
commit = "bfd36512f"
by = "roko-7d"
executor = "claude-session"
via = "manual"
forced = false
evidence = "Done 2026-10-02 at Will's request (session roko-7d, merge bfd36512f): the PRD pipeline is removed and plans are the only unit of work. `roko prd ...` and `roko do` exit 1 naming their replacement (`roko run --plan`, `roko plan generate`, `roko run`), `roko develop` is gone, the `/api/prds*` routes are gone (`POST /api/plans/generate` takes a prompt; a `slug` gets 422), `.roko/prd` is no longer created or read, an old `[prd]` config section loads with a warning, and CLAUDE.md, README and docs/v2/v3 describe `roko run` / `roko plan generate`."
+++
Workflow audit (2026-09-23) proposes deleting the PRD pipeline (~29 files/~8.2K LOC, 8 routes) and collapsing run/do/develop into `roko run`; by 09-25 it was ~80% executed in an uncommitted 227-file tree while CLAUDE.md still documents `roko prd` as wired.

Imported without verification from:
- `tmp/workflow-audit/00-INDEX.md#Design Decisions (from user)`
- `tmp/workflow-audit/01-PRD-REMOVAL.md#Scope`
- `tmp/workflow-audit/11-FINAL-STATUS.md#Headline Numbers`

How to verify: Check whether PRD removal is committed on main and CLAUDE.md/docs reflect it.
