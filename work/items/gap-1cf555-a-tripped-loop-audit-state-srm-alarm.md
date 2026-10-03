+++
id = "gap-1cf555"
kind = "gap"
title = "A tripped loop-audit state (SRM alarm, placebo move) has no clear/acknowledge command"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-learn/loop-audit"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-8 follow-up reports 2026-10-03 (PK43 gap-c1d920)"
discovered_from = "gap-c1d920; checked against PK44/gap-85d176 (5132-5134), which does not own this"
anchors = ["crates/roko-learn/src/loop_audit/state.rs::Auditor", "crates/roko-learn/src/loop_audit/faults.rs::clear"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn operator_clears_a_tripped_loop_audit' crates/roko-learn/ && cargo test -p roko-learn operator_clears_a_tripped_loop_audit"
+++

## Problem

Once the loop-audit ledger is tripped (an SRM alarm, an ordering violation on a clean loop, or a placebo-loop
move — `crates/roko-learn/src/loop_audit/state.rs:17,308-311`, `LoopAuditor::tripped`,
`crates/roko-learn/src/loop_audit/mod.rs:494`), nothing clears it: a `clear` function exists only for the
unrelated fault-injection flags (`crates/roko-learn/src/loop_audit/faults.rs:171,308`, a different mechanism —
testing canaries, not audit integrity), and a repo-wide search for `clear`/`acknowledge`/`reset` across
`loop_audit/` finds no equivalent for the "tripped" state. Once tripped, the state persists in the ledger
indefinitely, freezing every loop's enforcement forever — even after whatever caused the trip is fixed.

Checked whether backlog tasks 5132-5134 (now PK44's package, `gap-85d176`, done) own this: they add `GET
/api/learn/loops` routes and loop SSE (5132), admin canary/fault routes (5133), and `roko learn loops
canary`/`fault` CLI subcommands (5134) — none of their task files mention "clear," "acknowledge" or "reset"
(confirmed: zero matches). They're about *exposing* and *triggering* loop-audit/fault state, not about resetting
a tripped audit. This gap is not covered by PK44 and needs its own tracking.

## Why it matters

Goal: cybernetic (M2 loop liveness). A one-way trip means a single false alarm (or a genuinely transient issue
that's since been fixed) permanently disables every loop's enforcement with no operator recourse short of
editing the ledger file by hand — the same class of "no command clears a persisted X" operator-UX gap as
gap-d90a93 (provider-health quarantine) from an earlier batch.

## Where

- `crates/roko-learn/src/loop_audit/state.rs::LoopAuditor::tripped` (sets it) and `mod.rs:494` (where it's
  checked/applied).
- `crates/roko-learn/src/loop_audit/faults.rs::clear` (the existing, unrelated fault-flag clear, as a reference
  for the *shape* of command an audit-trip clear would need, not its substance).

## Current state

No code path clears a tripped audit state. Confirmed not owned by PK44/5132-5134.

## Plan

1. Add a clear/acknowledge mechanism for the tripped state — likely an admin CLI command and/or REST route
   (mirroring `roko config provider reset-health`'s pattern from gap-d90a93, or `roko effects approve/reject`'s
   pattern for operator-gated state changes) that requires an explicit operator action, not an automatic timeout,
   since clearing a real integrity trip should be a deliberate decision.
2. Record who cleared it and when, same as other audit-adjacent actions in this codebase log their actor.

## Done when

- An operator can clear a tripped loop-audit state through a real command, with the action recorded.
- The `[[verify]]` command passes.

## Notes

- Discovered during PK43's work (gap-c1d920, done). Distinct from faults.rs's `clear`, which is for injected test
  faults, not audit integrity trips — don't conflate the two mechanisms when implementing this.
