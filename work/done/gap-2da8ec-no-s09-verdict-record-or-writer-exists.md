+++
id = "gap-2da8ec"
kind = "gap"
title = "No S09 verdict record or writer exists; holm.decide has no caller, so every showcase claim is NOT_YET_MEASURED"
status = "done"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-10-04
updated = 2026-10-05
last_verified = 2026-10-05
last_verified_rev = "55267cfd0"
source = "wave-14 follow-up reports 2026-10-04 (gap-dbe8e7, gate 14b)"
discovered_from = "gap-dbe8e7 (in flight, work/gap-dbe8e7; view contracts fixed, verdict data deliberately left for later)"
anchors = ["benchmarks/viabilitybench/analysis/holm.py::decide", "benchmarks/viabilitybench/showcase/build_bundle.py"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_build_reads_an_s09_verdict_record_into_claim_state' benchmarks/viabilitybench/showcase/test_bundle.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/showcase/test_bundle.py -q -k test_build_reads_an_s09_verdict_record_into_claim_state"

[closed]
at = 2026-10-05
at_ts = "2026-10-05T17:35:05Z"
commit = "55267cfd0"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-05T15:20:22Z"
forced = false
evidence = "Gate 20a (merged 55267cfd0): verify test_build_reads_an_s09_verdict_record_into_claim_state passes; bench suite 759 passed. holm.py writes vb.verdict/1 (verdicts.json) per hypothesis and build_bundle.py reads it into tile claim states and negatives (verify_bundle re-derives it). A directional p can't tell 'wrong way' from 'no signal', so claim_state defaults to INCONCLUSIVE unless the caller passes adverse=True (gap-04496d wires the direction and the dead holm_reject reader)."
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

## Progress

- gap-2da8ec: implemented. Before designing the record, found the archived empirical paper's own
  spec for this exact gap: `tmp/cybernetic-harness/paper/archive/2026-10-02-empirical-draft/
  FIGURES-TABLES.md`'s T6 ("pre-registered primaries and decisions") names `tab_t6_primaries.py`
  as the never-written script, and `tab_t3_headline.py` (shipped) already has dead code reading
  a `holm_reject` MetricRecord per hypothesis with a comment, "no holm_reject record in the
  inputs (T6 holds the decisions)" -- a second, independent confirmation of the same gap from a
  different angle. T6's own column list ("claim state: SUPPORTED, NOT SUPPORTED, INCONCLUSIVE,
  NOT RUN") is richer than a bare reject/not-reject bit, which is why this is a small new record
  rather than just writing `holm_reject` MetricRecords (that reader stays dead; wiring it is a
  larger, separate change this item did not scope).
- Added `vb.verdict/1` (one record per S09 primary) and `vb.verdicts/1` (the document,
  `verdicts.json`, beside `report.py`'s `metrics.json`) to `analysis/holm.py`: `claim_state(primary,
  result, adverse=False)`, `verdict_records(result, experiment_id, metrics=, run_ids=, adverse=)`
  and `write_verdicts(out_dir, experiment_id, result, ...)`.
- Ambiguous rule, picked conservatively (as asked): a directional p-value is already one-sided, so
  a Holm rejection only ever happens in the hypothesized direction, and failing to reject cannot by
  itself distinguish "the effect runs the other way" (NOT_SUPPORTED) from "not enough signal either
  way" (INCONCLUSIVE) -- that needs the point estimate's sign, which Holm's own p-value does not
  carry. Default with no `adverse` entry for a primary: INCONCLUSIVE. `adverse` lets a future caller
  (whoever runs the per-hypothesis analysis and has the estimate) say otherwise per primary.
  `metrics`/`run_ids` are the record's links to what it rests on, both optional per primary: a
  decision can exist before its inputs are catalogued this way.
- `showcase/build_bundle.py`: reads `--verdicts`/`<results>/<experiment>/verdicts.json` (same
  pattern as `--econ`), validates each record (`verdict_problem`, same shape as
  `econ_report_problem`; no new `schema/validate.py` kind, matching how econ reports are checked
  too), copies it into the bundle as `data/verdicts.jsonl`, and threads it through
  `project_views` (new `verdicts` parameter, default `()`): a tile's `claim_state` is copied from
  the matching verdict record when one exists for its hypothesis, else stays
  `NOT_YET_MEASURED`; the p1-head-to-head claim's `state` does the same for H1; a
  `NOT_SUPPORTED` tile also becomes one of the overview's `negatives` (`kind: "other"`, since a
  verdict record alone cannot say which of contracts.ts's three more specific kinds applies).
  `verify_bundle.py` reads `data/verdicts.jsonl` the same way and passes it into the same
  `project_views` call, so "re-derives every view byte for byte" stays true with verdicts in the
  mix, not just without them.
- Tests: `analysis/test_holm.py` (new, 3 tests) on `claim_state`/`verdict_records`/
  `write_verdicts` directly; `showcase/test_bundle.py` gained
  `test_build_reads_an_s09_verdict_record_into_claim_state` (the named verify test: a verdict
  record moves a tile's claim_state and the head-to-head claim's state off the default, and a
  NOT_SUPPORTED one becomes a negative, while an un-recorded hypothesis keeps
  NOT_YET_MEASURED) and `test_a_verdict_record_of_another_experiment_is_refused` (the builder
  holds a verdict record to its own experiment, like an econ report).
- Verified load-bearing twice: reverted `project_views`' tile loop to the old `"claim_state":
  CLAIM_STATE` literal, confirmed the named test fails exactly on that assertion; separately
  reverted `claim_state()`'s `adverse` branch to always return `INCONCLUSIVE`, confirmed both new
  `test_holm.py` tests that exercise `adverse` fail. Restored both and re-confirmed green.
- Suites run in the benchmark worktree's own fresh venv (none existed in this worktree; built
  from the main checkout's installed pytest version): the named `[[verify]]` command directly;
  full `analysis/` + `showcase/` together (139 passed, 1 pre-existing skip unrelated to this
  change -- the optional numpy/pandas analysis stack is not installed here either).
- Not done here, left for later: `tab_t6_primaries.py` itself (the full T6 table: estimate, test
  statistic, replay-validation status, deviation ids, the exploratory and deviations blocks) and
  wiring real per-hypothesis p-value assembly (H1's envelope chain, H2's pass^3 test, H3-H7's own
  estimators) into a `decide()` call anywhere live -- this item's writer is exercised by tests and
  a synthetic `p` mapping, not yet by an actual experiment's numbers.
