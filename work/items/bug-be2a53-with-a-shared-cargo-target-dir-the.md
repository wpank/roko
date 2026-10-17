+++
id = "bug-be2a53"
kind = "bug"
title = "With a shared CARGO_TARGET_DIR, the compile gate can pass code that does not compile (cargo reuses a same-named crate's build from another checkout)"
status = "open"
triage = "verified"
severity = "p1"
goal = "tooling"
size = "M"
subsystem = ["roko-gate"]
created = 2026-10-11
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "200ced75c"
source = "CI-like local sweep 2026-10-11 (roko-90): compile_real_project with a shared CARGO_TARGET_DIR"
anchors = ["crates/roko-core/src/child_env.rs::gate_env", "crates/roko-core/src/child_env.rs::build_dir_is_own", "crates/roko-cli/src/graph_execution/delivery.rs::regression_target_dir", "crates/roko-gate/tests/compile_real_project.rs"]
links = { depends_on = [], blocks = [], related = ["bug-07ce0f", "bug-779ae7", "bug-df2b12", "bug-0d0d37", "bug-4b17d9", "gap-7c3e13", "gap-834de5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn build_dir_is_own' crates/roko-core/src/child_env.rs && grep -qw 'fn a_shared_target_dir_does_not_pass_another_checkouts_build' crates/roko-gate/tests/compile_real_project.rs && cargo test -p roko-core --lib build_dir && cargo test -p roko-gate --test compile_real_project a_shared_target_dir_does_not_pass_another_checkouts_build"

[[verify]]
command = "grep -qw 'fn regression_target_dir' crates/roko-cli/src/graph_execution/delivery.rs && grep -qw 'fn c5_a_task_check_never_passes_on_another_worktrees_build' crates/roko-cli/tests/plan_branch_integration.rs && cargo test -p roko-cli --lib regression_builds_in_its_own_target_dir && cargo test -p roko-cli --test plan_branch_integration c5_a_task_check_never_passes_on_another_worktrees_build"
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

## Current state

Confirmed 2026-10-17 at `200ced75c`, cargo 1.96.1. Cargo names a workspace member's build the same in every checkout.
Its metadata hash and fingerprint use paths relative to the workspace root, and the dep-info lists sources relative to
the package. Freshness is then decided by modification time: a unit counts as fresh when every source in this checkout
is older than the last build of that unit, wherever that build came from.

- **Gate level, real worktrees.** A repository with two `git worktree add` checkouts under `.roko/worktrees/`, and
  `CARGO_TARGET_DIR` set to the main checkout's `target/`. Attempt b broke the crate before attempt a's gate built a's
  valid edit. `CompileGate` passed b, and cargo reported `"fresh":true` for
  `repo/target/debug/deps/libroko_gate_fixture-16239ce87217853d.rmeta`, which was a's build. The item's original
  reproduction failed 2 of 3 runs.
- **Plan level, a real `roko plan run --worktree-per-task`.** The C5 fixture: two parallel tasks editing one crate,
  a scripted provider, and an exported `CARGO_TARGET_DIR`, with sharing brought back through
  `[gates] env_passthrough = ["CARGO_TARGET_DIR"]`. Task B's `cargo check` passed on A's build. B's non-compiling
  `src/b.rs` reached `roko/plan/c5`, and the run reported `plan_outcomes.c5 = "succeeded"` and delivered it into the
  batch branch. No whole-plan check ran for this single-crate project, which is gap-834de5.
- **Earlier sightings.** The 2026-08-26 dogfood run (bug-07ce0f, parked, "Stale worktrees cause cargo to compile
  wrong crate version"). The 2026-09-29 parallel push, where a worker compiled against another worker's roko-core.
  bug-779ae7, which gave the tautology-probe crates their own names. The golden-path harness, which already builds
  with a relative `CARGO_TARGET_DIR` (`6cd8b2997`).

Fixed on `work/bug-be2a53`. The second option of the plan was taken: drop an inherited shared dir.

- **Check commands.** Every check builds its environment through `child_env::apply_check_env`. That function now
  leaves out an inherited `CARGO_TARGET_DIR`, `CARGO_BUILD_TARGET_DIR` or `CARGO_BUILD_BUILD_DIR` that names a
  directory outside the command's own (`build_dir_is_own`, `CheckEnvRemoval::SharedBuildDir`), so cargo builds in
  the checkout's own `target/`. A relative value that stays inside is kept. An absolute value is compared with
  symlinks resolved. A `build.build-dir` template is the checkout's own when it is rooted at `{workspace-root}` or
  holds `{workspace-path-hash}`. A command whose directory is not set yet keeps none of them, so `ShellGate` now
  sets the step's directory before its environment.
- **The delivery's regression check** no longer shares a target dir. It used the process's `CARGO_TARGET_DIR`, else
  the person's `workdir/target`. It now builds in `.roko/state/regression-target/` (`regression_target_dir`),
  beside its stable checkout, so it is warm after the first delivery. bug-700d7c's order is unchanged: the run
  branch moves only after that check passes.
- **Opt-out.** Naming one of these variables exactly in `[gates] env_passthrough` keeps it, sharing included. The
  docs (`[gates] env_passthrough`, `docs/v2/19-CONFIG.md`) say what that costs.

## Plan

1. Confirm the hazard on a real worktree run: two attempts of one plan, with different edits to the same crate, gated
   with one shared `CARGO_TARGET_DIR`.
2. If confirmed, give each checkout its own target directory (an APFS clone of a warm one keeps it fast), or drop an
   inherited `CARGO_TARGET_DIR` for gates run in attempt worktrees. Decide with the delivery regression's deliberate
   sharing in mind.
3. Make the reproduction a test that fails while the hazard exists.

## Done when

- [x] Gates in two checkouts of one repository never reuse each other's build of a changed crate, and a test pins it.
  Pins:
  - `roko-gate` `compile_real_project::a_shared_target_dir_does_not_pass_another_checkouts_build`: real git worktrees
    and `CompileGate`. It failed before the fix and passes after.
  - `roko-core` `child_env` tests `a_build_dir_is_the_checks_own_only_inside_its_directory` and
    `check_env_keeps_only_the_build_dirs_of_the_checks_directory`.
  - The check-env probes in roko-gate and roko-cli now also export a shared dir for each variable, and assert that
    no check path builds there.
  - `roko-cli` `delivery::tests::regression_builds_in_its_own_target_dir`.
  - `plan_branch_integration::c5_a_task_check_never_passes_on_another_worktrees_build`.

## Notes

- Not covered here, and filed instead:
  - Agents' own builds still use the shared dir: roko's tool commands and the provider CLIs (bug-df2b12).
  - `work/README.md` tells workers to export one `CARGO_TARGET_DIR` (bug-0d0d37).
  - A `build.target-dir` in a cargo config file outside the checkout, such as `~/.cargo/config.toml` (bug-4b17d9).
  - Warm per-worktree targets through a safe APFS clone, the plan's first option (gap-7c3e13).
- Every attempt worktree's checks now build cold, as they already did without an exported `CARGO_TARGET_DIR`.
- bug-07ce0f (parked) describes the same hazard and can be marked a duplicate of this item.
