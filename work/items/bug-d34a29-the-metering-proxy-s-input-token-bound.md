+++
id = "bug-d34a29"
kind = "bug"
title = "The metering proxy's input-token bound (request bytes + 256) assumes text-only requests; an image by URL can cost more"
status = "open"
triage = "unverified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-bench-fix1's report on bug-c30764)"
anchors = ["benchmarks/viabilitybench/driver/faultproxy.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["bug-c30764", "gap-e003ec"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_the_input_bound_covers_image_parts' benchmarks/viabilitybench/driver/test_faultproxy.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_faultproxy.py -k test_the_input_bound_covers_image_parts -q"
+++

## Problem

bug-c30764 made the metering proxy refuse a call when its input could cross the task's cap. The bound it uses, `_input_bound` (`driver/faultproxy.py:552`, called at :400), is the request's bytes plus `PREAMBLE_TOKENS = 256` (:128). One byte per token is safe for text. An image part sent by URL is a few dozen bytes in the request, but can cost hundreds or thousands of input tokens, so the bound can undershoot, and the cap can be crossed again.

## Why it matters

Pilot benchmark (epic spec-567e52): the caps bound each task's spend. p3, because the pilot's families send text only, so this matters only once an arm sends images.

## Where

`_input_bound` and its call site in `faultproxy.py`.

## Plan

1. Count image parts (`image_url`, base64 data) at a conservative per-image token maximum for the provider's model, or refuse requests with non-text parts when the arm's config says text only.
2. Add `test_the_input_bound_covers_image_parts`.

## Done when

- [ ] The bound is an upper bound for every request the proxy forwards.
- [ ] The `[[verify]]` command passes.
