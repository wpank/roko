+++
id = "bug-2ae60f"
kind = "bug"
title = "The custody log has no cross-process lock, so two roko processes can fork its chain"
status = "done"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/safety_provenance"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "efb9acf44"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-ff95f5"
anchors = ["crates/roko-cli/src/safety_provenance.rs", "crates/roko-cli/src/custody.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-ff95f5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib custody_appends_are_serialized_across_processes"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T01:38:34Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-02T00:41:45Z"
forced = false
evidence = "Gate 6g on 698ca793d, merged as efb9acf44 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-cli, roko-core, roko-learn and roko-neuro; lib tests pass (roko-cli 3431, roko-core 1986, roko-learn 1234, roko-neuro 239); all eight canaries, golden_path_suite, secret_canary and C2 pass; graph_timeout_matrix 7/7 including the worktree-mode case; bin 445; portal tsc clean and vitest 807/807; Cargo.lock unchanged. Implemented in this round; the item's notes name the change and its test."
+++

## Problem

Appends to `.roko/custody.jsonl` are serialized within one process only. Two roko processes appending at once fork the chain, and that workspace's next resume fails closed. roko-cli's `custody.rs` also keeps its own copy of the custody hash formula (`canonical_payload`, `compute_hash`) beside roko-agent's `Custody::compute_hash` (wk-guard2).

## Plan

Take a file lock around custody appends (safety_provenance's append or `custody::log_chained`), and make roko-cli call roko-agent's hash. Add a test named `custody_appends_are_serialized_across_processes`.

## Done when

- The test passes, and one hash implementation remains.

## Notes

- Reported on 2026-10-02 by wk-tamper, working on gap-ff95f5, during the overnight close-out round.
- 2026-10-02 (wk-tamper): implemented on work/gap-7147bb; cargo verification deferred to the batch check.
  `custody::log_chained` takes an exclusive OS lock on `<log>.lock` (`std::fs::File::lock`) around reading the head
  and appending, so appends from other processes wait. Under the lock it calls roko-agent's
  `CustodyLogger::log_chained`, which seals with `Custody::compute_hash`. roko-cli's `canonical_payload` and
  `compute_hash` are gone, `check_chain_link` uses `record.compute_hash()`, and one hash implementation remains.
  Test: `custody_appends_are_serialized_across_processes` starts three child processes of the test binary
  (`custody_append_child`, `#[ignore]`d) that append 25 records each, and checks the 75 records form one chain.
