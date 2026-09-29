+++
id = "bug-9c88ac"
kind = "bug"
title = "roko serve overwrites router state learned by concurrent CLI runs"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-learn/cascade-router", "roko-serve/state"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-learn/src/cascade_router.rs::CascadeRouter::save", "crates/roko-serve/src/state.rs:1397", "crates/roko-serve/src/lib.rs:1206"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'with_locked_json_transaction' crates/roko-learn/src/cascade_router.rs && cargo test -p roko-learn concurrent_saves_merge_observations"
+++

`CascadeRouter::save` replaces the whole file with this process's in-memory copy, without a lock or merge.
`roko serve` loads the router at startup and saves it on shutdown (`crates/roko-serve/src/state.rs:1270-1272`); Graph plan runs save at run end, under a different workspace lock.
Observations a CLI run persisted while serve was up are erased when serve shuts down, and vice versa (last writer wins).
Fix: save as a locked read-merge-write of this process's delta so concurrent writers never drop each other's learning.
