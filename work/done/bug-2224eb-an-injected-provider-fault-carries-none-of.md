+++
id = "bug-2224eb"
kind = "bug"
title = "An injected provider_fault carries none of its upstream's own headers and the same OpenAI error shape for every upstream, so an agent that compares it with the answers it gets otherwise can tell the fault is not the provider's"
status = "done"
triage = "verified"
severity = "p3"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "821a5a3f2"
source = "bug-fd8200 worker (claude-agent), 2026-10-17 (a stub upstream whose 200 carries x-request-id and x-ratelimit-remaining-requests, behind a FaultProxy: the forwarded answer relays both; a rate_limit fault at p=1 answers with content-length, content-type, date, retry-after and server only)"
discovered_from = "bug-fd8200"
anchors = ["benchmarks/viabilitybench/driver/faultproxy.py::_serve", "benchmarks/viabilitybench/driver/faultproxy.py::send_json"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-fd8200", "gap-394f28", "gap-db8dab"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_an_injected_fault_looks_like_the_upstreams_own' benchmarks/viabilitybench/driver/test_faultproxy.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_faultproxy.py -k test_an_injected_fault_looks_like_the_upstreams_own -q"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T19:49:53Z"
commit = "821a5a3f2"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-17T17:26:16Z"
forced = false
evidence = "821a5a3f2: an injected http_5xx or rate_limit fault answers as its upstream does. Its body is the upstream's own error by the upstream's name (faultproxy.UPSTREAM_ERRORS, upstream_errors; OpenAI's for any other name): OpenAI's as before; z.ai's {\"error\":{\"code\":\"1302\",\"message\":\"Rate limit reached for requests\"}} and {\"error\":{\"message\":\"Internal Error\"}} (docs.z.ai), compact; Moonshot's rate_limit_reached_error with its documented literal, server_error and server_unavailable (503); Cerebras's flat, compact message/type/param/code (a captured 401's shape) with \"Requests per minute limit exceeded - too many requests sent.\". Its headers are those of the upstream's last forwarded answer (FaultProxy._remember, _mirrored) minus NOT_MIRRORED, with every per-answer value (request, trace and log ids, cf-ray, a cookie's token) drawn anew in its form (_redrawn), so no real id repeats; send_json takes header pairs, so z.ai's three vary lines stay three. Decided under Will's decision 9 (## Decision): nothing benchmark-internal reaches an agent, for benchmark validity. The verify passes: test_an_injected_fault_looks_like_the_upstreams_own (driver/test_faultproxy.py), 4 passed (Cerebras, z.ai, Moonshot, OpenAI): behind a stub answering with the provider's headers, a rate_limit and an http_5xx at 500 and 503 carry exactly a forwarded answer's header names, each as often, plus Retry-After for the rate limit; ids, cf-ray and cookie tokens are fresh in their form on every fault and the rest is the provider's; bodies are the provider's own, compact where theirs are; statuses, Retry-After and the log as before; no 429 word a harness reads as other than a rate limit, no harness word in a 5xx, nothing the proxy writes names the task or a fault; a fault before anything was forwarded carries no provider header. On main's proxy all four fail (Server, Date, Retry-After, Content-Type and Content-Length only); 25 repeated runs pass. Otherwise the same: main's proxy and this one under every profile and four upstream names, 480 requests with 203 faults, give the same statuses, Retry-After, log rows (timing aside) and meter; litellm 1.103.2 as run_msa calls it maps every new body by its status as before (RateLimitError, InternalServerError, ServiceUnavailableError, none an abort for mini-swe-agent); the direct loop reads status and Retry-After only; a scan of Roko's 68977 string literals finds only rate-limit words beyond the old bodies'. What the sources leave open keeps each harness's reading as before (Cerebras's 429 type, null param and code, no quota word; Cerebras's and Moonshot's 5xx words short of \"server error\" and \"temporarily unavailable\"). One change by design: z.ai's 1302, for which Roko's openai_compat::classify_error waits 5 s, as for a real z.ai rate limit. Bench pytest: 826 passed, 17 skipped (14 with no roko binary in the worktree, the analysis stack, two opt-in sweeps). Filed gap-db8dab (p3: no captured 429 or 5xx of any provider checks the templates, and a fault before an upstream's first answer carries none of its headers; a capture needs the operator's approval)."
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

- [x] An injected fault's answer carries the header names and the body shape of its upstream's own errors.
- [x] The `[[verify]]` command passes.

## Notes

- Found by the bug-fd8200 worker, 2026-10-17, while making the proxy's answers a provider's own.
- A z.ai template with code `1302` would change how Roko waits (`openai_compat::classify_error`: 5 s for 1302): that is
  what a real z.ai rate limit does, but it changes the instrument, so it belongs before the lock and in the
  pre-registration.
- Fixed 2026-10-17 (faultproxy's "A fault looks like its upstream's own error"). The body is the upstream's own error,
  keyed by its name (`UPSTREAM_ERRORS`, `upstream_errors`; OpenAI's for any other name): OpenAI's as before; z.ai's
  `{"error":{"code":"1302","message":"Rate limit reached for requests"}}` for a rate limit and `{"error":{"message":
  "Internal Error"}}` for a 5xx (docs.z.ai's codes: "-" for an internal error), compact; Moonshot's
  `{"error": {"type": "rate_limit_reached_error", "message": "Organization-level RPM limit reached"}}`, its documented
  literal, and `server_error` / `server_unavailable` (503) for a 5xx; Cerebras's flat, compact
  `{"message", "type", "param", "code"}`, the shape of a captured 401, with "Requests per minute limit exceeded - too
  many requests sent." The headers are those of the upstream's last forwarded answer (`FaultProxy._remember`,
  `_mirrored`), minus the hop-by-hop ones, those that describe that answer's own body and Retry-After
  (`NOT_MIRRORED`); a value that is new on every answer (`PER_ANSWER`: a request, trace or log id, `cf-ray`, a
  cookie's token) is drawn anew in the same form (`_redrawn`), so no real id is sent twice. The proxy's own answers
  take header pairs (`send_json`), so z.ai's three `vary` lines stay three.
- What the sources leave open is chosen so that each harness reads a fault as before, and is filed as gap-db8dab (a
  capture of each provider's own 429 and 5xx, with the operator's approval): Cerebras's 429 `type`, its null `param`
  and `code` (no quota word, which Roko's `is_billing_message` reads as billing), Cerebras's and Moonshot's 5xx words
  ("Internal error"; for Moonshot's 503 "The service is unavailable, please try again later", short of "server
  error" and "temporarily unavailable", which Roko's provider health reads as a 30 s cooldown, not 5), and the
  headers of a fault that comes before the upstream has answered anything, which carries none.
- How each harness takes a fault, checked old against new: litellm 1.103.2 as run_msa calls it maps every new body by
  its status as it mapped the old (429 RateLimitError, 500 InternalServerError, 503 ServiceUnavailableError; none is
  one mini-swe-agent aborts on); the direct loop reads the status and Retry-After only; a scan of Roko's 68977 string
  literals finds in the new bodies, beyond the old ones' words, only rate-limit words in the 429s ("rate limit
  reached", "too many requests"; "limit reached" is a usage window only beside "resets") and no billing, quota,
  usage-window, auth or context word. The one change is by design: z.ai's code 1302, for which
  `openai_compat::classify_error` waits 5 s, as for a real z.ai rate limit (## Decision). Statuses, Retry-After, the
  log rows and the meter are the same, old proxy against new, over 480 requests under every profile.
- Refusals (the policy's and the caps' 403s) are unchanged: OpenAI's shape, bug-fd8200's words, no mirrored header.
  Only an agent's own request outside the policy, or past a cap, meets one.

## Decision (2026-10-17)

Whether an injected fault may answer in OpenAI's shape and with none of its upstream's headers, whatever the
upstream, had no recorded default beyond bug-fd8200's words. Under Will's decision 9 of 2026-10-15 (technical questions
with no recorded default take the coordinator's recommendation, the conservative option, recorded on the item with the
reasoning; Will can veto it later), the coordinator decided that nothing benchmark-internal reaches an agent: a fault
answers in its upstream's own error shape and words, with the headers of that upstream's own answers and freshly
drawn ids, and gives no other sign that the benchmark made it. The change lands before the pre-registration lock
(gap-394f28), since it changes what the agents and their harnesses see.

Why: benchmark validity. H6 measures how the arms take a provider's errors, not a benchmark's (S08 §4.6), and the
Roko and mini-swe-agent shells reach the proxy's port and can read headers (`curl -i`), so a fault an agent can tell
from its provider's own error is a benchmark signal, not a disturbance. The conservative option keeps every
harness's reading of a fault as it was wherever the sources leave a field open, and follows the provider's own
documentation where it is plain: so a z.ai rate limit carries code 1302, for which Roko waits 5 s as it does for a
real one. That one changes how the instrument behaves under a z.ai `rate_limit` fault, and the pre-registration
should name it.
