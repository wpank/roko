+++
id = "bug-4b17d9"
kind = "bug"
title = "A cargo config build.target-dir outside the checkout still makes every checkout's checks share one target dir, so a gate can pass another checkout's build"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "M"
subsystem = ["roko-core/child_env"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "bug-be2a53 worker 2026-10-17: the fix covers the environment variables only, not cargo config files"
discovered_from = "work:bug-be2a53"
anchors = ["crates/roko-core/src/child_env.rs::build_dir_is_own"]
links = { depends_on = [], blocks = [], related = ["bug-be2a53"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn a_config_target_dir_outside_the_checkout_is_not_shared' crates/roko-gate/tests/compile_real_project.rs && cargo test -p roko-gate --test compile_real_project a_config_target_dir_outside_the_checkout_is_not_shared"
+++

## Problem

bug-be2a53 drops an inherited `CARGO_TARGET_DIR`, `CARGO_BUILD_TARGET_DIR` or `CARGO_BUILD_BUILD_DIR` that names a
directory outside a check's own, so the check builds in its checkout's `target/`. Cargo also reads
`build.target-dir` and `build.build-dir` from config files: `$CARGO_HOME/config.toml` (`~/.cargo/config.toml`) and
every `.cargo/config.toml` from the check's directory up. Two kinds of value there name one directory for every
checkout:

- an absolute path;
- a relative path in a file above the checkout, since cargo takes it relative to that file's directory.

Either way the reuse hazard remains. A check in attempt worktree B can pass on attempt A's build of a crate, because
cargo names the crate's build alike in both and B's copy is older.

## Why it matters

It is the bug-be2a53 failure: a gate that reports green on code that does not compile. A global target dir in
`~/.cargo/config.toml` is a common way to share builds across projects, and it reaches every attempt worktree.

## Where

- `crates/roko-core/src/child_env.rs::build_dir_is_own`: the rule for the environment variables.
- `crates/roko-core/src/child_env.rs::apply_check_env`: where a check's environment is set.

## Current state

Only the environment variables are covered (bug-be2a53). Config files are not read.

## Plan

1. Find the target and build dirs a check would really use. Either parse the config files cargo would read, or run
   `cargo metadata --no-deps` and read its `target_directory`.
2. When one lies outside the check's checkout, give the check an explicit `CARGO_TARGET_DIR` inside its checkout.
3. Pin it with a fixture whose `.cargo/config.toml`, above two checkouts, names one target dir.

## Done when

- [ ] Checks in two checkouts never share a target dir that a cargo config file names, and a test pins it.
