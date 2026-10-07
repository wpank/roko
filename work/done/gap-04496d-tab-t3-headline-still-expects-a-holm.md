+++
id = "gap-04496d"
kind = "gap"
title = "tab_t3_headline still expects a holm_reject MetricRecord, and claim_state's adverse flag has no caller"
status = "done"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-10-05
updated = 2026-10-05
last_verified = 2026-10-05
last_verified_rev = "fdebdbe03"
source = "wave-20 follow-up reports 2026-10-05 (gap-2da8ec, work/gap-2da8ec)"
discovered_from = "gap-2da8ec (open; its fix produces vb.verdict/1, a different format from what tab_t3_headline reads)"
anchors = ["benchmarks/viabilitybench/analysis/tab_t3_headline.py", "benchmarks/viabilitybench/analysis/holm.py::decide"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_tab_t3_headline_shows_a_real_holm_decision' benchmarks/viabilitybench/analysis/test_tab_t3_headline.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_tab_t3_headline.py -k test_tab_t3_headline_shows_a_real_holm_decision -q"

[closed]
at = 2026-10-05
at_ts = "2026-10-05T19:22:53Z"
commit = "fdebdbe03"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-05T17:36:21Z"
forced = false
evidence = "Gate 22 (merged fdebdbe03): verify test_tab_t3_headline_shows_a_real_holm_decision passes; bench suite 763 passed. analysis/holm_decisions.py is the first production caller of holm.decide: one decision projected as verdicts.json and as the holm_reject MetricRecords tab_t3_headline reads (provenance copied from the cell's envelope record). H1's adverse flag comes from envelope_ratio_r/_c's sign against the reference arm, so NOT_SUPPORTED is reachable; H2-H7 keep the conservative default until a metric is tied to them."
+++

## Problem

`build_bundle.py` now reads `vb.verdict/1` records into claim states (`gap-2da8ec`'s fix, on
`work/gap-2da8ec`, not yet merged: `analysis/holm.py` gains `verdict_records`/`write_verdicts`,
and `build_bundle.py` reads `--verdicts/<results>/<experiment>/verdicts.json` into
`project_views`), but two gaps remain, both confirmed directly:

1. **`tab_t3_headline.py` still expects a `holm_reject` `MetricRecord` per hypothesis that
   nothing produces.** `benchmarks/viabilitybench/analysis/tab_t3_headline.py:84`: `rec =
   inputs.one("holm_reject", experiment=figlib.P1_CORE, arm=figlib.ANY, clauses={"hypothesis":
   hypothesis})`, falling back at line 86 to "no holm_reject record in the inputs (T6 holds the
   decisions)." when none exists — which is always, today: `gap-2da8ec`'s fix writes a
   completely different artifact (`verdicts.json`, `vb.verdict/1` records), never a
   `MetricRecord` named `holm_reject`. The table's own fallback message already names the
   mismatch; nothing bridges the two formats.
2. **`claim_state` gives `NOT_SUPPORTED` only when a caller passes `adverse=True`, and no
   caller does.** `analysis/holm.py::claim_state(primary, result, adverse: bool = False)`
   (line 189) and `verdict_records(..., adverse: Mapping[str, bool] | None = None)` (line 202)
   both default to treating every primary as non-adverse, by design — Holm's own p-value
   "can only ever reject in the hypothesized direction," so distinguishing `NOT_SUPPORTED` from
   `INCONCLUSIVE` needs the point estimate's sign as a side fact the analysis has to supply.
   Confirmed: `write_verdicts`/`verdict_records` have no production caller anywhere outside
   `holm.py`'s own tests (grepped the whole `benchmarks/viabilitybench/` tree); `build_bundle.py`'s
   consumer side is tested only against hand-built `verdicts.json` fixtures
   (`showcase/test_bundle.py::write_verdicts_doc`). So not only does no caller pass
   `adverse=True` yet — no production caller exists at all, meaning every real claim currently
   defaults to the conservative `INCONCLUSIVE`, never `NOT_SUPPORTED`, regardless of what the
   data actually shows.

## Why it matters

Goal: proof, same goal as `gap-2da8ec`. (1) means the T3 headline table — presumably the
primary place a human reads the Holm decision — still can't show it, even though the
underlying decision data now exists elsewhere in a different shape. (2) means the showcase's
negatives list (which `gap-2da8ec`'s own fix wires a `NOT_SUPPORTED` verdict into as an
overview negative) can never actually populate until something computes and passes the
direction-of-effect fact — right now it's architecturally impossible for any real experiment
to show a refuted claim as refuted, only ever as "not yet enough signal."

## Where

- `benchmarks/viabilitybench/analysis/tab_t3_headline.py:84-86` (the `holm_reject` reader).
- `benchmarks/viabilitybench/analysis/holm.py::claim_state`, `verdict_records`,
  `write_verdicts` (the `adverse` parameter and its all-default-False production reality).
- Wherever a real experiment's analysis pipeline would call `write_verdicts` (not yet
  identified; no such call exists yet).

## Plan

1. For (1): either make the Holm-decision writer also emit a `holm_reject` `MetricRecord` per
   hypothesis (for `tab_t3_headline.py` to keep reading its existing format), or switch the
   table to read `vb.verdict/1`/`verdicts.json` directly, matching `build_bundle.py`'s new
   path. Pick one rather than maintaining two formats for the same fact.
2. For (2): wire a real caller that computes each primary's point-estimate sign (adverse or
   not) from the same analysis that computes Holm's p-values, and passes it as the `adverse`
   mapping to `verdict_records`/`write_verdicts`.
3. Regression tests: `tab_t3_headline.py` renders a real Holm decision (once (1) is resolved);
   an experiment whose point estimate goes the wrong way produces a `NOT_SUPPORTED` verdict,
   not `INCONCLUSIVE` by default (once (2) is resolved).

## Done when

- `tab_t3_headline.py` shows a real Holm decision, from one canonical source.
- A real adverse result produces `NOT_SUPPORTED`, not the conservative default.
- The `[[verify]]` command passes.

## Notes

- 2026-10-05 (wave-20 follow-up, gap-2da8ec, work/gap-2da8ec not yet merged): confirmed both
  gaps directly by reading `tab_t3_headline.py`, `holm.py`'s `claim_state`/`verdict_records`
  signatures, and grepping every call site of `write_verdicts`/`verdict_records` across the
  whole `benchmarks/viabilitybench/` tree.

## Progress

- gap-04496d: implemented. New module `analysis/holm_decisions.py` (no existing file touched):
  the first production caller of `holm.decide`/`write_verdicts` outside `holm.py`'s own tests.
  `write(inputs, experiment_id, p, out_dir, ...)` calls `decide` once and projects the same
  `HolmResult` two ways: `holm.write_verdicts` (gap-2da8ec's `verdicts.json`) and
  `holm_reject_records`, one `holm_reject` `vb.metric_record/1` per primary -- the exact format
  `tab_t3_headline.py:84` already reads. Chose "produce holm_reject from the same verdicts"
  (the item's own alternative, over switching T3 to read `vb.verdict/1` directly): it needed no
  change to `tab_t3_headline.py`, `figlib.py`'s loader, or any of the other scripts sharing its
  `inputs.one(metric, ...)` convention -- lower blast radius than teaching `figlib` a second
  schema-tagging convention (`vb.verdict/1` uses `schema_version`; figlib's own `ROW_SCHEMAS`
  side-channel keys on a different field, `schema`) for a fact only one table reads.
  `holm_reject_records` computes none of a record's provenance itself: `experiment_id`, `arms`,
  `run_ids`, `seeds`, `commits`, `config_hashes`, `analysis_commit`, `computed_at`,
  `preregistered`, `prereg_id`, `blinded`, `label_source`, `cost_basis`, `price_snapshot_id` and
  `n` are all copied verbatim from a `template` record of the same cell (`envelope_level` by
  default); only `metric`, `value`, `estimator` and one extra `record_filter` clause are its own.
  Checked every record this builds against `schema/validate.py`'s real `metric-record` kind
  (not just the dry-run marker): no errors.
- `adverse` (the side fact `claim_state` needs to tell `NOT_SUPPORTED` from `INCONCLUSIVE`,
  gap-2da8ec's own conservative default) now has a real production source for H1:
  `find_adverse` reads `envelope_ratio_r`/`envelope_ratio_c` against `figlib.REFERENCE_ARM` --
  the exact records `tab_t3_headline.py`'s own `_ratio` footer helper already reads -- and
  `adverse_from_ratio` classifies their sign (R below 1, or C above 1) without computing a new
  statistic, matching "copy it; compute nothing new." H2-H7 have no ratio wired to a hypothesis
  anywhere in this codebase yet, so they still fall back to `verdict_records`'s own default;
  noted as a real, acknowledged limit, not silently glossed over.
- Confirmed the architectural claim in "Why it matters" is now false: built a verdict where H1's
  node is not rejected (p = 0.9) but `adverse` is true, and got `claim_state: NOT_SUPPORTED`, not
  `INCONCLUSIVE` -- a real experiment can now show a refuted H1 as refuted.
- Tests, `analysis/test_tab_t3_headline.py` (new, 4 tests): the named
  `test_tab_t3_headline_shows_a_real_holm_decision` (builds the `test_figures_p1.Fixture`'s
  records minus its own hand-built `holm_reject` rows -- two independently-written records for
  the same hypothesis is a `figlib` error -- writes a real verdict and its `holm_reject`
  projection, and checks T3's footer shows "Holm rejects/does not reject the null", not the dead
  "no holm_reject record" fallback); `test_adverse_from_ratio_...` (pure, both signs and the
  no-ratio case); `test_write_passes_adverse_...` (the NOT_SUPPORTED-not-rejected proof above,
  plus confirming every other primary keeps the conservative default); and
  `test_holm_reject_records_copy_their_provenance_...` (every copied field equals the
  template's, field by field).
- Verified load-bearing twice: reverted `adverse_from_ratio` to always return `None`, confirmed
  both adverse-related tests fail (one on the direct unit assertions, one on `find_adverse`
  coming back empty); separately reverted `holm_reject_records` to tag every primary's clause
  `hypothesis == "H1"`, confirmed the named test fails -- `figlib` itself raises on two
  `holm_reject` records fitting one cut, an even louder failure than a wrong assertion. Restored
  both and re-confirmed green.
- Suites run in the benchmark venv (fresh per worktree, no `.venv` carries over): the named
  `[[verify]]` command directly; full `analysis/` + `showcase/` together (143 passed, 1
  pre-existing unrelated skip, same as before this wave's changes).
- Not done here, left for later: a real per-hypothesis p-value assembly pipeline (`p` is still
  the caller's own input to `holm_decisions.write`, same boundary gap-2da8ec left at the writer
  itself) and `adverse` wiring for H2-H7, which need their own ratio or sign convention wired to
  a hypothesis first (none exists yet for any of the six).
