+++
id = "gap-04496d"
kind = "gap"
title = "tab_t3_headline still expects a holm_reject MetricRecord, and claim_state's adverse flag has no caller"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench"]
created = 2026-10-05
updated = 2026-10-05
last_verified = 2026-10-05
source = "wave-20 follow-up reports 2026-10-05 (gap-2da8ec, work/gap-2da8ec)"
discovered_from = "gap-2da8ec (open; its fix produces vb.verdict/1, a different format from what tab_t3_headline reads)"
anchors = ["benchmarks/viabilitybench/analysis/tab_t3_headline.py", "benchmarks/viabilitybench/analysis/holm.py::decide"]
lane = "bench"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_tab_t3_headline_shows_a_real_holm_decision' benchmarks/viabilitybench/analysis/test_tab_t3_headline.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_tab_t3_headline.py -k test_tab_t3_headline_shows_a_real_holm_decision -q"
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
