+++
id = "gap-deb2a8"
kind = "gap"
title = "[runner] log_prompts and prompt_log_retention are documented, but nothing reads them"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-10-08
updated = 2026-10-08
last_verified = 2026-10-08
source = "stash-triage-2026-10-08 (roko-worktree-archive/2026-10-08-stash-triage/TRIAGE.md)"
discovered_from = "archive/stash-2026-09-21-main-2"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::dispatch", "crates/roko-core/src/config/schema.rs::CoreRunnerConfig"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'fn log_prompts' crates/roko-cli/src/graph_task_dispatch/prompt_log.rs && cargo test -p roko-cli --lib graph_task_dispatch::prompt_log"
+++

## Problem

`[runner] log_prompts` and `[runner] prompt_log_retention` are documented in `CoreRunnerConfig`
(`crates/roko-core/src/config/schema.rs`) and set in `roko.toml`, and `/.roko/prompt-logs/` is gitignored, but no
code read either key: turning `log_prompts` on wrote nothing.

## Why it matters

The key exists to debug unexpected agent behaviour by reading the exact prompt an attempt sent. A documented switch
that does nothing misleads whoever turns it on.

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::dispatch` (batch path) and
  `crates/roko-cli/src/graph_task_dispatch/streaming.rs::dispatch_streaming`, where each attempt's request is built.
- `crates/roko-core/src/config/schema.rs::CoreRunnerConfig::log_prompts`.

## Current state

The 2026-09-21 stash `archive/stash-2026-09-21-main-2` had a `write_prompt_log` for the batch path only, writing to
`.roko/learn/prompt-logs/` (triage: `salvage-02-runner-log-prompts.patch`).

## Plan

A `prompt_log` module: with the key on, every attempt writes its system and user prompt, secrets redacted by
`LogScrubber`, to `.roko/prompt-logs/<plan>-<task>-<unix ms>.txt` before its agent runs, on both dispatch paths, and
prunes the directory to `prompt_log_retention` files, oldest first.

## Done when

- [ ] With `log_prompts = true`, each attempt leaves one redacted prompt file; the count stays within the retention.
- [ ] `[[verify]]` passes.
