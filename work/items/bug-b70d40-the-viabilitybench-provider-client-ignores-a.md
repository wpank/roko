+++
id = "bug-b70d40"
kind = "bug"
title = "The ViabilityBench provider client ignores a top-level cached_tokens and prices that input at the full rate"
status = "open"
triage = "unverified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-bench-proxy's report on gap-e003ec)"
anchors = ["benchmarks/viabilitybench/driver/provider.py::_usage", "benchmarks/viabilitybench/driver/ledger.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-e003ec", "bug-b72a37"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_usage_reads_a_top_level_cached_tokens' benchmarks/viabilitybench/driver/test_driver.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_driver.py -k test_usage_reads_a_top_level_cached_tokens -q"
+++

## Problem

`driver/provider.py::_usage` (:146-156) reads cached input only from `usage.prompt_tokens_details.cached_tokens`, the OpenAI layout. Some OpenAI-compatible providers report cached input as a top-level `usage.cached_tokens`; wk-bench-proxy met the case while building the metering proxy. `_usage` ignores that field, so the driver treats the cached tokens as uncached input and prices them at the full input rate.

## Why it matters

Pilot benchmark (epic spec-567e52): cost per verified success is the headline metric, and cached input is often much cheaper. An inflated cost on one provider biases every comparison that involves it. The proxy's meter and the driver's ledger then also disagree.

## Where

`_usage` and `_count` in `driver/provider.py`, and whatever prices `Usage.cached_tokens` in the ledger.

## Current state

At ad391f99a only the nested field is read.

## Plan

1. Read `cached_tokens` from `prompt_tokens_details`, or else from the top level, and cap it at `prompt_tokens` as today. Note which layout each provider in the price snapshot uses.
2. Make the metering proxy's parser agree (gap-e003ec's `faultproxy.py`), so the meter and the ledger match.
3. Add `test_usage_reads_a_top_level_cached_tokens`.

## Done when

- [ ] A response with a top-level `cached_tokens` is priced with the cached rate for those tokens.
- [ ] The `[[verify]]` command passes.

## Notes

- bug-b72a37 is the same class of error in Roko's own OpenAI-compatible providers.
