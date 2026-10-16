+++
id = "gap-6b7954"
kind = "gap"
title = "A flagged loop re-enters probation only after T_re: the audit never measures a version shift or a repair, so a flag from a passing state holds 14 days at h_suspect"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "M"
subsystem = ["roko-learn/loop_audit"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "gap-6f31c3"
discovered_from = "gap-6f31c3 (worker, 2026-10-16): L-sec reads empty state on a fresh workspace's first run"
anchors = ["crates/roko-learn/src/loop_audit/mod.rs::loop_evidence", "crates/roko-learn/src/loop_audit/state.rs::Auditor::rule", "crates/roko-learn/src/loop_audit/census.rs::Tally"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-6f31c3", "spec-7a71d6", "spec-c6e21b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn version_shift_readmits_a_flagged_loop' crates/roko-learn/ && cargo test -p roko-learn version_shift_readmits_a_flagged_loop"
+++

## Problem

S03 §4.6 lets a flagged or demoted loop back into probation when "T_re elapsed, or ι between state versions > 0.2, or
a recorded repair". The audit tick builds each loop's evidence in `crates/roko-learn/src/loop_audit/mod.rs::loop_evidence`,
which never sets `LoopEvidence::version_influence` or `LoopEvidence::repaired`; both keep their defaults. So
`state.rs::Auditor::rule` re-admits a loop only after T_re (14 days by default), however much its learned state
changed after the flag.

A flag raised by a passing state therefore holds for two weeks at h_suspect = 0.5, which withholds the loop on half
its chains instead of a fifth. Example since gap-6f31c3: in a fresh workspace the section bandit has learned nothing
when the first plan run starts, so L-sec's rows read empty state and, past N_ε learned-arm opportunities, the tick
flags it `dormant:cut`. The run saves the bandit's posteriors, and the next runs read them, but L-sec stays flagged
until T_re.

## Why it matters

Goal `cybernetic` (S03 §4.6). While enforce is off, the holdout rate is the auditor's one lever on what runs, and a
stale flag spends it on a loop that has recovered: the bandit learns only on its learned arm, so it learns at half the
rate for those two weeks. The same holds for any loop flagged `dormant:cut` or `write_only` before its state exists.

## Where

- `crates/roko-learn/src/loop_audit/mod.rs::loop_evidence` (`version_influence`, `repaired` left unset).
- `crates/roko-learn/src/loop_audit/state.rs::Auditor::rule` (the re-probation rule).
- `crates/roko-learn/src/loop_audit/census.rs::Tally` (no split of a loop's rows by state version).
- Decision rows carry the state each was made from: `state.version` and `state.digest` (S01 P0-10).

## Current state

The rule exists and is unit-tested with a hand-set `version_influence`; the census never measures it, and nothing
records a repair.

## Plan

1. Measure, per loop, the influence between its latest two state versions from the rows: for example the distance
   between the learned proposals of opportunities made from the newer state and from the older one, over matched
   contexts, or a cheaper proxy agreed with the S03 author (an empty state turning non-empty counts as a shift).
2. Fill `version_influence` in `loop_evidence`; record a repair (S03 §4.7's R1-R4) as `repaired`.
3. Test `version_shift_readmits_a_flagged_loop`: a loop flagged `dormant:cut` on empty state goes back to probation
   once its rows read a learned state, before T_re.

## Done when

The verify passes, and a loop flagged on empty state is re-admitted by the tick that sees its state learned.

## Notes

Measurement only; enforce stays off (D10). Found while closing gap-6f31c3.
