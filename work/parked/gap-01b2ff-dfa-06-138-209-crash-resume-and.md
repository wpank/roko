+++
id = "gap-01b2ff"
kind = "gap"
title = "DFA-06 #138/#209: Crash/resume and provider proof matrices (runner hang after worktree edits, agent death before first event)"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-graph/resume"]
created = 2026-09-03
updated = 2026-09-28
source = "tmp/archive/dogfood-audit-2026-09-03/01-findings-register.md#Proof cases for the next dogfood cycle"
discovered_from = "audit:tmp/archive/dogfood-audit-2026-09-03/01-findings-register.md#Proof cases for the next dogfood cycle"
anchors = ["crates/roko-cli/tests/graph_crash_resume.rs", "tests/proof/provider-matrix.sh", "backlog #284"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Dogfood saw runners hang after agents edited files and wait forever on dead children; LostEffect/AgentSilence code exists but the end-to-end proof matrix (#138) and provider matrix (#209) stayed blocked on #284. P0-GE-1 added a 4-test crash/resume harness on 09-20.

Imported without verification from:
- `tmp/archive/dogfood-audit-2026-09-03/01-findings-register.md#Proof cases for the next dogfood cycle`
- `tmp/archive/dogfood-audit-2026-09-03/05-status-update-2026-09-01.md#15. #138/#209 — Runner Hang Proof Cases (Worktree Changes)`
- `tmp/archive/dogfood-audit-2026-09-03/06-status-update-2026-09-03.md#Remaining Open Work`
- `tmp/archive/dogfood-2026-08-26/DOGFOOD-DEBRIEF.md#F2: Runner hangs after agent completion (persistent)`
- `tmp/archive/dogfood-2026-08-26/DOGFOOD-DEBRIEF.md#F7: Agent process dies silently, runner waits forever`
- `tmp/dogfood/2026-09-19-session.md#2026-09-20 Continuation`

Warning: every file this item cites is gone (`crates/roko-cli/tests/graph_crash_resume.rs`, `tests/proof/provider-matrix.sh`) — likely obsolete or moved.

How to verify: Look for the proof-matrix tests/scripts; run a kill-before-first-event fixture on the Graph engine.
