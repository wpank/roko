+++
id = "gap-2e6afb"
kind = "gap"
title = "roko learn loops opens on a 10-column table only S03 explains and says nothing in plain words: on v7's Undertow workspace 4 of 20 loops are measured, none judged (beta needs 200 opportunities, the most measured has 101), and no line says so"
status = "open"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-learn/loop-audit", "roko-cli/commands"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
source = "round-v8 roko-learnguard survey ($V8/roko-learnguard/survey, roko a0089c19c)"
discovered_from = "agent:roko-learnguard (round v8)"
anchors = ["crates/roko-learn/src/loop_audit/census.rs", "crates/roko-cli/src/commands/learn_loops.rs"]
links = { depends_on = [], blocks = [], related = ["gap-2cc5be"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn learn_loops_leads_with_a_plain_summary' crates/ && cargo test -p roko-learn -p roko-cli learn_loops_leads_with_a_plain_summary"
+++

## Problem

`roko learn loops` opens on a 10-column table (state, reason, qualifiers, evidence, n_L/n_D, ε, ι_net, β [LCB, UCB])
that only S03 explains, and says nothing in plain words.

On v7's Undertow workspace (2026-11-12):
- every one of the 20 loops is on probation, flagged or retired, and no β is judged;
- L-know, L-play, L-sec and L-placebo are measured, at 67–101 opportunities each;
- β needs N_β opportunities on both arms first (`n_below_n_beta`).

A reader can't tell from it which loops are working, which are measured, and what it would take to judge one.

## Why it matters

- The guide's "how roko gets better" section rests on this census. Its honest answer is "measured, not yet judged",
  and the command should say that in one line, with the numbers.
- Goal: learning.

## Where

- `crates/roko-learn/src/loop_audit/census.rs`: the census rows and their text rendering.
- `crates/roko-cli/src/commands/learn.rs` and `crates/roko-cli/src/commands/learn_loops.rs`: the command.

## Plan

1. **A summary line before the table.** For example: "20 loops: 4 measured, 0 judged (β needs N_β = 200
   opportunities, both arms together; the most measured, L-know, has 101), 5 flagged dormant, 3 retired."
2. **Below the table, one line per measured loop,** in words: what it changes, how often it was exposed, and how many
   more opportunities until β is judged.
3. **The table and the JSON stay as they are.**
4. **Test:** `learn_loops_leads_with_a_plain_summary`.

## Done when

- The `[[verify]]` passes.
- On a copy of Undertow's workspace, the first line of `roko learn loops` says how many loops are measured and how
  far each is from being judged.

## Notes

- Found by roko-learnguard's round v8 survey (`$V8/findings/roko-learnguard.md`).
- Related: gap-2cc5be (the HTTP route's list leaves out the same β fields).
