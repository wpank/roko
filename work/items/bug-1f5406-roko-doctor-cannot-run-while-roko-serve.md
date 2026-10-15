+++
id = "bug-1f5406"
kind = "bug"
title = "roko doctor cannot run while roko serve runs: serve holds the exclusive workspace lock for its lifetime and doctor's shared lock is refused"
status = "open"
triage = "verified"
severity = "p3"
goal = "hermes"
size = "S"
subsystem = ["roko-cli/doctor"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "hermes-roko round v5, roko-chathost, 2026-10-15"
anchors = ["crates/roko-cli/src/commands/util.rs::cmd_doctor", "crates/roko-cli/src/commands/server.rs::cmd_serve"]
links = { depends_on = [], blocks = [], related = ["gap-115bae"], supersedes = [], duplicate_of = "" }
+++

## Problem

`roko doctor` cannot run while `roko serve` runs in the same workspace. It prints "Another roko process holds an
exclusive lock on this workspace" and exits. `commands/util.rs::cmd_doctor` takes a shared workspace lock so that
it "can coexist with an active plan runner", but `commands/server.rs::cmd_serve` holds the exclusive lock for the
server's whole life, so the shared lock is never granted. Seen 2026-10-15 21:23 (hermes-roko round v5,
roko-chathost): with roko serve on :6791 in a scratch project, `roko doctor` failed at once; after serve stopped, it
ran and reported the new `chat_host_runs` line.

## Why it matters

A workspace used with a chat host (Hermes, OpenClaw) has roko serve running most of the time, and that is when a
person asks `roko doctor` why chat-started runs are refused (the `chat_host_runs` check, gap-115bae) or why serve
misbehaves. The answer is "stop the server first", which loses the server's state and every client's connection.

## Where

- `crates/roko-cli/src/commands/util.rs::cmd_doctor`: the shared lock.
- `crates/roko-cli/src/commands/server.rs::cmd_serve`: the exclusive lock serve holds.
- `crates/roko-cli/src/workspace_lock.rs`: the lock kinds.

## Current state

Open at `5875fa441`.

## Plan

Let the read-only doctor variants run beside a server: either serve holds a lock that admits shared readers, or
`roko doctor` reads the workspace without the lock (it changes nothing), saying that serve is running.

## Done when

With `roko serve` running in a workspace, `roko doctor` prints its report (including `chat_host_runs`), and `roko
doctor clean` / `doctor disk --fix` still refuse.
