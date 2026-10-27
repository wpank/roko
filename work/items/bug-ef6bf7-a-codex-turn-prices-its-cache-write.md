+++
id = "bug-ef6bf7"
kind = "bug"
title = "A Codex turn prices its cache-write tokens at the uncached input rate"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-agent/codex"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "gap-ceffb3 price research, 2026-10-27 (OpenAI prompt-caching guide; config/prices/2026-09-28.probes.md)"
discovered_from = "gap-ceffb3"
anchors = ["crates/roko-agent/src/provider/codex_cli/stream.rs::estimate_codex_cost", "benchmarks/viabilitybench/driver/run_codex.py::turn_usage"]
links = { depends_on = [], blocks = [], related = ["gap-ceffb3", "gap-8672dd"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_codex_turns_cache_writes_are_priced_at_the_write_rate' crates/roko-agent/src/ && cargo test -p roko-agent a_codex_turns_cache_writes_are_priced_at_the_write_rate"
+++

## Problem

A Codex turn's API-equivalent cost leaves out the cache-write premium. `estimate_codex_cost` prices
`input_tokens - cached_input_tokens` as uncached input, `cached_input_tokens` as cache reads and `output_tokens` as
output, with no cache writes (`cache_write_5m: 0`, `cache_write_1h: 0`), and `CodexUsage` does not read
`turn.completed.usage.cache_write_input_tokens`, which Codex 0.152.0 reports (`config/prices/2026-09-28.probes.md`).
For GPT-5.6 models OpenAI bills a cache write at 1.25 times the input rate for its default 30-minute lifetime, or
2.5 times for 12 hours (developers.openai.com/api/docs/guides/prompt-caching), and a write is not a cached read, so
those tokens fall in `input_tokens - cached_input_tokens` and are priced at the input rate: on gpt-5.6-sol, $4 per
million written tokens instead of $5 (30 minutes) or $10 (12 hours). The bench's `run_codex.turn_usage` drops the
field the same way.

## Why it matters

Goal: truth. `config/prices/2026-10-27.toml` (gap-ceffb3) gives roko.toml's Codex model, gpt-5.6-sol, and the models
Codex serves on a ChatGPT login (gpt-5.6-terra, gpt-5.6-luna) rows with these write rates, so every Codex turn that
writes the cache now gets an understated `api_equiv_usd`.

## Where

- `crates/roko-agent/src/provider/codex_cli/stream.rs::CodexUsage` and `::estimate_codex_cost`: the roko side.
- `benchmarks/viabilitybench/driver/run_codex.py::turn_usage`: the bench side (`price_turns` prices what it returns).

## Current state

Both measured probe turns of 2026-10-16 reported `cache_write_input_tokens` 0, so how often real runs write the cache
is not known. That the written tokens are inside `input_tokens` follows from OpenAI's usage docs (`cached_tokens`
excludes tokens charged as writes, and `cache_write_tokens` is a part of the input count); no probe has shown it yet.

## Plan

1. Read `cache_write_input_tokens` into `CodexUsage`, take it out of the uncached input, and price it at the row's
   `cache_write_5m`, which holds OpenAI's 30-minute rate for GPT-5.6 models (prices-2026-10-27's header). Codex
   reports no split by lifetime; if it ever does, put the 12-hour writes in `cache_write_1h`.
2. Do the same in `run_codex.turn_usage` (`tokens_cache_write_5m`).
3. Test a turn that writes the cache on gpt-5.6-sol.

## Done when

`a_codex_turns_cache_writes_are_priced_at_the_write_rate` passes (the `[[verify]]`).

## Notes

A cheap way to confirm step 1's premise: one Codex probe whose prompt repeats a long prefix, run as the 2026-10-16
probes were (subscription, $0 billed), then compare `input_tokens` with the sum of the other counts.
