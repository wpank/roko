+++
id = "gap-dad97b"
kind = "gap"
title = "run_roko.py must read bug-31438d's new record fields: turns_unknown, model_reported, substitution, attempt_key and helper rows"
status = "open"
triage = "unverified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-model-truth's report on branch work/bug-31438d at ee6a541ef)"
anchors = ["benchmarks/viabilitybench/driver/run_roko.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = ["bug-31438d"], blocks = [], related = ["bug-31438d", "bug-62e3f4", "bug-55fd84", "gap-b7ab99"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_run_roko_reads_the_model_truth_fields' benchmarks/viabilitybench/driver/test_run_roko.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_roko.py -k test_run_roko_reads_the_model_truth_fields -q"
+++

## Problem

bug-31438d's branch changes what Roko records: `turns_unknown`, `model_reported`, `model_mismatch`, `substituted_from`, `attempt_key`, and helper-call rows with role "helper". `benchmarks/viabilitybench/driver/run_roko.py` at 7fa54b873 predates all of these. It numbers attempts by row index (`enumerate(evidence.episodes, 1)` at :310, `enumerate(evidence.cost_rows, 1)` at :341) and by the `/a(\d+)` pattern (`ATTEMPT_ID`, :123), and it knows none of the new fields.

## Why it matters

Pilot benchmark (epic spec-567e52): once bug-31438d merges, the Roko arm's records would misread turns, miss substitutions that Roko now reports, and mis-number attempts whenever rows arrive out of order or helpers add rows.

## Where

`settle` and the evidence readers in `run_roko.py`.

## Plan

Once bug-31438d merges:

1. Map `extra.turns_unknown` to `turns=None`.
2. Check `model_reported`, `model_mismatch` and `substituted_from` on episodes (under `extra`), on cost rows and on efficiency rows, and flag `model_mismatch` from them.
3. Take the attempt number from `attempt_key`, not from the row index or `/a(\d+)`.
4. Treat rows with role "helper" as helper calls of their attempt.
5. Expect the verdict's `model_reported` to be null when the model is unreported.
6. Count an attempt's calls as `turns + helpers.calls`, and compare that with the proxy.
7. Add `test_run_roko_reads_the_model_truth_fields`, with fixtures built from the branch's record format.

## Done when

- [ ] The Roko arm reads the new fields, and its checks use them.
- [ ] The `[[verify]]` command passes.
