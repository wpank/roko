+++
id = "bug-13bbe9"
kind = "bug"
title = "CI coverage: canaries that run a nested cargo fail under cargo-llvm-cov (the gate inherits its RUSTC_WRAPPER but not its marker variables)"
status = "done"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["roko-core/child-env"]
created = 2026-10-11
updated = 2026-11-19
last_verified = 2026-11-19
last_verified_rev = "930673ae1"
source = "PR #85 coverage run 38012399914"
discovered_from = "dec-01be49 (the coverage run got past the doctor tests)"
anchors = ["crates/roko-core/src/child_env.rs::gate_env", ".github/workflows/coverage.yml"]
links = { depends_on = [], blocks = [], related = ["bug-62b77a"], supersedes = [], duplicate_of = "" }

[closed]
at = 2026-11-19
at_ts = "2026-11-19T16:57:29Z"
commit = "930673ae1"
forced = false
evidence = "930673ae1 (bug-62b77a, which duplicated this item) implements option 2: roko_core::child_env::is_coverage_wrapper leaves a RUSTC_WRAPPER or RUSTC_WORKSPACE_WRAPPER naming cargo-llvm-cov out of gate commands (gate_env) and every check command (check_env), a person sccache kept. attempt_diff_canary c5_tampering_attempt_is_flagged and c5_empty_diff_is_rejected_before_verify, which failed exactly as in PR #85 (run 38012399914) and PR #89 (run 44558280294) under a stand-in cargo-llvm-cov set as cargo llvm-cov sets it, pass under it after the fix; the check-environment probe now asserts the wrapper reaches no check path. The coverage job on PR #89 confirms it on Linux."
+++

## Problem

CI's coverage job (`cargo llvm-cov`, `.github/workflows/coverage.yml`) fails in roko-cli's `attempt_diff_canary`:
`c5_tampering_attempt_is_flagged` and `c5_empty_diff_is_rejected_before_verify` (PR #85, run 38012399914). The
fixture plan's whole-plan check runs `cargo clippy --workspace --no-deps -- -D warnings`, and that nested cargo fails:
`cargo-llvm-cov clippy-driver rustc -vV` exits 1 with "expected subcommand 'llvm-cov', found argument …clippy-driver".

The same canaries pass in the canaries job and in Test + Clippy, which run without coverage. The failure was hidden
until 2026-10-09: the coverage run stopped earlier, at the doctor tests (dec-01be49, now fixed).

## Why it matters

The coverage job stays red. Because `cargo llvm-cov` stops at the first failing test binary, every roko-cli
integration test after `attempt_diff_canary` gets no coverage, and other canaries that run nested cargo would likely
fail the same way.

## Where

- `crates/roko-core/src/child_env.rs::gate_env`: a gate command keeps `RUSTC*` and `CARGO_*` variables by
  allowlist prefix. So it inherits `RUSTC_WRAPPER=cargo-llvm-cov` but drops cargo-llvm-cov's own `__CARGO_LLVM_COV_*`
  marker variables, and without them the wrapper parses its arguments as a command line.
- `.github/workflows/coverage.yml`, which runs `cargo llvm-cov` over lib and integration tests.

## Plan (options)

1. The coverage job measures unit tests only (`cargo llvm-cov --lib --bins`). The canaries already run without
   coverage in the canaries job. Simplest, but coverage numbers drop.
2. A gate command does not inherit a `RUSTC_WRAPPER` or `RUSTC_WORKSPACE_WRAPPER` that names `cargo-llvm-cov`: an
   instrumented parent should not instrument a nested fixture build.
3. The gate keeps the wrapper's companion variables (`__CARGO_LLVM_COV_*`) as well, so the nested build is also
   instrumented.

Option 2 is the smallest product change that keeps coverage complete.

## Done when

- [ ] The coverage job gets past `attempt_diff_canary` (and the other canaries) on a PR.
