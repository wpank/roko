+++
id = "bug-4e336b"
kind = "bug"
title = "roko init writes a model ladder whose lower rungs name models the workspace cannot run, so every task starts on the top rung; init --model is ignored"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/init"]
created = 2026-10-13
updated = 2026-10-15
last_verified = 2026-10-15
last_verified_rev = "7b6a5af44"
source = "hermes-plugin-roko docs walkthrough 2026-10-13 (roko 5de43f9ab)"
anchors = ["crates/roko-cli/src/commands/init.rs::InitModels", "crates/roko-cli/src/commands/init.rs::render_init", "crates/roko-cli/src/commands/util.rs::cmd_init", "crates/roko-cli/src/dispatch/model_routing.rs::RoutingLadder", "crates/roko-cli/src/dispatch/model_routing.rs::ladder_report", "crates/roko-core/src/agent.rs::resolve_model", "crates/roko-cli/src/doctor.rs::check_routing_ladder", "crates/roko-cli/src/commands/setup.rs::ensure_global_env"]
links = { depends_on = [], blocks = [], related = ["gap-1625c0", "gap-cf427e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn init_writes_a_runnable_ladder_and_honours_model' crates/roko-cli/ && cargo test -p roko-cli --lib init_writes_a_runnable_ladder_and_honours_model"

[[verify]]
command = "grep -rqw 'fn ladder_binds_builtin_rungs_and_reports_the_rest' crates/roko-cli/ && cargo test -p roko-cli --lib ladder_binds_builtin_rungs_and_reports_the_rest"
+++
## Problem

`roko init` writes D11's ladder whatever the machine has: `cheap = gpt-oss-120b`, `mid = glm-4.7`,
`strong = gpt-5.4-mini`, `top = claude-sonnet-4-6`. It defines only `[models.claude-sonnet-4-6]` (on
`claude_cli` when `claude` is on PATH), so the ladder, which keeps only runnable rungs, starts every task on
`top`, a CLI agent. Seen again 2026-10-15 at `7b6a5af44` on Will's machine, whose `~/.roko/.env` sets
CEREBRAS, ZAI, OPENAI and MOONSHOT keys: none of them reached the ladder. Even `gpt-5.4-mini`, a built-in model
(`roko config models list` lists it as "available without config"), never reaches a ladder: the rung binding
reads only `[models.*]`, so its rung is skipped with "no [models.*] entry", and a run only logs that. `roko init --model glm-5-1` changes
nothing either: `cmd_init` never receives the global `--model` flag, and `default_model` stays
`claude-sonnet-4-6`.

## Why it matters

A new workspace routes everything to its most expensive rung. A run a chat host (Hermes) starts carries the
`stage` outbound policy, which only roko's own tool loop honours, so with only a CLI rung such runs cannot run
at all (gap-cf427e adds the up-front refusal). The first runs a newcomer starts from Hermes fail.

## Where

- `crates/roko-cli/src/commands/init.rs`: the template (`render_init_template_for`), which serialized
  `RokoConfig::default()`'s ladder and appended one model block.
- `crates/roko-cli/src/commands/util.rs::cmd_init` and the `Init` dispatch in `main.rs`: `--model` was dropped.
- `crates/roko-cli/src/dispatch/model_routing.rs::RoutingLadder::from_config`: what makes a rung runnable (a
  `[models.*]` entry by key or slug, tool use, provider credentials).
- `crates/roko-core/src/agent.rs::resolve_model`: how dispatch resolves a built-in model (the first provider of
  its kind, which can be Cerebras or Z.ai for an OpenAI model), and `config/schema.rs`'s env-synthesized
  `openai` provider, whose base URL lacked `/v1`.
- `crates/roko-cli/src/commands/config_cmd.rs::cmd_model_list`: the Key column of built-in models.

## Current state

Open at `7b6a5af44`. `InitProvider::detect` only knew Claude: `claude` on PATH, else `ANTHROPIC_API_KEY`. roko
loads `~/.roko/.env` into the process before any command runs, so keys kept there are visible to init.

## Plan

1. Give init a table of providers (API key or CLI) and, per D11 rung, the models that can take it, most
   preferred first: Cerebras gpt-oss-120b, Z.ai GLM-4.7 or Moonshot Kimi K2.6, OpenAI gpt-5.4-mini or
   Anthropic Claude Haiku 4.5, then Sonnet on the Claude CLI or the Anthropic API; OpenRouter serves every rung.
2. Write `[providers.*]`, `[models.*]` and `[routing.ladder]` rungs only for models whose provider this machine
   reaches, with each tier's start rung named, and the top rung's model as `default_model`. With no provider,
   write no rung.
3. Say when no API model has a rung (a chat host's runs need one), and which keys would fill the missing rungs.
4. Thread `--model` into init: it pins every task to that model (a one-rung ladder and `default_model`),
   found by key or slug among init's models, else in `~/.roko/config.toml`; an unknown model fails before
   anything is written.
5. Let a ladder rung name a built-in model: with no `[models.*]` entry, it binds when the built-in can call
   tools and its provider is available, so a workspace an older `roko init` wrote (`strong = gpt-5.4-mini`) gets
   that rung once `OPENAI_API_KEY` is set. Dispatch resolves a built-in to the provider that holds its own key
   first, and the env-synthesized `openai` provider gets the `/v1` base URL its adapter needs.
6. Show the rungs a workspace cannot run, with why: `roko doctor` (`routing_ladder`) and `roko init` in a
   workspace that has a `roko.toml`. Make `roko config models list` say "ok" for a built-in only when the
   provider dispatch would use has its key (a `claude` CLI on PATH counted for the Anthropic API's models).
7. `roko setup` creates `~/.roko/.env`, the file the notes about keys point to, when it is missing (owner-only,
   comments only); `roko init` writes only the workspace.

## Done when

`init_writes_a_runnable_ladder_and_honours_model` and `ladder_binds_builtin_rungs_and_reports_the_rest` pass (the
`[[verify]]` commands): for several machines, every rung init writes names a model it writes on a provider it
configures, a key always gives an API model a rung, `--model X` writes `default_model = "X"` and a ladder pinned
to X, and a rung naming a built-in model runs once its key is set. A fresh `roko init` on Will's machine writes
the four API/CLI rungs his keys allow.

## Notes

- A key added after `roko init` reaches the ladder only through a rung that names a built-in model; rerunning
  init in place is not supported yet (roko-chathost's `roko config preset chat-host` may cover it).
- Behaviour change: a workspace without `[routing.ladder]` uses D11's default ladder, whose `strong` and `top`
  rungs name the built-ins `gpt-5.4-mini` and `claude-sonnet-4-6`. Those rungs now run wherever their key is set,
  so such a workspace's tasks start on them instead of on the router's pick.
- Gemini is left out: its tool loop needs `tool_format = "gemini_native"`, which config validation does not
  list, and no key was available to check it.
- 2026-10-15 (status-quo assessment, finding 9): still open at b42578bab. No commit since 2026-10-13 touches `graph_task_dispatch/verification.rs` or `plan_generate/`, and no other item links here. Seen again in the 10-14 tally runs: all 28 ladder records are rung `top`, reason `start`, because the workspace's cheap, mid and strong rungs (gpt-oss-120b, glm-4.7, gpt-5.4-mini) had no usable provider (`tmp/status-quo/2026-10-15/programme/E2a-matrix-retag-AU-EX-IS-QA-RC.md:168`, row EX7). The 49-run sweep found no climb anywhere (71 ladder records, all step 0). The cheap-model ladder claim (gap-f30b8e, funded by decision 5) can't be shown on a workspace `roko init` made until this is fixed.

## Original notes

### Problem

`roko init` writes a `[routing.ladder]` whose `cheap`, `mid` and `strong` rungs name `gpt-oss-120b`, `glm-4.7`
and `gpt-5.4-mini`, none of which the workspace's `[models]` defines (it defines only `claude-sonnet-4-6` on
`claude_cli`). The ladder keeps only runnable rungs, so every task starts on `top` (seen 2026-10-13, roko
`5de43f9ab`, a fresh Rust crate on Will's machine). `roko init --model glm-5-1` changes nothing either: the
written `default_model` stays `claude-sonnet-4-6`.

### Why it matters

A new workspace routes everything to its most expensive rung, and with only the Claude CLI configured, a chat
host's runs cannot run at all (see the related up-front refusal item).

### Where

- `crates/roko-cli/src/commands/init*.rs` (the template roko.toml and the `--model` flag).
- `crates/roko-cli/src/dispatch/model_routing.rs::RoutingLadder::start`.

### Plan

1. Write ladder rungs only for models the new workspace can run, or none, and honour `roko init --model`.
2. Test: `roko init --model X` writes `default_model = "X"`, and every rung it writes resolves to a configured
   model.

### Done when

`init_writes_a_runnable_ladder_and_honours_model` passes (the `[[verify]]` command).
