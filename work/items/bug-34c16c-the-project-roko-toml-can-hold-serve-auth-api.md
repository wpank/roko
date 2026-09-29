+++
id = "bug-34c16c"
kind = "bug"
title = "The project roko.toml can hold serve.auth.api_key, and agents can read it"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-core/child_env", "roko-core/config"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-guard2's report)"
anchors = ["crates/roko-core/src/child_env.rs", "crates/roko-core/src/config/serve.rs", "crates/roko-std/src/tool/builtin/sandbox.rs"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["bug-a66941", "bug-7eef96", "bug-39d54c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn no_agent_readable_config_file_holds_the_serve_api_key' crates/roko-core/src/ && cargo test -p roko-core --lib no_agent_readable_config_file_holds_the_serve_api_key"
+++

## Problem

bug-a66941 made the key files unreadable to agents: `KEY_FILE_NAMES` (`crates/roko-core/src/child_env.rs:435`) lists `.env`, `secrets.toml`, `credentials.json` and `config.toml`, under `~/.roko` and `<workdir>/.roko`.

But `serve.auth.api_key` (`crates/roko-core/src/config/serve.rs:208`) is a plain field of the config, so it can be set in the project `roko.toml`, which isn't a key file and which agents read freely.

## Why it matters

Secrets and guard (epic spec-ba7bea): an agent that reads the project config gets the control plane's API key, and with it serve's authenticated routes.

## Where

`KEY_FILE_NAMES` and `is_key_file`, the serve auth config, and the guard's key-file refusal (`roko-std/src/tool/builtin/sandbox.rs`).

## Plan

1. Don't let a secret live in an agent-readable file:
   - refuse to load `serve.auth.api_key` from the project `roko.toml` (it may come from the environment, `~/.roko/config.toml` or the secret store);
   - or warn and ignore it there, pointing to `roko config set-secret`.
2. Check the other secret-bearing fields the same way (`server.auth_token`, `privy_*`, webhook secrets).
3. Add `no_agent_readable_config_file_holds_the_serve_api_key`.

## Done when

- [ ] No file an agent may read can hold the serve API key or another roko credential.
- [ ] The `[[verify]]` command passes.
