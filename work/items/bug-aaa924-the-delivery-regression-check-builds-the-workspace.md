+++
id = "bug-aaa924"
kind = "bug"
title = "The delivery regression check builds the workspace from a cold target dir on every delivery"
status = "open"
triage = "unverified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph-execution"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:10, wk-merge-safety's report on bug-a3760a)"
anchors = ["crates/roko-cli/src/graph_execution/delivery.rs::regression_output", "crates/roko-cli/src/runner/gate_dispatch.rs::gate_signal"]
lane = "rust-cold"
parent = "spec-a0e40a"
links = { depends_on = ["bug-a3760a"], blocks = [], related = ["spec-f830c4", "gap-c89b40"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn regression_checkout_reuses_a_warm_target_dir' crates/roko-cli/src/ && cargo test -p roko-cli --lib regression_checkout_reuses_a_warm_target_dir"
+++

## Problem

After a plan merges, `GitDeliveryBackend::run_regression` runs the regression command (by default `cargo check --workspace --quiet`) in a temporary detached worktree of the merge commit. bug-a3760a (`809ae920d`) moved it there. The command gets no `CARGO_TARGET_DIR`, so cargo builds into that worktree's own empty `target/`. Every delivery compiles the whole workspace from nothing, and the build output is deleted with the worktree.

## Why it matters

Delivery is the integration step of the golden path (epic spec-a0e40a), and spec-f830c4 will run it once per plan. A cold build of the workspace (36 members, about 1M LOC) takes many minutes and gigabytes of disk for each plan, and it competes with running agents for the machine (gap-c89b40).

## Where

- `crates/roko-cli/src/graph_execution/delivery.rs`:
  - `GitDeliveryBackend::regression_output` adds the worktree and spawns the command in it.
  - `GitDeliveryBackend::new` sets the default command, and `with_regression_command` replaces it.
- `crates/roko-cli/src/runner/gate_dispatch.rs::gate_signal`: gates already point cargo at the main checkout's target dir (`main_target_dir`, then `payload.with_target_dir`). Reuse that pattern.

## Current state

On `work/bug-a3760a` (`809ae920d`, part of Rust batch 2), `regression_output` spawns `program` with `current_dir(&scratch.checkout)` and no target-dir setting. At BASE the regression ran in `workdir` itself, so it reused that checkout's `target/`. Moving it into a temporary worktree was the right call for safety, but it made the build cold.

## Plan

1. Give the regression command a warm target dir. Set `CARGO_TARGET_DIR` to the main checkout's target dir, the one gates use, unless the environment already sets it. If sharing that dir causes lock contention with the operator's own builds, use a dedicated dir that persists between deliveries (for example `target/delivery-regression`) instead. It is warm from the second delivery on.
2. Keep the temporary worktree: only the build output is shared.
3. Add `regression_checkout_reuses_a_warm_target_dir`. Use a regression command that records `$CARGO_TARGET_DIR`, and check that the value is the configured shared dir, not a path inside the temporary checkout.

## Done when

- [ ] The regression command builds into a target dir that is outside the temporary checkout and persists between deliveries.
- [ ] The `[[verify]]` command passes.

## Notes

- Depends on bug-a3760a, which adds this code. It is not on BASE yet.
- Concurrent cargo builds on one target dir queue on cargo's lock (gap-c89b40).
