+++
id = "bug-131421"
kind = "bug"
title = "roko setup --quick writes no roko.toml in a fresh directory, and after init writes a key that validation rejects"
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
anchors = ["crates/roko-cli/src/commands/setup.rs"]
lane = "rust-cold"
parent = "spec-ae5f94"
links = { depends_on = [], blocks = [], related = ["bug-09690f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn setup_quick_in_empty_dir_writes_a_valid_config' crates/roko-cli/src/ && cargo test -p roko-cli --lib setup_quick_in_empty_dir_writes_a_valid_config"
+++

## Problem

In an empty directory, `roko setup --quick` skips init because `.roko/` already exists: roko's own log creates it. It then prints `roko.toml already contains all detected providers` and writes no `roko.toml`. After `roko init`, it writes `providers.anthropic.default_model`, a key that `roko config providers validate` rejects.

## Why it matters

A new user's first commands fail, and the README's quick start depends on them. Epic spec-ae5f94.

## Where

`crates/roko-cli/src/commands/setup.rs`; the check at about line 105.

## Current state

Reproduced on 2026-09-29 with `target/debug/roko` built at `33e107da1`, in scratch directories with an empty `HOME` and no API keys (wk-readme).

## Plan

1. Decide whether init is needed from `roko.toml`, not from `.roko/`.
2. Write only keys that validation accepts.
3. Add a test starting from an empty directory.

## Done when

- [ ] `setup --quick` in an empty directory leaves a valid `roko.toml`.
- [ ] The `[[verify]]` command passes.

## Notes

- Premise confirmed at `98a77c510` (same binary): in an empty directory `setup --quick` prints `.roko/ already exists, skipping init.` and writes no `roko.toml`; after `roko init` it appends `providers.anthropic.default_model`, which validation rejects.
- Both setup paths now decide on init from `roko.toml` (`init::needs_init`). The quick path's writer moved to `config_cmd::add_detected_providers`: it writes only schema keys, parses the file to find configured providers, and appends through `append_checked_config`. The interactive path's appends use the same checked append.
- Implemented on `work/bug-e1327f` at `8f7dde1d8`; cargo verification deferred to the batch check.
