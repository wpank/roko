+++
id = "bug-fd8200"
kind = "bug"
title = "The metering proxy's answers tell a client its task key and that a fault was injected: cap refusals say task 'F8-l3-0038.s1' could pass its input-token cap, injected faults say 'injected rate limit' and 'injected upstream error', and the policy speaks of this run's models"
status = "done"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "f6627ba25"
source = "bug-d68575 worker (claude-agent), 2026-10-17 (a FaultProxy with a stub upstream, configure(task='F8-l3-0038.s1', input_token_cap=1000) and a 10 KB request: 403 naming the key; profiles rate_limit and http_5xx at p=1: 429 'injected rate limit', 500 'injected upstream error')"
discovered_from = "bug-d68575"
anchors = ["benchmarks/viabilitybench/driver/faultproxy.py::_serve", "benchmarks/viabilitybench/driver/faultproxy.py::_policy"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-d68575", "bug-3c921e", "gap-394f28"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_no_proxy_answer_names_the_task_or_a_fault' benchmarks/viabilitybench/driver/test_faultproxy.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_faultproxy.py -k test_no_proxy_answer_names_the_task_or_a_fault -q"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T12:47:18Z"
commit = "f6627ba25"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-17T11:04:24Z"
forced = false
evidence = "f6627ba25: every answer the metering proxy makes itself is in a provider's words, in OpenAI's error shape with param (faultproxy's \"What a client is told\"): a refusal is a 403 permission_error with REFUSALS' words (\"This request exceeds the input limit.\" for both input caps, \"The model `x` is not available.\", \"The requested endpoint is not available.\", ...), an http_5xx fault a server_error (\"The server had an error while processing your request. Sorry about that!\"), a rate_limit fault a 429 requests / rate_limit_exceeded (\"Rate limit exceeded. Please try again later.\") with its Retry-After. The old messages, with the task key, the counts and the caps, are the proxy log's new detail field, which analysis/report.py's PROXY_FIELDS never publishes, and a control request without the token gets the 404 of a route that does not exist instead of a 401 naming the control endpoint. Decided under Will's decision 9 (## Decision below): no benchmark-internal identifier reaches an agent, for benchmark validity and against contamination through the public repository. The verify passes: test_no_proxy_answer_names_the_task_or_a_fault (driver/test_faultproxy.py), 1 passed: under the task F8-l3-0038.s1, the six refusals, http_5xx at 500, 502 and 503, rate_limit, an unreachable upstream, a GET on the chat path and a control request without the token keep their statuses and Retry-After, no body or header names the key, the instance, a fault, the run, a cap, a refusal, the proxy or the policy's models, none holds a word a harness classifies by (HARNESS_WORDS), and the log keeps each reason. On main's faultproxy.py (loaded by a pytest plugin) it fails on the cap refusal's \"task 'F8-l3-0038.s1'\", and test_control_endpoint_needs_the_token on the 401. The instrument behaves as before, old against new on every refusal, the 5xx at 500, 502, 503 and 504, rate_limit and an unreachable upstream: the same status, Retry-After and logged refusal; litellm 1.103.2 called as run_msa calls it (openai/ prefix, api_base) raises the same exception for each (APIError for every 403 it sends, InternalServerError, BadGatewayError, ServiceUnavailableError, Timeout, RateLimitError), so mini-swe-agent 2.4.6 retries or aborts as before; the direct loop reads only the status; and no Roko classifier (error_classify.rs, provider_error_kind, map_provider_error, classify_agent_crash) matches a phrase the new words hold and the old did not, by a scan of its 30196 Rust string literals (\"does not exist\" is classify_agent_crash's missing model, hence \"is not available\"). One difference by design: the old cap messages carried counts, so a bound holding 429 made Roko's provider_error_kind count a rate limit; the new words hold no number. Bench pytest: 810 passed, 16 skipped (13 with no roko binary in the worktree, the analysis stack, two opt-in sweeps), against 809 passed and the same 16 skipped on main. Filed bug-2224eb (p3: an injected fault carries none of its upstream's own headers, such as x-request-id, and the same OpenAI error shape for every upstream, so an agent comparing it with forwarded answers can tell it apart)."
+++

## Problem

The metering proxy (`faultproxy.py`) answers what it refuses or fails on purpose in the OpenAI error shape, and the
message reaches the harness that called it (Roko, mini-swe-agent, the direct loop's provider client) and, in the Roko
and mini-swe-agent arms, the agent's own shell, whose sandbox admits the proxy's port:

- a call refused at the task's input-token cap names the task key: `task 'F8-l3-0038.s1' could pass its input-token
  cap of 1000: 0 counted, and this call may add up to 10328` (code `input_token_cap`); the per-attempt cap
  (`conversation <n> of task '<key>' ...`) and the unbounded-input refusal name it the same way. An agent can get
  one on purpose: one oversized request to the proxy's `/chat/completions` from its shell;
- an injected fault says it was injected: `injected rate limit` (429) and `injected upstream error` (5xx), so a harness
  that shows a provider error to its model, or an agent that reads one, learns that the `provider_fault`
  disturbance is at work, which it is meant to hide (S08 §4.6);
- the policy's model refusal speaks of the run: `the proxy forwards only this run's models (...)`.

To see it (2026-10-17, on bug-d68575's branch): a `FaultProxy` with a stub upstream, `configure(task="F8-l3-0038.s1",
input_token_cap=1000)` and a 10 KB request: 403 with the message above. With the profile `rate_limit` (p = 1): 429
`injected rate limit`; with `http_5xx`: 500 `injected upstream error`.

## Why it matters

Goal `proof`. The task key names the family (an F8 instance is a honeypot) and the ladder level, and under Will's
decision 9 the coordinator decided on 2026-10-17 that no benchmark-internal identifier reaches an agent
(bug-3c921e's Decision). A fault that announces itself is no disturbance: H6 measures how the arms take a provider's
errors, not a benchmark's. Before the pre-registration lock (gap-394f28), since it changes what the agents see.

## Where

- `benchmarks/viabilitybench/driver/faultproxy.py::FaultProxy._serve`: the `_error(...)` answers for
  `unbounded_input`, `input_token_cap`, `attempt_input_cap`, `http_5xx` and `rate_limit`.
- `faultproxy.py::FaultProxy._policy`: `model_not_allowed`'s message.

## Current state

bug-d68575 took the benchmark's name out of the error types (`vb_cap_exceeded` and the others); the messages are as
they were.

## Plan

1. Word each answer as a provider's own: for example "this request exceeds the input limit", "rate limit exceeded",
   "the server had an error while processing your request", "model not available"; keep the detail (the task, the
   counts, the fault) in the proxy's log row, which only the driver reads.
2. Check that no runner reads these messages (the meter reads the log, `vb.py::_mark_faults` the log too).
3. Add `test_no_proxy_answer_names_the_task_or_a_fault` to `driver/test_faultproxy.py`: every refusal's and every
   fault's body names neither the task key nor `injected`, `fault`, `run` or `task`.

## Done when

- [x] No answer of the metering proxy names the task, the run, or that a fault was injected.
- [x] The `[[verify]]` command passes.

## Notes

- Found by the bug-d68575 worker, 2026-10-17, while taking the benchmark's name out of the proxies' answers.
- The egress proxy's answers (`egress.py`) name neither the task nor a fault; bug-d68575 made them neutral.
- Fixed 2026-10-17 (faultproxy's "What a client is told"). Every answer the proxy makes itself is in a provider's
  words, in OpenAI's error shape with `param`: a refusal is a 403 `permission_error` (`REFUSALS`: "This request exceeds
  the input limit.", "The model `x` is not available.", ...), an `http_5xx` fault a `server_error` ("The server had an
  error while processing your request. Sorry about that!"), a `rate_limit` fault a 429 `requests` /
  `rate_limit_exceeded` ("Rate limit exceeded. Please try again later.") with its `Retry-After`. The old messages,
  with the task, the counts and the caps, are the log's new `detail` field, which `analysis/report.py`'s
  `PROXY_FIELDS` never publishes. A control request without the token now gets the 404 of a route that does not
  exist, not a 401 that named "the control endpoint", so an agent cannot confirm `/_vb/control` is there.
- How each harness takes the answers is unchanged, checked old against new on every refusal, both faults (5xx at
  500, 502, 503, 504) and an unreachable upstream: the same status and `Retry-After`; through litellm 1.103.2 called
  as `run_msa` calls it (`openai/` prefix, `api_base`), the same exception (APIError for every 403, InternalServerError,
  BadGatewayError, ServiceUnavailableError, Timeout, RateLimitError) and so the same retry-or-abort in mini-swe-agent
  2.4.6; the direct loop reads only the status. Roko's classifiers (`error_classify.rs`, `provider_error_kind`,
  `map_provider_error`, `agent_exec::classify_agent_crash`) match words in the raw body, so the words avoid every one
  of theirs (a scan of the Rust string literals each old and new text holds): "does not exist", OpenAI's own wording
  for an unknown model, is the crash classifier's missing model, hence "is not available". `test_faultproxy.py`'s
  `HARNESS_WORDS` pins these.
- One difference by design: the old cap messages carried the counts, so a bound such as 14290 put "429" in the text,
  which Roko's `provider_error_kind` reads as a rate limit before it reads the "403" as an auth failure. The new
  answers carry no number.

## Decision (2026-10-17)

Whether what the metering proxy answers a client may name the task, the run, a refusal or an injected fault had no
recorded default. Under Will's decision 9 of 2026-10-15 (technical questions with no recorded default take the
coordinator's recommendation, the conservative option, recorded on the item with the reasoning; Will can veto it
later), the coordinator decided that no benchmark-internal identifier reaches an agent: every answer the proxy makes
itself is worded as a provider's own and names neither the task key, the run, the refusal, a cap nor that a fault was
injected, and the reason stays in the proxy's log. The change lands before the pre-registration lock (gap-394f28).

Why: benchmark validity, and contamination through the public repository. The task key names the family (an F8
instance is a honeypot) and the ladder level, and an agent could get it on purpose with one oversized request from
its shell; a model refusal listed the run's models, a `model_swap` disturbance's target among them; and a fault that
says it was injected is no disturbance, since H6 measures how the arms take a provider's errors (S08 §4.6). The key,
the refusal names and the old sentences are also search keys into the public repository, where the driver, the
generators and the truth suites are. Neutral words are the conservative option: the statuses, the headers, the meter,
the log's refusals and faults and how each harness classifies an answer are as before, so nothing an agent is graded
on changes, only what it can read.
