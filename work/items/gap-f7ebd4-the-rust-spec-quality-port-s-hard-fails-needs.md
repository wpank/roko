+++
id = "gap-f7ebd4"
kind = "gap"
title = "The Rust spec-quality port's hard_fails needs HF3's accept_tests == 0 condition to stay in parity with speclint"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-gate/spec_quality"]
created = 2026-09-30
updated = 2026-10-01
last_verified = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-specq's report)"
anchors = ["crates/roko-gate/src/spec_quality.rs", "benchmarks/viabilitybench/speclint/speclint.py"]
lane = "rust-cold"
parent = "spec-e57870"
links = { depends_on = [], blocks = [], related = ["bug-019f02", "bug-c1b845"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -A4 '// HF3' crates/roko-gate/src/spec_quality.rs | grep -q 'accept_tests'"
+++

## Problem

After bug-019f02, speclint counts `[task.accept]` tests, and its HF3 hard fail ("verify passes on the unchanged base") applies only to a task with no accept tests. The Rust port counts `accept_tests` (`crates/roko-gate/src/spec_quality.rs:236`, :376), but HF3 in its `hard_fails` (:898) is still `if red_on_base == RedOnBase::Pass && implementer && !all_pass_on_base` (:931), without the matching `&& task.accept_tests == 0`. HF1 (:910) already has it. Today HF3 is static-unknown in Rust (`STATIC_UNKNOWN`, :77), so this matters only once Rust gets a dynamic mode.

## Why it matters

Specs a cheap model can execute (epic spec-e57870): the Python and Rust linters must agree (`speclint/rust_parity.py`). p3.

## Where

HF3 in `hard_fails`.

## Plan

1. Add the condition, and a parity fixture with an accept-only task.

## Done when

- [ ] Rust's HF3 matches speclint's.
- [ ] The `[[verify]]` command passes.

## Notes

- Implemented on `work/bug-8417d9` at `979b2c9e9`; cargo verification deferred to the batch check.
- 2026-10-01 (wk-specq): the premise's condition is backwards. Since `e12249d1e` (bug-c1b845), speclint treats a
  task with a pinned acceptance test as expecting red: `expects_red = implementer and (accept or not
  all_pass_on_base)`. `&& accept_tests == 0` would have dropped HF3 for accept-only tasks, which speclint flags.
  - Rust now computes `expects_red = implementer && (task.accept_tests > 0 || !all_pass_on_base)`, so the two
    differ nowhere. Before, they differed only for a task whose own steps all declare `pass_on_base` beside a
    pinned test.
  - The accept-tests fixture gains T5 (a pinned test beside a `pass_on_base` step) and T6 (the same step
    alone), vendored to roko-gate.
  - With the base passing, `hf3_counts_a_pinned_acceptance_test_as_expecting_red` (Rust) and
    `test_hf3_counts_a_pinned_acceptance_test_as_expecting_red` (Python) both expect HF3 for T1 and T5, and not
    for T6.
  - The speclint Python tests pass (91), as do roko-gate's `spec_quality` tests (16), including the golden-fixture
    parity on T5 and T6.
