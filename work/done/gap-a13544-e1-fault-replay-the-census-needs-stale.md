+++
id = "gap-a13544"
kind = "gap"
title = "E1 fault replay: the census needs stale, degenerate and cut signatures first, task 5130"
status = "done"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-learn/loop-audit"]
created = 2026-10-03
updated = 2026-10-05
last_verified = 2026-10-05
last_verified_rev = "8e04833bb"
source = "tmp/backlog/2026-10-02-complete-and-wire 5130 (blocked in wave 9, PK44)"
discovered_from = "gap-85d176"
anchors = ["crates/roko-learn/src/loop_audit"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-85d176"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f crates/roko-cli/tests/loop_audit_faults.rs && grep -qw 'fn structural_faults_detected_and_localized' crates/roko-cli/tests/loop_audit_faults.rs && cargo test -p roko-cli --features fault-injection --test loop_audit_faults structural_faults_detected_and_localized"

[closed]
at = 2026-10-05
at_ts = "2026-10-05T18:58:20Z"
commit = "8e04833bb"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-05T15:53:44Z"
forced = false
evidence = "Gate 21 (merged 8e04833bb): verify structural_faults_detected_and_localized passes under fault-injection. The census gains STALE (decision rows' stale flag), DEGENERATE (all-flat scored content rows over N_eps) and CUT (readers record what they read) signatures; E1 replays 5 epochs x 100 dry-run contexts per case and detects and localises every fault with no collateral: L-know Cut and Stale at P2 (n_L 30), Unlogged, Degenerate and LabelOnly at P6, L-route Mask at P4; the healthy baseline flags nothing. Gate fixes 133fa6c32 (error_patterns rename) and 093df5573 (crate docs before the cfg)."
+++

## Problem

Task 5130 of PK44 (gap-85d176, gated in wave 9) is the E1 fault replay: inject each structural fault and check that
the loop auditor detects and localises it (S03's C2, time to detection). PK44's worker stopped before it because the
measured census doesn't yet produce the signatures E1's pass criteria need:
- STALE: decision rows never carry a stale read status;
- DEGENERATE: nothing measures a loop's degeneracy;
- CUT: an emptied reader makes the knowledge rows count as "no opportunity", so the loop is flagged that way, not as
  a cut;
- the integration test needs the content-row builder (`content_audit`), which is private.

## Why it matters

Without E1, M2's claim that it detects broken loops (and how fast) is untested, and `roko learn loops fault` can't
report time to detection.

## Where

`crates/roko-learn/src/loop_audit/` (census, state), the decision-row builders in
`crates/roko-cli/src/graph_task_dispatch/decision_log.rs`, PK44's fault hooks (`fault-injection` feature), and the
planned `crates/roko-cli/tests/loop_audit_faults.rs`.

## Current state

PK44's fault hooks, dry-run canaries, routes and CLI are merged; E1 isn't written.

## Plan

1. Decide each signature (a design choice per S03 §4.9): how a stale read is recorded on a row, what measures
   degeneracy, and how a cut reader is told apart from no opportunity.
2. Make the content-row builder reachable from the test, implement the signatures, then E1 as 5130 says.

## Done when

- [ ] The `[[verify]]` command passes.

## Notes

- Full spec: `tmp/backlog/2026-10-02-complete-and-wire/5130-*.md`.
- Left PK44's package item at gate 9b (2026-10-03).

## Progress

- 5130: implemented at 452a55484 (stale, degenerate and cut signatures in the census; E1 in
  `crates/roko-cli/tests/loop_audit_faults.rs`); cargo verification deferred to the batch gate.
