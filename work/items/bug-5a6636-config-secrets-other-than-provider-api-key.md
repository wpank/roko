+++
id = "bug-5a6636"
kind = "bug"
title = "Config secrets other than provider api_key_env (extra_headers, file secrets, serve.auth.api_key) aren't added to the log scrubber"
status = "open"
triage = "unverified"
severity = "p2"
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
links = { depends_on = [], blocks = [], related = ["gap-0e2c40", "bug-cef888", "bug-7830f5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn the_scrubber_knows_every_config_secret' crates/roko-agent/src/ && cargo test -p roko-agent --lib the_scrubber_knows_every_config_secret"
+++

## Problem

The process scrubber (`LogScrubber`, `crates/roko-agent/src/safety/scrub.rs`) learns config secrets only from providers' `api_key_env` (:261). Other secrets the config resolves are never registered, so they can appear unredacted in records and logs:

- `extra_headers` values;
- file secrets (`*_file`);
- `serve.auth.api_key`.

## Why it matters

Secrets and guard (epic spec-ba7bea): gap-0e2c40's check C2, that no provider key reaches an agent, a gate or a log, can't hold while the scrubber misses these.

## Where

The scrubber's construction from config.

## Plan

1. Register every secret-bearing config value: the same set `config show` redacts, or the fingerprint's `is_secret_key` rule.
2. Add `the_scrubber_knows_every_config_secret`.

## Done when

- [ ] Every config secret is redacted from records and logs.
- [ ] The `[[verify]]` command passes.
