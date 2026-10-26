+++
id = "bug-2a5566"
kind = "bug"
title = "A direct-loop attempt whose every call ended in an HTTP error books an unknown cost, so the ledger charges its whole reservation for zero billed tokens"
status = "done"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-18
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "d6dc06de7"
source = "gap-942f88 worker (claude-agent), 2026-10-18: the same pricing in vb refine's first live run charged BL15's whole $0.40 for 20 calls metered not_billed"
discovered_from = "gap-942f88"
anchors = ["benchmarks/viabilitybench/driver/mini_loop.py::_settle", "benchmarks/viabilitybench/driver/ledger.py::price"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["gap-942f88", "bug-7b1b7f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_an_attempt_of_http_errors_books_a_known_zero_cost' benchmarks/viabilitybench/driver/test_driver.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_driver.py -k test_an_attempt_of_http_errors_books_a_known_zero_cost -q"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T10:46:07Z"
commit = "d6dc06de7"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-26T10:26:18Z"
forced = false
evidence = "d6dc06de7: mini_loop._settle now resolves `model = attempt.model_reported or attempt.model_requested` and prices by `ctx.snapshot.row(model)`, so an attempt whose every call met an HTTP error (billed_unknown=False, zero usage, model_reported never set) prices at the requested model's row instead of Cost(None, None, \"unknown\"); a reported model with no price row (served by the wrong model) is left unsubstituted and still prices unknown (test_a_model_without_a_price_row_costs_null, unchanged, still passes). New test test_an_attempt_of_http_errors_books_a_known_zero_cost: StubServer's broken() handler answers every call with an HTTP 500 through a full offline `vb run --limit 1`; asserts calls == 1 + RETRIES, record/ledger api_equiv_usd == 0.0, source == \"provider_usage\", billed_usd == 0.0, model_reported is None, and both validate() clean. benchmarks/viabilitybench/driver/test_driver.py + test_ledger.py: 55 passed, 0 failed."
+++

## Problem

An attempt of the direct loop whose every model call ended in an HTTP error books an unknown cost, and so charges its
whole reservation against its budget line, although the provider billed nothing. `mini_loop._settle` prices the
attempt with the price row of `attempt.model_reported`; an error answer reports no model, so the row is None and
`ledger.price` returns an unknown cost, although the usage itself is known: an `HTTPError` is `billed_unknown=False`,
so the attempt's usage stays at zero tokens. The ledger row then has `billed_usd = null`, and `ledger._charged`
counts it at `reserved_usd`.

Seen in the same shape on 2026-10-18 in gap-942f88's first refiner run, whose first version priced the same way:
20 calls refused with a 403, the proxy metered each `not_billed`, and the 20 ledger rows charged $0.02 each, filling
BL15's $0.40 cap with $0 spent. `vb refine` now prices zero usage by the requested model's row; `mini_loop` does not.

## Why it matters

Goal `proof`. A run that meets provider errors (a 403 from an edge, a 401, a 429 storm) would stop at its line's cap
with nothing spent, and the ledger report would show spend that never happened. With the User-Agent ban
(bug-7b1b7f), Pilot A's 60 `cheap_direct` runs would charge their worst cases, about $0.29 each, past BL0's $14.

## Where

- `benchmarks/viabilitybench/driver/mini_loop.py::_settle`: prices by `ctx.snapshot.row(attempt.model_reported)`.
- `benchmarks/viabilitybench/driver/ledger.py::price`: unknown when the row is None, even for zero usage.

## Current state

`refine_live._refine_one` prices by the reported model, else the requested one (gap-942f88). An attempt whose usage
is unknown (a dropped connection, a timeout: `billed_unknown=True`) must stay unknown; only known zero usage is at
issue.

## Plan

1. In `_settle`, price by the reported model's row, else the requested model's, so known usage with no reported model
   is priced at the requested model's rates (zero tokens cost $0).
2. Test: an attempt whose calls all answer HTTP 500 books a known $0 row, and one whose call timed out stays unknown.

## Done when

- [ ] The `[[verify]]` command passes.

## Notes

- The run record's `costs` come from the same attempt cost (`records.build`), so they change with it.
