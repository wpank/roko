+++
id = "q-c02c36"
kind = "question"
title = "Baselining excludes clippy/cargo-check, so a clippy-dirty crate's audit ladder can never step down: extend it?"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-cli/runner", "roko-gate/audit"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-10 follow-up reports 2026-10-04 (PK64 gap-2e4a81)"
discovered_from = "gap-2e4a81"
anchors = ["crates/roko-cli/src/runner/gate_dispatch.rs::run_focused_baseline_verify", "crates/roko-cli/src/runner/gate_report.rs::filter_preexisting_failures"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

`run_focused_baseline_verify` (`crates/roko-cli/src/runner/gate_dispatch.rs:1998-2011`) only re-runs `test`-kind
cargo commands against the base commit to build `baseline_failed_gates`:

```rust
let steps = steps.into_iter().filter(|step| {
    cargo_command_fingerprint(&step.command).is_some_and(|fingerprint| fingerprint.action == "test")
}).collect::<Vec<_>>();
```

`clippy`/`cargo check` steps never get a baseline computed. `filter_preexisting_failures`
(`crates/roko-cli/src/runner/gate_report.rs:52-73`) can only exempt a current failure when it finds a matching
failed gate of the *same name* in `baseline` — since `baseline` never carries a clippy entry, a clippy failure
can never be exempted as "pre-existing," however long the crate has been clippy-dirty on its base commit.

So on a crate whose `cargo clippy -- -D warnings` already fails at the base commit (unrelated to the agent's
work), S05's V1 rung fails every single attempt, deterministically — there is no way for that crate to produce a
green unit at V1. The strictness ladder's step-down condition (`UCB95(θ) < θ_max/2` over two consecutive
windows, S05 §4.6/DP3) needs green units to even measure θ against; with zero green units at V1 for that crate,
the ladder can climb up but can structurally never step back down.

## Why it matters

Goal: cybernetic, M4 deep audits / release readiness. Any crate that isn't already clippy-clean becomes
permanently stuck at the deepest verify rung for every task touching it, regardless of whether the agent's
actual changes are fine — not because of evidence the crate is risky, but because the baseline-exemption
machinery structurally excludes clippy from ever being exempted.

## Where

- `crates/roko-cli/src/runner/gate_dispatch.rs::run_focused_baseline_verify` (the `action == "test"` filter).
- `crates/roko-cli/src/runner/gate_report.rs::filter_preexisting_failures` (the exemption logic this feeds).
- `crates/roko-gate/src/audit/feedback.rs` (the DP3 ladder's step-down condition, which needs green units).

## Why this needs Will

Extending the baseline-exemption machinery to cover clippy/cargo-check means re-running them against the base
commit on every attempt (a real cost: a full clippy pass per attempt, not just the changed files) — the original
exclusion of cargo checks from the "red on base" baseline was presumably a deliberate cost/complexity trade-off,
not an oversight. Whether that trade-off still holds now that it also means "a dirty crate's ladder can never
step down" is a decision, not a fact to fix in code: either accept the ladder staying stuck for dirty crates (and
treat "make every crate clippy-clean" as the real fix), or pay the cost of baselining clippy too.

## Notes

- Discovered during PK64's work (gap-2e4a81, done).
- If Will decides to extend baselining to cargo checks, that becomes a regular `gap`/`bug` item with its own
  implementation and verify, scoped to control the added per-attempt cost.
