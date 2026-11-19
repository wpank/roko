+++
id = "find-816b4b"
kind = "finding"
title = "When no rung clears break-even, cascade::start_rung falls back to the last rung even when its own forecast ranks that rung lowest (Undertow phase 0 run 2 T1: strong at P(VS) 0.11 against mid's 0.45); the fallback is left open: last rung, highest LCB, or lowest cost per verified success"
status = "done"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/self_model"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
last_verified_rev = "ad16cc844"
source = "round-v7-hermes-pilot#H-self-model"
discovered_from = "agent:hermes-pilot (round v7, self-model shadow review of Undertow phases 0-1)"
anchors = ["crates/roko-learn/src/self_model/cascade.rs::start_rung"]
links = { depends_on = [], blocks = [], related = ["bug-a35286", "gap-74893d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn start_rung_fallback_with_no_rung_at_break_even' crates/roko-learn/src && cargo test -p roko-learn start_rung_fallback_with_no_rung_at_break_even"

[closed]
at = 2026-10-20
at_ts = "2026-10-20T18:21:35Z"
commit = "ad16cc844"
by = "roko-speed"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-20T17:30:46Z"
forced = false
evidence = "ad16cc844. Decided: no fallback. The default start rule, expected_total (gap-74893d), starts on the rung with the lowest expected total to a verified pass, so there is no fallback left to choose; the last rung stands only when no total is finite (every P(VS) zero). break_even keeps S04's last-rung fallback, and start_rung's doc says so and why it is not the default. The literal verify passes on the branch: start_rung_fallback_with_no_rung_at_break_even, on phase 0 run 2's T1 forecasts (Undertow e578cf71, the exact q50 and q90 from predictions.jsonl): break_even falls back to strong (P(VS) 0.11); expected_total picks cheap by cost alone ($0.33 to a verified pass against mid's $0.50 and strong's $0.61) and strong at $3/h ($1.28 against $1.63 and $1.81: about nine 1.5-minute tries beat two 19-minute ones)."
+++

## Problem

When no rung clears break-even, `cascade::start_rung` returns the last rung, whatever its forecast. In Undertow's
phase 0 run 2 (`e578cf71`, 6 outcomes), the self-model would have started T1 on strong (gpt-5.4-mini) at P(VS) 0.11
(LCB 0.02). Mid stood at 0.45 (LCB 0.31) and cheap at 0.37 (LCB 0.18). The cancelled run `5ebd3310` got the same pick.
The fallback assumes the last rung is the most capable. Here the forecast ranks it lowest, because its only evidence
is two failures (bug-a35286).

## Why it matters

In active mode, this starts a task on the rung the self-model itself thinks least likely to pass. It is shadow-only on
Undertow today.

## Where

`crates/roko-learn/src/self_model/cascade.rs::start_rung`: `Some(start.unwrap_or(last))`.

## Plan

Choose the fallback. This is left open:
- the last rung, as now, the most capable by the ladder's construction;
- the rung with the highest P(VS) LCB;
- the rung with the lowest expected cost per verified success (`cost_to_vs`).

On phase 0 run 2's T1 forecasts they give strong, mid and cheap. The last uses the forecasts' q50 costs: cheap $0.013
/ 0.37, mid $0.122 / 0.45, strong $0.049 / 0.11.

## Done when

The fallback is decided, documented at `start_rung`, and tested with the T1 forecasts above. Checked by the test in
`[[verify]]`.

## Notes

- Evidence: `tmp/hermes-roko-v7/pilot/evidence/self-model-shadow-p0-p1.txt`, the rows "P0 cancelled" and "P0 run 2"
  T1.
