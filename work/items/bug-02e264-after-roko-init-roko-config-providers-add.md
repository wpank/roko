+++
id = "bug-02e264"
kind = "bug"
title = "After roko init, roko config providers add anthropic appends a duplicate [models.\"claude-sonnet-4-6\"] table, and the file stops parsing"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-cli/commands/config_cmd"]
created = 2026-09-30
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "bf40f3269"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-childenv's report, checked on work/bug-17f0e4 at 6b332b25f)"
anchors = ["crates/roko-cli/src/commands/config_cmd.rs"]
lane = "rust-cold"
parent = "spec-ae5f94"
links = { depends_on = [], blocks = [], related = ["bug-e2cfdf", "bug-e1327f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn providers_add_after_init_keeps_the_config_parseable' crates/roko-cli/src/ && cargo test -p roko-cli providers_add_after_init_keeps_the_config_parseable"

[closed]
at = 2026-10-01
by = "coordinator (session 7622b882)"
evidence = "Batch 20b gate on cad1a56e1 (MAIN bf40f3269 has the same crates): check --workspace --tests, nightly fmt and clippy -D warnings clean; lib tests roko-agent 2278, roko-cli 3261, roko-core 1956, roko-learn 1207, roko-gate 690, roko-std 227 and roko-cli bin 429 all pass, including providers_add_after_init_keeps_the_config_parseable (bin). Merged 6a26c7544 (work/bug-02e264 fa9091279 + closure fix 057f2994a)."
+++

## Problem

`roko init` writes a `[models."claude-sonnet-4-6"]` table. `roko config providers add anthropic` then appends its own stanza with a `[models.{slug}]` table for each catalog model (`crates/roko-cli/src/commands/config_cmd.rs:608`), without checking which tables already exist. The second `[models."claude-sonnet-4-6"]` makes the TOML invalid, since a table can't be defined twice, so every command fails to load the config.

## Why it matters

Release blockers (epic spec-ae5f94): the two commands a new user runs first break the workspace. bug-e2cfdf covers the stanza's schema-invalid keys; this is the duplicate table.

## Where

The stanza rendering and appending in `config_cmd.rs`'s `providers add`.

## Plan

1. Edit the config structurally (`toml_edit`): add missing tables, and skip or merge existing ones, instead of appending text.
2. Add `providers_add_after_init_keeps_the_config_parseable`: `init`, then `providers add anthropic`, then load.

## Done when

- [ ] `providers add` after `init` leaves a config that loads.
- [ ] The `[[verify]]` command passes.

## Notes

- 2026-09-30 (wk-childenv): Implemented on `work/bug-02e264` at `f76bb5f10`; cargo verification deferred to the batch
  check. Instead of editing with `toml_edit`, `providers add` parses roko.toml read-only first: an existing
  `[providers.<name>]` means already configured, and a catalog model the file already holds (by table key or slug) is
  left out of the appended stanza and reported as kept. Only tables the file lacks are appended, so no table is
  defined twice, and the file's comments and layout are untouched. A roko.toml that does not parse is now an error
  rather than being appended to.
