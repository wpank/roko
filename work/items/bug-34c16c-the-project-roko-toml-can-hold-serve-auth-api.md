+++
id = "bug-34c16c"
kind = "bug"
title = "The project roko.toml can hold serve.auth.api_key, and agents can read it"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-core/child_env", "roko-core/config"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
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

## Notes

- Premise confirmed at `942d2a6c3` by reading the code: `serve.auth.api_key` is a plain field; `KEY_FILE_NAMES` covers only `.roko/*`; and roko's own tooling tells users to put the key in `roko.toml` (`roko config set serve.auth.api_key`, the `roko doctor` fix hint). The global merge does not read `serve.*` from `~/.roko/config.toml`, so today the key's only file home is the project config.
- Decision: refuse agents the file while it holds a secret, rather than refusing to load it. A loader-side refusal would break every setup that followed roko's own hint, and the tests that load such files. A secret is what `roko config show` redacts (a shared `is_secret_key` in the loader, plus `extra_headers`). Covered: `serve.auth.api_key`, `server.auth_token`, `deploy.railway_api_token`, `webhooks.github.secret`, `chain.wallet_key`, a literal `platforms.*.token`, and MCP env tokens. Not counted: `{ env = "..." }` tokens, `*_env` names, `api_keys` hashes, `privy_*` ids. Files: any `roko.toml`, the file `ROKO_CONFIG` names, and the legacy `~/.config/roko/config.toml`. `~/.roko/config.toml` stays a key file whatever it holds.
- Enforced in roko-std's `refuse_key_file` (file tools, grep's walk, the bash check), SafetyLayer's path policy, and the Claude guard (line by line, since python 3.9 has no TOML parser; also a Grep of the directory holding such a roko.toml, unless its glob or type leaves it out). The loader warns once per file and points to `ROKO__SERVE__AUTH__API_KEY` in `.roko/.env`, which keeps the file readable.
- Not covered (report): a Bash command that reads the project wholesale (`grep -r`, `rg`, `cat *`) can still reach a secret in roko.toml, and the Claude permission deny rules are static, so only the hooks apply the content check. The durable fix is to keep secrets out of roko.toml: `roko config set` and the doctor hint should write `ROKO__SERVE__AUTH__API_KEY` to `.roko/.env` instead (roko-cli). ACP's builtin tools (`roko-acp/src/builtin_tools.rs`) were not checked.
- Implemented on `work/bug-ceab60` at `bc9725965`; cargo verification deferred to the batch check.
