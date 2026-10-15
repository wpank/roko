+++
id = "bug-b3849c"
kind = "bug"
title = "Gate commands, whole-plan checks and post-merge regression checks run cargo on every core at normal priority, so one run can take the whole machine"
status = "open"
triage = "unverified"
severity = "p2"
goal = "core"
size = "M"
subsystem = ["roko-gate"]
created = 2026-10-15
updated = 2026-10-15
source = "tmp/hermes-roko-v5/findings/repro-will.md (rows C2, C4, 'Gate time per real run'; roko-gates, 2026-10-15)"
discovered_from = "hermes-roko-v5 round: a demo run from Hermes in the roko repo, then the 17:02 kernel panic"
anchors = ["crates/roko-gate/src/gate_env.rs::inherit_gate_env_from", "crates/roko-core/src/child_env.rs::apply_gate_env", "crates/roko-core/src/config/gates.rs::GatesConfig"]
links = { depends_on = [], blocks = [], related = ["bug-a7e018", "bug-7b62ff"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn gate_commands_run_throttled' crates/roko-gate/ && cargo test -p roko-gate gate_commands_run_throttled"
+++

## Problem

Every command roko runs to check work (a task's verify steps and workspace rungs, a plan's whole-plan check,
the delivery's post-merge regression check) starts at the niceness roko runs at, normally 0, and its cargo
builds use every core (`CARGO_BUILD_JOBS` unset, so cargo's default of one job per CPU, and test binaries one
thread per CPU). Nothing in `[gates]` bounds them.

Seen 2026-10-15 (hermes-roko-v5 round, `tmp/hermes-roko-v5/findings/repro-will.md` rows C2/C4 and "Gate time
per real run"): a one-line demo task started from Hermes in the roko repo ran a 24 m 44 s delivery check
(`cargo fmt --all --check` + `cargo clippy --workspace`) at nice 0 with all cores, while Will had FL Studio
running. A second such check was running when the Mac kernel-panicked (WindowServer watchdog). A `roko serve`
started without a throttling wrapper ran a whole-workspace build with about 12 rustc at nice 0 and pushed the
load to 165.

## Why it matters

Hermes and `roko serve` start runs on the person's own machine, in the background, while they use it. A run's
checks must not take the machine. Will's workspace is the roko repo (36 crates, 1.2M LOC), where any check that
builds is minutes to tens of minutes of compilation.

## Where

- `crates/roko-gate/src/gate_env.rs::inherit_gate_env_from`: every gate command's environment (task verify steps
  through `ShellGate`, `plan_verify.rs::run_plan_verify`, `delivery.rs::GitDeliveryBackend::regression_output`,
  `runner/gate_dispatch.rs`, the other gates).
- `crates/roko-core/src/child_env.rs::apply_gate_env`: the gate environment policy those use.
- `crates/roko-core/src/config/gates.rs::GatesConfig`: `[gates]`.

## Plan

1. `[gates] cargo_jobs` (default: half the cores, at least 2; `0` = no cap) and `[gates] nice` (default 10; `0`
   leaves the priority alone).
2. A gate command gets `CARGO_BUILD_JOBS` and `RUST_TEST_THREADS` = `cargo_jobs` unless it already has them
   (set explicitly, or inherited from roko's own environment), and starts at niceness `nice` (raised only: a
   command already nicer stays as it is). Environment, not command rewriting, so authored commands run as
   written.
3. A Graph run applies its workspace's `[gates]` limits before anything runs, and logs them.

## Done when

`gate_commands_run_throttled` passes (the `[[verify]]`): a gate command spawned with no `CARGO_BUILD_JOBS` in
roko's environment sees the job cap in `CARGO_BUILD_JOBS` and `RUST_TEST_THREADS` and runs at niceness 10. A
real run's task gate, whole-plan check and regression check show the same (probe steps, `ps`).

## Notes

- Agents' own tool commands (`roko-std` bash/run_tests, which use `apply_gate_env` directly) and the plan-load
  spec gate's red-on-base runs (`spec_red_on_base.rs`, which does not use the gate environment at all) are not
  covered by this item.
