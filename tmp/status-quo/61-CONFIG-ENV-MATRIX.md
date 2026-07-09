# Config And Environment Matrix
> Re-verified 2026-07-08 @ HEAD 5852c93c.

Roko has a real unified config schema, but config behavior is spread across core schema, CLI compatibility, TUI metadata, serve runtime, and app-specific env vars.

See [83-ENV-VAR-MANIFEST.md](83-ENV-VAR-MANIFEST.md) for the generated direct env-var inventory (99 fixed vars + the dynamic `ROKO_SECRET_*` / `ROKO__*` surfaces).

**Verification note (2026-07-08):** the High-Impact groups below were cross-checked against the code grep behind the manifest. Several vars listed here are **aspirational / forward-looking — no `env::var` read exists in this tree**: `ROKO_MODEL_SLUG` (only a loader doc-comment, `loader.rs:31`), `ROKO_WORKDIR`, `ROKO_STATE_ROOT`, `ROKO_BIND`, `ROKO_PORT` (runtime uses `PORT`, `roko-serve/src/lib.rs:275`), `CEREBRAS_API_KEY`, `OPENROUTER_API_KEY`, `MIRAGE_STATE_DIR` (set as a *child-process env key* in `commands/server.rs:337`, never read here), `MIRAGE_BLOCK_INTERVAL_MS`, `MIRAGE_SNAPSHOT_INTERVAL_SECS`. They are retained as intended/ops-facing knobs but marked below with † to distinguish plan from reality. Code-verified secret dynamic surface: `ROKO_SECRET_<CATEGORY>_<PROVIDER>` (`roko-core/src/secrets/`). Confirmed still live: hierarchical `ROKO__SECTION__FIELD` overrides (`loader.rs:338-382`).

## Config Sources

| Source | Current role |
|---|---|
| Built-in defaults | `roko-core::config::schema::RokoConfig::default`. |
| Project config | `roko.toml` loaded through unified loader. |
| Global config | Merged into project config for missing provider/model fields. |
| Legacy aliases | Compatibility migration handles older Mori-style/default model fields. |
| Named env vars | `ROKO_MODEL`, `ROKO_BACKEND`, `ROKO_PROVIDER`, etc. |
| Hierarchical env vars | `ROKO__AGENT__DEFAULT_MODEL` style overrides. |
| CLI flags | `--model`, `--role`, `--effort`, `--repo`, `--config`, etc. |
| Runtime env | Serve, relay, Mirage, MCP, CI, deployment, and provider-specific vars. |

## Canonical Config Target

| Domain | Target | Current duplicates / aliases | Migration action |
|---|---|---|---|
| Project config | `roko.toml` loaded as `roko_core::config::schema::RokoConfig` through the core loader | `.roko/roko.toml`, `.roko/config/config.toml`, older generated examples | Keep compatibility reads, but write/migrate toward project `roko.toml`. |
| Global config | `~/.roko/config.toml` for user defaults | Legacy `$XDG_CONFIG_HOME/roko/config.toml` | Document precedence and migrate old global files. |
| CLI resolved config | A `ValidatedConfig` or a documented CLI wrapper around it | `load_resolved_config()` validates through core but still returns legacy `ConfigLayer` output | Make CLI consume the core-loaded provider/model/env state; keep legacy fields only for CLI-only settings. |
| Env overrides | Named vars plus hierarchical `ROKO__...` overrides | Direct `std::env::var` reads in app-specific code | Centralize provenance reporting and document each direct env read as core, app-specific, secret, or test-only. |
| Secrets | Provider env vars, workspace `.roko/secrets.toml`, user `~/.roko/credentials.json` | `ROKO_API_KEY`, `ROKO_SERVER_AUTH_TOKEN`, provider-specific keys, `ROKO_SECRET_*` | Keep secret sources distinct and scrub all payloads/logs. |

## High-Impact Env Vars

| Group | Vars | Notes |
|---|---|---|
| Provider keys | `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `OPENAI_BASE_URL`, `GEMINI_API_KEY`, `MOONSHOT_API_KEY`, `PERPLEXITY_API_KEY`, `CEREBRAS_API_KEY`†, `ZAI_API_KEY`, `OPENROUTER_API_KEY`†, `ROKO_SECRET_*` | Must be scrubbed in logs/errors/API. († = not read in code as of HEAD 5852c93c.) |
| Model/provider overrides | `ROKO_MODEL`, `ROKO_MODEL_SLUG`†, `ROKO_BACKEND`, `ROKO_PROVIDER`, `ROKO_EFFORT`, `ROKO_ROLE` | Core loader and CLI both know some of these. († = doc-comment only, `loader.rs:31`.) |
| Serve/TUI | `ROKO_SERVE_URL`, `ROKO_SERVER_URL`, `ROKO_SERVER_AUTH_TOKEN`, `ROKO_SPA_DIR`, `ROKO_AGENT_RELAY_URL`, `ROKO_MIRAGE_URL` | Need docs for local dev vs deployed mode. |
| MCP/scripts | `ROKO_MCP_CONFIG`, `ROKO_MCP_SCRIPTS_DIR`, `ROKO_MCP_SCRIPTS_ENV_ALLOWLIST`, `ROKO_MCP_SCRIPTS_TIMEOUT_SECS`, `ROKO_WORKSPACE_ROOT` | High-risk execution surface; keep allowlists explicit. |
| Chain/ISFR | `ETH_RPC_URL`, `MIRAGE_RPC_URL`, `ROKO_TEST_RPC_URL`, `ISFR_SERVICE_URL`, `ISFR_STRICT_PROXY` | Split test/mock/live-chain docs. |
| Relay apps | `ROKO_AGENT_RELAY_BIND`, `ROKO_AGENT_RELAY_RPC_WS_URL`, `ROKO_AGENT_RELAY_CHAIN_ID`, `ROKO_AGENT_RELAY_*` | App-specific env not part of core schema. |
| Runtime/deploy | `ROKO_WORKDIR`†, `ROKO_STATE_ROOT`†, `ROKO_BIND`†, `ROKO_PORT`†, `PORT`, `RAILWAY_VOLUME_MOUNT_PATH`, `FLY_APP_NAME`, `RAILWAY_PUBLIC_DOMAIN` | Operational root/bind settings, not all core config. Runtime actually binds via `PORT` (`roko-serve/src/lib.rs:275`); `ROKO_PORT`/`ROKO_BIND`/`ROKO_WORKDIR`/`ROKO_STATE_ROOT` are †not-read. |
| Mirage | `MIRAGE_STATE_DIR`†, `MIRAGE_RPC_URL`, `MIRAGE_BLOCK_INTERVAL_MS`†, `MIRAGE_SNAPSHOT_INTERVAL_SECS`†, `MIRAGE_SHUTDOWN_SECRET`, `ROKO_MIRAGE_URL` | Separate Mirage app config from serve proxy config. `MIRAGE_STATE_DIR` is only *set* as a child-process env key (`commands/server.rs:337`), not read in-tree. |
| UX/accessibility | `NO_COLOR`, `CLICOLOR`, `CLICOLOR_FORCE`, `ROKO_HIGH_CONTRAST`, `ROKO_REDUCED_MOTION` | TUI/CLI rendering. |
| Testing | `CI`, `SKIP_FRONTEND_BUILD`, `ROKO_TEST_OLLAMA`, `MIRAGE_TEST_PORT` | Should not leak into normal docs as production knobs. |

## Drift Points

- `crates/roko-cli/src/config.rs` still has CLI-specific config structs next to the core schema. This is acceptable only if the split is documented.
- `crates/roko-cli/src/config.rs` loads/validates core config in places, but compatibility functions still hand callers legacy `ConfigLayer` values.
- TUI config metadata lists known `ROKO_*` env vars but does not cover every env var found in source.
- Serve app state reads some env vars directly (`ROKO_MIRAGE_URL`, `ROKO_AGENT_RELAY_URL`) after config load.
- Provider detection and provider config validation happen in multiple paths.
- App-specific env vars for Mirage, relay, watcher, and demo are not part of one operator reference.

## Migration Checklist

- [ ] Publish one config precedence table.
- [ ] Add test that every documented env var maps to a config field or is marked app-specific.
- [ ] Add test that every `std::env::var("ROKO_*")` is documented.
- [ ] Keep secret env vars out of StateHub, frontend payloads, and logs.
- [ ] Make `roko config doctor` report source/provenance for key fields.
- [ ] Make `roko config migrate` cover legacy aliases and remove stale generated examples.
- [ ] Split core config docs from deployment/app env docs.
- [ ] Make CLI return/propagate core `ValidatedConfig` for provider/model/env fields.
- [ ] Add direct-env-var inventory to CI so new `ROKO_*` variables cannot appear undocumented.
- [ ] Regenerate `83-ENV-VAR-MANIFEST.md` when direct env reads or Clap env bindings change.
