+++
id = "bug-49a966"
kind = "bug"
title = "roko config init writes v1 keys the core schema rejects"
status = "done"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/config"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-7a3527"
anchors = ["crates/roko-cli/src/config_cmd.rs::run_init_wizard", "crates/roko-cli/src/config.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-7a3527"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn init_wizard_output_validates_against_the_core_schema' crates/roko-cli/src/ && cargo test -p roko-cli --lib init_wizard_output_validates"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:12Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:59:25Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
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
