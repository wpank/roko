+++
id = "bug-7b1b7f"
kind = "bug"
title = "The driver's chat client is refused by api.cerebras.ai's Cloudflare (403, error 1010) for urllib's default User-Agent, so every direct-loop and refiner call to gpt-oss-120b fails"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-18
updated = 2026-10-18
last_verified = 2026-10-18
source = "gap-942f88 worker (claude-agent), 2026-10-18: live vb refine at d77feb626, 20/20 calls 403 error 1010; $0 User-Agent probe through the proxy"
discovered_from = "gap-942f88"
anchors = ["benchmarks/viabilitybench/driver/provider.py::OpenAICompatible", "benchmarks/viabilitybench/driver/faultproxy.py::_forward"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["gap-942f88"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_the_driver_client_sends_its_own_user_agent' benchmarks/viabilitybench/driver/test_faultproxy.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_faultproxy.py -k test_the_driver_client_sends_its_own_user_agent -q"
+++

## Problem

Every call the driver's chat client (`provider.OpenAICompatible`) makes to Cerebras is refused before it reaches the
API: HTTP 403 with Cloudflare's problem body for error 1010, `browser_signature_banned` ("The site owner has blocked
access based on your browser's signature", zone `api.cerebras.ai`). The client sends urllib's default
`User-Agent: Python-urllib/3.12`, and the metering proxy forwards the client's User-Agent upstream
(`FaultProxy._forward`), so the provider's edge sees that signature and bans it.

Seen live on 2026-10-18 (gap-942f88's first refiner run, `vb refine` at d77feb626): all 20 calls answered 403 in
about 50 ms each, metered `not_billed` by the proxy. A $0 probe through the same proxy with a model that does not
exist showed the cause: `User-Agent: Python-urllib/3.12` got the 403; `vb-driver/1.0.0`, or no User-Agent at all,
reached the API, which answered its own 404 `model_not_found`.

## Why it matters

Goal `proof`. The direct loop (`mini_loop`, arms `cheap_direct` and `fd_api`) and the refiner's cost sample
(`vb refine`) call models only through this client. Pilot A's `cheap_direct` block on gpt-oss-120b would fail every
call, and each failed attempt also books an unknown cost at its reservation (bug-2a5566), so the pilot would
stop at BL0's cap having spent nothing. Blocks gap-942f88's live run.

## Where

- `benchmarks/viabilitybench/driver/provider.py::OpenAICompatible.complete`: the request headers; no User-Agent is
  set, so urllib sends `Python-urllib/<version>`.
- `benchmarks/viabilitybench/driver/faultproxy.py::FaultProxy._forward`: relays the client's User-Agent upstream
  (unchanged by the fix).

## Current state

The client sets `Content-Type` and `Accept` only. Roko's and mini-swe-agent's own clients send their own
User-Agents, which the proxy forwards; only the driver's stdlib client is affected.

## Plan

1. Send a User-Agent of the driver's own (`provider.USER_AGENT`) on every request.
2. Test that the upstream behind the proxy receives it, and never urllib's default.

## Done when

- [ ] The `[[verify]]` command passes.
- [ ] A live call through the proxy to Cerebras is answered by the API, not by Cloudflare's 1010.

## Notes

- The User-Agent reaches only the provider: no agent sees the driver's requests, so naming the driver there does not
  name the benchmark to an agent (bug-d68575).
