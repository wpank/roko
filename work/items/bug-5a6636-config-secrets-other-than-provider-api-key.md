+++
id = "bug-5a6636"
kind = "bug"
title = "Config secrets other than provider api_key_env (extra_headers, file secrets, serve.auth.api_key) aren't added to the log scrubber"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-agent/safety/scrub"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "4cf2e329b"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-canary's report)"
anchors = ["crates/roko-core/src/config/loader.rs", "crates/roko-core/src/obs/scrub.rs"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["gap-0e2c40", "bug-cef888", "bug-7830f5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn the_scrubber_knows_every_config_secret' crates/roko-core/src/ && cargo test -p roko-core --lib the_scrubber_knows_every_config_secret"
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

## Notes

- Premise held at 4cf2e329b, at other anchors: roko-agent's `safety/scrub.rs` no longer holds the process scrubber. gap-5f4852 moved it to roko-core's `obs/scrub.rs`, which roko-fs's `RunScrubber::install` fills at startup, and the loader added only providers' `api_key_env` values (`resolve_runtime_layers_with_context`). The anchors and the `[[verify]]` now point at roko-core, where the test lives.
- Change: after resolving a config, the loader adds every secret it holds to the installed scrubber, the fields `secret_fields` finds in the effective config (secret-named fields such as `serve.auth.api_key`, provider header values with file secrets read, `agent.env` credentials), each under its field; a `Bearer <token>` header adds the token alone too. `roko_core::obs::add_secret_values` does the adding; `secret_fields` and the new list share one walk (`visit_secrets`).
- Implemented on `work/bug-7830f5` at `8d94db4ed`; cargo verification deferred to the batch check.
