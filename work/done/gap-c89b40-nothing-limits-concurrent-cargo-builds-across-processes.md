+++
id = "gap-c89b40"
kind = "gap"
title = "Nothing limits concurrent cargo builds across processes that share a target dir"
status = "done"
triage = "verified"
severity = "p2"
goal = "tooling"
subsystem = ["roko-cli/gates"]
created = 2026-09-28
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/cybernetic-harness/assessment-2026-09-28/process-capacity.md"
discovered_from = "audit:tmp/cybernetic-harness/assessment-2026-09-28/process-capacity.md"
anchors = ["crates/roko-cli/src/runner/gate_dispatch.rs::compile_coordinator", "crates/roko-cli/src/runner/gate_dispatch.rs::acquire_compile_ownership", "crates/roko-cli/src/graph_task_dispatch/verification.rs::verify_compile_permit"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "sed -n '/fn acquire_compile_ownership/,/^}/p' crates/roko-cli/src/runner/gate_dispatch.rs | grep -qE 'lock_exclusive|FileExt|build_slot'"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:49Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:11:58Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++
roko's compile permits (`gates.compile_concurrency`, `gate_dispatch.rs:538`) only coordinate tasks inside one roko process. Parallel plan runs, other sessions and developer builds that share the cargo target dir queue on cargo's build lock instead. Those waits count against verify timeouts, which produces false task failures, retries and wasted spend.

The portal session alone ran 6–7 Rust agents at once, against an advised maximum of 5 per target dir. Free disk dropped to about 41 GiB when a second target dir was created.

Fix: a cross-process build slot (a lock file with a counter, or a small daemon), with verify timeouts that start only once the slot is acquired.

## Notes

2026-10-01 (wk-gates): implemented on work/bug-951930; cargo verification deferred to the batch check.
`acquire_compile_ownership` now also takes a cross-process build slot (`acquire_build_slot`): an fs2 exclusive lock on
one of `compile_concurrency` lock files in `<git common dir>/roko-build-slots/`, polled until free. It waits in
whatever is left of the step's wait, and the step's own timeout still starts only after both are held. The slots are
keyed like the in-process permits, per repository, so every worktree's runs share them, and `git status` never sees
them. Outside git, or when the files can't be opened or locked, the build runs on the in-process permit alone.
Processes in different repositories that share one `CARGO_TARGET_DIR` are not covered. It returns `CompileOwnership`
(the slot, then the permit). Tests: `build_slots_are_shared_through_lock_files` and
`build_slots_live_in_the_git_common_dir`.
