+++
id = "bug-40de03"
kind = "bug"
title = "cells()/arm_metrics() group only by (arm, model), so cheap_direct_msa's mini-swe-agent runs pool into cheap_direct's cell"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/analysis"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-4 follow-up reports 2026-10-02 (PK21)"
discovered_from = "gap-1149aa (PK21's own package); arms/cheap_direct_msa.toml's header comment, which currently misdescribes this as already handled"
anchors = ["benchmarks/viabilitybench/analysis/metrics.py::cells", "benchmarks/viabilitybench/analysis/metrics.py::arm_metrics", "benchmarks/viabilitybench/arms/cheap_direct_msa.toml"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_cells_separates_harnesses_sharing_one_arm_id' benchmarks/viabilitybench/analysis/test_analysis.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_analysis.py -k test_cells_separates_harnesses_sharing_one_arm_id -q"
+++

## Problem

`benchmarks/viabilitybench/arms/cheap_direct_msa.toml` deliberately gives the mini-swe-agent harness the *same*
nominal arm id as the bash-only loop's arm: `[arm] id = "cheap_direct"` (its own header comment explains why: "the
same task as cheap_direct, gpt-oss-120b billed through the API, but run by the pinned mini-swe-agent... instead of
our own bash-only loop"). The two loops are distinguished only by `provenance.network_policy.harness` on each run
record (`"mini-swe-agent"` vs the bash loop's harness name) — confirmed by
`driver/test_run_msa.py::test_msa_runner_meters_through_the_proxy`, which asserts
`record["arm"] == "cheap_direct"` with the comment "the same nominal arm as mini_loop.py's".

The arm file's own comment then claims: "...so analysis/metrics.py never pools the two loops' runs as one cell's
duplicate." That claim is false as implemented. `analysis/metrics.py::cells()` groups purely by
`(row["arm"], row["model"])`:

```python
def cells(records): return sorted({(row["arm"], row["model"]) for row in with_models(records)
                                    if row["task"]["family"] != PLAN_SLICE}, key=...)
```

and `arm_metrics()` filters rows by `Clause("arm", "==", arm)` (plus `model` when given) — neither function reads or
partitions by `provenance.network_policy.harness`, or by anything else that would separate the two harnesses. Since
both loops ultimately run `gpt-oss-120b` under `arm == "cheap_direct"`, every `cheap_direct_msa.toml` run lands in
the exact same `("cheap_direct", "gpt-oss-120b")` cell as the bash-loop's runs, and `arm_metrics()` computes one
pooled VS rate, $/VS, pass^k and false-green count across both — there is no way to query "the mini-swe-agent loop's
numbers" versus "the bash loop's numbers" separately through either function.

`test_analysis.py:433` pins `cells()`'s 2-tuple `(arm, model)` return shape (`assert metrics.cells(records) ==
[("cheap_direct", "glm-4.7"), ("cheap_direct", "gpt-oss-120b")]`), so a fix that adds a third element changes an
existing, asserted contract — call this out for whoever picks it up.

## Why it matters

Pilot B's optional "mini-swe" block (`experiments/pilot_b.toml`, `arm = "cheap_direct_msa"`) exists specifically to
compare the bash-only direct loop against a well-known third-party harness (mini-swe-agent) on the same task/model.
If the analysis pools them into one cell, that comparison is silently impossible to make — the two direct loops
"can't be compared" (as reported), even though the data to compare them already sits side by side in
`records.jsonl`, distinguished by `provenance.network_policy.harness`.

## Where

- `benchmarks/viabilitybench/analysis/metrics.py::cells` — the grouping function, 2-tuple `(arm, model)`.
- `benchmarks/viabilitybench/analysis/metrics.py::arm_metrics` — filters by the same two fields.
- `benchmarks/viabilitybench/arms/cheap_direct_msa.toml` — the arm config whose header comment's claim about
  `analysis/metrics.py` is currently false.
- `benchmarks/viabilitybench/analysis/test_analysis.py:433` — pins the 2-tuple shape a fix must change knowingly.
- `benchmarks/viabilitybench/driver/test_run_msa.py:105` — confirms records really do carry
  `arm == "cheap_direct"` for MSA runs, with `provenance.network_policy.harness == "mini-swe-agent"` as the only
  distinguishing field (line 106).

## Current state

Unfixed. Both loops' records exist today with the harness field correctly set, so no backfill is needed — only the
grouping/analysis functions need to read it.

## Plan

1. Extend `cells()`'s key to `(arm, model, harness)` where more than one harness exists for an otherwise-identical
   `(arm, model)` pair, keeping it `(arm, model)` (2-tuple) when only one harness ran — or, simpler, always emit a
   3-tuple `(arm, model, harness)` and update every caller (`report.py::build`, `arm_metrics`) and
   `test_analysis.py:433`'s pinned assertion together in the same change, since they must change in lockstep.
2. `arm_metrics()` gains an optional `harness` filter clause alongside `arm`/`model`, defaulting to "all harnesses of
   this arm" for callers that don't care, and `cell_name()`/report rendering label the mini-swe-agent cell
   distinctly (e.g. `cheap_direct (mini-swe-agent)`).
3. Update `cheap_direct_msa.toml`'s header comment once this lands — its current claim is the thing that is wrong.

## Done when

- A records set containing both `cheap_direct` (bash loop) and `cheap_direct_msa` (mini-swe-agent, same nominal
  arm id) runs produces two distinguishable cells/metrics, not one pooled cell.
- The `[[verify]]` command passes.

## Notes

- Do not change `[arm] id = "cheap_direct"` in `cheap_direct_msa.toml` itself — that id is shared deliberately so
  both loops are billed/capped under the same line (S08 §4.9 decision 3); the fix belongs in the analysis layer,
  which already has the data it needs via `provenance.network_policy.harness`.
- `test_analysis.py:433` must change in the same commit as `cells()`, or the suite will fail on an intentional
  contract change rather than a regression.

## Progress

- Took the "simpler" option from the Plan: `cells()` always emits a 3-tuple `(arm, model, harness)`, never a 2-tuple;
  `harness_of()` (already existed, already used by `check_unique`'s dedup key) is now also threaded through
  `with_models()` (adds a `"harness"` key alongside `"model"`), `cell_name()` (new `harness=""` param, appends
  `[harness]` when non-empty) and `arm_metrics()` (new `harness=None` param; `None` pools every harness of the arm,
  matching every pre-fix caller's behavior; a caller that cares passes the harness its own `cells()` 3-tuple named).
- Updated every caller that destructures `cells()` or calls `cell_name()`/`arm_metrics()` on its result:
  `report.py::build` (3-tuple destructure, `harness=harness` into `arm_metrics`), `report.py::render` (its `cells`
  dict key now uses `metrics.cell_name(arm, model, metrics.harness_of(row))`, so two harnesses no longer collide
  under one report-dict key), `envelope.py::level_table` (same pattern). Left `report.py::render`'s false-greens/
  excluded listing (`_identity`-based, no provenance data) and `econ.py`/`tab_t7_envelope.py` untouched — out of this
  task's anchors, cosmetic-only, no pooling/correctness issue.
- Running the full `analysis` suite (not just the named verify) surfaced two *other* pinned assertions that the new
  `harness == ""` clause in every non-`None`-harness `record_filter` broke:
  `test_metric_records_validate_and_carry_provenance` (line ~292) and `test_an_arm_with_two_models_is_reported_per_model`
  (line ~459) both pinned exact/prefix `record_filter` strings that predate the harness clause. Updated both to
  include `and harness == ""` in the right position (right after `arm ==`/`model ==`, before `task.family !=`) —
  an intentional contract change, same category as `test_analysis.py:433`/480/481, just not named in this item's
  anchors. Caught by running the suite, not by the named `[[verify]]` alone, per this wave's instruction.
- Added `test_cells_separates_harnesses_sharing_one_arm_id`: two `cheap_direct`-arm cells sharing one nominal arm id,
  distinguished only by `run_record(..., harness=...)` (new test-helper kwarg, sets
  `provenance.network_policy.harness`), asserts `cells()` returns two distinct 3-tuples, `arm_metrics(harness=...)`
  filters to each one's own runs only, `arm_metrics()` with no `harness` arg still pools both (back-compat), and
  `report.render()` prints them as two separate cells.
- Corrected `cheap_direct_msa.toml`'s header comment: it previously claimed metrics.py already kept the two loops
  apart; now it distinguishes `check_unique`'s (pre-existing) duplicate detection from `cells()`/`arm_metrics()`'s
  (this fix's) cell separation, both of which are now actually true.
- Verify: `grep -qw 'def test_cells_separates_harnesses_sharing_one_arm_id' .../test_analysis.py && .venv/bin/python
  -m pytest .../test_analysis.py -k test_cells_separates_harnesses_sharing_one_arm_id -q` -> 1 passed. Full
  `benchmarks/viabilitybench/analysis` suite (fresh venv, requirements.lock + requirements-analysis.lock): 105
  passed, 0 failed.
