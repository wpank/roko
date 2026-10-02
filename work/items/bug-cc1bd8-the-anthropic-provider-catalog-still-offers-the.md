+++
id = "bug-cc1bd8"
kind = "bug"
title = "The anthropic provider catalog still offers the retired claude-haiku-3-5"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-core/provider_catalog"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "c7560e213"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-0c0747"
anchors = ["crates/roko-core/src/provider_catalog.rs", "crates/roko-core/src/config/model_registry.rs::cheapest_builtin_model", "crates/roko-cli/src/commands/config_cmd.rs::test_provider_credit", "crates/roko-cli/src/doctor.rs::probe_provider_credit"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-0c0747"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -n 'claude-haiku-3-5' crates/roko-core/src/provider_catalog.rs && ! git grep -nE 'claude-haiku-3-5|claude-3-5-haiku-20241022' -- crates/roko-cli/src docs && grep -qw 'fn the_cheapest_builtin_anthropic_model_is_haiku' crates/roko-core/src/config/model_registry.rs && cargo test -p roko-core --lib the_cheapest_builtin_anthropic_model_is_haiku"
+++

## Problem

The anthropic entry in `provider_catalog.rs` still offers claude-haiku-3-5, which Anthropic has retired, so `roko config providers add` can write a model that no longer serves.

## Plan

Remove it, and check the other catalog models against the providers' current model lists.

## Done when

- The verify passes.

## Notes

- Reported on 2026-10-01 by wk-model-truth, working on bug-0c0747, during the evening close-out round.
- 2026-10-02 (wk-climain): implemented on work/bug-28c193; cargo verification deferred to the batch check. The scope
  was widened by the coordinator to the credit probes and the docs.
  - Catalog: `claude-haiku-3-5` (Claude Haiku 3.5, `claude-3-5-haiku-20241022`, retired 2026-02-19) is replaced by
    `claude-haiku-4-5`. The row copies the registry's `BUILTIN_MODELS` row: 200K context, 8,192 output tokens,
    tools and vision, no thinking flag. It lists no cost because `BUILTIN_PRICING` prices it (bug-0c0747's rule).
  - Probes: `roko config providers health --check-credits` (`commands/config_cmd.rs::test_provider_credit`) and
    `roko doctor`'s credit check (`doctor.rs::probe_provider_credit`) requested the retired model. They now request
    `model_registry::cheapest_builtin_model(AnthropicApi)`, the lowest-priced built-in Anthropic model
    (claude-haiku-4-5 today), and skip the probe if the registry has none. Test:
    `the_cheapest_builtin_anthropic_model_is_haiku`.
  - Docs: `docs/v3/05-AGENT.md`, `docs/v3/depth/05-agent/provider-adapters-12.md` and the example TOML in
    `docs/v2/ACP-INTEGRATION-GUIDE.md` name claude-haiku-4-5. The routing example in
    `docs/v3/depth/35-architecture/newcomer-overview.md` named `claude-haiku-3` and the dated Sonnet 4 and Opus 4 IDs;
    it now uses claude-haiku-4-5, claude-sonnet-4-6 and claude-opus-4-6.
  - The verify now also covers the probes, the docs and the new test.
  - Other catalog models: the Anthropic ones (claude-sonnet-4-6, claude-opus-4-6) are current; their catalog limits
    (200K context, 16K and 32K output) are below the models' current 1M/128K, as in the registry, and were left as
    they are. The other providers' models (gpt-4.1, gpt-4.1-mini, o3-mini, gemini-2.5-flash/pro, grok-3,
    llama-3.3-70b variants, moonshot-v1-128k, glm-4-plus, qwen-plus, MiniMax-Text-01, step-2-16k, sonar-pro,
    deepseek-chat) could not be checked against the providers' current lists without a live lookup.
  - The other probe branches still name their own slugs (gemini-2.0-flash, sonar, gpt-4o-mini,
    llama-4-scout-17b-16e-instruct); they are outside this item.
