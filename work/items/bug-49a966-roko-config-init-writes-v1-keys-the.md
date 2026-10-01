+++
id = "bug-49a966"
kind = "bug"
title = "roko config init writes v1 keys the core schema rejects"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/config"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "c58c7c2ba"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-7a3527"
anchors = ["crates/roko-cli/src/config_cmd.rs::run_init_wizard", "crates/roko-cli/src/config.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-7a3527"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn init_wizard_output_validates_against_the_core_schema' crates/roko-cli/src/ && cargo test -p roko-cli --lib init_wizard_output_validates"
+++

## Problem

`run_init_wizard` (the `roko config init` wizard) still writes v1 keys that the core schema rejects: `[executor].*`, `tools.prefer_mcp`, `tools.global_denied`, `tools.mcp_timeout_secs`, `[prompt]` and `[[gate]]`. `parse_value_for_key` also still accepts the `executor.*` ones, so a fresh setup produces a config that `roko config validate` flags.

## Plan

Write only current-schema keys, and add a test named `init_wizard_output_validates_*` that runs the wizard's output through the core schema validation.

## Done when

- `cargo test -p roko-cli --lib init_wizard_output_validates` passes.

## Notes

- Reported on 2026-10-01 by the worker on gap-7a3527, during the evening close-out round.
- 2026-10-01 (wk-cfg): implemented on work/bug-ccfa0d; cargo verification deferred to the batch check.
  Premise checked at `c58c7c2ba`: `run_init_wizard` wrote `agent.model` (a v1 alias), `tools.prefer_mcp`,
  `tools.global_denied`, `tools.mcp_timeout_secs`, `prompt.token_budget`, `prompt.role`, `[[gate]]` and six
  `executor.*` keys, none of them in the core schema. It now writes `agent.command`, `agent.args`,
  `agent.default_model`, `budget.prompt_token_budget` (the current home of the token budget), `[[gates.rungs]]`
  (compile and clippy, the commands `roko config migrate` gives the legacy cargo gates), `runner.plan_timeout_secs`
  and `serve.auth.*`. The role text has no current key: the wizard no longer asks for it, `--role` prints a note that
  it is not saved, and `--non-interactive` no longer fills a default role (`commands/config_cmd.rs`).
- Test `init_wizard_output_validates_against_the_core_schema` runs the written file through
  `validate_known_config_paths` and `RokoConfig::from_toml`; `wizard_writes_global_with_yes_flag` checks the new
  keys. The verify now guards the cargo filter with a grep, so it can't pass on a missing test. The `executor.*`
  arms of `parse_value_for_key` go with gap-666ab3. The `--role` flag's help text in `main.rs` still says
  "Pre-set role string": left alone because `main.rs` is wk-climain's area this round.
