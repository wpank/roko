+++
id = "bug-fd8200"
kind = "bug"
title = "The metering proxy's answers tell a client its task key and that a fault was injected: cap refusals say task 'F8-l3-0038.s1' could pass its input-token cap, injected faults say 'injected rate limit' and 'injected upstream error', and the policy speaks of this run's models"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "bug-d68575 worker (claude-agent), 2026-10-17 (a FaultProxy with a stub upstream, configure(task='F8-l3-0038.s1', input_token_cap=1000) and a 10 KB request: 403 naming the key; profiles rate_limit and http_5xx at p=1: 429 'injected rate limit', 500 'injected upstream error')"
discovered_from = "bug-d68575"
anchors = ["benchmarks/viabilitybench/driver/faultproxy.py::_serve", "benchmarks/viabilitybench/driver/faultproxy.py::_policy"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-d68575", "bug-3c921e", "gap-394f28"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_no_proxy_answer_names_the_task_or_a_fault' benchmarks/viabilitybench/driver/test_faultproxy.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_faultproxy.py -k test_no_proxy_answer_names_the_task_or_a_fault -q"
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

- [ ] No answer of the metering proxy names the task, the run, or that a fault was injected.
- [ ] The `[[verify]]` command passes.

## Notes

- Found by the bug-d68575 worker, 2026-10-17, while taking the benchmark's name out of the proxies' answers.
- The egress proxy's answers (`egress.py`) name neither the task nor a fault; bug-d68575 made them neutral.
