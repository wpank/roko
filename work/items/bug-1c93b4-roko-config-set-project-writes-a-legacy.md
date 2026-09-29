+++
id = "bug-1c93b4"
kind = "bug"
title = "roko config set --project writes a legacy agent.model key that validation rejects, and refuses v2 keys"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["cli", "config"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "98a77c510"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:15, wk-readme's quick-start check for bug-09690f; details in bug-09690f's Notes)"
anchors = ["crates/roko-cli/src/commands/config_cmd.rs", "crates/roko-cli/src/resolved_overrides.rs"]
lane = "rust-cold"
parent = "spec-ae5f94"
links = { depends_on = [], blocks = [], related = ["bug-09690f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn config_set_project_keeps_the_file_valid' crates/roko-cli/src/ && cargo test -p roko-cli --lib config_set_project_keeps_the_file_valid"
+++

## Problem

`roko config set --project agent.default_model X` rewrites `roko.toml` and adds a legacy `agent.model` key, which `roko config validate` then rejects. The command also refuses v2 keys such as `budget.max_plan_usd`.

## Why it matters

A new user's first commands fail, and the README's quick start depends on them. Epic spec-ae5f94.

## Where

`ConfigCmd::Set` in `crates/roko-cli/src/commands/config_cmd.rs`, and `crates/roko-cli/src/resolved_overrides.rs`.

## Current state

Reproduced on 2026-09-29 with `target/debug/roko` built at `33e107da1`, in scratch directories with an empty `HOME` and no API keys (wk-readme).

## Plan

1. Write only the v2 key, and accept every key the v2 schema defines.
2. After writing, validate the result, and refuse to leave an invalid file.
3. Add a test.

## Done when

- [ ] `config set --project` keeps `roko.toml` valid and accepts v2 keys.
- [ ] The `[[verify]]` command passes.

## Notes

- Premise confirmed at `98a77c510` (same binary): `config set --project agent.default_model X` writes `agent.model`, which a v2 file ignores (so `default_model` does not change) and validation rejects; `budget.max_plan_usd` is refused as an unknown key.
- The setter writes the key it is given, types keys outside its table from validation's schema tree (`loader::schema_value_for_path`), writes v1 names under their v2 names (`loader::V1_RENAMED_KEYS`), and refuses a project edit that validation would reject. The old `config_set_global_writes_global_file` test wrote `agent.model` into the real `~/.roko/config.toml`; it now uses a temp file.
- Implemented on `work/bug-e1327f` at `3bf3af074`; cargo verification deferred to the batch check.
