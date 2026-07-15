# SH01-T06B2C1/B2C3A integration-native correction evidence

- Recorded: `2026-07-14T10:44:17Z`
- Integration base: `d0942fc63ef734017736294843e9112b78e8a656`
- Integration-native source commit: `37d411ce2`
- Historical precursor: `3041d095d4daebed2c9e05c63eacb18e668e37e3`
- Superseded historical candidate tip: `fd4238a4c8fea5ee384787927734e79eb5dfa2ba`
- Superseded historical source commit: `783d645c7e07a13be72f28914a1173aee122295e`

## Correction and production-source disposition

The historical candidate was useful attribution evidence but is not a mergeable integration candidate. This correction is based directly on current integration and preserves its `event_loop.rs` production source byte-for-byte.

Current integration already contains both intended production changes from `fd4238a4c`/`3041d095d`:

1. RunGate emits `dispatching gate pipeline` only after `mark_gate_active` succeeds; the duplicate path restores ownership and returns before that information event.
2. RunVerify binds `plan_verify_rung = gate_dispatch::RUNG_PLAN_VERIFY` and uses it for the effect key, gate effect, persisted dispatch event, and timeout lookup.

Exact structural comparison was reproduced with these commands; both exited `0` with no output:

```text
diff -u <(git show fd4238a4c8fea5ee384787927734e79eb5dfa2ba:crates/roko-cli/src/runner/event_loop.rs | sed -n '6349,6398p') <(sed -n '6366,6415p' crates/roko-cli/src/runner/event_loop.rs)
diff -u <(git show fd4238a4c8fea5ee384787927734e79eb5dfa2ba:crates/roko-cli/src/runner/event_loop.rs | sed -n '6561,6632p') <(sed -n '6578,6649p' crates/roko-cli/src/runner/event_loop.rs)
```

Before the focused regression was added, the integration `event_loop.rs` blob was `4573bb208fc57f6e72048b33d368c6d27dae8131`. The base-to-source-commit diff adds only the 19-line `plan_verify_uses_canonical_gate_test_timeout` regression; there is no production edit. The regression proves the plan-verify sentinel selects `gate_test_secs`, not the compile or clippy timeout bucket.

No master, manifest, index, changelog, lockfile, configuration, gate-dispatch source, or other source file is changed.

## Historical conflicts and supersession

The historical candidate's merge-tree against integration tip `128dc950c1659b49b85dc7d052ee0ea0dbc7bb12` failed on two paths because its merge base was the original dirty-root commit rather than the reconstructed integration lineage:

- `crates/roko-cli/src/runner/event_loop.rs` conflicted at integration's later mutable `owner_expiry`/lost-effect handling, outside the two intended precursor hunks. Correct disposition: preserve the integration deadline/lost-effect implementation. The intended RunGate and RunVerify production blocks were already present and therefore required no replay here.
- `tmp/status-quo/execution-evidence/SH01-T06A-C1-C2-CORRECTION.md` had an inherited add/add conflict between historical and reconstructed deadline evidence. Correct disposition: preserve integration's canonical reconstructed evidence. This correction does not edit or duplicate that file.

Accordingly, `fd4238a4c` is superseded as an integration candidate by this direct integration-based correction. Its history remains evidence of attribution and validation, not a branch to merge mechanically.

## Validation

All validation used a fresh isolated Cargo target with incremental compilation disabled, two build jobs, and debug information disabled:

- `cargo test -p roko-cli --lib runner::event_loop::tests::plan_verify_uses_canonical_gate_test_timeout -- --exact` — 1 passed, 0 failed.
- `cargo test -p roko-cli --lib runner::event_loop` — 47 passed, 0 failed.
- `cargo test -p roko-cli --lib runner::gate_dispatch` — 9 passed, 0 failed.
- `cargo test -p roko-cli --test runner_facades_e2e` — 3 passed, 0 failed.
- `cargo check -p roko-cli --lib` — passed.
- `rustfmt --edition 2024 --check crates/roko-cli/src/runner/event_loop.rs` — passed.
- `git diff --check` — passed.

The temporary RustEmbed symlink and isolated 2.1 GiB Cargo target were removed after the gates. No validation artifact remains.

## Integration compatibility

`git merge-tree --write-tree d0942fc63ef734017736294843e9112b78e8a656 37d411ce2` exited `0` and produced tree `81e3a995064fa9a7b79590308867d597e4f8d48f`. The merge base is exactly `d0942fc63ef734017736294843e9112b78e8a656`; there is no content or evidence conflict.

This integration-native correction is ready for fresh independent review. Canonical status must remain unchanged until that review and post-merge verification are complete.

## Integrated result

- Corrected candidate: `bfe7b281abda9bb18b84364b6dcbf5f9b3e6693b`.
- Independent review: `ACCEPTED` in
  `SH01-T06B2C1-B2C3A-INTEGRATION-CORRECTION-REVIEW.md`; review commit
  `81b92cd20d4f8301fd03b1e97bf91b895d261205`.
- Integration merge: `915d3c246c93f0227de4f32790b491ecb9aa2029`.
- Post-merge proof: focused canonical timeout 1/1, event-loop 47/47,
  gate-dispatch 9/9, and runner facades 3/3 pass; diff check and integration
  status are clean.
- Final status: `DONE` for this bounded CTRL-02 precursor attribution. No
  enclosing SH plan task is marked complete by this reconstruction.
