+++
id = "gap-6bdfc2"
kind = "gap"
title = "This repo's roko.toml sets no data-model boundary ([agent.data_llm] or [serve.mcp] allow_without_data_llm), so roko refuses every run Hermes starts in the roko repo"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-core/config", "roko-serve/mcp"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-39"
anchors = ["roko.toml", "crates/roko-core/src/config/serve.rs::chat_run_refusal", "crates/roko-core/src/config/agent.rs::DataLlmConfig"]
links = { depends_on = [], blocks = [], related = ["gap-cf427e", "dec-648cce"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c \"import tomllib,sys; c=tomllib.load(open('roko.toml','rb')); sys.exit(0 if (c.get('agent',{}).get('data_llm') or {}).get('model') else 1)\""
+++

## Problem

A run a chat host (Hermes) starts over `/mcp` is refused before it is queued unless the workspace sets the
data-model boundary: `[agent.data_llm]`, naming a model roko calls over an API with its key set, or
`[serve.mcp] allow_without_data_llm = true` (`ServeConfig::chat_run_refusal`, `crates/roko-core/src/config/serve.rs:277`;
`ChatRunBlocker::NoDataBoundary`). This repo's `roko.toml` sets neither (its `[agent]` section has no `data_llm`
table, and it has no `[serve.mcp]`), and `~/.roko/config.toml` sets neither, so every run Hermes starts in the roko
repo is refused with "a run from a chat host needs the data-model boundary". Will's own hello-world from Hermes
failed for exactly this reason (memory `hermes_plugin_roko.md`); `plans/demos/README.md:106-110` tells users to add
one by hand. Status-quo assessment 2026-10-15 at 7b6a5af44, finding 39
(`tmp/status-quo/2026-10-15/backlog/A10-91-92-domains-parked.md` lines 133-135).

## Why it matters

Goal `hermes`: running roko's own plans from Hermes in the roko repo is the demo. The refusal is correct fail-closed
behaviour; the gap is that the repo that develops roko doesn't configure the boundary its own docs require.

## Where

- `roko.toml` (`[agent]` at line 15; `[serve]` at line 464).
- `crates/roko-core/src/config/agent.rs::DataLlmConfig` (default model `claude-haiku-4-5`) and
  `crates/roko-core/src/config/serve.rs::chat_run_refusal`.

## Current state

Since 2026-10-15 the boundary also needs `[agent.data_llm]` to name a model roko can call over an API with its key
set; the default, `claude-haiku-4-5`, needs `ANTHROPIC_API_KEY`, which this machine's runs don't use (Claude runs
go through the CLI subscription).

## Plan

Options (the conservative one is recommended under Will's 2026-10-15 decision 9; he can veto):

1. **(recommended)** Add `[agent.data_llm]` to `roko.toml` with `model` set to a `[models.*]` key the cheap ladder
   already calls over an API (for example the Cerebras `gpt-oss-120b` entry the demos ran on). It keeps the screen.
   A contributor without that provider's key still gets the refusal, which names the fix.
2. `[serve.mcp] allow_without_data_llm = true`: runs start, but untrusted tool output reaches the main model
   unscreened. Not recommended.
3. Leave `roko.toml` alone and set `[agent.data_llm]` in the user's global config. Per-user, but invisible to other
   clones; back up `~/.roko/config.toml` first (memory `roko_config_set_global_default.md`).

Then start a run from Hermes (or `POST /mcp` with the plugin's fake host) in this repo and check it is accepted.

## Done when

The `[[verify]]` command passes and a chat-host run in this repo is accepted (`roko config validate` clean).

## Notes

Don't weaken the refusal itself. gap-cf427e covers refusing a run up front when no tool-loop model can run it.
