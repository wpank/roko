+++
id = "gap-ceffb3"
kind = "gap"
title = "No dated price snapshot covers claude-sonnet-4-6 or every model roko.toml routes to"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-core/pricing"]
created = 2026-10-02
updated = 2026-10-15
last_verified = 2026-10-02
source = "backlog wave reports 2026-10-02 (PK12 gap-08120e)"
discovered_from = "gap-08120e"
anchors = ["crates/roko-core/src/pricing_snapshot.rs::pricing_snapshot_builtin_copy_is_the_newest_file", "config/prices/2026-09-28.toml", "roko.toml"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq 'slug = \"claude-sonnet-4-6\"' config/prices/*.toml && cargo test -p roko-core pricing_snapshot_builtin_copy_is_the_newest_file"

[[verify]]
command = "grep -rq 'slug = \"glm-5.1\"' config/prices/*.toml && grep -rqw 'fn pricing_snapshot_builtin_prices_the_default_models' crates/roko-core/src/ && cargo test -p roko-core pricing_snapshot_builtin_prices_the_default_models"
+++

## Problem

`config/prices/` has only one dated snapshot, `2026-09-28.toml`, and it does not price every model `roko.toml`
currently routes to: `roko.toml:129` sets `slug = "claude-sonnet-4-6"` for a routed role, but
`config/prices/2026-09-28.toml` has no `claude-sonnet-4-6` row (its Anthropic entries are `claude-opus-5-5`,
`claude-sonnet-5`, `claude-fable-5-1`, `claude-haiku-4-5`). No open or done backlog item adds a newer snapshot:
`gap-9e3134` (PK13, tasks 2114/2115) and `gap-a0043b` (PK08, task 2112) are about wiring plan runs to the *existing*
snapshot and matching `roko.toml`'s cheap-model rates to it, not about adding a snapshot that actually covers every
routed model.

## Why it matters

Goal: truth. Until a snapshot prices `claude-sonnet-4-6` (and any other routed-but-unpriced model), every call to it
is an unpriced call ($0 cost recorded), which understates spend and — once backlog task 2111's plan-ceiling fix
lands — would also trip whatever rule 2111 applies to unpriced calls.

## Where

- `config/prices/2026-09-28.toml`: the only existing dated snapshot.
- `crates/roko-core/src/pricing_snapshot.rs`: `BUILTIN_SNAPSHOT_ID` (line 35, currently `"prices-2026-09-28"`),
  `BUILTIN_SNAPSHOT_TOML` (line 38, `include_str!("../../../config/prices/2026-09-28.toml")`), and the
  `pricing_snapshot_builtin_copy_is_the_newest_file` test (line 799) that enforces the built-in copy matches whichever
  snapshot file is actually newest on disk.
- `roko.toml:129`: the `claude-sonnet-4-6` routing entry with no corresponding price row.

## Current state

No backlog task or work item adds a new dated snapshot. `pricing_snapshot_builtin_copy_is_the_newest_file` will fail
the moment a newer snapshot file is added without updating `BUILTIN_SNAPSHOT_ID`/`BUILTIN_SNAPSHOT_TOML` to match,
which is the test's purpose; it currently passes only because no newer file exists yet.

## Plan

1. Price every model `roko.toml` currently routes to (at minimum `claude-sonnet-4-6`) and write a new dated
   snapshot, e.g. `config/prices/<today>.toml`, following the existing file's row format (`slug, provider, input,
   cache_read, cache_write_5m, cache_write_1h, output, reasoning_in_output, source_url, verified`).
2. Update `BUILTIN_SNAPSHOT_ID` and `BUILTIN_SNAPSHOT_TOML` in `crates/roko-core/src/pricing_snapshot.rs` to point at
   the new file.
3. Once the new snapshot exists, the tests for backlog tasks 2114/2115 (inside `gap-9e3134`) should name it
   explicitly (e.g. assert against the new dated id) rather than whatever snapshot happens to be newest, per PK07's
   note — flag this to whoever picks up 2114/2115 if this item is still open when they start.

## Done when

- `config/prices/<new-date>.toml` exists and prices every model `roko.toml` routes to, including `claude-sonnet-4-6`.
- `pricing_snapshot_builtin_copy_is_the_newest_file` passes with the new file as the built-in.
- The `[[verify]]` command passes.

## Notes

Don't touch `gap-9e3134`'s or `gap-a0043b`'s own files — this is a separate, prerequisite gap (a snapshot that
actually covers every routed model), not a fix to either package's existing tasks.

2026-10-15 (status-quo assessment at 7b6a5af44, finding 22; tmp/status-quo/2026-10-15/backlog/A02-21-22-honest-measurement.md section 4): still open, and it now blocks the paper's headline metric. `config/prices/` still holds only `2026-09-28.toml`, so `api_equiv_usd` is null on all 72 real-model verdicts on record (70 glm-5.1 via Z.ai, 2 claude-sonnet-4-6) and no real run can compute cost per verified task (epic 2100's live outcome). Decision 2113 Q2 (answered 2026-10-02) asked for exactly this newer snapshot. Widen the scope: `glm-5.1` (provider `zai`; routed by `~/.roko/config.toml` `[models.glm-5-1]` and used by every live Hermes and e2e run) needs a row as well as `claude-sonnet-4-6` (provider `anthropic`, roko.toml:141). The other roko.toml slugs with no row are gemma-4-31b, glm-5v-turbo, gpt-4o, gpt-4o-mini, gpt-5.6-sol, llama-3.3-70b-versatile, qwen-3.8-27b and sonar: price one only where its provider publishes a per-token rate. Sources: copy every rate from the provider's own pricing page fetched on the snapshot date and record that page in `source_url` and the fetch date in `verified`: Anthropic https://platform.claude.com/docs/en/about-claude/pricing (claude-sonnet-4-6, including its cache-read and 5m/1h cache-write rates), Z.ai https://docs.z.ai/guides/overview/pricing (glm-5.1, glm-5v-turbo), OpenAI https://developers.openai.com/api/docs/pricing, and for any other provider its own published pricing page. Never invent, extrapolate or copy a sibling model's rate: a model whose page gives no rate stays out of the file (unknown cost), as the file's header requires. Suggest adding a second verify line: `grep -rq 'slug = "glm-5.1"' config/prices/*.toml` (fails now). Once claude-sonnet-4-6 has a row, the Claude CLI attempts' `cost_source = unknown` with `priced = true` contradiction (finding 23, filed separately) stops showing on the default model, but the labelling bug stays for any unpriced model.

2026-10-27 (done): `config/prices/2026-10-27.toml`, id `prices-2026-10-27`, 20 rows, is the new snapshot and the
built-in copy (`BUILTIN_SNAPSHOT_ID`, `BUILTIN_SNAPSHOT_TOML`). Every rate was read that day from the provider's own
page, which each row's `source_url` names; the ten rows of prices-2026-09-28 were read again and are unchanged.

- roko.toml's routed slugs. Covered: gpt-oss-120b, qwen-3.8-27b, glm-4.7, kimi-k2.6, gpt-5.4-mini, gpt-5.6-sol (the
  Codex model), gpt-4o, gpt-4o-mini, claude-haiku-4-5, claude-opus-5-5, claude-sonnet-4-6. Left out, since no
  provider page gives a per-token rate: gemma-4-31b (Cerebras serves it only on Dedicated Inference), glm-5v-turbo
  (not on Z.ai's pricing page), llama-3.3-70b-versatile (Groq: Enterprise, "Contact Sales") and sonar (Perplexity no
  longer lists Sonar Chat Completions, whose support ended on 2026-09-27). The file's header says so too.
- Defaults in `crates/roko-core/src/config/` and `defaults.rs`: claude-sonnet-4-6 (default model, standard tasks,
  the ladder's top rung), claude-haiku-4-5 (fast tasks, the data model, auto-fix), claude-opus-4-6 (complex tasks),
  the default ladder's gpt-oss-120b, glm-4.7 and gpt-5.4-mini, and ZAI_MODEL's glm-5.1 are covered; the demo-only
  ANTHROPIC_MODEL (claude-sonnet-4-20250514: Claude Sonnet 4 is retired on the Claude API) and OLLAMA_MODEL
  (gemma3:7b, a local model) are not. The built-in registry's o3, o4-mini, codex-mini, gemini-2.5-pro,
  gemini-2.5-flash and sonar-pro, which routing reaches only by name, are not covered and keep their
  `BUILTIN_PRICING` rates.
- Also in the file: glm-5.1 (the model of `~/.roko/config.toml`'s glm-5-1), gpt-5.6-terra and gpt-5.6-luna (what
  Codex serves on a ChatGPT login, for gap-8672dd), gpt-4o-2024-05-13 (which would otherwise take gpt-4o's row), and
  the bench arms' gpt-5.4, gpt-5.5, claude-sonnet-5 and claude-fable-5-1.
- OpenAI's GPT-5.6 cache writes last 30 minutes (the default) or 12 hours, not 5 minutes or 1 hour: their
  `cache_write_5m` holds the 30-minute rate and `cache_write_1h` the 12-hour rate, as the header says. gpt-5.6-sol's
  $4/$20 is a promotional price, offered at least through 2026-11-21.
- Tests that assumed the old built-in copy now name what they test: roko-serve
  `list_models_serves_the_rates_calls_are_priced_at` (o3 for the built-in-table case), roko-agent
  `an_unlisted_codex_model_has_no_cost` (gpt-5.6-sol is priced now, `codex_cost_uses_snapshot` checks it), roko-cli
  `learn_econ_prices_lists_every_snapshot_row` (row count). The backlog 2114/2115 tests of gap-9e3134 already pin
  prices-2026-09-28.
- Filed: gap-67aecd (roko.toml prices five routed models at rates no provider page gives; kimi-k2.6 retires by the
  end of December 2026), bug-ef6bf7 (a Codex turn prices its cache writes at the input rate).
