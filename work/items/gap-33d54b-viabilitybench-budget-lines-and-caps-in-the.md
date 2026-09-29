+++
id = "gap-33d54b"
kind = "gap"
title = "ViabilityBench budget lines and caps in the run ledger (S09.E2)"
status = "done"
triage = "verified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver", "benchmarks/viabilitybench/experiments"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "50ba68c08"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e12"
discovered_from = "tmp/cybernetic-harness/specs/S09-experiments.md (§4.6, §6 E2; checklist S09.E2); workstreams/assessment/W10-benchmarks-proof.md (circular budget gate, recs 2 and 7)"
anchors = ["benchmarks/viabilitybench/experiments/budget.toml", "benchmarks/viabilitybench/driver/ledger.py", "benchmarks/viabilitybench/driver/test_ledger.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = ["gap-28ebea"], blocks = [], related = ["gap-c33709", "gap-327242"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_dispatch_over_line_cap_is_refused' benchmarks/viabilitybench/driver/test_ledger.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_ledger.py -k test_dispatch_over_line_cap_is_refused -q"

[[verify]]
command = "grep -qw 'def test_reconcile_flags_drift_over_five_percent' benchmarks/viabilitybench/driver/test_ledger.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_ledger.py -k test_reconcile_flags_drift_over_five_percent -q"

[closed]
at = 2026-09-29
commit = "50ba68c08"
by = "wk-bench-ledger (commit trailer)"
evidence = "experiments/budget.toml holds S09 v1.2's budget lines (BL0-BL11 and BL13; caps sum to $390, BL12 held back for D43), the pilot's $15 experiment cap over PILOT-A and PILOT-B, and a $400 programme stop. The v1.2 rebalance (BL1 160, BL6 44, BL7 26, BL13 6) is marked confirm = \"lock\"; W10 rec 7's BL0 raise to $14 is recorded as proposed_cap_usd and is not in force. ledger.py reserves each attempt's worst case over every run in the results root and refuses a dispatch that would pass a line, experiment or programme cap before any provider call; vb run stops before a task that could pass one. New: vb ledger report and vb ledger reconcile (per-day drift against a provider CSV, flagged above 5%, with zai and Moonshot exports and their inferred reasoning_in_output covered). Checks: both [[verify]] commands pass (-k test_dispatch_over_line_cap_is_refused: 2 passed, including an offline vb run whose stub server gets 0 requests; -k test_reconcile_flags_drift_over_five_percent: 1 passed), and the whole benchmarks/viabilitybench suite passes (221 passed, 1 skipped)."
+++

## Problem

The driver's ledger (gap-28ebea) records spend but enforces no budget. S09 §4.6 defines budget lines BL0–BL11,
whose caps total $390. Nothing yet refuses a dispatch that would take a line over its cap.

The checklist's gating is also circular (W10):

- S08.T6 and S08.T11 wait on a `budget` gate, defined as "the S09 ledger shows the item fits the remaining cap";
- that ledger is S09.E2, which can only be built after S08.T6.

## Why it matters

The pilot must stay within about $15, and the caps must bind before the first paid call. Both pilots depend on
this item.

## Where

- **New:** `benchmarks/viabilitybench/experiments/budget.toml` and `driver/test_ledger.py`. The paths follow D4.
- **Extended:** `driver/ledger.py`.
- **For comparison:** `scripts/dev_benchmark.py` has a ceiling for a whole run, but no per-line reservation.

## Current state

Checked at `41c7ffbd6`: nothing exists, and no money has been spent.

## Plan

1. **`budget.toml`.** Lines BL0–BL11, with the planned amounts and caps from S09 §4.6. Add:
   - BL0's cap raised from $10 to $14, to pay for Pilot B's seeds 2–3 (W10 rec 7), if the author agrees;
   - an experiment-level cap that holds the whole pilot, BL0 plus the five BL8 runs, to $15;
   - a stop for the programme as a whole.
2. **Reservation.**
   - Before every dispatch, reserve the attempt's worst-case cost: its token caps priced at snapshot rates.
   - Refuse the dispatch if spent + reserved + worst case would exceed the line's cap or the experiment's cap.
   - Release the unused reservation after the attempt.
3. **Ledger rows** carry `line`, `attempt_key`, `reserved_usd` and `price_snapshot_id`.
4. **`vb ledger report`** shows spent, reserved and cap for each line.
5. **`vb ledger reconcile --provider cerebras --csv …`** compares the ledger with a provider's usage export. It
   flags any difference above 5% (S08 SC5).
6. **Move the gate.** Build items spend nothing, and only the two pilots (gap-c33709 and gap-327242) depend on this
   item. That replaces the circular gate.

## Done when

- [x] A dispatch that would exceed a line cap or the experiment cap is refused before any provider call.
- [x] Reconciliation flags a synthetic export that differs by more than 5%.
- [x] Both `[[verify]]` commands pass.

## Notes

- **Raising BL0 changes S09's totals.** With BL0 at $14, S09's caps sum to $394. That is still within its $400
  limit, and at least $100 stays unallocated (S09 SC3). S09 §7.3's "Σ caps = $390" then needs a one-line
  amendment.
- **Order:** this item extends gap-28ebea's `ledger.py`, so it runs after it. No hot files.
- **From wk-filer (2026-09-29):** reconcile should also cover zai and Moonshot exports; the price snapshot infers `reasoning_in_output` only for glm-4.7 and kimi-k2.6 (`config/prices/2026-09-28.toml:18-19`).
- **From gap-419298 (2026-09-29):** S09 v1.2 adds budget line BL13 for the plan-level slice ($5 planned, $6 cap). To keep the caps at $390, it lowers BL1 to 160, BL6 to 44 and BL7 to 26, and BL12 stays reserved for D43. The ledger and its caps must use the v1.2 list. The author confirms the rebalance at the lock.
