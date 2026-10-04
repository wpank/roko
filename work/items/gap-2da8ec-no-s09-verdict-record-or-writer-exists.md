+++
id = "gap-2da8ec"
kind = "gap"
title = "No S09 verdict record or writer exists; holm.decide has no caller, so every showcase claim is NOT_YET_MEASURED"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-14 follow-up reports 2026-10-04 (gap-dbe8e7, gate 14b)"
discovered_from = "gap-dbe8e7 (in flight, work/gap-dbe8e7; view contracts fixed, verdict data deliberately left for later)"
anchors = ["benchmarks/viabilitybench/analysis/holm.py::decide", "benchmarks/viabilitybench/showcase/build_bundle.py"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_build_reads_an_s09_verdict_record_into_claim_state' benchmarks/viabilitybench/showcase/test_bundle.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/showcase/test_bundle.py -q -k test_build_reads_an_s09_verdict_record_into_claim_state"
+++

## Problem

No S09 verdict record or reader exists, so every showcase Overview claim reports
`NOT_YET_MEASURED` and the negatives list stays empty, regardless of what any experiment
actually found. `benchmarks/viabilitybench/showcase/build_bundle.py` (on `work/gap-dbe8e7`, not
yet merged) hardcodes `CLAIM_STATE = "NOT_YET_MEASURED"` (line 66) applied to every tile (line
338), with its own comment explaining why: "Claim states are S09's test records' verdicts, and
no results directory records one yet" (lines 30-31, 65). Its negatives projection is similarly
empty by construction: "Results against the thesis come from S09's test records, which no
results directory holds yet: no negatives" (line 345).

The statistical machinery to produce a real verdict already exists on the analysis side —
`benchmarks/viabilitybench/analysis/holm.py::decide` (line 150, producing a `HolmResult` from
Holm-Bonferroni-adjusted p-values per `s09_graph()`'s multiplicity graph) — but `decide` has
exactly one kind of caller anywhere in the repo: its own tests (`test_lock.py`,
`test_envelope.py`). Nothing calls it to produce a *persisted* verdict record, and
`build_bundle.py` has nothing to read even if one existed, because no file format for "a
results directory's S09 verdict record" has ever been defined.

## Why it matters

Goal: proof, S09 statistical decisions / the showcase Overview (S10 §4.3 A, §5.2). The Overview
is specifically meant to show measured claim states (`verified`, `refuted`, etc.) once an
experiment's results support them — "not yet measured" is correct only until an experiment
actually runs `holm.decide` and that decision is recorded somewhere. Right now there is no path
from a finished experiment's data to a claim state at all; `NOT_YET_MEASURED` isn't a transient
default being filled in by other code, it is the only value this code path can ever produce.

## Where

- `benchmarks/viabilitybench/analysis/holm.py::decide`, `::HolmResult` (the decision function;
  has no writer).
- `benchmarks/viabilitybench/showcase/build_bundle.py` (`CLAIM_STATE`, `project_views`'s
  negatives logic; the reader to add).
- Wherever an experiment's results currently get analyzed (e.g. `analysis/lock.py`,
  `analysis/envelope.py`) — the natural place to call `decide` and write its result.

## Current state

`decide()` is implemented, tested in isolation, and otherwise unused. No record format, writer,
or reader exists anywhere for its output.

## Plan

1. Define a results-directory record format for an S09 verdict: at minimum the claim/hypothesis
   id, the adjusted p-value, the decision (reject/fail-to-reject), and whatever
   `build_bundle.py`'s claim_state/negatives projections need per-claim.
2. Add a writer: wherever an experiment's analysis currently runs, call `holm.decide` over its
   hypotheses and write one verdict record per claim to the results directory.
3. Add a reader in `build_bundle.py`: when a results directory holds verdict records for the
   experiment being bundled, use them for `claim_state` and the negatives list instead of the
   `NOT_YET_MEASURED`/empty defaults.
4. Regression test: a results directory with a recorded verdict produces a bundle whose
   Overview tile for that claim is not `NOT_YET_MEASURED`.

## Done when

- An experiment with a recorded S09 verdict produces a bundle whose Overview reflects it.
- The `[[verify]]` command passes.

## Notes

- 2026-10-04 (wave-14 follow-up, gap-dbe8e7, gate 14b not yet merged): confirmed on
  `work/gap-dbe8e7`. `gap-dbe8e7`'s own scope (view contract shapes) deliberately left this as
  future work (its Progress note: "claims stay NOT_YET_MEASURED until a results directory
  records S09 verdicts") rather than fixing it, so this is a genuine follow-up, not a duplicate.
