+++
id = "bug-d5051e"
kind = "bug"
title = "roko config set still accepts v1 tools.* and prompt.* keys the core schema rejects"
status = "open"
triage = "unverified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/config"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-49a966"
anchors = ["crates/roko-cli/src/config.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-49a966"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib config_set_rejects_v1_keys"
+++

## Problem

`parse_value_for_key` still accepts the v1 `tools.prefer_mcp`, `tools.global_denied`, `tools.mcp_timeout_secs` and `prompt.token_budget`, `prompt.role`, `prompt.files` arms, which the core schema rejects. Only `config show` reads those CLI-side fields. main.rs's `--role` help text still says "Pre-set role string".

## Plan

Reject them with the REMOVED_CONFIG_KEYS message, delete the CLI-side fields, and fix the help text. Add a test named `config_set_rejects_v1_keys`.

## Done when

- `cargo test -p roko-cli --lib config_set_rejects_v1_keys` passes.

## Notes

- Reported on 2026-10-01 by wk-cfg, working on bug-49a966, during the evening close-out round.
