+++
id = "gap-fbd580"
kind = "gap"
title = "PK90 Showcase and deploy: `ShowcaseLedger`: reserve, settle and freeze against global caps, with admin freeze… (+1 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
rank = 90
size = "M"
hold = "waits on Will's deferred decision(s) 9303 (tmp/backlog/2026-10-02-complete-and-wire/DECISIONS.md)"
subsystem = ["roko-serve/showcase"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK90"
anchors = ["crates/roko-serve/src", "crates/roko-serve/src/routes"]
lane = "rust-cold"
parent = "spec-0b3a32"
links = { depends_on = ["gap-f61823", "gap-08120e", "gap-9e3134", "gap-4119fb", "gap-fcb44c"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn ledger_reserve_respects_daily_total_and_concurrency' crates/roko-serve/src/ && cargo test -p roko-serve --lib ledger_"

[[verify]]
command = "grep -rqw 'fn unpriced_model_is_refused' crates/roko-serve/src/ && grep -rqw 'fn zero_cost_with_tokens_freezes' crates/roko-serve/src/ && cargo test -p roko-serve --lib unpriced_model_is_refused && cargo test -p roko-serve --lib zero_cost_with_tokens_freezes"
+++

## Problem

This package delivers 2 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK90, slice 93xx, phase 9), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9352 | M | p2 | `ShowcaseLedger`: reserve, settle and freeze against global caps, with admin freeze and caps routes (G6) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9352-showcase-ledger-reserve-settle-freeze.md` |
| 2 | 9353 | S | p2 | Showcase pricing fails closed: unpriced models are refused, and tokens with zero cost freeze the ledger | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9353-showcase-pricing-fails-closed.md` |

## Why it matters

Phase 9: domains, assistant, held and parked work, cleanup, showcase and deploy. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9300-showcase-deploy-and-release.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-serve/src/routes/showcase/admin.rs`, `crates/roko-serve/src/showcase/ledger.rs`, `crates/roko-serve/src/showcase/mod.rs`, `crates/roko-serve/src/showcase/pricing.rs`.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: PK10 (gap-f61823), PK12 (gap-08120e), PK13 (gap-9e3134), PK83 (gap-4119fb), PK85 (gap-fcb44c).
- On hold until Will takes the deferred decision(s) 9303 (spend or a public release); see `DECISIONS.md`.
- Suggested model: opus.
- 2026-10-04 (gate-13c follow-up, PK86 gap-3516d6, task 9337): confirmed at main HEAD
  `908f7ec40`. `deploy/showcase/preflight.sh`'s P9 check (`POST /api/showcase/admin/freeze`)
  SKIPs rather than FAILs today, because that route 404s — self-documented in the script's own
  header (lines 21-24: "They stop being skipped the day a route answers something other than
  404"). No fix needed in preflight.sh itself; once this item's admin-freeze route (9352) lands,
  P9 becomes a hard PASS/FAIL check automatically. Flagging so whoever implements 9352 knows
  `deploy/showcase/preflight.sh` is already watching for it.
