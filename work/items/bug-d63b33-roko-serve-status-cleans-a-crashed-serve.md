+++
id = "bug-d63b33"
kind = "bug"
title = "roko serve status cleans a crashed serve's serve.json, token and registry entry but leaves hub.sock.path and hub.token until the next start"
status = "open"
triage = "verified"
severity = "p3"
goal = "hermes"
size = "S"
subsystem = ["roko-cli/serve"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "roko-autostart round, roko-serve worker note at 513890e42"
anchors = ["crates/roko-cli/src/serve_lifecycle.rs"]
parent = "spec-13c7ba"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

When a `roko serve` dies hard (SIGKILL or a crash), `roko serve status` treats it as stale. It removes `serve.json`, `serve.token` and the serve's registry entry (`~/.roko/runtime/serves/<hash>.json`). Two files stay behind until the next serve start replaces them: the StateHub IPC socket path file (`hub.sock.path`) and `hub.token`. The roko-serve worker reported this at `513890e42` in the roko-autostart round on 2026-10-19.

## Why it matters

- `hub.token` is a credential file. It should not outlive the serve that wrote it.
- A leftover socket path can mislead a client that looks for the StateHub.

The parent spec is spec-13c7ba.

## Where

- The stale cleanup in `crates/roko-cli/src/serve_lifecycle.rs` (status, which cleans what a dead serve left).
- The StateHub IPC server's files in roko-serve.

## Current state

- A graceful shutdown removes both files: the IPC cleanup is awaited since gap-fb1594.
- Only the stale path, where nothing ran a shutdown, leaves them.

## Plan

When status cleans a stale serve while holding the workspace lock, also remove `hub.sock.path` and `hub.token`. It removes them only when they are the dead serve's own files, using the same owned-PID or instance check as `serve.json`.

## Done when

- The stale-crash case in `serve_lifecycle::serve_detach_lifecycle_end_to_end`, or a new test, also asserts that both files are gone.
- A verify you write passes.

## Notes

Low priority. A new serve start replaces both files.
