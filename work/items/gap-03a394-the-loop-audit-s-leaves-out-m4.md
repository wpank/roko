+++
id = "gap-03a394"
kind = "gap"
title = "The loop audit's β leaves out M4's false-green penalty, so a loop that helps by weakening checks reads as a benefit"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "M"
subsystem = ["roko-learn/loop_audit"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "gap-03fcd0 follow-up 2026-10-16"
discovered_from = "gap-03fcd0"
anchors = ["crates/roko-learn/src/loop_audit/census.rs::Attempts::benefit", "crates/roko-learn/src/loop_audit/estimators.rs::BenefitRow"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-03fcd0", "spec-7a71d6", "gap-d76b1b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn census_beta_subtracts_audited_false_greens' crates/roko-learn/ && cargo test -p roko-learn census_beta_subtracts_audited_false_greens"
+++

## Problem

Since gap-03fcd0 the audit tick and `roko learn loops` estimate each loop's benefit β, but on U^pass only: a unit's
outcome is its first attempt's learning label (1 for a pass, 0 for a failure that is the agent's). S03 §4.4 defines
the outcome as U_t = 1[verdict_t = Passed] − (M_t/π_t)·fg_t, which subtracts the false greens M4's audit finds, at the
audit's logged selection probability π_t (floored at q). The census joins no audit result: every `BenefitRow` it
builds has `audit: None`, and `Tally::new` sets `audit_floor: 1.0` so the bounds fit outcomes in [0, 1]. So β is β_pass,
β_fg is 0 by construction, and a loop that raises its pass rate by weakening the checks (S03 §8's Goodhart risk) reads
as a benefit. β_cost also prices only the unit's first attempt, where S01's A-RES prices the chain (the sum over its
attempts, unknown when any attempt's is).

## Why it matters

Goal `cybernetic`: S03 §8 relies on the false-green term to stop a loop from "helping" by weakening tests, and E3/C5
expects a harmful loop to be demoted however it harms. It matters once M4 audits run by default (gap-d76b1b turns them
on at 10%). Related: gap-03fcd0 (production β), spec-7a71d6 (the epic), gap-d76b1b.

## Where

- `crates/roko-learn/src/loop_audit/census.rs::Attempts::benefit` builds each unit's `BenefitRow` (outcome, stratum,
  covariate, cost) from the run's verdict, open line and prediction rows; `Tally::new` sets the `BenefitConfig`.
- `crates/roko-learn/src/loop_audit/estimators.rs::BenefitRow::outcome` already applies the penalty when a row carries
  an `AuditDraw` (`selected`, `pi`, `false_green`), and its bounds follow `audit_floor`.
- M4's audit records live in roko-gate's vault (`crates/roko-gate/src/audit/ledger.rs`, `roko.audit/1`). roko-gate
  depends on roko-learn, so roko-learn cannot use roko-gate's types to read them.

## Current state

β, its confidence sequence at α/K, guard 2 and the live/harm/null rules work on U^pass (gap-03fcd0). Nothing joins an
audit result to a unit.

## Plan

1. Choose where the join lives: S05's `audit.selection` and `audit.result` (S01 §5.7) written into the run's files,
   or a reader of the vault's JSON lines in roko-learn.
2. Fill `BenefitRow.audit` with the unit's audit draw (π_t at least q) and its false-green label.
3. Use S03's q in the census's `BenefitConfig` whenever `[audit] enabled` is on: the bound must be fixed before the arm
   is drawn, so it cannot depend on whether this unit was audited.
4. Decide whether β_cost should price the chain (A-RES) or keep the first attempt, as β's outcome does, and document it.

## Done when

`census_beta_subtracts_audited_false_greens` passes: a loop whose learned arm passes more often only through false
greens that M4's audit finds gets a β whose interval lies below its β_pass's.

## Notes

- Keep gap-03fcd0's unit (each chain's first attempt) and its validity argument (S03 §4.5).
- Loop-audit `enforce` stays off; this item changes what is measured, not what runs.
