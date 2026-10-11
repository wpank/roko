+++
id = "bug-be2a53"
kind = "bug"
title = "With a shared CARGO_TARGET_DIR, the compile gate can pass code that does not compile (cargo reuses a same-named crate's build from another checkout)"
status = "open"
triage = "unverified"
severity = "p1"
goal = "tooling"
size = "M"
subsystem = ["roko-gate"]
created = 2026-10-11
updated = 2026-10-11
source = "CI-like local sweep 2026-10-11 (roko-90): compile_real_project with a shared CARGO_TARGET_DIR"
anchors = ["crates/roko-core/src/child_env.rs::gate_env", "crates/roko-cli/src/graph_execution/delivery.rs::shared_target_dir", "crates/roko-gate/tests/compile_real_project.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

When two directories hold crates with the same name, and their builds share one `CARGO_TARGET_DIR`, cargo can reuse
one directory's build for the other and skip compiling the second one's changed sources. A compile gate then passes
code that does not compile.

Reproduction (2026-10-11): run `roko-gate`'s `compile_real_project` test binary with `CARGO_TARGET_DIR` set to one
shared temp directory. `compile_gate_fails_on_syntax_error`, whose fixture crate `fixture` has a syntax error, passes
the gate whenever `compile_gate_passes_on_valid_project` (a valid crate of the same name in another temp directory)
builds first: the test fails with "syntax error should fail the gate", twice out of two. Run alone, or with no shared
target directory, it fails the gate correctly.

## Why it matters

Measured trust: a gate that reports green on code that does not compile. Roko runs gates in per-task attempt
worktrees of one repository, so the crate names are always the same. A shared target directory reaches the gates in
two ways:
- the gate environment passes `CARGO_TARGET_DIR` through (`child_env.rs::gate_env`, prefix `CARGO_`), and many
  developers set it globally;
- delivery's regression check deliberately builds into the process's `CARGO_TARGET_DIR` when it is set
  (`graph_execution/delivery.rs::shared_target_dir`).

Whether a real attempt run can hit this has not been checked. The memory of an earlier session also records cargo
reusing another worktree's crates under a shared target directory.

## Where

- `crates/roko-core/src/child_env.rs::gate_env`
- `crates/roko-cli/src/graph_execution/delivery.rs::shared_target_dir`
- `crates/roko-gate/src/` (the compile, test and clippy gates)
- `crates/roko-gate/tests/compile_real_project.rs` (the reproduction)

## Plan

1. Confirm the hazard on a real worktree run: two attempts of one plan, with different edits to the same crate, gated
   with one shared `CARGO_TARGET_DIR`.
2. If confirmed, give each checkout its own target directory (an APFS clone of a warm one keeps it fast), or drop an
   inherited `CARGO_TARGET_DIR` for gates run in attempt worktrees. Decide with the delivery regression's deliberate
   sharing in mind.
3. Make the reproduction a test that fails while the hazard exists.

## Done when

- [ ] Gates in two checkouts of one repository never reuse each other's build of a changed crate, and a test pins it.
