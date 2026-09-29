+++
id = "bug-524a3b"
kind = "bug"
title = "serve's hints tell users to put serve.auth.api_key in roko.toml; point them, and config set, at ROKO__SERVE__AUTH__API_KEY in .roko/.env"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-serve", "roko-cli/config", "roko-cli/doctor"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-guard2's report on bug-34c16c)"
anchors = ["crates/roko-serve/src/lib.rs", "crates/roko-cli/src/serve_client.rs", "crates/roko-cli/src/auth.rs", "crates/roko-cli/src/doctor.rs", "crates/roko-cli/src/config_cmd.rs"]
lane = "rust-cold"
parent = "spec-ae5f94"
links = { depends_on = [], blocks = [], related = ["bug-34c16c", "gap-e9660f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'serve.auth.api_key in roko.toml' crates/roko-serve/src/lib.rs && ! grep -q 'api_key in roko.toml' crates/roko-cli/src/serve_client.rs && grep -rqw 'fn config_set_writes_a_serve_secret_to_the_env_file' crates/roko-cli/src/ && cargo test -p roko-cli config_set_writes_a_serve_secret_to_the_env_file"
+++

## Problem

roko tells users to keep the serve API key in the agent-readable `roko.toml`:

- roko-serve's startup message: "or set serve.auth.api_key in roko.toml" (`crates/roko-serve/src/lib.rs:949`);
- the client's hint: "supply a key with ROKO_API_KEY, [serve.auth] api_key in roko.toml, …" (`crates/roko-cli/src/serve_client.rs:49`);
- `auth.rs` (:19, :32, :70) documents the same source.

The doctor's fix, `roko config set serve.auth.api_key <your-key>` (`doctor.rs:863`), writes the global config by default (`EditTarget::Auto` resolves to the global path, `config_cmd.rs:830`). That file is a key file under `~/.roko`, but with `--project` the same command writes `roko.toml`.

## Why it matters

Release blockers (epic spec-ae5f94): following roko's own advice puts a secret where agents read it (gap-e9660f, bug-34c16c). The loader already honours `ROKO__*` overrides (`loader.rs:524`), and `.roko/.env` is a key file agents can't read.

## Where

The hints above, and `config set`'s handling of secret keys.

## Plan

1. Change every hint to `ROKO__SERVE__AUTH__API_KEY` in `.roko/.env` (or `ROKO_API_KEY` for clients).
2. Make `roko config set` of a secret key (`serve.auth.api_key`, `server.auth_token`, …) write `ROKO__…` to `.roko/.env`, whatever the target flag, and say so.
3. Add `config_set_writes_a_serve_secret_to_the_env_file`.

## Done when

- [ ] No hint or command puts a secret in `roko.toml`.
- [ ] The `[[verify]]` command passes.

## Notes

- If the global config resolves to the legacy `~/.config/roko/config.toml`, that file is outside `~/.roko` and not a key file. Check this path too.
