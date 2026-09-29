+++
id = "bug-3f3990"
kind = "bug"
title = "The Linux firejail plugin sandbox ignores sandbox.allowed_paths and filesystem_write, which macOS Seatbelt enforces"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
size = "M"
subsystem = ["roko-cli/runner", "roko-plugin"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-ci2's report on find-8cc7ac)"
anchors = ["crates/roko-cli/src/runner/extension_loader.rs::macos_seatbelt_profile", "crates/roko-cli/src/runner/extension_loader.rs", "crates/roko-plugin/src/manifest.rs"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["find-8cc7ac", "gap-585bd2"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn firejail_command_enforces_allowed_paths_and_filesystem_write' crates/roko-cli/src/runner/ && cargo test -p roko-cli --lib firejail_command_enforces_allowed_paths_and_filesystem_write"
+++

## Problem

Plugin subprocesses run under kernel confinement (`crates/roko-cli/src/runner/extension_loader.rs`):

- **macOS.** The Seatbelt profile (`macos_seatbelt_profile`, :326-357) allows `file-write*` only when the plugin's capabilities include `filesystem_write`, and then only under the roots derived from `sandbox.allowed_paths` (`seatbelt_allowed_roots`).
- **Linux.** The firejail branch (:249-316) passes `--nonewprivs`, `--noroot`, `--caps.drop=all`, `--seccomp`, `--ipc-namespace`, an nproc limit, `--net=none` when there is no network grant, and `--private=<worktree>`. It never reads `sandbox.allowed_paths` or `capabilities.filesystem_write`.

So on Linux, a plugin that declares no write capability can still write inside its private home (the worktree) and anywhere else firejail leaves writable. Its declared `allowed_paths` limit nothing.

## Why it matters

Secrets and guard (epic spec-ba7bea): the plugin manifest's sandbox fields (`crates/roko-plugin/src/manifest.rs`) promise confinement that only macOS delivers. Linux is where CI and deployed workers run.

## Where

The firejail branch of the launcher in `extension_loader.rs`, next to `macos_seatbelt_profile` and `seatbelt_allowed_roots`.

## Current state

At ad391f99a the two platforms differ as described. No test covers the firejail arguments.

## Plan

1. Map the same policy onto firejail. With no `filesystem_write`, the worktree is read-only (`--read-only=<worktree>`, plus the private home). With `filesystem_write`, only the allowed roots are writable (`--read-only=/` with `--read-write=<root>` per allowed root, or whitelist rules).
2. Factor the policy (writable roots, network) into one function that both platforms render, so they can't drift apart again.
3. Add `firejail_command_enforces_allowed_paths_and_filesystem_write`, a unit test on the built argument list, plus a Linux-only integration test where firejail exists.

## Done when

- [ ] On Linux, a plugin without `filesystem_write` can't write, and one with it can write only under its `allowed_paths`.
- [ ] The `[[verify]]` command passes.

## Notes

- Check the flag semantics against the firejail version CI installs. `--private=` sets the home directory; it doesn't restrict writes elsewhere.
