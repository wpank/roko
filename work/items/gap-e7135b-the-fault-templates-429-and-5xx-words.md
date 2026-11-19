+++
id = "gap-e7135b"
kind = "gap"
title = "The fault templates' 429 and 5xx words still rest on documentation, and the proxy keeps no copy of a real 429 or 5xx a live run meets"
status = "open"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-18
updated = 2026-10-18
last_verified = 2026-10-18
source = "gap-db8dab worker (claude-agent), 2026-10-18: no arm provider documents a way to get a 429 or 5xx on request, so the captures are unknown-model errors"
discovered_from = "gap-db8dab"
anchors = ["benchmarks/viabilitybench/driver/faultproxy.py::_relay_body", "benchmarks/viabilitybench/driver/error_capture.py::mask"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["gap-db8dab", "bug-2224eb"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_the_proxy_keeps_a_masked_copy_of_a_real_429' benchmarks/viabilitybench/driver/test_faultproxy.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_faultproxy.py -k test_the_proxy_keeps_a_masked_copy_of_a_real_429 -q"
+++

## Problem

The words of the providers' 429 and 5xx answers that `faultproxy.UPSTREAM_ERRORS` sends with an injected
`rate_limit` or `http_5xx` fault still rest on documentation and public reports, and nothing would catch the real
ones when they happen. gap-db8dab captured each arm provider's own error on 2026-10-18 (`vb capture-errors`,
`driver/testdata/provider_errors/`), but none of Cerebras, z.ai, Moonshot and OpenAI documents a way to get a 429 or a
5xx on request, and provoking one would abuse the API. So each capture is the provider's error for a model that does
not exist: it settled the envelope, its key order, the layout, the Content-Type and the headers, not the words of a
rate limit or a server error. When a live run meets a real 429 or 5xx, the metering proxy forwards it and logs only its
status (`proxy.jsonl`); its body and headers are gone.

Still open, per the module's notes: Cerebras's 429 `type` (`too_many_requests_error`) and its 5xx words ("Internal
error"); Moonshot's 5xx words; whether z.ai's 429 and 5xx carry a Content-Type (its 400 carries none); and which
headers each provider adds to a 429 (rate-limit headers do not appear on their 404s).

## Why it matters

Goal `proof`. S08 §4.6's `provider_fault` must look like the provider's own error (H6), and a word can change how a
harness takes a fault (bug-2224eb's Decision). Pilots A and B make hundreds of live calls, which will meet some real
429s and 5xx at no extra cost, if they are kept.

## Where

- `benchmarks/viabilitybench/driver/faultproxy.py::_relay_body` (and `_relay_stream`): where an upstream's error
  answer is read and relayed.
- `benchmarks/viabilitybench/driver/error_capture.py::mask`: the masking a kept capture needs (ids, cookies, account
  ids drawn anew; keys never written).

## Current state

`vb capture-errors` and its fixtures exist (gap-db8dab). `test_the_fault_templates_match_captured_errors` holds the
templates to the fixtures' envelope, layout, Content-Type and headers, not to their words.

## Plan

1. When the proxy relays an upstream answer with status 429 or 5xx, write it masked (`error_capture.mask`) to
   `<run_dir>/provider_errors/<provider>-<status>-<n>.json` in the `vb.provider_error/1` shape, at most a few per
   provider and status per run, and never a key.
2. After a pilot, copy any such capture into `driver/testdata/provider_errors/`, and correct `UPSTREAM_ERRORS`'s words
   and headers to it, extending `test_the_fault_templates_match_captured_errors` to compare the words.

## Done when

- [ ] The `[[verify]]` command passes.

## Notes

- Never provoke a 429 or a 5xx to get a capture (Will, 2026-10-18).
