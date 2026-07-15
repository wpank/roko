# SH01-T06A-C1-C2-CORRECTION implementation evidence

Assignment:
- Plan: `tmp/status-quo/self-heal/plans/SH01-runner-lifecycle/tasks.toml`
- Base SHA: `206e9079812b27f738d95f91d1135d0f663c836f`
- Branch/worktree: `agent/SH01-DEADLINES-INTEGRATION-CORRECTION` at `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/workers/SH01-DEADLINES-INTEGRATION-CORRECTION`
- Integration branch: `status-quo/integration-status-quo-20260714T073140Z`
- Reserved write scope: `crates/roko-cli/src/runner/deadlines.rs`, `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-core/src/config/timeouts.rs`, and this evidence file

Requirement:
- Original defect or missing behavior: the current integration base already contains the deadline representation/selection hardening, timeout default assertions, and direct eligible-event activity update reconstructed from the historical precursor. It lacks the accepted duration saturation boundary regression and the helper-based exact/monotonic activity regression. The previously accepted branch cannot be merged because it was based on `1649c18b2` and conflicts with the deliberately retained precursor changes in the integrated `event_loop.rs`.
- Acceptance requirements: preserve the current integrated event loop byte-for-byte outside the accepted helper/call/test hunks; retain the centralized saturating deadline conversion, allocation-free exact-owner earliest selection and stable ties; retain all five timeout default assertions; make the routed activity refresh one exact eligibility-checked operation; and prove wrong-effect, stale-time, and post-phase-transition behavior.
- Explicit non-goals: do not rebase, merge, cherry-pick, or mechanically resolve the old branch; do not remove or rewrite integrated startup model validation, worktree rediscovery, gate logging, lost-effect, sibling-drain, timeout-ledger, or other precursor behavior; do not edit the master, manifests, indexes, lockfiles, or unrelated source.
- Dependencies and integration base: all prerequisites are represented by integration commit `206e9079812b27f738d95f91d1135d0f663c836f`.

Prior accepted candidate and conflict disposition:
- Old accepted candidate: `c2f3f18fb94501aa631a401b9ce31c46378aa605`, based on `1649c18b2c3d2b3602bfe17398b0e1454a19c5ef`.
- Old independent review: `739750232d54369a9f8712fdd093fca0ef8f3304`, verdict ACCEPTED for that exact candidate.
- Integration preflight: from clean integration HEAD `206e90798`, `git merge-tree --write-tree HEAD review/SH01-DEADLINES-c2f3f18fb945` exited 1 with a content conflict in `crates/roko-cli/src/runner/event_loop.rs`; no merge was attempted and integration stayed clean.
- Conflict cause: integration already used the historical two-step `event_is_eligible` then `record_agent_activity` path and contained other intentionally retained precursor hunks, while the accepted candidate replaced that two-step block with `refresh_eligible_agent_activity` on a much older base. Mechanical merge/rebase would make preservation of the newer integrated event loop ambiguous.
- Disposition: this candidate was authored afresh from the exact integration head. It adds only the accepted duration boundary test and the accepted helper/call/test behavior. The existing timeout file is already blob-identical to the old accepted candidate (`b6f66284330c9812f4783e6239e2583bdce93e75`) and therefore needs no edit.

Reproduction:
- The integration base directly checks exact Agent ownership and then separately calls `record_agent_activity`. Runtime behavior is already correct, but no focused routed-path regression proves that eligibility, refresh, and monotonicity remain one operation.
- The integration base defines the centralized `duration_millis_u64` conversion but lacks a direct test of exact ordinary conversion and saturation for `Duration::MAX`.
- The rejected merge preflight above reproduces why the old accepted history cannot safely enter the current integration graph unchanged.

Implementation:
- `crates/roko-cli/src/runner/deadlines.rs`: adds the accepted exact/saturating conversion boundary regression; production deadline arithmetic and allocation-free stable owner selection were already present and remain unchanged.
- `crates/roko-cli/src/runner/event_loop.rs`: adds `refresh_eligible_agent_activity`, replaces only the existing adjacent eligibility/update sequence with the helper call, and adds the accepted exact-effect/stale-clock/phase-transition regression. Every other integrated event-loop hunk remains unchanged.
- `crates/roko-core/src/config/timeouts.rs`: unchanged because its current blob exactly matches the accepted candidate and already asserts hard-run 3600s, task-attempt 600s, gate-effect 900s, agent-silence 180s, and scheduler-no-progress 600s defaults.
- Compatibility/failure semantics: no public API, serialization, configuration key, migration, cancellation, producer-loss, ledger, or gate behavior changes. Ineligible events remain ignored; eligible exact-owner events refresh only `last_agent_activity_at`; stale times cannot regress it; attempt and phase clocks stay independent.

Verification:
- `cargo test -p roko-cli --lib runner::deadlines` — PASS, 8 passed, 0 failed.
- `cargo test -p roko-core config::timeouts` — PASS, 7 passed, 0 failed; unrelated integration test binaries selected zero filtered tests.
- `cargo test -p roko-cli --lib runner::event_loop::tests_post_gate_reflection_lessons::eligible_agent_activity_refresh_is_exact_and_monotonic -- --exact` — PASS, 1 passed, 0 failed.
- `cargo test -p roko-cli --lib runner::event_loop` — PASS, 46 passed, 0 failed, including the integrated lost-effect and timeout-ledger regressions absent from the old candidate.
- `cargo check -p roko-cli --lib` — PASS.
- `cargo clippy -p roko-cli --lib -- -D warnings` — PASS.
- `rustfmt --edition 2024 --check crates/roko-cli/src/runner/deadlines.rs crates/roko-cli/src/runner/event_loop.rs crates/roko-core/src/config/timeouts.rs` — PASS.
- `git diff --check` — PASS.
- Source-diff search for startup-model/worktree, `RUNG_PLAN_VERIFY`, mutable-resource, lost-effect/producer, sibling, ledger, and gate-log terms — no matches; only the accepted helper/call/test and duration-test hunks changed.
- `git hash-object` proved the working deadline blob equals the old accepted deadline blob `1c04368da1c91aecbcfa8a5304429250d3f9889b`, and the unchanged timeout blob equals the old accepted timeout blob `b6f66284330c9812f4783e6239e2583bdce93e75`.
- Cargo commands used `CARGO_INCREMENTAL=0`, two build jobs, and the populated integration target. A transient worker-local `demo/demo-app/dist` symlink supplied the existing integration SPA build output for RustEmbed; it was removed before staging and is absent from final status.

Review readiness:
- Corrected implementation commit: the implementation and this evidence form one atomic commit; its immutable SHA is recorded in the coordinator/reviewer handoff.
- Required independent review: review this new immutable integration-based candidate; the old ACCEPTED verdict does not transfer across the conflict reconstruction.
- Required reviewer focus: prove the diff contains only the two source files plus this evidence; confirm timeout blob equivalence; confirm every current integrated precursor hunk survives; rerun the focused gates; and verify the old merge conflict has been eliminated by the new base rather than hidden in a manual merge.

Integration:
- Corrected candidate: `51bb0a0e5d0f20bf358198d02e06ecd5cb711f16`.
- Renewed independent review: `ACCEPTED` in
  `SH01-T06A-C1-C2-CORRECTION-REVIEW-RENEWED.md`; review commit
  `58ee07f2b97ec1d08893cf5a510fad965b763d7d`.
- Integration merge: `df76374841eaed25f6a15df2dd6b690626bb2de2`.
- Post-merge proof: the first run exposed stale cross-worktree Cargo artifacts (the
  source already contained all three allegedly missing APIs). Package-scoped Cargo
  cleanup removed the contaminated target entries; the fresh rebuild then passed
  deadlines 8/8, timeout config 2/2, exact eligible activity 1/1, and the full
  event-loop module 46/46. `git diff --check` and integration status are clean.
- Final status: `DONE` for this bounded CTRL-02 precursor reconstruction. This does
  not mark any enclosing SH plan task complete.
