+++
id = "gap-362b00"
kind = "gap"
title = "roko demo warm caches nothing: roko_agent::file_cache has no caller, so ROKO_DEMO_CACHE does nothing"
status = "open"
triage = "verified"
severity = "p3"
size = "S"
subsystem = ["roko-cli/demo"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "gap-171238 (2026-10-27)"
discovered_from = "gap-171238"
anchors = ["crates/roko-cli/src/demo_cmd.rs::cmd_demo_warm", "crates/roko-agent/src/file_cache.rs::global_file_cache"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-171238"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn demo_warm_writes_the_cache' crates/roko-cli/ && cargo test -p roko-cli demo_warm_writes_the_cache"
+++

## Problem

`roko demo warm` (`crates/roko-cli/src/demo_cmd.rs::cmd_demo_warm`; its help says "Pre-warm the LLM
response cache with demo prompts") sends five fixed demo prompts, one live model call each through
`run::run_once` and `serve_runtime::dispatch_bench_prompt`, prints `cached` after each call that
returns, and then prints how many files `.roko/demo-cache/` holds. Nothing writes that directory:

- the file cache it counts, `roko_agent::file_cache` (`FileCache`, `global_file_cache`,
  `prompt_cache_key`), has no caller outside its own module and tests;
- its module doc says `ROKO_DEMO_CACHE=1` or `--demo-cache` turns it on, but no `--demo-cache` flag
  exists, and the only reader of `ROKO_DEMO_CACHE` is `FileCache::is_enabled`, which nothing calls;
- `cmd_demo_warm` tells the operator to set `ROKO_DEMO_CACHE=1` first, in a code comment only.

So the five calls cost what live calls cost, cache nothing, and the printed count is whatever the
directory held before. `roko config env` lists `ROKO_DEMO_CACHE` ("Enable file caching for demo
scenarios", `crates/roko-core/src/config/env_registry.rs`), and setting it changes nothing.

`warm` also runs under `crate::config::Config::default()` instead of the workspace's config: only
`[routing]` is read from `roko.toml` (in `dispatch_bench_prompt`), so the model it calls is the
built-in default, not the one the demo will use.

How to see it: `grep -rnE 'global_file_cache|FileCache::open|prompt_cache_key' crates --include=*.rs | grep -v crates/roko-agent/src/file_cache.rs`
prints nothing.

## Why it matters

A demo prepared with `roko demo warm` is neither deterministic nor free to replay, and the command's
output says it is cached. It is the "built but never connected" pattern of CLAUDE.md rule 2.

## Where

- `crates/roko-cli/src/demo_cmd.rs::cmd_demo_warm`: the five prompts, `Config::default()`, the count
  of `.roko/demo-cache/`.
- `crates/roko-agent/src/file_cache.rs`: `FileCache`, `global_file_cache` (reads `ROKO_DEMO_CACHE`),
  `prompt_cache_key`.
- `crates/roko-cli/src/serve_runtime.rs::dispatch_bench_prompt` and `call_bench_model`: the
  single-prompt path where a lookup and a store would go.
- `crates/roko-core/src/config/env_registry.rs`: the `ROKO_DEMO_CACHE` entry.

## Current state

Checked 2026-10-27 at 0191f7858. `docs/v3/28-CLI.md` §`roko demo` (gap-171238) describes the
behaviour above and cites this item.

## Plan

- Option A, wire it: in the single-prompt path, when the cache is on, look up
  `prompt_cache_key(model, prompt)` in `FileCache::open_default(workdir)` before the model call and
  store the response after; have `warm` turn the cache on for its own calls (without touching the
  process environment) and use the workspace's config.
- Option B, remove it: if demos no longer need a response cache, delete `warm`, `file_cache.rs` and
  the `ROKO_DEMO_CACHE` entry, and update 28-CLI.md.

A keeps what the help text promises; B is less code. Ask before choosing B.

## Done when

`roko demo warm` against a fake provider leaves one `.roko/demo-cache/` entry per prompt, and a
second run makes no provider call: test `demo_warm_writes_the_cache` passes. (Option B retires this
verify instead.)

## Notes

Found while writing 28-CLI.md's `roko demo` section (gap-171238). Use a fake provider, never a paid
one, in the test.
