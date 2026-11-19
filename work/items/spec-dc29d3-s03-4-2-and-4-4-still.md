+++
id = "spec-dc29d3"
kind = "spec"
title = "S03 §4.2 and §4.4 still read L-sec's ι as Jaccard on draws netted of an A/A floor; since q-beea61 it is the bandit's exclusion odds against the floor policy"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/loop_audit"]
created = 2026-10-18
updated = 2026-10-18
last_verified = 2026-10-18
source = "q-beea61 (worker, 2026-10-18)"
discovered_from = "q-beea61 (closed at b2496bc06): the census measures L-sec on exclusion odds, S03 never updated"
anchors = ["tmp/cybernetic-harness/specs/S03-loop-liveness-audit.md"]
lane = "paper"
links = { depends_on = [], blocks = [], related = ["q-beea61", "gap-6f31c3"], supersedes = [], duplicate_of = "" }
+++

## Problem

S03 still describes L-sec's influence as it was before q-beea61. §4.2's registry table gives L-sec's action-level ι
as `Jaccard(incl_L, incl_D)` (`tmp/cybernetic-harness/specs/S03-loop-liveness-audit.md:165`), and §4.4 makes the
A/A floor from a second independent draw mandatory for every stochastic reader (lines 230-233). Since q-beea61 (Will's
option b, 2026-10-18; commit b2496bc06) the census measures L-sec's ι on the section bandit's exclusion odds against
the floor policy (p_ex at `SECTION_EXCLUSION_FLOOR` on every droppable section the bandit drew), with no A/A floor:
one draw of a policy that explores on purpose differs from a second as much as from π⁰, so the floor on draws reads
it inert whatever its posteriors say. The spec and the code now disagree about L-sec's ι.

## Why it matters

Goal `cybernetic` (S03 §4.4, §4.6). A reader who takes S03 as the definition of the `iota` fields on L-sec's
`loop.health` rows, or who re-implements the census from it, nets out an A/A floor on draws and gets L-sec `inert`
by construction, the problem q-beea61 settled. q-beea61's own option (c) ("say so in S03") shows the spec is meant to
say how an exploration loop is measured.

## Where

- `tmp/cybernetic-harness/specs/S03-loop-liveness-audit.md:165` (§4.2's table, L-sec's "Action-level ι").
- `tmp/cybernetic-harness/specs/S03-loop-liveness-audit.md:230-233` (§4.4, influence and the A/A floor).
- The shipped rule, read-only: `crates/roko-learn/src/loop_audit/census.rs::exclusion_policies`,
  `crates/roko-learn/src/loop_audit/exposure.rs::ExclusionOdds`, the writer
  `crates/roko-cli/src/graph_task_dispatch/decision_log.rs::section_proposals` (`proposals.exclusion` on every
  sections row, both arms), and L-sec's note in `crates/roko-learn/src/loop_audit/loops.toml`.

## Current state

The code, the registry note and q-beea61 (work/done) agree. S03 v1.1 says nothing of the change.

## Plan

1. §4.2, L-sec's row: action-level ι = the mean, over the droppable sections the bandit drew, of p_ex − p_floor (its
   exclusion odds against the floor policy), logged as `proposals.exclusion`.
2. §4.4: a policy that randomizes on purpose (the section bandit) is measured on its action probabilities against
   its floor policy, not on draws, and has no A/A floor there: its odds are what it executes and never fall below
   the floor, so their distance from the floor policy is linear in them. Cite q-beea61.

## Done when

S03 §4.2 and §4.4 state L-sec's ι as the census measures it.

## Notes

Spec text only (`tmp/cybernetic-harness/specs/`), filed as `kind = "spec"` with no `[[verify]]`, like spec-290672.
Found while closing q-beea61.
