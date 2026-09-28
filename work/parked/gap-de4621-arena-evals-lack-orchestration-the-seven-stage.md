+++
id = "gap-de4621"
kind = "gap"
title = "Arena evals lack orchestration, the seven-stage flywheel and on-chain settlement"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-chain/arena"]
created = 2026-09-28
updated = 2026-09-28
source = "gaps-md#recently-closed-epic-status/e40"
anchors = ["crates/roko-chain/src/arena.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++

R03 backs arenas with an authenticated, owner-aware, restart-safe service. Attempts bind external scoring evidence, and settlement, the leaderboard, prize/reputation effects and the event outbox commit atomically. Still missing: eval orchestration (actually running candidates), the seven-stage flywheel, token/on-chain settlement, and cross-arena transfer detection.

Fix: add eval orchestration that submits attempts to the existing service. Settlement adapters can come later.
