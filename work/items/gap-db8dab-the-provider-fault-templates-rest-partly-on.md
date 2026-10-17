+++
id = "gap-db8dab"
kind = "gap"
title = "The provider-fault templates rest partly on documentation and public reports: no captured 429 or 5xx of Cerebras, z.ai, Moonshot or OpenAI checks their bodies, and a fault before an upstream's first answer carries none of its headers"
status = "open"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "bug-2224eb worker (claude-agent), 2026-10-17 (inference-docs.cerebras.ai shows no error body; a captured Cerebras 401 is flat, public reports of its 429s show a message or no body; docs.z.ai lists 1302 for a 429 and no code for an internal error; platform.kimi.ai lists rate_limit_reached_error, server_error and server_unavailable)"
discovered_from = "bug-2224eb"
anchors = ["benchmarks/viabilitybench/driver/faultproxy.py::UPSTREAM_ERRORS", "benchmarks/viabilitybench/driver/faultproxy.py::_mirrored"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-2224eb", "bug-fd8200", "gap-394f28"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_the_fault_templates_match_captured_errors' benchmarks/viabilitybench/driver/test_faultproxy.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_faultproxy.py -k test_the_fault_templates_match_captured_errors -q"
+++

## Problem

bug-2224eb made an injected `http_5xx` or `rate_limit` fault answer in its upstream's own error shape and words
(`faultproxy.UPSTREAM_ERRORS`) and with the headers of the upstream's last forwarded answer (`FaultProxy._mirrored`).
Parts of that rest on sources weaker than a capture of the provider's own 429 and 5xx, and such a capture needs a call
the operator must approve (a 429 or a 5xx; a 401 costs nothing but shows only the error headers):
- Cerebras: the flat `message`/`type`/`param`/`code` shape is that of a captured 401 (a public issue, 2026-01); the
  429's words ("Requests per minute limit exceeded - too many requests sent.") take the form of the per-minute
  message public reports show ("Tokens per minute limit exceeded - too many tokens processed."), the request-rate
  one so that "too many requests" reads as a rate limit to Roko's text classifiers where "too many tokens" would
  read as a context overflow to its CLI-crash one; its `type` (`too_many_requests_error`) is unchecked, `param` and
  `code` are null so that no quota word reaches Roko's billing classifier, and its 5xx words ("Internal error") are
  a placeholder that every harness classifies as it did the old answer.
- z.ai: the 5xx body carries no business `code`, since the code column of its docs' table shows "-" for an internal
  error; the 429's code 1302 and words are documented.
- Moonshot: the 429's type and literal words and the 5xx types are its error reference's; its 5xx words are only
  described there, so they are placeholders as Cerebras's are ("Internal error"; for a 503 "The service is
  unavailable, please try again later", short of "temporarily unavailable" and "server error", which Roko's provider
  health reads as a 30 s cooldown); none of its headers is known.
- Every provider: a fault that comes before its upstream has answered anything in the run carries none of the
  provider's headers, since the proxy has seen none and no capture records them.

## Why it matters

Goal `proof`. S08 §4.6's `provider_fault` must look like the provider's own error: H6 measures how the arms take a
provider's errors, not a benchmark's. A field the provider never sends is a tell to an agent that compares, and a word
it does send can change how a harness takes the fault (Roko reads a quota word as billing and does not retry; it waits
5 s for z.ai's 1302). Under Will's decision 9, nothing benchmark-internal reaches an agent (bug-2224eb's Decision).
Before the pre-registration lock (gap-394f28), since it changes what the agents see.

## Where

- `benchmarks/viabilitybench/driver/faultproxy.py::UPSTREAM_ERRORS`: the per-provider bodies, with what each rests on.
- `benchmarks/viabilitybench/driver/faultproxy.py::FaultProxy._mirrored`: the headers a fault takes from the
  upstream's last forwarded answer, none before it has answered.

## Current state

Since bug-2224eb (2026-10-17): the bodies follow docs.z.ai's error codes, platform.kimi.ai's error reference,
OpenAI's error codes, and a captured Cerebras 401 with public reports of its 429s; the headers are mirrored. No
capture of any provider's own 429 or 5xx exists in the repository.

## Plan

1. With the operator's approval, capture for each arm provider (Cerebras, z.ai, Moonshot, OpenAI) one 429 and one
   5xx, or at least a 401, with the headers and the body, request ids and cookies masked, and keep them as fixtures
   (`driver/testdata/provider_errors/<provider>.json`).
2. Correct `UPSTREAM_ERRORS` to the captures, and give each upstream the capture's headers (fresh ids in the same
   form) for a fault that comes before its first forwarded answer.
3. Add `test_the_fault_templates_match_captured_errors` to `driver/test_faultproxy.py`: each provider's fault body
   has its capture's keys and words, and a fault on a fresh proxy carries its capture's header names.

## Done when

- [ ] Every provider's fault body and headers match a captured error of that provider's.
- [ ] The `[[verify]]` command passes.

## Notes

- Found by the bug-2224eb worker, 2026-10-17, while giving faults their upstream's shape: Cerebras's documentation
  shows no error body, and the public reports of its 429s show either a message or no body at all.
- A capture costs a provider call; get the operator's approval first, and never commit a real key, request id or
  cookie value.
