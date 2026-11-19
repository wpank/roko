+++
id = "bug-6d651d"
kind = "bug"
title = "roko daemon install is not per project (runs in $HOME on 6677), writes no serve.json, 401s its own health probe, sets a systemd watchdog nothing feeds, and duplicates the plugin's --launchd login item"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
size = "M"
subsystem = ["roko-cli/daemon"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "roko-autostart round research, 2026-10-19 (roko 3b34aed92)"
anchors = ["crates/roko-cli/src/daemon.rs::daemon_install", "crates/roko-cli/src/daemon/launchd.rs::generate_plist", "crates/roko-cli/src/daemon/systemd.rs"]
parent = "spec-13c7ba"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

`roko daemon install` (`crates/roko-cli/src/daemon.rs` `daemon_install` → `daemon/launchd.rs::generate_plist`) writes one login item per user with a fixed label `dev.nunchi.roko`, `WorkingDirectory=$HOME`, port 6677 and no `--workdir`. So it serves the home folder, not a project. It has four further problems:

- **No `serve.json`.** The daemon writes `.roko/daemon.json`, not `serve.json`, so `roko plan run` and the Hermes plugin cannot find it, and a plan run then hits the workspace lock.
- **No launch token.** Under default auth its own `/api/health` probe (`daemon.rs` ~211) gets 401.
- **Linux kill loop.** The systemd unit (`daemon/systemd.rs` ~29-73) sets `WatchdogSec=60`, but nothing sends `sd_notify`, so systemd would kill it every 60 s.
- **Duplication.** The Hermes plugin's `hermes roko init --launchd` writes a second, per-project login item of its own (label `dev.roko.serve.<slug>`).

## Why it matters

There should be one per-project service install, owned by roko. The roko-autostart round (2026-10-19, see the parent spec) chose "Hermes starts roko" with no login item by default. A login item remains an opt-in for people who want roko always on.

## Where

- `crates/roko-cli/src/daemon.rs`: `daemon_install`, `daemon_install_launchd`, `daemon_start` and `run_daemon_http_server`
- `crates/roko-cli/src/daemon/launchd.rs`
- `crates/roko-cli/src/daemon/systemd.rs`

## Plan

After the serve lifecycle gap lands:

1. Make install per project: `roko serve install` or `roko daemon install --workdir`. It should:
   - use the label `dev.nunchi.roko.serve.<hash>`;
   - set `WorkingDirectory=<project>`;
   - capture PATH;
   - load and unload with `launchctl bootstrap`/`bootout`.
2. Have it run `roko serve` (which writes `serve.json` and the launch token), not the daemon's own HTTP server.
3. Remove or implement the systemd watchdog.
4. Have the plugin's `--launchd` call it instead of writing its own plist.

## Done when

A verify you write passes: a test that the generated plist names the project's workdir and runs `serve`, plus the systemd unit without an unsatisfied watchdog.

## Notes

Not part of the 2026-10-19 round.
