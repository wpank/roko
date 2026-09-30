+++
id = "bug-9bb0be"
kind = "bug"
title = "roko-cli's --config <path> path (Config::from_file) still hard-fails on a provider or model typo, unlike the loader after bug-ab8118"
status = "open"
triage = "unverified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["roko-cli/config"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-childenv's report, checked on work/bug-17f0e4 at 6b332b25f)"
anchors = ["crates/roko-cli/src/config.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-ab8118"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn config_from_file_treats_a_provider_typo_like_the_loader' crates/roko-cli/src/ && cargo test -p roko-cli --lib config_from_file_treats_a_provider_typo_like_the_loader"
+++

## Problem

bug-ab8118 made the config loader treat a typo inside a `[providers.*]` or `[models.*]` entry like any other typo. `--config <path>` goes through roko-cli's `Config::from_file` (`crates/roko-cli/src/config.rs:114`), which wk-childenv reports still hard-fails on such a typo through `deny_unknown_fields`.

## Why it matters

The same file loads with one command and fails with another, depending on whether it was given with `--config`. p3.

## Where

`Config::from_file`.

## Plan

1. Route `from_file` through the loader's parse path (with the strip and the warning), or apply the same rule there.
2. Add `config_from_file_treats_a_provider_typo_like_the_loader`.

## Done when

- [ ] A typo inside a provider or model entry behaves the same with and without `--config`.
- [ ] The `[[verify]]` command passes.
