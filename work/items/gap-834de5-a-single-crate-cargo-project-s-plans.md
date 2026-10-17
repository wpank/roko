+++
id = "gap-834de5"
kind = "gap"
title = "A single-crate Cargo project's plans get no whole-plan check: the plan footprint never names a package at the workspace root"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "bug-be2a53 worker 2026-10-17: a C5 fixture run with a shared target dir delivered a non-compiling crate as succeeded"
discovered_from = "work:bug-be2a53"
anchors = ["crates/roko-cli/src/graph_execution/plan_set.rs::owner", "crates/roko-cli/src/graph_execution/plan_verify.rs::default_plan_verify"]
links = { depends_on = [], blocks = [], related = ["bug-be2a53"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_root_package_gets_the_default_whole_plan_check' crates/roko-cli/src/graph_execution/ && cargo test -p roko-cli --lib a_root_package_gets_the_default_whole_plan_check"
+++

## Problem

A plan in a Cargo project whose package sits at the workspace root gets no default whole-plan check. That covers
any single-crate repository with `[package]` in its root `Cargo.toml`.

- `CargoWorkspace::owner` skips a package whose root is the workspace root (`!root.as_os_str().is_empty()`).
- `AreaResolver::enclosing_project` skips the root directory as well.
- So an edit to `src/a.rs` becomes `Area::File("src/a.rs")`.
- `default_plan_verify` makes steps only for `Area::Package` and `Area::AllPackages`, so it returns none.

The delivery then merges the plan into the batch branch with no check at all.

Seen 2026-10-17 while working on bug-be2a53. The fixture of `plan_branch_integration::c5_…` has root package `c`,
two tasks editing `src/a.rs` and `src/b.rs`, and no `[meta] verify`. Run with
`[gates] env_passthrough = ["CARGO_TARGET_DIR"]`, which brings back the shared target dir, it delivered a
non-compiling `src/b.rs` into `roko/batch/<run>`. The run summary said `plan_outcomes.c5 = "succeeded"`, because
no whole-plan check ran to catch it.

## Why it matters

The whole-plan check (gap-60233f) catches tasks that each pass their own checks but break the build together
(canary C4). Single-crate Rust projects are common. Their plans never run it, so a broken integration is delivered
and reported as succeeded.

## Where

- `crates/roko-cli/src/graph_execution/plan_set.rs::owner` skips the root package; `enclosing_project` skips the
  root directory.
- `crates/roko-cli/src/graph_execution/plan_verify.rs::default_plan_verify` makes steps for packages only.
- Entry point: the whole-plan checks of `run_graph_plan` (`plan_runner.rs`, "Whole-plan checks (gap-60233f)").

## Current state

A workspace whose packages sit under subdirectories gets `cargo fmt`, `clippy` and `test -p <package>` steps.
Excluding the root package keeps it from claiming the paths of every member package.

## Plan

1. When no other package owns a path, and the path is not inside a member package's tree, fall back to the root
   package. The footprint then names it, and `default_plan_verify` gets `-p <root package>` steps.
2. Add a test with a root-package workspace.

## Done when

- [ ] A plan that edits a root package's sources gets the default whole-plan steps for that package.
