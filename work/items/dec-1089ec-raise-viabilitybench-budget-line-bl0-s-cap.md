+++
id = "dec-1089ec"
kind = "decision"
title = "Raise ViabilityBench budget line BL0's cap from $10 to $14 for Pilot B's seeds 2–3"
status = "open"
triage = "unverified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/experiments", "cybernetic-harness/specs"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-bench-ledger's report on gap-33d54b)"
anchors = ["benchmarks/viabilitybench/experiments/budget.toml", "tmp/cybernetic-harness/specs/S09-experiments.md"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-33d54b", "dec-39c781", "gap-327242", "gap-c33709"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -A4 'id = .BL0.' benchmarks/viabilitybench/experiments/budget.toml | grep -q 'cap_usd = 14'"
+++

## Problem

Budget line BL0 (the pilot: verifiers and cost accounting, F1 and F4) is planned at $6 with a $10 cap. That is its value in S09 §4.6 and in `experiments/budget.toml` (:22-26, merged by gap-33d54b in e43d3a033).

W10's recommendation 7 is to raise the cap to $14, so Pilot B can run seeds 2–3. gap-33d54b kept $10 until the author agrees.

## Why it matters

Pilot benchmark (epic spec-567e52): if the cap isn't raised, Pilot B's extra seeds can't run under BL0. The ledger enforces the caps, so the run would stop.

## Decision

The options:

- **(a) Approve.**
  - Set BL0's `cap_usd` to 14 in `budget.toml`.
  - Reconsider the pilot's experiment cap, currently $15 over BL0 plus BL8 (`budget.toml`:132-133).
  - Amend S09 §7.3's "Σ caps = $390" to $394. That stays within S09's $400 limit, and at least $100 stays unallocated (SC3).
- **(b) Keep $10.** Run Pilot B's seeds 2–3 under another line, or not at all.

## Where

`experiments/budget.toml` (the BL0 line and the pilot experiment cap). S09 §4.6 (the budget table) and §7.3 (the caps total) are in `tmp/`, so they are edited by hand.

## Done when

- [ ] Will decides. Under (a), `budget.toml` and S09 carry $14 and $394, and the `[[verify]]` command passes. Under (b), close this item as wontfix, with the decision as its evidence.

## Notes

- dec-39c781 (D28–D36) doesn't cover this line.
