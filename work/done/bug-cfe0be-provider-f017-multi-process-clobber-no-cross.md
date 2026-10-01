+++
id = "bug-cfe0be"
kind = "bug"
title = "Multi-process clobber: no cross-process atomicity for provider-health.json"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-learn/provider_health"]
created = 2026-09-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F017"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F017"
anchors = ["crates/roko-learn/src/provider_health.rs::save_snapshot", "crates/roko-learn/src/provider_health.rs::spawn_save_worker", "crates/roko-learn/src/provider_health.rs::ProviderHealthRegistry"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'with_locked_json_transaction' crates/roko-learn/src/provider_health.rs && cargo test -p roko-learn provider_health::tests::concurrent_registries_merge_on_save  (roko_fs::with_locked_json_transaction at crates/roko-fs/src/atomic.rs:55 is the existing locked read-modify-write helper; the test must be added by the fix)"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:56Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:12:13Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++
Multiple concurrent roko processes (e.g., parallel plan runs) each read and write `provider-health.json` independently. Without cross-process locking, concurrent writes can corrupt the file or overwrite each other's health state updates.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F017`

How to verify: Confirm in crates/roko-learn/src/provider_health.rs whether still true: Multi-process clobber: no cross-process atomicity for `provider-health.json`

Verified 2026-09-28: writes are now atomic (save_snapshot -> roko_fs::atomic_write_json, crates/roko-learn/src/provider_health.rs:689), so torn or corrupt files are prevented. But each process persists its whole in-memory map (save worker ~:639-680), guarded only by an in-process save_lock Mutex (:398). With no cross-process lock or read-merge, concurrent runs overwrite each other's updates (last writer wins). Severity lowered to p2: health state is advisory and self-heals.

## Notes

- 2026-10-01 (wk-tiers): implemented on work/bug-7cdce7; cargo verification deferred to the batch check.
  - `save_snapshot` used by the debounced save worker and by `save()`: it now runs
    `roko_fs::with_locked_json_transaction`, a sibling advisory lock with a read-merge-write.
  - Each provider keeps whichever record saw the most recent outcome (`last_failure_at`/`last_success_at`), and ours
    wins a tie. So two runs sharing a workspace no longer drop each other's providers, and a stale in-memory copy
    cannot undo a newer trip.
  - A file that does not parse is replaced, with a warning, rather than blocking every later save.
  - Tests: `concurrent_registries_merge_on_save` and `an_unreadable_health_file_is_replaced_on_save`.
  - Not done:
    - A running process does not adopt the other process's merged state into memory until it reloads.
    - Counters of a provider both processes used are not summed; the newer record wins whole.
