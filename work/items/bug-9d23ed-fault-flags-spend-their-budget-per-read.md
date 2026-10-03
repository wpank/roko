+++
id = "bug-9d23ed"
kind = "bug"
title = "Fault flags spend their budget per read site, tests share one process-wide registry, and L-route can never credit P7"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-learn/loop-audit"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-9 follow-up reports 2026-10-03 (PK44 gap-85d176)"
discovered_from = "gap-85d176"
anchors = ["crates/roko-learn/src/loop_audit/faults.rs::Registry", "crates/roko-learn/src/loop_audit/canary.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn fault_budget_counts_decisions_not_reads' crates/roko-learn/ && cargo test -p roko-learn fault_budget_counts_decisions_not_reads"
+++

## Problem

Three fault-injection/canary issues from PK44's work (gap-85d176, done):

1. **A fault flag's decision budget is spent once per read site, not once per logical decision.**
   `crates/roko-learn/src/loop_audit/faults.rs::Registry::active` (lines 164-175): every call increments
   `flags[index].decisions += 1` and expires the flag once `decisions >= max_decisions`
   (`FaultEvent::Hit`/`FaultEvent::Expired`). If one logical decision is checked from more than one call site
   (e.g. a route check and a downstream content-decision/census read of the same thing), each site's call counts
   as its own "hit" against the same budget — the flag can expire well before `max_decisions` *decisions* have
   actually happened, only after that many *reads* have.
2. **Feature-gated tests share one process-wide fault registry.** `faults.rs:106`: `static REGISTRY:
   Mutex<Option<Registry>> = Mutex::new(None)` — one static for the whole test binary. Reported: the two
   dry-run planner tests already have to take a lock to avoid interfering with each other's fault state, which
   is itself evidence of the sharing problem (a workaround, not a fix) — any other `#[cfg(feature =
   "fault-injection")]` test added later needs to know to do the same, or it will silently share state with
   whatever else is running.
3. **L-route's canary can never confirm the loop settled its outcome.** The canary's P7 probe
   (`canary.rs:41,326,333`: "P7: settle a synthetic outcome on the nonce's artifact... P7 credited: a synthetic
   settled outcome moved the counters") checks whether some *counter* moved as evidence of settlement. For
   L-route specifically, the loop's observable effect is a router *preference* (a choice among models/arms), not
   a counter — so P7 has nothing to check and `credited` is always `false` for this loop, regardless of whether
   L-route is actually healthy and settling outcomes correctly.

## Why it matters

Goal: cybernetic (fault-injection test infrastructure, M2 loop liveness canaries). (1) means a fault's declared
"last this many decisions" budget doesn't mean what it says whenever a decision is checked from more than one
site — tests relying on a specific decision count to trigger/expire a fault could be flakier or wronger than
they look. (2) is a latent test-isolation hazard. (3) means L-route's canary is structurally unable to pass P7,
so its wire-trace diagnostic always looks "cut" at that probe even when the loop works, undermining the
canary's whole purpose for this specific loop.

## Where

- `crates/roko-learn/src/loop_audit/faults.rs::Registry::active`, `::active_in` (1), `REGISTRY` (2).
- `crates/roko-learn/src/loop_audit/canary.rs` (P7 probe, 3).

## Current state

All three confirmed as described; none fixed.

## Plan

1. For (1): either de-duplicate hits per logical decision (track by a decision id, not per call), or document
   that `max_decisions` means "reads," not "decisions," if that's acceptable.
2. For (2): give fault-injection tests their own isolated registry instance (or document the locking requirement
   clearly so future tests don't skip it).
3. For (3): give P7 a loop-specific check for L-route (something that actually reflects a router preference
   having taken effect), or exempt L-route from P7 with a documented reason, rather than leaving it permanently
   uncreditable.

## Done when

- A fault's budget expires after the intended number of logical decisions, not reads.
- Fault-injection tests don't need an undocumented lock to avoid interfering with each other.
- L-route's canary can credit P7 when the loop is actually healthy.
- The `[[verify]]` command passes.

## Notes

- Discovered during PK44's work (gap-85d176, done).
