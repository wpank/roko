+++
id = "bug-d5051e"
kind = "bug"
title = "roko config set still accepts v1 tools.* and prompt.* keys the core schema rejects"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/config"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "f4323cf9d"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-49a966"
anchors = ["crates/roko-cli/src/config.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-49a966"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn config_set_rejects_v1_keys' crates/roko-cli/src/ && cargo test -p roko-cli --lib config_set_rejects_v1_keys"
+++

## Problem

`parse_value_for_key` still accepts the v1 `tools.prefer_mcp`, `tools.global_denied`, `tools.mcp_timeout_secs` and `prompt.token_budget`, `prompt.role`, `prompt.files` arms, which the core schema rejects. Only `config show` reads those CLI-side fields. main.rs's `--role` help text still says "Pre-set role string".

## Plan

Reject them with the REMOVED_CONFIG_KEYS message, delete the CLI-side fields, and fix the help text. Add a test named `config_set_rejects_v1_keys`.

## Done when

- `cargo test -p roko-cli --lib config_set_rejects_v1_keys` passes.

## Notes

- Reported on 2026-10-01 by wk-cfg, working on bug-49a966, during the evening close-out round.
- 2026-10-01 (wk-cfg): implemented on work/bug-ccfa0d; cargo verification deferred to the batch check.
  Correction to the report this item came from: not every CLI-side field was dead. `Config.prompt.role` is the
  agent role label (default `implementer`, set by `--role` and by worker templates) that model selection reads
  (`run.rs`, `run_inline.rs`, `chat_inline/session.rs`, `commands/util.rs`), and `Config.prompt.token_budget` is the
  `roko chat` system-prompt budget that `--effort` sets. Neither was ever read from roko.toml on the core path:
  `Config::from_roko_config` always used the defaults.
- Change: `config set` now refuses any key in `REMOVED_CONFIG_KEYS`, or inside a removed section, with its reason
  (`removed_config_key_reason`, checked first in `parse_value_for_key`). The v1 arms are gone. `REMOVED_CONFIG_KEYS` lists
  `tools.prefer_mcp`, `tools.global_denied` (points to `tools.deny`), `tools.mcp_timeout_secs`, `prompt.token_budget`
  (points to `budget.prompt_token_budget`, which roko-serve reads), `prompt.role` (points to `--role`), `prompt.files`,
  `prompt.budgets` and `prompt.context_budgets`, so old files load with a warning. Deleted the dead CLI-side
  pieces: `Config.tools` with `ToolsConfig`, `ToolsLayer`, `PromptLayer`, and `PromptConfig`'s `files`, `budgets`
  and `context_budgets` with `PromptFile`, `RoleBudget` and `ContextBudgetConfig`. `Config.prompt` is now
  `#[serde(skip)]` and run-time only (`role`, `token_budget`). `config show` stops printing the five v1 lines, and
  `ConfigSources.prompt_token_budget` now tracks `budget.prompt_token_budget` (main.rs's `fully_default` reads it). The
  `config init --role` help, in `main.rs` and `docs/v2/CLI-REFERENCE.md`, says the flag is ignored.
- Tests: `config_set_rejects_v1_keys` (roko-cli) and `removed_config_key_reason_covers_keys_and_sections`
  (roko-core). `old_executor_section_still_parses` now also covers v1 `[tools]` and `[prompt]` keys, and the tests
  that set or read the v1 fields moved to `tools.deny`, `budget.prompt_token_budget` or `agent.model`.
