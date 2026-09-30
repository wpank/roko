+++
id = "bug-cef888"
kind = "bug"
title = "Every .env value of 8 or more characters counts as a secret, so non-secret settings kept in .env are redacted from records"
status = "open"
triage = "verified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["roko-agent/safety/scrub"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "4cf2e329b"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-canary's report)"
anchors = ["crates/roko-fs/src/observability.rs", "crates/roko-cli/tests/secret_canary.rs"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["bug-5a6636", "gap-0e2c40"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn non_secret_env_settings_are_not_redacted' crates/roko-fs/src/ && cargo test -p roko-fs --lib non_secret_env_settings_are_not_redacted"
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

## Notes

- Premise held at 4cf2e329b, at other anchors: the rule was in roko-fs's `RunScrubber::install` (`observability.rs`), which registered every `.env` value of 8 or more characters. The anchors and the `[[verify]]` now point at roko-fs, where the test lives.
- Decision: an entry is a secret when its name looks like a credential's (`roko_core::child_env::is_secret_env_name`), or its value is a URL that carries credentials (`postgres://user:pass@host/db`, `https://token@host/repo`). The rule lives in `RunScrubber::build_from_env_file`, which `install` uses. The child environment rule (every `.env` name stays out of child processes) and `roko share`'s scrubber (every `.env` value, since shared transcripts are published) are unchanged.
- The canary tests build with `build_from_env_file`; the short-value test names its long secret `LONG_TOKEN` and checks that a long setting stays.
- Implemented on `work/bug-7830f5` at `4bf830872`; cargo verification deferred to the batch check.
