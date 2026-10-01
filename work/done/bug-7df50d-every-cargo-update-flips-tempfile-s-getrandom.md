+++
id = "bug-7df50d"
kind = "bug"
title = "Every cargo update flips tempfile's getrandom dependency between 0.4.3 and 0.3.4 in Cargo.lock"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["workspace/Cargo.lock"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-ci2's report on find-8cc7ac)"
anchors = ["Cargo.lock", "Cargo.toml"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = ["find-8cc7ac"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "d=$(mktemp -d) && cp Cargo.lock \"$d/\" && cargo update --quiet; after=$(awk '/^name = \"tempfile\"/{p=1} p&&/^$/{exit} p' Cargo.lock | grep -o 'getrandom 0\\.[0-9]*'); cp \"$d/Cargo.lock\" Cargo.lock; before=$(awk '/^name = \"tempfile\"/{p=1} p&&/^$/{exit} p' Cargo.lock | grep -o 'getrandom 0\\.[0-9]*'); test -n \"$after\" && test \"$after\" = \"$before\""

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:21Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:12:50Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

At ad391f99a, `Cargo.lock` gives tempfile 3.27.0 the dependency `getrandom 0.4.3`. The lock holds three getrandom versions (0.2.17, 0.3.4, 0.4.3), and 0.4.3 has five users. wk-ci2 reports that every `cargo update` re-resolves tempfile's edge to `getrandom 0.3.4`, so the lock changes on every update, even when nothing else did.

## Why it matters

Hygiene (epic spec-9a3131): lock churn produces noise diffs and merge conflicts between parallel branches, and it hides real dependency changes in review.

## Where

`Cargo.lock` (tempfile's entry) and the workspace `Cargo.toml` (`resolver = "2"`, `rust-version = "1.91"`).

## Current state

The cause is not established. tempfile's requirement probably admits both getrandom 0.3 and 0.4, and the resolver's choice depends on how the lock was produced.

## Plan

1. Reproduce: `cargo update`, then diff tempfile's entry.
2. Find out why the resolver picks 0.3.4: tempfile's version requirement, feature unification, or resolver settings.
3. Make the result stable. Commit the lock that `cargo update` produces (if 0.3.4 is fine), or pin the edge through a direct dependency, a `[patch]` or a resolver setting.
4. The verify command runs a full `cargo update`, compares tempfile's getrandom major version before and after, and restores the lock.

## Done when

- [ ] `cargo update` leaves tempfile's getrandom edge unchanged.
- [ ] The `[[verify]]` command passes. It needs network access for the index.

## Notes

- 2026-10-01 (wk-filer4): implemented on work/gap-2bc1b9; cargo verification deferred to the batch check. Cause: tempfile 3.27.0 asks for `getrandom = ">=0.3.0, <0.5"` (its Cargo.toml, target deps). Both getrandom 0.3.4 and 0.4.3 are in the graph for other crates, so either satisfies the edge. A full `cargo update` re-resolves without the old lock and picks 0.3.4 (wk-ci2's observation), while the committed lock had 0.4.3 from an incremental resolve. Cargo.lock now records tempfile -> getrandom 0.3.4, what a full update produces, so the update leaves it alone. Both versions stay locked (4 users each). The gate's cargo step runs the item's verify, which re-runs `cargo update`, compares and restores the lock.
