+++
id = "bug-1f5406"
kind = "bug"
title = "roko doctor cannot run while roko serve runs: serve holds the exclusive workspace lock for its lifetime and doctor's shared lock is refused"
status = "done"
triage = "verified"
severity = "p3"
goal = "hermes"
size = "S"
subsystem = ["roko-cli/doctor"]
created = 2026-10-15
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "64c6b4394"
source = "hermes-roko round v5, roko-chathost, 2026-10-15"
anchors = ["crates/roko-cli/src/commands/util.rs::cmd_doctor", "crates/roko-cli/src/commands/server.rs::cmd_serve"]
links = { depends_on = [], blocks = [], related = ["gap-115bae"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn doctor_recognizes_the_serve_that_holds_the_workspace' crates/ && cargo test -p roko-cli --lib doctor_recognizes_the_serve_that_holds_the_workspace"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T08:46:45Z"
commit = "64c6b4394"
by = "roko-chathost"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-16T08:09:06Z"
forced = false
evidence = "64c6b4394 + b05d0581e: when the shared workspace lock is refused and the lock's owner PID is the one the workspace's roko serve advertises in .roko/runtime/serve.json (doctor::serve_holding_workspace), cmd_doctor's read-only variants read beside it without the lock. The report's first line is `[ok] workspace_lock: roko serve (PID N) holds this workspace, so doctor read it without the workspace lock`; doctor disk and doctor network print it as a note; the one check that writes, the stale-lock-file cleanup, is skipped (`[skipped] stale_locks`); the shared lock diagnostic no longer calls serve's lock a conflict to kill. doctor clean and doctor disk --fix still refuse. Tests: doctor::tests::doctor_recognizes_the_serve_that_holds_the_workspace (the verify) and doctor_beside_serve_says_so_and_leaves_lock_files_alone; roko-cli lib 3706 and bin 466 passed under a scratch HOME; clippy -D warnings clean. Live (scratch project, scratch HOME, roko serve on :6791): before, roko doctor exited 2 with 'Another roko process holds an exclusive lock on this workspace'; after, the whole report with the workspace_lock line first and no lock failure; doctor disk notes it and exits 0; doctor clean still refuses."
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

## Notes

2026-10-16, roko-chathost (hermes-roko round v5): a fix is drafted but not applied, compiled or tested; stopped to
keep usage free for the round's final dogfood. The draft is
`tmp/hermes-roko-v5/roko-chathost/tmp/next/patch_1f5406.py` in the main checkout (local, gitignored); apply it on a
branch from main with `python3 <it> <worktree root>`. It adds:
- `workspace_lock::workspace_lock_owner_pid` (the PID the exclusive holder wrote, read without the lock);
- `doctor::serve_holding_workspace` (the holder is this workspace's serve when that PID is the one
  `.roko/runtime/serve.json` advertises) and `doctor::beside_serve_check` (a first `[ok] workspace_lock: roko serve
  (PID N) holds this workspace, so doctor read it without the workspace lock` line), and `DoctorReport::with_check`;
- in `commands/util.rs::cmd_doctor`: when the shared lock is refused and serve holds it, the read-only variants run
  without the lock (`doctor clean` and `doctor disk --fix` still take the exclusive lock and refuse);
- the test `doctor::tests::doctor_recognizes_the_serve_that_holds_the_workspace` (the `[[verify]]` above).
Probing the running serve's health automatically was left out on purpose: a serve whose `public_routes` leave out
`health` would turn that into a new failure.

