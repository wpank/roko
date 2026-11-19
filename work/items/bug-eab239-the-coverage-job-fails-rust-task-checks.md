+++
id = "bug-eab239"
kind = "bug"
title = "The coverage job fails rust_task_checks_refuse_an_unformatted_change every run: cargo llvm-cov's RUSTC_WRAPPER reaches the nested cargo clippy of the test's Rust checks without its marker variables, so it refuses (expected subcommand 'llvm-cov')"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-11-19
updated = 2026-11-19
last_verified = 2026-11-19
source = "round v8 lead: release PR #89's coverage job (run 44518589408; #88's run 43183197370)"
discovered_from = "agent:roko-recovery (round v8, 2026-11-19)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/rust_checks.rs", "crates/roko-cli/src/graph_task_dispatch/verify_depth.rs::run_check_command"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn with_check_env' crates/roko-cli/src && grep -rqw 'fn env_without_compiler_wrappers' crates/roko-cli/src && cargo test -p roko-cli --lib -- rust_task_checks_refuse_an_unformatted_change rust_checks verify_depth"
+++

## Problem

The coverage job (`cargo llvm-cov` on `ubuntu-latest`) fails `graph_task_dispatch::rust_checks::tests::
rust_task_checks_refuse_an_unformatted_change` every run: #88's (run 43183197370) and #89's (run 44518589408).
The formatted change's `rust:clippy` fails:

```
`cargo clippy -p beta --no-deps -- -D warnings`: exit code: 101
error: process didn't exit successfully: `.../cargo-llvm-cov .../clippy-driver .../rustc -vV` (exit status: 1)
error: expected subcommand 'llvm-cov', found argument '.../clippy-driver'
```

`cargo llvm-cov` builds and runs the tests with itself as `RUSTC_WRAPPER`. The test's Rust checks run a nested
`cargo clippy` through roko's check environment, which keeps `RUSTC_WRAPPER`: its allowlist admits the `RUSTC`
prefix, so a person's `sccache` still reaches their checks. The marker variables `cargo-llvm-cov` reads as a wrapper
don't come through, though, so it takes the call for a cargo subcommand, and refuses. The Test + Clippy job, which runs
no wrapper, passes the test.

Reproduced on macOS: with a stand-in for `cargo-llvm-cov` as `RUSTC_WRAPPER` in the test binary's environment, the
test fails at the same line (`rust_checks.rs:531`, rust:clippy). Without it, the test passes.

## Why it matters

Goal core. The coverage job blocks release PR #89. The test checks roko's Rust checks, not a compiler wrapper, and a
coverage harness's wrapper must not decide it.

## Where

- `crates/roko-cli/src/graph_task_dispatch/rust_checks.rs`: the test's `attempt_writing`.
- `crates/roko-cli/src/graph_task_dispatch/verify_depth.rs::run_check_command`: how a check's command gets its
  environment.

## Plan

1. A test's dispatcher can give its checks an environment of their own in place of roko's
   (`GraphTaskDispatcher::with_check_env`, test builds only). The checks still filter it through roko's check
   environment, as they filter roko's own.
2. The Rust checks test gives its checks roko's environment without `RUSTC_WRAPPER` and `RUSTC_WORKSPACE_WRAPPER`.
3. Users' checks don't change: a person running `sccache` through `RUSTC_WRAPPER` keeps it.

## Done when

The test passes with a stand-in `cargo-llvm-cov` as `RUSTC_WRAPPER` (red before) and without one, and the coverage job
passes it. Checked by the `[[verify]]`.
