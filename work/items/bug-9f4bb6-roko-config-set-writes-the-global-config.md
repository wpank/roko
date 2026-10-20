+++
id = "bug-9f4bb6"
kind = "bug"
title = "roko config set writes the global config by default, --project refuses globally defined models and drops comments, and serve --model never reaches served runs"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
size = "M"
subsystem = ["roko-cli/config"]
created = 2026-10-13
updated = 2026-10-13
last_verified = 2026-10-13
source = "hermes-plugin-roko docs walkthrough 2026-10-13 (roko 5de43f9ab)"
anchors = ["crates/roko-cli/src/config_cmd.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn config_set_project_accepts_a_globally_defined_model' crates/roko-cli/ && cargo test -p roko-cli config_set_project_accepts_a_globally_defined_model"
+++

## Problem

Three `roko config set` / `--model` surprises found while writing hermes-plugin-roko's getting-started guide
(2026-10-13, roko `5de43f9ab`):

- `roko config set KEY VALUE` without `--project` writes the global `~/.roko/config.toml`, even when run inside a
  workspace. It prints the path, but a user following "set this in your project" changes every workspace.
- `roko config set --project agent.default_model glm-5-1` is refused ("`roko config validate` would reject it:
  [routing.unresolved_model] agent.default_model references unresolved model 'glm-5-1'") although
  `roko config models list` in the same workspace lists `glm-5-1` (it comes from the global config). When it
  does write, `--project` rewrites `roko.toml` and drops its comments.
- `roko serve --model gpt-5.4-mini` does not reach runs started over `/mcp` or `POST /api/run`; they still route
  by the ladder.

## Why it matters

The commands a guide would use to point a workspace at a model either change the wrong file, refuse a model the
workspace can use, or do nothing for served runs.

## Where

- `crates/roko-cli/src/config_cmd.rs` (set, project validation, serialization).
- `crates/roko-cli/src/commands/serve*.rs` and the runtime that starts served runs (`--model`).

## Plan

1. Validate a `--project` write against the merged config, and keep comments (`toml_edit`).
2. Pass `serve --model` to the runs serve starts, or say in `--help` that it does not apply to them.
3. Consider asking `--global` explicitly when `config set` runs inside a workspace.

## Done when

`config_set_project_accepts_a_globally_defined_model` passes (the `[[verify]]` command).
