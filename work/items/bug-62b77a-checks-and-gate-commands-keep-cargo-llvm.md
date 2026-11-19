+++
id = "bug-62b77a"
kind = "bug"
title = "Checks and gate commands keep cargo-llvm-cov's RUSTC_WRAPPER without its marker variables, so a nested cargo under coverage fails (expected subcommand 'llvm-cov'): the coverage job fails attempt_diff_canary's c5 tests, as it failed the Rust checks test (bug-eab239)"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-core/child_env"]
created = 2026-11-19
updated = 2026-11-19
last_verified = 2026-11-19
source = "round v8 lead: release PR #89's coverage job at ffd178a22 (run 44558280294)"
discovered_from = "agent:roko-recovery (round v8, 2026-11-19)"
anchors = ["crates/roko-core/src/child_env.rs::check_env", "crates/roko-core/src/child_env.rs::gate_env", "crates/roko-gate/src/check_env_probe.rs", "crates/roko-cli/src/check_env_probe.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-eab239"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn is_coverage_wrapper' crates/roko-core/src/child_env.rs && cargo test -p roko-core --lib -- child_env::tests"
+++

## Problem

`cargo llvm-cov` runs the tests with its own executable as `RUSTC_WRAPPER`. Beside it, it sets
`__CARGO_LLVM_COV_RUSTC_WRAPPER` and the other `__CARGO_LLVM_COV_RUSTC_WRAPPER_*` variables, which tell it that a call
is a rustc call to wrap (cargo-llvm-cov `src/wrapper.rs`). roko's check and gate environments keep `RUSTC_WRAPPER`, since
the allowlist admits the `RUSTC` prefix so that a person's sccache still reaches their checks. They drop the
`__CARGO_LLVM_COV_*` variables, because no prefix admits them. A nested cargo in a check then calls
`cargo-llvm-cov <rustc> …` without the marker, and cargo-llvm-cov takes it for a `cargo llvm-cov` command and refuses:
"expected subcommand 'llvm-cov', found argument '…/clippy-driver'".

bug-eab239 fixed this for one test only, `rust_task_checks_refuse_an_unformatted_change`, with a test-only filter. On
2026-11-19 PR #89's coverage job at ffd178a22 (run 44558280294) failed two more tests the same way:
`tests/attempt_diff_canary.rs`'s `c5_empty_diff_is_rejected_before_verify` and `c5_tampering_attempt_is_flagged`. Their
roko process's rust:clippy check (`cargo clippy --workspace --no-deps -- -D warnings`) exited 101 with that error.

## Why it matters

Goal core. The coverage job of release PR #89 fails, and so would any test whose roko runs a check that builds Rust.
The filter belongs in the environment policy every check and gate command shares, not in one test after another.

## Where

- `crates/roko-core/src/child_env.rs`:
  - `gate_env`: gate commands, and the commands agents run through roko's tools;
  - `check_env`: every check command, through `apply_check_env`.
- `crates/roko-gate/src/check_env_probe.rs` and `crates/roko-cli/src/check_env_probe.rs`: the probe each check path's
  environment test runs.
- bug-eab239's test-only filter: `GraphTaskDispatcher::with_check_env`, and `env_without_compiler_wrappers` in
  `rust_checks.rs`.

## Plan

1. Both policies leave out a rustc wrapper variable (`RUSTC_WRAPPER`, `RUSTC_WORKSPACE_WRAPPER`) whose program is
   cargo-llvm-cov. The check environment logs it under a reason of its own. A person's own wrapper, such as sccache,
   stays.
2. The probe's child runs under a stand-in cargo-llvm-cov, set up as `cargo llvm-cov` sets the real one. Every check
   path's test then asserts that the wrapper does not reach its command.
3. bug-eab239's test-only filter goes, since the policy covers that test too.
4. Proof: attempt_diff_canary and the Rust checks test, run under a stand-in cargo-llvm-cov that refuses without its
   marker, fail before the fix as in CI, and pass after it.

## Done when

No check or gate command inherits cargo-llvm-cov's wrapper, a person's sccache still reaches them, and the
`[[verify]]` passes.
