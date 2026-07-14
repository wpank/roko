# SH01-T06A-C1-C2 correction renewed independent review

- Review timestamp: `2026-07-14T10:00:02Z`
- Verdict: **ACCEPTED**
- Candidate: `51bb0a0e5d0f20bf358198d02e06ecd5cb711f16`
- Candidate base: `206e9079812b27f738d95f91d1135d0f663c836f`
- Integration compatibility tip: `310ec1b2754aa87c55a3a75f0188f20e8d0feaa0`
- Historical worker commit: `c2f3f18fb94501aa631a401b9ce31c46378aa605`
- Historical accepted review: `739750232d54369a9f8712fdd093fca0ef8f3304`

## Context and independence

I read the complete canonical master checklist, the SH01 manifest and issues 44/52, the historical worker evidence, the historical accepted review, the candidate's correction evidence, and the exact source and history. I reviewed in a fresh worktree based on the candidate and did not modify candidate source, the master, manifests, indexes, lockfiles, or integration. Cargo validation used a newly created isolated target directory; no worker build artifact was reused.

The historical acceptance is context only. Its reviewed commit cannot be merged mechanically into the present base: `git merge-tree --write-tree 206e9079812b27f738d95f91d1135d0f663c836f 739750232d54369a9f8712fdd093fca0ef8f3304` exited `1` with its sole content conflict in `crates/roko-cli/src/runner/event_loop.rs` (base/ours/theirs blobs `d155f48b...`, `12c892ad...`, and `183dd64b...`). I therefore reviewed the reconstructed behavior rather than inheriting the old verdict.

## Scope and semantic review

`git diff --name-status 206e9079812b27f738d95f91d1135d0f663c836f..51bb0a0e5d0f20bf358198d02e06ecd5cb711f16` reports exactly:

```text
M crates/roko-cli/src/runner/deadlines.rs
M crates/roko-cli/src/runner/event_loop.rs
A tmp/status-quo/execution-evidence/SH01-T06A-C1-C2-CORRECTION.md
```

There is no manifest, status, index, lockfile, or `timeouts.rs` change. Blob verification gives:

| Path | Base | Candidate | Historical worker |
|---|---|---|---|
| `runner/deadlines.rs` | `51cf12f1...` | `1c04368d...` | `1c04368d...` |
| `runner/event_loop.rs` | `12c892ad...` | `4573bb20...` | `183dd64b...` |
| `config/timeouts.rs` | `b6f66284...` | `b6f66284...` | `b6f66284...` |

Thus the deadline correction is blob-equivalent to the reviewed historical correction and the timeout configuration is byte-for-byte unchanged. The event-loop blob intentionally differs because the reconstruction preserves the present base while adding only the eligible-agent activity behavior and regression.

The event-loop helper checks the exact active attempt, `Agent` event kind, exact owned effect, unclaimed state, and non-cancelling state before calling `record_agent_activity`. It holds mutable ownership for the check/update sequence. The registry method repeats those eligibility guards and advances activity monotonically. The adjacent base call sequence was replaced by this single helper, so the semantic conflict is deliberately and safely resolved without importing historical precursor changes.

I audited all changed source lines. They comprise only the deadline conversion regression, the activity helper and call replacement, and the exact/monotonic activity regression. No excluded precursor hunk was imported: there are no changes to model dispatch, startup/worktree behavior, plan verification, resource mutation, producer/lost-effect behavior, sibling handling, ledgers, gate activation/logging, timeout records, or drain behavior.

## Compatibility

The current integration tip changes only control-plane documents/evidence relative to the candidate base; its blobs for all three inspected Rust paths equal the base blobs. `git merge-tree --write-tree 310ec1b2754aa87c55a3a75f0188f20e8d0feaa0 51bb0a0e5d0f20bf358198d02e06ecd5cb711f16` exited `0` and produced tree `e76f54108f421168b1cd792e610ba2db31258c3c`, with no conflict.

## Independent reproduction

All commands below passed from the candidate worktree. Cargo used:

```text
CARGO_TARGET_DIR=/private/tmp/roko-sh01-deadline-51bb-review.0WVYRd
CARGO_INCREMENTAL=0
CARGO_BUILD_JOBS=2
CARGO_PROFILE_DEV_DEBUG=0
CARGO_PROFILE_TEST_DEBUG=0
```

Results:

- `cargo test -p roko-cli --lib runner::deadlines` — 8 passed, 0 failed.
- `cargo test -p roko-core config::timeouts` — 7 passed, 0 failed (all other test binaries had only filtered tests).
- `cargo test -p roko-cli --lib runner::event_loop::tests_post_gate_reflection_lessons::eligible_agent_activity_refresh_is_exact_and_monotonic -- --exact` — 1 passed, 0 failed.
- `cargo test -p roko-cli --lib runner::event_loop` — 46 passed, 0 failed.
- `cargo check -p roko-cli --lib` — passed.
- `cargo clippy -p roko-cli --lib -- -D warnings` — passed with zero warnings.
- `rustfmt --edition 2024 --check crates/roko-cli/src/runner/deadlines.rs crates/roko-cli/src/runner/event_loop.rs crates/roko-core/src/config/timeouts.rs` — passed.
- `git diff --check 206e9079812b27f738d95f91d1135d0f663c836f..51bb0a0e5d0f20bf358198d02e06ecd5cb711f16` — passed.

The first cold config-test invocation continued compiling after its orchestration session handle was lost; a duplicate invocation correctly waited on Cargo's target lock and was interrupted without touching the compiler owner. Once that owner finished, the same captured command completed successfully as recorded above. This was an execution-control incident, not a candidate failure.

The temporary review-local `demo/demo-app/dist` symlink used solely to satisfy RustEmbed compilation and the isolated 2.3 GiB Cargo target were removed after validation. `git status --short` was empty before this immutable review record was added. No validation artifact remains in the worktree.

## Verdict

**ACCEPTED.** Candidate `51bb0a0e5d0f20bf358198d02e06ecd5cb711f16` faithfully reconstructs the SH01 deadline/activity correction on the current base, preserves `timeouts.rs`, excludes precursor scope, passes the required focused and full validation, and merges cleanly with integration tip `310ec1b2754aa87c55a3a75f0188f20e8d0feaa0`.

Next action: merge the candidate into integration, merge this renewed review evidence, then run the master's post-merge verification before changing canonical status.
