+++
id = "bug-4e5731"
kind = "bug"
title = "ModelCallService prices a call by the model name it was given, so a [models.*] key that is not its slug costs $0 (the episode distiller's rows)"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-agent/model_call_service", "roko-cli/learning"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "bug-a0432b live check, 2026-10-16 (tmp/hermes-roko-v5/findings/roko-planner.md, P9)"
anchors = ["crates/roko-agent/src/model_call_service.rs::cost_predict", "crates/roko-agent/src/task_runner.rs::CostTable", "crates/roko-cli/src/learning_helpers.rs::distillation_model_caller_on"]
links = { depends_on = [], blocks = [], related = ["bug-a0432b", "bug-1bc222"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_model_key_is_priced_at_its_slugs_rate' crates/roko-agent/src && cargo test -p roko-agent --lib -- a_model_key_is_priced_at_its_slugs_rate"
+++

## Problem

`ModelCallService` prices a call by the model name it was given. When that name is a `[models.<key>]` key that is
not the model's slug, nothing prices it and the call is recorded at $0.

Seen in bug-a0432b's live check (2026-10-16, scratch project with `[models.gpt-5-4-mini]`, slug `gpt-5.4-mini`, on
OpenAI): a chat host's plan generation wrote two rows to `.roko/learn/costs.jsonl`:

| role | model | in / out tokens | cost_usd |
|---|---|---|---|
| strategist (via `agent_exec`) | `gpt-5.4-mini` | 8,321 / 755 | 0.01152 |
| episode-distiller (via `ModelCallService`) | `gpt-5-4-mini` | 12,398 / 204 | 0.0 |

At gpt-5.4-mini's built-in rate the distiller's call cost about $0.01. The same happens whenever
`[agent] default_model`, which the distiller runs on otherwise, is such a key.

## Why it matters

Spend is understated wherever a service call names a model by its key: the distiller after every plan generation
and every persisted episode, and possibly chat sessions, serve's dispatch and gateway, and ACP, which build the same
service (goal `truth`; daily and plan ceilings read these rows).

## Where

- `crates/roko-agent/src/model_call_service.rs`: the attempt loop (`ProviderCallCell::execute`) prices with
  `self.cost_table.calculate(attempt_model, &result.usage)`, and `ModelCallService::cost_predict` the same way.
- `crates/roko-agent/src/task_runner.rs::CostTable`: rows are keyed by slug (`from_config_with_defaults` inserts
  `profile.slug`), and `calculate` falls back to `builtin_pricing`, also by slug.
- `crates/roko-cli/src/learning_helpers.rs::distillation_model_caller_on`: builds the distiller's service with the
  default (empty) cost table, so a configured price for a custom model is not used either.

## Plan

1. In `ModelCallService`, price with the resolved model's slug (the `[models.<key>]` entry's `slug` when the name
   is a key), in the attempt loop and in `cost_predict`.
2. Give the distiller's service `CostTable::from_config_with_defaults(&config.models)`.
3. Check the other `ModelCallService::new` callers (chat_session, dispatch_v2, serve_runtime, roko-serve dispatch,
   gateway, providers, ACP) for the same gap.

## Done when

`[[verify]]` passes: a service whose model key differs from its slug prices a call at the slug's rate.
