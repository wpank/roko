+++
id = "gap-0132c2"
kind = "gap"
title = "S01 S5.10 omits roko.fault/1's spend_cap_usd; C4 may pass vacuously at default settings once 5130 runs"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-learn/loop-audit"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-7 follow-up reports 2026-10-03 (PK42 gap-2b3c1b)"
discovered_from = "gap-2b3c1b"
anchors = ["crates/roko-learn/src/loop_audit/ledger.rs::FaultRecord", "crates/roko-learn/src/loop_audit/faults.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'spend_cap_usd' tmp/cybernetic-harness/specs/S01-instrumentation.md"
+++

## Problem

Three small spec/documentation gaps and one forward-looking statistical heads-up from PK42's loop-state-machine
audit (gap-2b3c1b, done):

1. **S01 §5.10 doesn't list `roko.fault/1`'s `spend_cap_usd` field.** The spec's field list for `roko.fault/1`
   (`tmp/cybernetic-harness/specs/S01-instrumentation.md:557-561`) is `{kind, event, fault_id, loop_id, fault,
   ttl_s, max_decisions, decisions_affected, actor, dry_run}` — no `spend_cap_usd`. The code has and uses it:
   `FaultRecord::spend_cap_usd` (`crates/roko-learn/src/loop_audit/ledger.rs:353`) is populated from
   `FaultSpec::spend_cap_usd` (`faults.rs:49`), defaulting to `HARMFUL_SPEND_CAP_USD` for `FaultKind::Harmful`
   (`faults.rs:125-126`).
2. **Nothing calls `faults::enable` outside a test, yet.** Confirmed: its only caller is
   `crates/roko-learn/src/loop_audit/canary.rs:507`, inside `#[cfg(feature = "fault-injection")] #[test] fn
   canary_localizes_injected_cut`. This isn't a bug — backlog task 5133 ("serve admin canary and fault routes,"
   S03 §5) is exactly the work that would give it a real, production caller (`POST
   /api/learn/loops/{id}/fault`). Recorded here so the dependency is explicit: `faults::enable` having no
   production caller is *expected* until 5133 lands, not evidence of dead code.
3. **A statistical heads-up for when 5130 lands.** PK42 reports: at default settings, C4's success criterion
   (S06 §... , "IAE of A3 < A0 on each of the five regulable kinds," `tmp/cybernetic-harness/specs/S06-*.md:58,273`)
   has an empirical-Bernstein confidence radius of roughly 2300/n, so no β rule (a significance threshold) fires
   within a 14-day evaluation window — C4 would pass "trivially" (vacuously, for lack of statistical power to
   fail it) rather than by genuine evidence. Backlog task 5130 ("E1: structural fault replay, 500 dry-run
   contexts") is the first task that will actually exercise C4 at volume, so this is the point where the issue
   becomes visible — not yet actionable before 5130 runs, but worth tracking so whoever picks up 5130 checks for
   it rather than being surprised by a vacuous pass.

Sub-finding **"S03 §3 still calls L-prompt-exp and L-model-exp mis-specified"** was checked separately and found
to be already accurate, not stale: `tmp/cybernetic-harness/specs/S03-*.md:94-95` (re-anchored at `e55d4c20f`,
the same commit this whole wave's premises are checked against) correctly classifies both as
`mis-specified`/`dormant:no_opportunity`. No action needed; not included as a finding here.

## Why it matters

(1) is a spec-completeness gap (same class as the config-schema-docs gaps tracked on gap-d2c64f/gap-fd9dfb in
earlier batches): the field exists and is used, the spec just doesn't say so. (2) prevents mistaking "no
production caller yet" for a defect when it's just sequencing. (3) heads off a predictable "C4 passed" false
signal before anyone relies on it.

## Where

- `tmp/cybernetic-harness/specs/S01-instrumentation.md` §5.10 (the `roko.fault/1` field list).
- `crates/roko-learn/src/loop_audit/ledger.rs::FaultRecord`, `crates/roko-learn/src/loop_audit/faults.rs` (the
  real field).
- `tmp/backlog/2026-10-02-complete-and-wire/5133-serve-admin-canary-and-fault-routes.md` (the dependency for (2)).
- `tmp/backlog/2026-10-02-complete-and-wire/5130-e1-structural-fault-replay-500-dry-run-contexts.md` (where (3)
  becomes visible) and `tmp/cybernetic-harness/specs/S06-*.md` (C4's definition).

## Current state

(1) and (3) unaddressed; (2) is expected, sequencing-only.

## Plan

1. Add `spend_cap_usd` to S01 §5.10's `roko.fault/1` field list.
2. When 5130 runs, check C4's actual pass/fail against the default settings' statistical power before trusting a
   pass; if it's vacuous as predicted, revisit the window length, the regulable-kind count, or the radius
   calculation before claiming C4 is met.

## Done when

- S01 §5.10 lists `spend_cap_usd`.
- Whoever runs 5130 has checked (and recorded) whether C4 passed on genuine statistical power or vacuously.
- The `[[verify]]` command passes.

## Notes

- (2) needs no fix — it's recorded only so `faults::enable`'s lack of a production caller isn't later mistaken
  for a regression.
