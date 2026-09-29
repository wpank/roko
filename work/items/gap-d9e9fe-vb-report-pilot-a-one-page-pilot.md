+++
id = "gap-d9e9fe"
kind = "gap"
title = "vb report --pilot: a one-page pilot result with confidence intervals and run ids"
status = "open"
triage = "unverified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/analysis"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e12"
discovered_from = "tmp/cybernetic-harness/workstreams/assessment/W10-benchmarks-proof.md (rec 8; §4, the quick proof's deliverable)"
anchors = ["benchmarks/viabilitybench/analysis/report.py", "benchmarks/viabilitybench/analysis/pilot_page.py", "benchmarks/viabilitybench/analysis/bootstrap.py", "benchmarks/viabilitybench/analysis/test_analysis.py", "benchmarks/viabilitybench/reports/pilot/"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = ["gap-b24517", "gap-327242"], blocks = [], related = ["gap-c33709"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_pilot_page_has_intervals_and_run_ids' benchmarks/viabilitybench/analysis/test_analysis.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_analysis.py -k test_pilot_page_has_intervals_and_run_ids -q"

[[verify]]
command = "test -f benchmarks/viabilitybench/reports/pilot/REPORT.md && grep -q 'pilot, descriptive' benchmarks/viabilitybench/reports/pilot/REPORT.md && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/report.py --check benchmarks/viabilitybench/reports/pilot"
+++

## Problem

Pilots A and B leave two bundles of metric records, and nothing turns them into the page the author wants to show.
That page needs, for each arm:

- VS, $/VS, pass^3 and the false greens;
- confidence intervals;
- the run ids behind every number;
- an honest label.

## Why it matters

This page is the first proof point, "cheaper at equal quality" (W10 rec 8). The whitepaper's evaluation section
(E1.12, gap-2aad7d) cites it.

## Where

The paths follow D4.

- **New:**
  - `benchmarks/viabilitybench/analysis/pilot_page.py`;
  - `analysis/bootstrap.py`, a minimal paired bootstrap that S08.T16 extends later.
- **Changed:**
  - `analysis/report.py`, which gains `--pilot`;
  - `analysis/test_analysis.py`.
- **Output:** the committed page `reports/pilot/REPORT.md`, with the merged `metrics.json` beside it.

## Current state

Checked at `41c7ffbd6`: nothing exists. The metric code comes from gap-b24517.

## Plan

1. **Merge** the bundles from Pilots A and B, pairing runs by (task, seed).
2. **Report for each arm,** overall and by level ℓ:
   - the VS rate, $/VS and pass^3;
   - the false-green rate, cap censoring and `infra_error` counts;
   - for `fd_claude`, $/VS from U′ with R beside it, and billed dollars shown separately.
3. **Confidence intervals.**
   - 95% intervals from a paired bootstrap (B = 10,000). It resamples tasks within family × ℓ strata, and seeds
     within tasks.
   - Also give the cost ratios `roko_fixed`/`fd_claude` and `cheap_direct`/`fd_claude`, with their intervals.
4. **Provenance.**
   - Every number links to its run ids.
   - List F4's "visible pass, VS = 0" cases.
   - Include the verifier-CI record.
5. **Label** the page "pilot, descriptive: two Python families, 20 tasks × 3 seeds, not pre-registered". Show D3's
   bar for context only: the pilot does not test H1.

## Done when

- [ ] Rendered from a fixture bundle, the page shows intervals and run ids for every number.
- [ ] The real page is committed, and `--check` accepts its bundle.
- [ ] Both `[[verify]]` commands pass.

## Notes

- **What will resolve.** At this sample size only the cost ratio is likely to resolve, because token prices differ
  10–25×. VS differences under about 0.3 will not (W10).
- **Say so on the page.** State these limits, and report a null harness effect if that is what the data show.
- **No hot files.**
