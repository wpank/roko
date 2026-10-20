+++
id = "bug-3eec27"
kind = "bug"
title = "roko serve's roko.toml reload drops its --port/--bind/--enable-terminal overrides, so every reload warns that [server] needs a restart and stores the file's port"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-serve/config"]
created = 2026-10-15
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "fe0d20412"
source = "hermes-roko round v5, roko-chathost, 2026-10-15"
anchors = ["crates/roko-serve/src/routes/config.rs::reload_config_from_disk_with_hook", "crates/roko-cli/src/commands/server.rs::cmd_serve"]
links = { depends_on = [], blocks = [], related = ["gap-115bae"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn reload_keeps_the_settings_serve_took_from_its_command_line' crates/ && cargo test -p roko-serve --lib reload_keeps_the_settings_serve_took_from_its_command_line"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T00:02:22Z"
commit = "fe0d20412"
by = "roko-chathost"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-15T21:09:36Z"
forced = false
evidence = "fe0d20412: roko serve keeps the settings it took from its command line (--bind, --port, --enable-terminal) as roko_serve::state::ConfigOverrides on AppState, recorded in ServerBuilder::start_background (and run_server_with_state), and re-applies them to every config it loads again (routes/config.rs: the file watcher's reload and PUT /config), so a reload reports only what the file changed. A serve started without flags already reloaded a no-op edit with no warning (live, logs/reload-noflags.log). Tests: routes::config::tests::reload_keeps_the_settings_serve_took_from_its_command_line (the verify; a budget edit under --port 6791 --enable-terminal asks for no restart and keeps both, a file [server] port changes nothing, PUT /config keeps both) and tests::command_line_settings_are_the_config_overrides. Lib suites roko-cli 3649, roko bin 463, roko-core 2023, roko-learn 1352, roko-serve 1025 passed (plus the verify test on rerun; the one other failure is the racy JWKS test, bug-904a1c); clippy -D warnings clean."
+++

## Problem

`roko serve --port 6791` (likewise `--bind` and `--enable-terminal`) writes the flag into the `RokoConfig` it
stores at startup: `commands/server.rs::cmd_serve` sets `roko_config.server.port`, `server.bind` and
`serve.terminal_enabled`, and `roko_serve::run_server_with_state` sets `server.bind` again. The config watcher's
reload (`routes/config.rs::reload_config_from_disk_with_hook`) loads `roko.toml` from disk without those overrides.
So `hot_reload::config_diff` finds `[server]` (or `[serve]`) changed on every reload, whatever the edit was, and
serve logs:

```
WARN roko_core::config::hot_reload: config change requires restart to take effect section=Other("server") summary=server configuration changed (requires restart)
WARN roko_serve::config_watcher: config reload: server configuration changed (requires restart)
```

It then stores the file's values (port 6677) over the running ones. Seen 2026-10-15 21:22 and 21:25 (hermes-roko
round v5, roko-chathost, serves started with `--port 6791`): `roko config preset chat-host` added
`[agent.data_llm]` and `[serve.mcp]`, serve logged the correct `config hot-reloaded section=DataLlm` /
`section=ServeMcp`, and the spurious `[server]` warning next to it.

## Why it matters

The warning tells a person to restart roko serve for an edit that already applied, which is what the chat-host fix
(gap-115bae) promises not to need. Anything that reads `server.port` or `server.bind` from the stored config after a
reload sees the file's values, not the listener's.

## Where

- `crates/roko-cli/src/commands/server.rs::cmd_serve`: the flags written into the stored config.
- `crates/roko-serve/src/lib.rs::run_server_with_state`: `server.bind`.
- `crates/roko-serve/src/routes/config.rs::reload_config_from_disk_with_hook`: the reload that diffs and stores.

## Current state

Open at `5875fa441`. A serve started without these flags (as Will's is) does not show it.

## Plan

Keep serve's runtime overrides (bind, port, terminal) in `AppState` and apply them to every reloaded config before
it is diffed and stored, so a reload reports only what the file changed.

## Done when

A reload test on a state started with a port override and an `[agent]` edit reports no `[server]` change and keeps
the port, and a live `roko serve --port X` plus a `roko.toml` edit logs no `[server]` restart warning.
