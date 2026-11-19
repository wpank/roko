+++
id = "gap-7bf92d"
kind = "gap"
title = "cargo clippy -p roko-cli --no-deps -- -D warnings skips test code, which has ~78 pending lint findings under --all-targets"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-cli/hygiene"]
created = 2026-10-21
updated = 2026-10-21
last_verified = 2026-10-21
source = "bug-346498 worker, 2026-10-21"
discovered_from = "bug-346498"
anchors = ["crates/roko-cli/src/spec_red_on_base.rs", "crates/roko-cli/src/replay.rs", "crates/roko-cli/src/plan_generate/pipeline.rs::plan_generation_stops_at_its_spend_cap", "crates/roko-cli/src/orchestrator/worktree/tests.rs"]
links = { depends_on = [], blocks = [], related = ["bug-346498"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo clippy -p roko-cli --no-deps --all-targets -- -D warnings"
+++

## Problem

The mandated pre-commit command (`CLAUDE.md`), `cargo clippy --workspace --no-deps -- -D warnings`, and the
scoped per-crate form bug-346498's worker ran, `cargo clippy -p roko-cli --no-deps -- -D warnings`, build only
the lib and bin targets. `#[cfg(test)]` modules (e.g. `orchestrator/worktree/tests.rs`) and
`crates/roko-cli/tests/*.rs` integration tests never get built under that invocation, so clippy never lints
them.

`cargo clippy -p roko-cli --no-deps --all-targets -- -D warnings` (checked 2026-10-21, git 2.39.5, clippy from
rustc 1.96.0) fails with 78 errors across roughly 40 files, none touched by bug-346498's fix: `replay.rs` (10),
`spec_red_on_base.rs` (11), `spec_gate.rs` (5), `tui/state/tests.rs` (5), `graph_checkpoint.rs` (4),
`runner/extension_loader.rs` (3), `tests/smoke.rs` (3), `tests/common/mod.rs` (3),
`orchestrator/worktree/tests.rs` (1, `assertions_on_constants` on a `!cfg!(target_os = "macos")` assert), and
more. Most are style/pedantic-adjacent (`len_zero`, `manual_string_new`, `assertions_on_constants`,
`enum_variant_names`, `useless_vec`, `needless_collect`, `manual_range_contains`, ...), but one is a real
pattern smell rather than a style nit: `plan_generation_stops_at_its_spend_cap`
(`plan_generate/pipeline.rs:4015`) holds a `std::sync::Mutex` `MutexGuard` (`stages.lock().expect("stages")`)
across an `.await` at line 4037 (`clippy::await_holding_lock`) -- in test code, so not a live deadlock today,
but the kind of pattern that bites if the helper is ever reused outside a test.

## Why it matters

Nobody currently runs clippy over roko-cli's test code, so lint debt accumulates invisibly there, and any
future move to a `--all-targets`/`--tests` CI lane (gap-255493 asks for an all-target run) would fail
immediately on pre-existing, unrelated findings rather than on what that change actually touched.

## Where

- `crates/roko-cli/src/**` and `crates/roko-cli/tests/**`: the ~40 files `--all-targets` flags.
- `CLAUDE.md`'s mandated `cargo clippy --workspace --no-deps -- -D warnings`, and the per-crate form: neither
  covers them.

## Plan

1. Triage the 78 findings: most look like one-line fixes (`cargo clippy --fix` may resolve several
   automatically); `await_holding_lock` in `plan_generate/pipeline.rs` needs a read, not just a suppression.
2. Fix each, or `#[allow(...)]` with a reason where a lint is a false positive for test code; then decide
   whether `--all-targets` (or `--tests`) joins the mandated pre-commit/CI clippy invocation so this debt
   cannot silently regrow.
3. Test: the `[[verify]]` command below goes green.

## Done when

`cargo clippy -p roko-cli --no-deps --all-targets -- -D warnings` passes clean (the `[[verify]]` command).

## Notes

- Found while fixing bug-346498 (2026-10-21); that fix's own clippy pass intentionally used the mandated,
  non-`--all-targets` form (CLAUDE.md's pre-commit command), both to match project convention and to avoid
  pulling ~40 unrelated files into that change.
- Not a duplicate of gap-255493 (DA-11, parked): that item tracks an all-target *test* run never being
  recorded green for CI/release purposes. This one is specifically clippy lint debt in test code that the
  mandated lint command never exercises.
