+++
id = "gap-60654d"
kind = "gap"
title = "ViabilityBench: enforce token caps through the proxy, fill the meter cross-check, and bundle proxy.jsonl"
status = "open"
triage = "unverified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["bench"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (21:40, wk-bench-fix1's report on gap-e90ebd)"
anchors = ["benchmarks/viabilitybench/driver/vb.py", "benchmarks/viabilitybench/driver/records.py", "benchmarks/viabilitybench/analysis/report.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-e90ebd", "gap-e003ec", "gap-b24517"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'def test_proxy_caps_meter_check_and_bundle' benchmarks/viabilitybench/ && cd benchmarks/viabilitybench && .venv/bin/python -m pytest -q -k test_proxy_caps_meter_check_and_bundle"
+++

## Problem

After gap-e90ebd, every billed run is metered through the proxy, but three pieces stay unwired (wk-bench-fix1, 2026-09-29):

- `vb run` never sets the proxy's `input_token_cap`, so the roko_fixed arm's token caps are enforced nowhere.
- Run records leave `meter_cross_check_usd` null, although the proxy's per-task meter could fill it.
- The report's committable bundle (`analysis/report.py --bundle`) doesn't copy `proxy.jsonl`.

## Why it matters

The pilot's cost numbers must be cross-checked against an independent meter, and its caps must hold. Epic spec-567e52.

## Where

`driver/vb.py` (`_start_proxy` and `_run_one`), `driver/records.py`, `analysis/report.py`, and the arm files.

## Current state

The proxy meters every call; nothing consumes the meter beyond the Roko arm's check.

## Plan

1. Set `input_token_cap` per task from the arm's caps.
2. Fill `meter_cross_check_usd` from the proxy meter, and flag drift above 5%, as ledger reconcile does.
3. Copy a scrubbed `proxy.jsonl` into the bundle.
4. Add tests.

## Done when

- [ ] Caps are enforced, the cross-check is filled, and the bundle carries the meter log.
- [ ] The `[[verify]]` command passes.
