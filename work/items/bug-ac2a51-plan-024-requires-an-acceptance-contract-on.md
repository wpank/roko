+++
id = "bug-ac2a51"
kind = "bug"
title = "PLAN_024 requires an acceptance_contract on architecture-queue tasks while PLAN_046 always warns when one is present"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan-validate"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-5 follow-up reports 2026-10-02 (PK19 gap-de0b87)"
discovered_from = "gap-de0b87 (PLAN_024/PLAN_046, decision 3205)"
anchors = ["crates/roko-cli/src/plan_validate.rs::validate_architecture_queue_task"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn architecture_queue_task_with_acceptance_criteria_has_no_contract_warning' crates/roko-cli/ && cargo test -p roko-cli architecture_queue_task_with_acceptance_criteria_has_no_contract_warning"
+++

## Problem

`roko plan validate`'s two rules for an architecture-queue task's `acceptance_contract` contradict each other, so
such a task can never validate clean:

- `PLAN_024` (`crates/roko-cli/src/plan_validate.rs::validate_architecture_queue_task`, ~line 1248): an **error**
  when `task.acceptance_contract.is_none()` — "declares no typed acceptance_contract". An architecture-queue task
  is *required* to have one.
- `PLAN_046` (`crates/roko-cli/src/plan_validate.rs`, ~line 923-930): a **warning**, pushed unconditionally
  whenever `task.acceptance_contract` is `Some(..)` (`if let Some(contract_value) = &task.acceptance_contract`,
  no other condition) — "has an acceptance_contract, which is not enforced at run time; state its criteria in
  `acceptance` with verify `covers`, or pin a test with `[task.accept]`".

So an architecture-queue task either omits the contract (PLAN_024 error) or includes it (PLAN_046 warning,
unconditionally). There is no way to satisfy PLAN_024 and stay clean of PLAN_046; the two rules were never
reconciled after decision 3205 retired the contract evaluator. `crates/roko-gate/src/acceptance_contract.rs`'s own
module doc (line 8) confirms the retirement: "nothing evaluates evidence against it at run time, and `roko plan
validate` says so for every contract (PLAN_046)" — written before (or without updating) PLAN_024's requirement.

## Why it matters

`roko plan validate` is the gate plan authors run before trusting a plan. A rule set that cannot be satisfied for
an entire task category (architecture-queue) either trains authors to ignore warnings generally, or makes
"compliant" an unreachable state for that category — defeating the purpose of PLAN_024 and PLAN_046 both. Part of
the golden-path/"specs a cheap model can execute" work (gap-de0b87, done) that introduced PLAN_046 without
reconciling PLAN_024.

## Where

- `crates/roko-cli/src/plan_validate.rs::validate_architecture_queue_task` (PLAN_024, ~line 1248-1258).
- `crates/roko-cli/src/plan_validate.rs` (PLAN_046, ~line 923-935).
- `crates/roko-cli/tests/plan_validate.rs`: existing tests already pin PLAN_046 firing on a contract-bearing task
  (lines ~602-605, ~708-711, ~923-925, ~1037-1072) and PLAN_024 firing in its absence (line 737's rule list) — no
  existing test exercises an architecture-queue task that has *both* checked together.
- `crates/roko-gate/src/acceptance_contract.rs` (module doc, line 8: the retirement decision, 3205).

## Current state

Unreconciled. PLAN_024 still requires the retired `acceptance_contract` shape; PLAN_046 still warns on it
unconditionally. An architecture-queue task following PLAN_024's letter always gets PLAN_046's warning.

## Plan

One of:
1. Change PLAN_024 to require the *new* pattern PLAN_046's own message recommends (`acceptance` criteria with
   verify `covers`, or a pinned `[task.accept]` test) instead of a typed `acceptance_contract`, and drop the
   `acceptance_contract.is_none()` check.
2. Or, make PLAN_046 conditional: don't warn when the task *also* satisfies the newer `acceptance`/`covers`/
   `[task.accept]` pattern (i.e. the contract is redundant-but-harmless alongside real coverage), only warn when
   the contract is the task's *only* acceptance evidence.
3. Whichever is chosen, update `validate_architecture_queue_task`'s requirements list and add a test:
   an architecture-queue task with both forms of acceptance evidence should not warn/error on either rule.

## Done when

- An architecture-queue task can be written that passes `roko plan validate` with zero PLAN_024/PLAN_046
  diagnostics.
- The `[[verify]]` command passes.

## Notes

- Decision 3205 (contract evaluator retirement) and PLAN_046's wording are the source of truth for the *intended*
  replacement pattern; option 1 above likely matches that intent best.
- Don't weaken PLAN_024's other requirements (PLAN_020/021/022/023/025) — only its acceptance-contract clause.

## Progress

- bug-ac2a51: implemented at 23074c40a, options 1 and 2 together. PLAN_024 now asks an architecture-queue task for checked acceptance (criteria in `acceptance` that a verify step `covers`, or a pinned `[task.accept]` test), and PLAN_046 warns only when an `acceptance_contract` is the task's only acceptance. Option 1 alone was not enough: PLAN_025 still reads a packet's parity rows from its contract, so a clean packet keeps one. The contract's shape checks and PLAN_020-023, 025 and 026 are unchanged. The two architecture fixtures in `tests/plan_validate.rs` that expected the PLAN_046 warning now state covered criteria and expect no diagnostics, and the roko-gate module doc says when PLAN_046 fires. Side effect: `plans/archive/architecture-defi-critical-path`, which passes today with warnings, now fails PLAN_024 when validated by hand (CI skips `plans/archive/`; the other two archived queues already fail on PLAN_CONCURRENT_OVERLAP and PLAN_CONTEXT_SYMBOL). The verify's grep passes; cargo verification is deferred to the batch gate (`architecture_queue_task_with_acceptance_criteria_has_no_contract_warning`).
