+++
id = "gap-fb1594"
kind = "gap"
title = "roko serve cannot start itself in the background, report, stop or restart per project, and a running serve cannot prove which project and build it is"
status = "done"
triage = "verified"
severity = "p1"
goal = "hermes"
size = "L"
subsystem = ["roko-serve/lifecycle", "roko-cli/serve"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
last_verified_rev = "bb8ac3c9d"
source = "roko-autostart round research, 2026-10-19 (roko 3b34aed92)"
anchors = ["crates/roko-cli/src/commands/server.rs::cmd_serve", "crates/roko-serve/src/endpoint.rs::ServeEndpoint", "crates/roko-serve/src/routes/mod.rs::top_level_health", "crates/roko-cli/src/serve_client.rs::discover_workspace_server", "crates/roko-serve/src/showcase/idle.rs", "crates/roko-cli/src/daemon.rs::spawn_detached_child"]
parent = "spec-13c7ba"
links = { depends_on = [], blocks = [], related = ["bug-06ac5f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn serve_detach_lifecycle_end_to_end' crates/roko-cli/tests/ && cargo test -p roko-cli --test serve_lifecycle serve_detach_lifecycle_end_to_end"

[[verify]]
command = "grep -rqw 'fn health_reports_identity_on_loopback' crates/roko-serve/ && cargo test -p roko-serve health_reports_identity_on_loopback"

[[verify]]
command = "grep -qE '^cmd_install\\(\\)' dev.sh"

[closed]
at = 2026-10-19
at_ts = "2026-10-19T11:55:50Z"
commit = "bb8ac3c9d"
by = "commit trailer"
executor = "claude-agent"
via = "manual"
size = "L"
claimed_at = "2026-10-19T10:13:25Z"
forced = false
evidence = "bb8ac3c9d: roko serve --detach [--json] and status|stop|restart [--workdir W | --all] (crates/roko-cli/src/serve_lifecycle.rs, commands/server.rs); serve.json identity fields, /health identity on a loopback bind and the ~/.roko/runtime/serves registry (crates/roko-serve/src/endpoint.rs, routes/mod.rs, lib.rs); graceful SIGINT/SIGTERM/SIGHUP via cli_signals; --idle-exit via showcase/idle.rs; PORT no longer beats --port; sidecars get the bound URL; ./dev.sh install. Proof: cargo test -p roko-cli --test serve_lifecycle serve_detach_lifecycle_end_to_end (detach twice, status, fallback port, --all, restart, stop, SIGTERM, crash = stale, idle exit) and cargo test -p roko-serve health_reports_identity_on_loopback pass; cargo clippy -p roko-cli -p roko-serve --no-deps -- -D warnings is clean."
+++

## Problem

`roko serve` only runs in the foreground of a terminal. A client such as the Hermes plugin or `./dev.sh install` cannot do the following.

- **Start it in the background.** It cannot start a serve for a project in the background and learn its URL once it answers. There is no detach flag (`crates/roko-cli/src/main.rs` `Commands::Serve`).
- **Run a second project.** It cannot start a second project's serve while 6677 is taken. There is no automatic port, and a taken port just exits with `bind to …`.
- **Manage it.** It cannot ask whether a project's serve runs, stop it, or restart it on a new build. No such commands exist.
- **Tell serves apart.** It cannot tell this project's serve from another project's serve on the same port:
  - `serve.json` records only `pid`, `url`, `workdir` and `started_at`;
  - `/health` returns only `status`, `version`, `uptime_secs` and `features`;
  - `serve_client::discover_workspace_server` only checks that a TCP connect succeeds, though its doc says it calls `/api/health`.
- **Rely on cleanup.**
  - SIGTERM goes to the global handler (`main.rs` ~2618-2645), which exits with 143 and skips `state.shutdown()`. It leaves `serve.json`, `serve.token` and the lock PID behind; Will's machine had such leftovers from 2026-10-14.
  - SIGHUP (closing the terminal) does the same; see bug-06ac5f.
  - The showcase idle exit cancels `state.cancel`, but `cmd_serve` waits only on ctrl_c, so the process lingers.

There are also two smaller defects:

- the `PORT` env var overrides an explicit `--port` (`crates/roko-serve/src/lib.rs` ~387-395);
- with `--port 0`, agent sidecars get `--serve-url …:0` (`crates/roko-serve/src/routes/agents.rs` ~968-970).

## Why it matters

This is the parent spec's phase 1. The plugin must start one serve per project and be able to identify each one. `./dev.sh install` must be able to move running serves to a new build.

## Where

- **The command:** `crates/roko-cli/src/main.rs` (`Commands::Serve`) and `crates/roko-cli/src/commands/server.rs::cmd_serve` (~347-499).
- **Endpoint and startup:** `crates/roko-serve/src/endpoint.rs::ServeEndpoint` and `crates/roko-serve/src/lib.rs::start_background` (~370-712; the bind is ~554, `serve.json` is written ~577-597).
- **Health route:** `crates/roko-serve/src/routes/mod.rs::top_level_health` (~685).
- **Discovery:** `crates/roko-cli/src/serve_client.rs::discover_workspace_server` (~90-166).
- **Idle exit:** `crates/roko-serve/src/showcase/idle.rs`.
- **Existing code to reuse:**
  - `crates/roko-cli/src/daemon.rs::spawn_detached_child` (~896);
  - the setsid in `crates/roko-agent/src/openclaw/gateway_service.rs` (~224-231);
  - the flock probe in `crates/roko-cli/src/workspace_lock.rs` (~148-161);
  - `crates/roko-cli/src/commands/dev.rs::terminate_pid` (~164-196);
  - plan_runner's `CLI_OWNS_TERMINATION_SIGNALS`;
  - `roko --version`'s build info from `crates/roko-cli/build.rs` (`ROKO_GIT_HASH`, `ROKO_GIT_DIRTY`).
- **Script:** `dev.sh` (`cmd_build` ~792).

## Plan

The contract is §A of `tmp/roko-autostart/BRIEF.md`. The plugin is coded against it in parallel, so any change to it goes through the round lead. Summary:

1. **`roko serve --detach [--json] [--wait SECS]`** is idempotent. It starts the same binary in its own session with cwd set to the workdir, stdin closed, logs written to `.roko/logs/serve.log` and `PORT` removed. It returns once `serve.json` names the child and `/health` answers with the same `instance_id`.
2. **Port:** use `--port` if given, else the configured port if it is free, else port 0 with the real port recorded.
3. **New subcommands:** `roko serve status|stop|restart [--workdir W | --all] [--json] [--force]`. Busy serves (active runs) are not stopped without `--force`.
4. **Identity:**
   - `serve.json` gains `instance_id`, `port`, `version`, `git_sha`, `git_dirty`, `exe`, `idle_exit_secs` and `args`;
   - `/health` adds `pid`, `workdir`, `instance_id`, `git_sha`, `git_dirty`, `started_at` and `idle_exit_secs` on a loopback bind;
   - discovery requires the identity check.
5. **Signals:** SIGINT, SIGTERM and SIGHUP shut down gracefully and remove the serve's own files.
6. **`--idle-exit SECS`:** the serve exits once there are no runs, no SSE or WS clients, and no requests other than `/health` and `/ready`.
7. **Registry:** `~/.roko/runtime/serves/<hash>.json`, written when the serve binds and removed when it exits.
8. **`./dev.sh install [--no-build]`:** build the release, move it atomically into `~/.cargo/bin/roko`, then run `roko serve restart --all`.
9. **Smaller fixes:** `PORT` no longer beats `--port`; sidecars get the bound address.

## Done when

- The `[[verify]]` commands pass. `serve_lifecycle::serve_detach_lifecycle_end_to_end` covers detach (twice), status, the second project on a fallback port, restart (new instance id), stop (files and registry entry gone), SIGTERM cleanup and idle exit.
- `health_reports_identity_on_loopback` passes.
- `dev.sh` has `cmd_install`.
- `cargo clippy -p roko-cli -p roko-serve --no-deps -- -D warnings` is clean.

## Notes

- **One implementation:**
  - one detached-spawn helper, shared with `roko daemon start`;
  - one liveness check, used by status, stop, restart, discovery and `doctor::serve_holding_workspace` (bug-06ac5f).
  - Do not otherwise change `roko daemon`; its install has its own bug item.
- **Test isolation:** tests use a temp HOME (the registry lives under `~/.roko`), a TMPDIR outside the repo, and ports 6800-6849.
- **Skip the gist test:** never run `share_run_scrubs_output_text_and_prompt`; it publishes a public gist.
