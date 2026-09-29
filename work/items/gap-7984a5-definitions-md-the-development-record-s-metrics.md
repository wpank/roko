+++
id = "gap-7984a5"
kind = "gap"
title = "DEFINITIONS.md: the development record's metrics, fixed in advance"
status = "done"
triage = "verified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["work/telemetry"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d6c7b1a1d"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e13"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W12-evidence-from-dev-process.md (A6, B1)"
anchors = ["work/telemetry/DEFINITIONS.md"]
lane = "tracker"
parent = "spec-f2463d"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f work/telemetry/DEFINITIONS.md && grep -q '^### First-try merge' work/telemetry/DEFINITIONS.md && grep -q '^### Unassisted merge share' work/telemetry/DEFINITIONS.md && grep -q '^### Cost per merged item' work/telemetry/DEFINITIONS.md && grep -q '^### The switch' work/telemetry/DEFINITIONS.md"

[closed]
at = 2026-09-29
commit = "d6c7b1a1d"
by = "wk-harvest (claude-agent)"
evidence = "d6c7b1a1d: work/telemetry/DEFINITIONS.md, Version 1 · 2026-09-29, committed on its own with Definitions-SHA256: ee0525db5ff09afe9f0b1509cb0d22162b795f2beae153cf859a98b999f86720. It has one section per metric (merged item, attempts, first-try merge, claim-to-merge hours, conflict rate, post-merge verify failure, escape, intervention, unassisted merge share, cost per merged item, coverage, the switch), each with its formula, the fields it reads and its exclusions, plus a change log. The [[verify]] passes."
+++

## Problem

The development record (epic spec-f2463d) will report first-try merges, interventions, escapes and cost per merged
item, but none of these is defined yet. Definitions written after the data arrives can be tuned to the results, and a
reader cannot tell.

## Why it matters

Goal `proof`. Fixing the definitions first makes the whitepaper's development-process figures credible. It also fixes
the before/after comparison for the switch to Roko-executed items (W12 B1) while no Roko-executed item exists.

## Where

**New:** `work/telemetry/DEFINITIONS.md`, versioned and dated. The rollup (gap-ccb87e) prints its sha256 on every
report.

## Current state

Checked at `41c7ffbd6`: no such file. `field_rollup.py` defines run-level metrics for Roko plan runs; these are
item-level metrics for backlog work and must not reuse the name "autonomy index" (S01 addendum).

## Plan

Write one `###` section per metric, each with its formula, the event fields it reads and its exclusions:
- **First-try merge:** one claim, and both the worker's verify and the post-merge verify pass.
- **Attempts:** claims per merged item.
- **Claim-to-merge hours**, **conflict rate** and **post-merge verify failure**.
- **Escape:** within 14 days of the merge, the item's verify fails, or a `reg-*` item or a `Fixes:` commit names it.
- **Intervention:** a mechanical signal (a commit on the branch by someone other than its executor, `close --force`,
  a `decision-needed` release) or a field note with `--item`.
- **Unassisted merge share:** merged items with no intervention, over all merged items.
- **Cost per merged item:** all attempts, API-equivalent at the `prices-2026-09-28` snapshot; orchestration
  overhead reported separately.
- **Coverage:** the share of merged items with a full event trail. Every table states it.
- **The switch:** the before/after for Roko-executed items (W12 B1), stratified by kind × size × goal and labelled
  descriptive unless items are assigned by a seeded draw (W12 B4).

Head the file `Version 1 · 2026-09-29` and add a change log. Later changes bump the version and never rewrite old
entries.

## Done when

- [x] Every metric above has its own section.
- [x] The file is committed on its own, with `Definitions-SHA256: <hex>` in the commit message.
- [x] The `[[verify]]` command passes.

## Notes

- Docs only, and written before gap-ccb87e computes anything.
