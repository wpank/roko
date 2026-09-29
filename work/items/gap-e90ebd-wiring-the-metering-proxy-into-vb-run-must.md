+++
id = "gap-e90ebd"
kind = "gap"
title = "Wiring the metering proxy into vb run must configure each task and keep network admission for a loopback proxy URL"
status = "open"
triage = "unverified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-bench-rokoarm's report for gap-e003ec's integration)"
anchors = ["benchmarks/viabilitybench/driver/vb.py::admit", "benchmarks/viabilitybench/driver/vb.py::cmd_run", "benchmarks/viabilitybench/driver/run_roko.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = ["gap-e003ec"], blocks = [], related = ["gap-b7ab99", "bug-979a06"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'configure(task=' benchmarks/viabilitybench/driver/vb.py && grep -qw 'def test_a_loopback_proxy_url_still_needs_network_admission' benchmarks/viabilitybench/driver/test_driver.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_driver.py -k test_a_loopback_proxy_url_still_needs_network_admission -q"
+++

## Problem

gap-e003ec's branch (`work/gap-e003ec` at e9fe88aaa, not merged yet) builds `driver/faultproxy.py` and closes the item, but leaves the proxy out of `vb run`. Its notes list three hunks for `vb.py`:

- a flag;
- in `cmd_run`, start the proxy on the endpoint's upstream after admission, and swap in `proxy.endpoint(...)`;
- in `_run_one`, call `configure(task=...)`.

Wiring it has two traps:

- **(a) Unconfigured tasks.** The Roko arm matches proxy rows to attempts by task key (`run_roko.py` at e43d3a033, :50-59). Rows logged without `proxy.configure(task=key)` never match, so every attempt fails closed as `no_proxy_traffic`, and the task becomes `infra_error`. The driver must call `configure(task=ctx.key)` before each task.
- **(b) Skipped admission.** `admit` (vb.py:162) returns at once for an offline endpoint, and a loopback `--provider-url` counts as offline. A run through a loopback proxy therefore skips the `--allow-network` and `--max-cost-usd` checks, while the proxy spends on the real upstream.

## Why it matters

Pilot benchmark (epic spec-567e52): the proxy is the only meter outside the agent's reach (Roko's own records undercount, bug-62e3f4). Wired wrongly, it either fails every Roko task, or it removes the spend guard from the runs it meters.

## Where

`vb.py`'s `admit`, `cmd_run` and `_run_one`; `run_roko.py`'s proxy matching; and `faultproxy.py`, once it is merged.

## Current state

At e43d3a033 the Roko arm already reads the proxy's rows (a23aa210e), but nothing starts or configures the proxy.

## Plan

1. Wire the three hunks from gap-e003ec's notes.
2. Judge admission by the upstream URL, not the proxy's, so network runs still need `--allow-network` and `--max-cost-usd`.
3. Call `configure(task=ctx.key)` before each task.
4. Add `test_a_loopback_proxy_url_still_needs_network_admission`, and a Roko-arm run through the proxy that sees its traffic.

## Done when

- [ ] `vb run` meters through the proxy, every task's rows carry its key, and network admission still applies.
- [ ] The `[[verify]]` command passes.

## Notes

- Filed as an item, not as a note on gap-e003ec, because that item is closed on its branch, and a note there would be lost after the merge.
- bug-979a06 wants the proxy to hold the provider keys. `_endpoint` already withholds the key from an overriding URL (vb.py:424).
