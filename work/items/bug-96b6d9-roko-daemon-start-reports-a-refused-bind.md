+++
id = "bug-96b6d9"
kind = "bug"
title = "roko daemon start reports a refused bind only as a warning in its log, after its loops have started, and exits 0"
status = "open"
triage = "verified"
severity = "p3"
size = "S"
subsystem = ["roko-cli/daemon"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "gap-083a14 start-path check (2026-10-27)"
discovered_from = "gap-083a14"
anchors = ["crates/roko-cli/src/daemon.rs::daemon_start", "crates/roko-cli/src/daemon.rs::daemon_http_addr", "crates/roko-cli/src/daemon.rs::graceful_shutdown_daemon"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-083a14"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn daemon_start_refuses_unsafe_bind_before_starting' crates/roko-cli/src && cargo test -p roko-cli --lib daemon_start_refuses_unsafe_bind_before_starting"
+++

## Problem

`roko daemon start` checks its bind (`daemon_http_addr`, which calls `roko_serve::validate_bind_safety`) only
inside the HTTP task that `daemon_start` spawns last. By then the foreground daemon holds the workspace lock, has
written `daemon.json` (state `running`) and has started the scheduler, watchers, dispatch, feedback and dream loops
and the IPC server. When the check refuses the bind (a public bind with auth off and no
`serve.acknowledge_public_risk`, or, since gap-083a14, a public bind with `serve.auth.privy_app_id` and no Privy
allow-list), the task cancels the daemon, `graceful_shutdown_daemon` logs the refusal as
`WARN daemon HTTP server stopped with error`, and `daemon_start` returns `Ok`, so the process exits 0. Without
`--foreground`, the parent has already printed `daemon started (pid N)` and returned 0: the refusal reaches only
`.roko/logs/daemon.log` / `daemon.err`.

Expected: a refused bind fails `roko daemon start` (both forms) with the refusal's text and a non-zero exit, before
anything starts, as `roko serve` and `roko serve --detach` do.

## Why it matters

S11 G0b (gap-083a14) wants a half-configured public deploy to fail at startup with a message the operator sees.
`roko serve` returns the error from `ServerBuilder::start_background`, and `roko serve --detach` prints the child's
log tail. The daemon, which `roko --headless`, `roko daemon restart` and the launchd/systemd installs also start,
never listens on the refused bind, but it reports success.

## Where

- `crates/roko-cli/src/daemon.rs::daemon_start`: the foreground path loads `core_config` and starts every loop
  before the HTTP task runs; the detached path spawns the child without loading the config.
- `crates/roko-cli/src/daemon.rs::run_daemon_http_server` and `daemon_http_addr`: the only bind check.
- `crates/roko-cli/src/daemon.rs::graceful_shutdown_daemon`: turns the server task's error into a warning and returns
  `Ok`.

## Current state

The check runs, so the daemon never serves an unsafe bind, but late and quietly. The unit tests
`daemon_http_uses_configured_loopback_bind` and `daemon_http_rejects_unacknowledged_public_bind_without_auth` cover
`daemon_http_addr` alone. Found while checking gap-083a14's start paths, and reproduced on 2026-10-27 with main's
debug build from before gap-083a14, in a scratch workspace whose `roko.toml` binds `0.0.0.0` with auth off:

- `roko daemon start --foreground` ran the template scan and the dream loop's bootstrap, then logged
  `WARN roko_cli::daemon: daemon HTTP server stopped with error error=Public bind requires ...`, exited 0 and left
  `.roko/daemon.json` at `"state": "stopped"`.
- `roko daemon start` printed `daemon started (pid N)` and exited 0. The refusal was one `WARN` line in
  `.roko/logs/daemon.err`, and the child had exited.

## Plan

1. In `daemon_start`'s foreground path, call `daemon_http_addr(&core_config, port)?` right after
   `load_config_unified`, before `write_daemon_info` and the loops.
2. In the detached path, load the config and run the same check before `spawn_detached_child`, so the caller's
   terminal gets the error and a non-zero exit.
3. Optionally let `graceful_shutdown_daemon` return the server task's error, so a later bind failure (address in
   use) exits non-zero too.
4. Add the test `daemon_start_refuses_unsafe_bind_before_starting`: a workspace whose `roko.toml` binds `0.0.0.0`
   with auth off makes `daemon_start(dir, true, port)` fail with "Public bind requires" and leaves no `daemon.json`.
   Fence the global config (`HOME`) the way other config-loading tests in roko-cli do.

## Done when

The `[[verify]]` command passes: `daemon_start_refuses_unsafe_bind_before_starting` exists and passes.

## Notes

- `daemon_restart` stops the running daemon before its start can fail; checking the bind first is a possible
  follow-up, not part of this item.
- The check itself (`validate_bind_safety`) is right; this item is only about where the daemon calls it and how it
  reports a refusal.
