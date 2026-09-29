+++
id = "gap-b24517"
kind = "gap"
title = "ViabilityBench report: verified success, cost per verified success, pass^k and false greens with run ids (S08.T7)"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/analysis"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e12"
discovered_from = "tmp/cybernetic-harness/specs/S08-benchmark-suite.md (§4.12, §5.5, §6 T7; checklist S08.T7)"
anchors = ["benchmarks/viabilitybench/analysis/metrics.py", "benchmarks/viabilitybench/analysis/passk.py", "benchmarks/viabilitybench/analysis/report.py", "benchmarks/viabilitybench/analysis/test_analysis.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = ["gap-28ebea"], blocks = [], related = ["gap-d9e9fe"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_pass_k_matches_hand_computed_cases' benchmarks/viabilitybench/analysis/test_analysis.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_analysis.py -k test_pass_k_matches_hand_computed_cases -q"

[[verify]]
command = "grep -qw 'def test_check_rejects_simulated_or_incomplete_bundle' benchmarks/viabilitybench/analysis/test_analysis.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_analysis.py -k test_check_rejects_simulated_or_incomplete_bundle -q"
+++

## Problem

The pilot reports four numbers per arm, and no code computes them:

- the VS rate;
- the cost per verified success ($/VS);
- pass^3;
- the false greens (FG).

Each number needs the run ids behind it. Computed by hand in a notebook, the numbers would lose that provenance, and
nothing would stop someone publishing a bundle with missing runs or simulated rows.

## Why it matters

S08 §4.12 sets honesty rules for every published number:

- every MetricRecord lists its `run_ids`;
- an unknown label counts as 0, and the result with unknown labels counted as 1 is shown beside it;
- `infra_error` and `leak_suspected` runs are excluded, and their counts are reported.

The pilot items' verify commands use this item's `--check` mode.

## Where

All new, and the path follows D4: `benchmarks/viabilitybench/analysis/`, holding `metrics.py`, `passk.py`,
`report.py` and `test_analysis.py`. `vb report` calls `report.py`.

## Current state

Checked at `41c7ffbd6`: nothing exists. The formulas are in S05 §0 (VS, $/VS, pass^k and FG) and S09 §4.1.

## Plan

1. **`passk.py`.** The unbiased estimator C(c,k)/C(n,k) for each task, averaged over tasks and stratified by
   family × level.
2. **`metrics.py`.**
   - The VS rate.
   - $/VS = Σ `api_equiv_usd` / Σ VS. For CLI arms, U′ is the headline and R is shown beside it. `billed_usd` is
     reported separately.
   - FG: a reported pass with VS = 0. For direct and CLI arms, a reported pass means the visible checks pass on the
     final tree.
   - The counts of cap-censored and `infra_error` runs, per arm.
3. **`report.py --experiment X --out metrics.json`.** It writes `vb.metric_record/1` rows with `label_source`,
   `cost_basis`, `price_snapshot_id` and `run_ids`, and it lists every false green with its run id.
4. **`report.py --check <bundle>`.** It exits non-zero unless all of these hold:
   - every record validates, and none has `simulated: true`;
   - every MetricRecord has run ids;
   - the run counts match the experiment manifests in the bundle;
   - the ledger's spend is within every line's cap.

## Done when

- [ ] pass^k matches hand-computed cases.
- [ ] Every false green in a synthetic bundle is listed with its run id.
- [ ] `--check` rejects a bundle with a missing run and a bundle with `simulated: true`.
- [ ] Both `[[verify]]` commands pass.

## Notes

- **Confidence intervals** come with the pilot page (gap-d9e9fe). The full analysis toolkit is S08.T16, later.
- **Test data:** synthetic records only, with no provider calls.
- **Waits for:** the driver (gap-28ebea), for the record format. No hot files.
