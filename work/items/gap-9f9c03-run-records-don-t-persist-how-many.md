+++
id = "gap-9f9c03"
kind = "gap"
title = "Run records don't persist how many truth-suite checks ran, so labels.py's hidden.n is always a lower bound"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/audit"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-4 follow-up reports 2026-10-02 (PK53)"
discovered_from = "audit/labels.py's own module docstring and _hidden's n_known=False"
anchors = ["benchmarks/viabilitybench/audit/labels.py::_hidden", "benchmarks/viabilitybench/driver/records.py::build"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_hidden_n_known_true_when_record_has_check_count' benchmarks/viabilitybench/audit/tests/test_labels.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/audit/tests/test_labels.py -k test_hidden_n_known_true_when_record_has_check_count -q"
+++

## Problem

`audit/labels.py::_hidden` builds the `hidden` field of a `vs.label` row from a run record's census check and its
named failures, but a run record never keeps the total number of truth-suite checks that ran — only which ones are
named as failed (`vs.failed`, entries like `hidden.<check>`). So `_hidden`'s count is a proxy, not a real count:

```python
def _hidden(task, vs, check, failed):
    if check is None:
        return None
    named = sum(1 for item in failed if item.startswith("hidden."))
    count = 0 if check == 1 else max(named, 1)
    return {"suite": f"truth:{task['family']}@{vs['truth_suite_version']}", "n": count, "failed": count,
            "n_known": False}
```

When the check passed (`check == 1`), `n` is hard-coded to `0` — not "zero checks ran", but "we don't know how many
ran, and none are named as failed". When it failed, `n` is `max(named, 1)` — the count of *failed* checks reused as
a stand-in for the count of checks that *ran*, which under-counts whenever more checks ran than failed (the normal
case). The module docstring already says this honestly: "The record does not keep how many checks ran, so `n` is
that lower bound and `n_known` is false." `driver/records.py::build` (the function that assembles a `vb.run_record`,
~line 116-155) captures `truth_suite_version` and `failed` from the hidden-suite output, but no total-checks field.

## Why it matters

`n_known=false` already flags the row honestly, so nothing downstream is silently wrong — but it means any report or
audit estimator that wants a real denominator for "checks run" (e.g. a per-check pass rate, or a confidence interval
over the truth suite's own check count, distinct from the pass/fail label) cannot get one from `vs.label` rows today,
for every record produced so far. This is adjacent to M4's audit estimators (S05), which already distinguish
labelled vs. unlabelled evidence by confidence; an honest `n` would let them do the same for the truth suite itself.

## Where

- `benchmarks/viabilitybench/audit/labels.py::_hidden` (the lower-bound count) and its module docstring (~line 21).
- `benchmarks/viabilitybench/driver/records.py::build` (~line 116-155) — assembles the record from
  `result.hidden_output`; the fix needs the hidden-suite runner itself (wherever it executes and inspects
  `result.hidden_output`) to report a total check count, which `build` would then persist on the record.

## Current state

Unfixed; `n_known=false` is the documented, intentional signal of this gap today, not a silent error.

## Plan

1. Find where the hidden/truth-suite harness itself knows how many checks it ran (it presumably iterates named
   checks to decide pass/fail already) and have it report that count in `result.hidden_output`.
2. `driver/records.py::build` persists it on the record (a new field alongside `truth_suite_version`/`failed`).
3. `labels.py::_hidden` reads the real count when present, keeps the lower-bound fallback for older records that
   predate the field, and sets `n_known=True` only when the real count is available.

## Done when

- A record produced after the fix carries the true truth-suite check count, and `_hidden` reports it with
  `n_known=True`.
- Older records (without the new field) still produce the current lower-bound behavior with `n_known=False` — no
  regression for historical data.
- The `[[verify]]` command passes.

## Notes

- See the dated Notes line added to `tmp/backlog/2026-10-02-complete-and-wire/7120-audit-only-a1-kinds.md` for a
  related, separate finding (Rust `roko-gate` audit-only diff-finding kinds should reuse `tamper.py`'s exact kind
  names) — not the same bug, just found by the same worker in the same area.
