+++
id = "bug-4e7d40"
kind = "bug"
title = "roko config preset --global edits ~/.roko/roko.toml instead of ~/.roko/config.toml, and fails unless that file exists"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-cli/config"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "13f7a1358"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-onboard's report)"
anchors = ["crates/roko-cli/src/commands/tune.rs::cmd_config_preset", "crates/roko-cli/src/tui/config_meta.rs::save_pending_edits", "crates/roko-core/src/config/loader.rs::global_config_path"]
lane = "rust-cold"
parent = "spec-ae5f94"
links = { depends_on = [], blocks = [], related = ["bug-12153c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn preset_global_writes_the_global_config_file' crates/roko-cli/src/ && cargo test -p roko-cli preset_global_writes_the_global_config_file"
+++

## Problem

`cmd_config_preset` with `--global` (`crates/roko-cli/src/commands/tune.rs`) resolves the target as `global_config_path()`, which is `~/.roko/config.toml` (loader.rs:1966-1974), and creates that file empty if it is missing (tune.rs:92-169). It then applies the edits with `save_pending_edits(config_dir, …)`, passing only the file's parent directory (:180-182). `save_pending_edits` works on `<dir>/roko.toml` (config_meta.rs:726-731).

So a global preset edits `~/.roko/roko.toml`, which isn't the global config file, and fails unless that file happens to exist. The `config.toml` it just created stays empty.

## Why it matters

Release blockers (epic spec-ae5f94): `roko config preset … --global` is a documented setup path. It either fails, or writes settings that the loader never reads as the global config.

## Where

- `cmd_config_preset` (tune.rs:85 onward).
- `save_pending_edits` (tui/config_meta.rs:726).
- `global_config_path` (roko-core `config/loader.rs:1966`).

## Current state

At ad391f99a, `config set --global` resolves the global path itself (config_cmd.rs:826-835) and writes the right file. Only `preset` goes through the directory-based editor.

## Plan

1. Give `save_pending_edits` (or a new helper) the file path instead of a directory, or have `preset` use the same `set_config_key` path that `config set` uses.
2. Add `preset_global_writes_the_global_config_file`: with HOME set to a temporary directory, `preset --global` writes `~/.roko/config.toml`, and the loader reads the values back.

## Done when

- [ ] `roko config preset … --global --yes` edits `~/.roko/config.toml`, with or without an existing `~/.roko/roko.toml`.
- [ ] The `[[verify]]` command passes.

## Notes

- `commands::tune` is part of the binary, not the library, so the verify runs `cargo test -p roko-cli` without `--lib`.
- Premise confirmed at `13f7a1358` with `target/debug/roko`: `roko config preset budget --global --yes` under an empty HOME exits 2 with `read roko.toml: No such file or directory` and leaves `~/.roko/config.toml` empty.
- `save_pending_edits_to(path, target, ...)` edits the file it is given. A project file keeps the check-before-write; the global file is written as `config set --global` writes it. `save_pending_edits(root, ...)` remains the project wrapper for the TUI, and the preset's write step is `tune.rs::write_preset`.
- The test does not set HOME for the process: that is process-wide, racy under parallel tests, and `unsafe` in edition 2024 while the workspace denies `unsafe_code`. It passes `write_preset` the global path under a temporary HOME and reads the values back with the core loader.
- Implemented on `work/bug-647249` at `fa59ec6d6`; cargo verification deferred to the batch check.
