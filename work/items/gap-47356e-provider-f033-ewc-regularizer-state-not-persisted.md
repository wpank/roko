+++
id = "gap-47356e"
kind = "gap"
title = "EWC regularizer state not persisted in CascadeSnapshot"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-learn/bandits"]
created = 2026-09-01
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F033"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F033"
anchors = ["crates/roko-learn/src/model_router.rs::export_linucb_snapshot", "crates/roko-learn/src/model_router.rs::import_linucb_snapshot", "crates/roko-learn/src/cascade/persistence.rs::LinUCBSnapshot"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'ewc' crates/roko-learn/src/cascade/persistence.rs && cargo test -p roko-learn cascade_snapshot_round_trips_ewc_state"
+++
The EWC (Elastic Weight Consolidation) regularizer state that prevents catastrophic forgetting is not included in the `CascadeSnapshot` serialization. On every process restart, EWC state resets to zero, effectively discarding the accumulated regularization history.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F033`

How to verify: Confirm in crates/roko-learn/src/bandits.rs, crates/roko-learn/src/cascade_router.rs whether still true: EWC regularizer state not persisted in `CascadeSnapshot`

Verified 2026-09-28: EWC state lives on ArmState.ewc (crates/roko-learn/src/model_router.rs:441-443, serde-enabled). Production persistence, though, goes through CascadeSnapshot.linucb_state = LinUCBSnapshot {a_matrices, b_vectors, dim, observations} (crates/roko-learn/src/cascade/persistence.rs:15-23), and export_linucb_snapshot (model_router.rs:1408) / import_linucb_snapshot (:1440) drop it. LinUCBRouter::save/load, which would keep ArmState, are only used in tests. CascadeSnapshot now lives in cascade/persistence.rs, not cascade_router.rs. Severity p2.
