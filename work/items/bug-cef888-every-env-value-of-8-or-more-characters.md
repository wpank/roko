+++
id = "bug-cef888"
kind = "bug"
title = "Every .env value of 8 or more characters counts as a secret, so non-secret settings kept in .env are redacted from records"
status = "open"
triage = "unverified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["roko-agent/safety/scrub"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-canary's report)"
anchors = ["crates/roko-agent/src/safety/scrub.rs"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["bug-5a6636", "gap-0e2c40"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn non_secret_env_settings_are_not_redacted' crates/roko-agent/src/ && cargo test -p roko-agent --lib non_secret_env_settings_are_not_redacted"
+++

## Problem

The scrubber treats every `.env` value as a secret ("every `.env` value counts as a secret, as it does for child environments", `safety/scrub.rs:240-244`), once it reaches `MIN_SECRET_LEN` (:204), 8 characters. A non-secret setting kept in `.env`, such as `ROKO_LOG=debug,roko=trace`, a model name or a path, is then redacted from every record in which it appears.

## Why it matters

Secrets and guard (epic spec-ba7bea): records lose ordinary values (model names, paths) and become hard to read, for no security gain. p3.

## Where

`env_file_secrets` and the length rule in `scrub.rs`.

## Plan

1. Register only `.env` values whose names look like secrets (`*_KEY`, `*_TOKEN`, `*SECRET*`, `*PASSWORD*`), or add an allowlist of known non-secret names.
2. Add `non_secret_env_settings_are_not_redacted`.

## Done when

- [ ] Non-secret `.env` settings appear in records, and secrets don't.
- [ ] The `[[verify]]` command passes.
