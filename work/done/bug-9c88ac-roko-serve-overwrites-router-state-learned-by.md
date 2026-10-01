+++
id = "bug-9c88ac"
kind = "bug"
title = "roko serve overwrites router state learned by concurrent CLI runs"
status = "done"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-learn/cascade-router", "roko-serve/state"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "b11ca807d"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-learn/src/cascade_router.rs::CascadeRouter::save", "crates/roko-serve/src/state.rs:1397", "crates/roko-serve/src/lib.rs:1206"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'with_locked_json_transaction' crates/roko-learn/src/cascade_router.rs && cargo test -p roko-learn concurrent_saves_merge_observations"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "CascadeRouter::save is a with_locked_json_transaction read-merge-write that adds only what this router learned since load or its last save (counters and LinUCB A/b additive, arms keyed by slug); a corrupt file is backed up and replaced; test concurrent_saves_merge_observations (7769a5ae4, merged b11ca807d). Batch 6 gate (work/rust-batch-5 tree plus fmt-only and unused-import fixes fdb2a9b72, 579ffd0e6, a8dd7f09f, 2aa55ab1f): cargo check --workspace --tests clean; clippy -p roko-cli -p roko-core -p roko-agent -p roko-serve -p roko-learn -p roko-gateway --no-deps -D warnings clean; lib tests roko-cli 3087, roko-agent 2249, roko-core 1922, roko-learn 1177, roko-serve 955, roko-gate 685, roko-gateway 41, 0 failed; merged MAIN tree re-checked (cargo check --workspace --tests clean)."
+++

`CascadeRouter::save` replaces the whole file with this process's in-memory copy, without a lock or merge.
`roko serve` loads the router at startup and saves it on shutdown (`crates/roko-serve/src/state.rs:1270-1272`); Graph plan runs save at run end, under a different workspace lock.
Observations a CLI run persisted while serve was up are erased when serve shuts down, and vice versa (last writer wins).
Fix: save as a locked read-merge-write of this process's delta so concurrent writers never drop each other's learning.

## Notes

- Implemented on `work/bug-605a8a` at `7769a5ae4`; cargo verification deferred to the batch check.
