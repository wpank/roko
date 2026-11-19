+++
id = "bug-c049fb"
kind = "bug"
title = "rust:fmt passes an unformatted change when the working directory's path isn't canonical: rustfmt names files by their canonical path (/private/var/... for /var/...), which fmt_diff_paths can't make relative to the directory as given, so no changed file reads unformatted"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-11-19
updated = 2026-11-19
last_verified = 2026-11-19
source = "round v8 roko-recovery, while fixing bug-eab239 (2026-11-19)"
discovered_from = "agent:roko-recovery (round v8, 2026-11-19)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/rust_checks.rs::fmt_diff_paths"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-eab239"], supersedes = [], duplicate_of = "" }
+++

## Problem

The task's Rust checks (find-fc5cdd) refuse an attempt that leaves a file it changed unformatted. `fmt_judged` reads
the files `cargo fmt --check` shows a diff of from its `Diff in <path>:<line>:` headers, makes each relative to the
attempt's working directory (`fmt_diff_paths`), and fails the check when one is a file the attempt changed.

rustfmt names each file by its canonical path. When the working directory is reached through a symlink, the header's
path doesn't start with it. On macOS that's `/var/folders/...`, which is `/private/var/folders/...`, and the same holds
for any workspace under a symlinked directory. The prefix doesn't strip, the path stays absolute, no changed file
matches, and the check passes the unformatted change.

Seen on 2026-11-19 (round v8, while fixing bug-eab239):
- With `TMPDIR=/var/folders/.../T/rr8-tmp`, `rust_task_checks_refuse_an_unformatted_change` fails at
  `rust_checks.rs:521`: "rust:fmt refuses the unformatted change", but the attempt passed.
- With the scratchpad's `/private/tmp/...` it passes.
- `cargo fmt --check` in a crate under `/var/folders/...` prints `Diff in /private/var/folders/.../src/lib.rs:1:`.

## Why it matters

Goal core. A check that passes an unformatted change lets a later task, or CI, pay for it. That is what find-fc5cdd
set out to stop.

## Where

- `crates/roko-cli/src/graph_task_dispatch/rust_checks.rs::fmt_diff_paths`.

## Plan

1. `fmt_diff_paths` strips the working directory as given or its canonical form, whichever the header starts with.
2. Test: a header with the canonical path of a working directory given through a symlink names the changed file.

## Done when

An unformatted change fails `rust:fmt` in a working directory reached through a symlink, and the `[[verify]]`
passes.
