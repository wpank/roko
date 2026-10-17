+++
id = "bug-2224eb"
kind = "bug"
title = "An injected provider_fault carries none of its upstream's own headers and the same OpenAI error shape for every upstream, so an agent that compares it with the answers it gets otherwise can tell the fault is not the provider's"
status = "open"
triage = "unverified"
severity = "p3"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-17
source = "bug-fd8200 worker (claude-agent), 2026-10-17 (a stub upstream whose 200 carries x-request-id and x-ratelimit-remaining-requests, behind a FaultProxy: the forwarded answer relays both; a rate_limit fault at p=1 answers with content-length, content-type, date, retry-after and server only)"
discovered_from = "bug-fd8200"
anchors = ["benchmarks/viabilitybench/driver/faultproxy.py::_serve", "benchmarks/viabilitybench/driver/faultproxy.py::send_json"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-fd8200", "gap-394f28"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_an_injected_fault_looks_like_the_upstreams_own' benchmarks/viabilitybench/driver/test_faultproxy.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_faultproxy.py -k test_an_injected_fault_looks_like_the_upstreams_own -q"
+++

## Problem

An injected `http_5xx` or `rate_limit` fault is answered by the metering proxy itself (`faultproxy.py::FaultProxy._serve`,
`_Call.answer`), with only `Content-Type`, `Content-Length`, `Server`, `Date` and, for a rate limit, `Retry-After`.
A forwarded answer relays every header of the upstream's own but the hop-by-hop ones (`NOT_RELAYED`): a provider's
request id (`x-request-id`), its rate-limit headers (`x-ratelimit-*`), its CDN's (`cf-ray`) and so on. The body has
OpenAI's error shape for every upstream, whatever shape that upstream's own errors have (z.ai answers with numeric
codes such as `1302`, Cerebras with fields of its own). So an agent that compares a fault with the answers it gets
otherwise can tell that the fault did not come from the provider.

To see it (2026-10-17, after bug-fd8200): a stub upstream whose 200 carries `x-request-id` and
`x-ratelimit-remaining-requests`, behind a `FaultProxy`: the forwarded answer's headers are content-length,
content-type, date, server, x-ratelimit-remaining-requests and x-request-id; with the profile `rate_limit` (p = 1)
the 429's are content-length, content-type, date, retry-after and server.

## Why it matters

Goal `proof`. S08 §4.6's `provider_fault` disturbance is meant to look like the provider's own errors: H6 measures how
the arms take a provider's errors, not a benchmark's. bug-fd8200 made the fault's words a provider's; its headers and
its body's shape still differ from the upstream's. The Roko and mini-swe-agent agents' shells reach the proxy's port
and can read headers (`curl -i`). A harness that reads a provider-specific header or body field (a rate-limit header,
a body `retry_after`) also meets a fault that a real error of that provider would not be. Before the
pre-registration lock (gap-394f28), since it changes what the agents see.

## Where

- `benchmarks/viabilitybench/driver/faultproxy.py::FaultProxy._serve`: the `http_5xx` and `rate_limit` answers.
- `benchmarks/viabilitybench/driver/faultproxy.py::_Call.answer` and `_Handler.send_json`: the headers a self-made
  answer gets.

## Current state

Since bug-fd8200 the fault answers are in OpenAI's words and shape (`SERVER_ERROR`, `RATE_LIMITED`) and name no fault.
Nothing models a particular upstream's headers or error shape.

## Plan

1. Record, per arm provider (Cerebras, z.ai, Moonshot, OpenAI, OpenRouter), the headers and the body of its own 429
   and 5xx, from its documentation or one captured error each (no paid call is needed for a 401 shape; a 429 or 5xx
   needs a capture the operator approves).
2. Give each `Upstream` a fault template: its 429 and 5xx body, and the headers its answers carry (a request id in
   the provider's format, its rate-limit headers), and answer a fault with it. Keep the status and `Retry-After`.
3. Check that each harness takes a templated fault as it takes the current one (the status; litellm's mapping; Roko's
   classifiers and its `/error/code` handling for z.ai's numeric codes, which change Roko's retry delay).
4. Add `test_an_injected_fault_looks_like_the_upstreams_own` to `driver/test_faultproxy.py`: behind a stub upstream
   that answers with a provider's headers, an injected fault carries the same header names as a forwarded answer of
   that provider, and its body the provider's shape.

## Done when

- [ ] An injected fault's answer carries the header names and the body shape of its upstream's own errors.
- [ ] The `[[verify]]` command passes.

## Notes

- Found by the bug-fd8200 worker, 2026-10-17, while making the proxy's answers a provider's own.
- A z.ai template with code `1302` would change how Roko waits (`openai_compat::classify_error`: 5 s for 1302): that is
  what a real z.ai rate limit does, but it changes the instrument, so it belongs before the lock and in the
  pre-registration.
