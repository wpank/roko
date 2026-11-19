+++
id = "gap-2cc5be"
kind = "gap"
title = "GET /api/learn/loops lists β̂ without its interval, why β is not judged, or the arm counts, so the Loop Health list cannot show a loop's benefit being measured"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-serve/routes"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
source = "gap-1ee8c4 follow-up 2026-10-21"
discovered_from = "gap-1ee8c4"
anchors = ["crates/roko-serve/src/routes/learning/loops.rs::LoopRow", "crates/roko-serve/src/routes/learning/loops.rs::Audit::row"]
lane = "rust-cold"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = ["gap-1ee8c4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn loop_rows_carry_beta_interval' crates/roko-serve/ && cargo test -p roko-serve loop_rows_carry_beta_interval"
+++

## Problem

`GET /api/learn/loops` (and its showcase alias `/api/showcase/m2/loops`) gives one row per registered loop, which S10's
Loop Health view lists. Each row carries β̂ (`beta`, the latest health row's `beta.est`) and `n_opp`, but not β's
interval (`beta.lcb`, `beta.ucb`), why β is not judged yet (`beta.reason`), or the opportunities on each arm (`n_L`,
`n_D`). The ledger's health row has all of them, and since gap-1ee8c4 so do the `loop_health` hub event and the
`roko learn loops` table.

Found while doing gap-1ee8c4 (2026-10-21): `crates/roko-serve/src/routes/learning/loops.rs::Audit::row` fills `LoopRow`
from the latest health row and keeps only `beta.est`. `GET /api/learn/loops/{id}` returns the whole health row, so a
client that lists loops needs one more call per loop to show the interval.

## Why it matters

Goal `visibility`. In a young workspace β is not judged for most loops (S03's N_β is 200), so the list's `beta` is
`null` and says nothing; the reason (`n_below_n_beta`, `eps_below_min`) and the narrowing interval are what show that a
loop's benefit is being measured. The list route is the HTTP surface a dashboard reads first.

## Where

- `crates/roko-serve/src/routes/learning/loops.rs::LoopRow`: the row's fields.
- `crates/roko-serve/src/routes/learning/loops.rs::Audit::row`: fills them from the latest health row.

## Current state

`LoopRow` has `n_opp`, `eps`, `iota_net` and `beta`. The health row it reads has `n_L`, `n_D` and
`beta {est, lcb, ucb, reason}`.

## Plan

1. Add `beta_lcb`, `beta_ucb`, `beta_reason`, `n_learned` and `n_default` to `LoopRow`, named as the `loop_health`
   event names them (gap-1ee8c4), and fill them from the latest health row.
2. Test `loop_rows_carry_beta_interval`: a ledger whose latest L-know health row has a judged β gives a list row with
   the interval and the arm counts; a row whose β is not judged gives its reason.

## Done when

The `[[verify]]` command passes.

## Notes

Display only: no change to how β is judged.
