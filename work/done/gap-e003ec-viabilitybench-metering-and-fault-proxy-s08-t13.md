+++
id = "gap-e003ec"
kind = "gap"
title = "ViabilityBench metering and fault proxy (S08.T13)"
status = "done"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "73f5b55f4"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e12"
discovered_from = "tmp/cybernetic-harness/specs/S08-benchmark-suite.md (§4.6 provider_fault, §4.11, §6 T13; checklist S08.T13)"
anchors = ["benchmarks/viabilitybench/driver/faultproxy.py", "benchmarks/viabilitybench/driver/test_faultproxy.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = ["gap-28ebea"], blocks = [], related = ["gap-b7ab99"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_fault_rates_within_two_points' benchmarks/viabilitybench/driver/test_faultproxy.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_faultproxy.py -k test_fault_rates_within_two_points -q"

[[verify]]
command = "grep -qw 'def test_meter_equals_upstream_usage' benchmarks/viabilitybench/driver/test_faultproxy.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_faultproxy.py -k test_meter_equals_upstream_usage -q"

[closed]
at = 2026-09-29
commit = "73f5b55f4"
by = "wk-bench-proxy"
evidence = "Adds driver/faultproxy.py, the S08 T13 metering and fault proxy. It meters usage in S01 v1.2's disjoint classes, priced from prices-2026-09-28, with stream include_usage handling and a per-task input-token cap. It injects six seeded fault profiles, balanced per 20 requests, has a token-protected /_vb/control, and logs every call to proxy.jsonl. driver/test_faultproxy.py holds 11 offline tests against local stub upstreams. test_fault_rates_within_two_points puts all six profiles within 2 points over 1,000 requests each, with every fault seen by the client at the logged request. test_meter_equals_upstream_usage shows meter usage and cost equal the stub's reported usage, including under drop_usage and truncate. Both [[verify]] commands pass; the viabilitybench suite has 223 passed."
+++

## Problem

The API arms' costs rest on each client's own usage accounting, and nothing meters them independently. There is
also no way to inject the provider faults that S08's `provider_fault` hook needs:

- 5xx errors;
- 429 responses with `retry-after`;
- hangs;
- truncated streams;
- schema drift.

## Why it matters

- **An independent cost check** (S08 §4.11). Route the Roko arm's provider base URL through the proxy. The proxy
  then cross-checks Roko's own cost records and flags attempts that sent no traffic (W10 rec 5).
- **Fault runs.** S09's Pilot B lists 5 `provider_fault` runs, and H6 needs the hook later.
- **Why only p2.** The three-arm comparison does not depend on the proxy, because the direct arm meters itself.

## Where

All new, and the paths follow D4: `benchmarks/viabilitybench/driver/faultproxy.py` and
`driver/test_faultproxy.py`.

## Current state

Checked at `41c7ffbd6`: nothing exists.

## Plan

1. **The proxy.** A stdlib reverse proxy on 127.0.0.1 that speaks the OpenAI-compatible API and forwards only to
   allowlisted upstream hosts.
2. **Metering.**
   - Read `usage` from every response.
   - For streaming requests, add `stream_options.include_usage` where the provider supports it. Otherwise mark the
     request `usage_source = missing`.
   - Price usage from the snapshot, and enforce the per-attempt token cap by refusing further calls.
3. **Fault profiles:** `http_5xx(p)`, `rate_limit(p, retry_after)`, `hang(p, s)`, `truncate(p, bytes)`,
   `latency(ms, jitter)` and `schema_drift(p, …)`. Each is seeded by (seed, request number).
4. **Control.**
   - A token-protected endpoint switches the profile and sets the active task id.
   - Every request logs `{task, profile, fault_injected}` to `proxy.jsonl`.

## Done when

- [x] Against a local stub upstream, each profile's injected fault rate is within ±2 points of its target over
      1,000 requests.
- [x] The meter's totals equal the usage the stub reports.
- [x] Both `[[verify]]` commands pass.

## Notes

- **Scope.** The manifest sizes this item S; S08 says M. Keep to metering and the six profiles.
- **When the pilots use it.** Pilot A does not need the proxy. Pilot B uses it only if it has landed by then.
- **No hot files.**
- **Built 2026-09-29 (wk-bench-proxy).** `driver/faultproxy.py` implements the item and `driver/test_faultproxy.py`
  tests it, 11 tests, offline against local stub upstreams. Three choices go beyond the plan:
  - **Faults are balanced within blocks of 20 requests.** Each request still has probability p, but the rate stays
    on target for every seed. With independent draws, one seed in seven misses ±2 points over 1,000 requests at
    p = 0.3, which was measured over 300 seeds.
  - **Metering and logging come first.** Each call is metered and logged before the client can see the end of its
    response, so a runner can cross-check right after a call returns.
  - **The proxy counts input for the cap.** When a call's usage is missing, its request bytes count as input
    toward the cap.
  - **Upstream connections are kept alive and reused.** A call then pays no extra TLS setup. It also stops a run
    from exhausting ephemeral ports: with fresh connections, the 6,000-request test ran the loopback out of ports
    (`EADDRNOTAVAIL`) on back-to-back runs.
- **Not wired into `vb run` yet.** A direct-arm run needs three hunks in `vb.py`:
  - a flag;
  - in `cmd_run`, start the proxy on the endpoint's upstream after admission, and swap in `proxy.endpoint(...)`;
  - in `_run_one`, call `configure(task=...)`.

  Admission must keep judging the upstream URL, because a loopback `--provider-url` that fronts a paid provider
  otherwise counts as offline. The test `test_a_driver_run_through_the_proxy_matches_its_ledger` shows that the
  ledger and the meter agree when the driver is routed through the proxy.
