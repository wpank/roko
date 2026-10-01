+++
id = "bug-49a966"
kind = "bug"
title = "roko config init writes v1 keys the core schema rejects"
status = "open"
triage = "unverified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/config"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-7a3527"
anchors = ["crates/roko-cli/src/config_cmd.rs::run_init_wizard", "crates/roko-cli/src/config.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-7a3527"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib init_wizard_output_validates"
+++

## Problem

`run_init_wizard` (the `roko config init` wizard) still writes v1 keys that the core schema rejects: `[executor].*`, `tools.prefer_mcp`, `tools.global_denied`, `tools.mcp_timeout_secs`, `[prompt]` and `[[gate]]`. `parse_value_for_key` also still accepts the `executor.*` ones, so a fresh setup produces a config that `roko config validate` flags.

## Plan

Write only current-schema keys, and add a test named `init_wizard_output_validates_*` that runs the wizard's output through the core schema validation.

## Done when

- `cargo test -p roko-cli --lib init_wizard_output_validates` passes.

## Notes

- Reported on 2026-10-01 by the worker on gap-7a3527, during the evening close-out round.
