+++
id = "bug-524a3b"
kind = "bug"
title = "serve's hints tell users to put serve.auth.api_key in roko.toml; point them, and config set, at ROKO__SERVE__AUTH__API_KEY in .roko/.env"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-serve", "roko-cli/config", "roko-cli/doctor"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "8a88c6267"
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
- Premise held at 8a88c6267, and wider: besides the hints listed, the deploy check (`commands/server.rs`) showed `api_key = "<secret>"` under `[serve.auth]` in `roko.toml`, serve's masked-secret notes named `ROKO_SERVE_AUTH_API_KEY` and the like, which nothing reads, and the doctor's `roko config set` wrote `~/.roko/config.toml`, whose `serve` section the loader never merges.
- Decision: `roko config set <secret field> <value>` writes the field's `ROKO__` variable to the project's `.roko/.env` (the directory of `roko.toml`, else the workdir) whatever the target flag, `--global` included; `roko config set-secret ROKO__SERVE__AUTH__API_KEY <key>` stores one in `~/.roko/.env` for every workspace. It removes the field from `roko.toml`, the `ROKO_CONFIG` file and the legacy `~/.config/roko/config.toml` with `toml_edit`, keeping their comments; a key file keeps what it holds. The value is never printed or journaled. A provider header or `agent.env` secret is refused with a `${VAR}` hint.
- Also: `roko config validate` and every checked `roko.toml` write refuse a secret, as the loader does; `.env` values that dotenv would expand or split are single-quoted; the loader's new `env_override_name` gives a field's variable (used by the refusal message and serve's notes); a `ROKO__` override of a string field stays a string, so a numeric key no longer drops every override.
- Not done: `roko config set --help` (`main.rs:2882`, wk-canary's file) still says "in the chosen layer"; docs/v3 (`24-AUTH.md`, `26-HTTP-API.md`, `depth/24-auth/cli-credentials.md`) still show `api_key` in `roko.toml`; `roko deploy railway` checks the local key but does not forward `ROKO__SERVE__AUTH__API_KEY`, and forwarding it needs a decision because workers inherit the control plane's environment.
- The static part of the `[[verify]]` passes. Implemented on `work/gap-e9660f` at `ab63f2c49`; cargo verification deferred to the batch check.
