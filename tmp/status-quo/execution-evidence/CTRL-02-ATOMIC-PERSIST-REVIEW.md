# CTRL-02 atomic persistence precursor independent review

Assignment:
- Plan: Wave 0 `CTRL-02`; bounded attribution of the atomic-write and JSONL-recovery portion of the July 14 precursor.
- Exact base: `bafcebb686d12bd83b0c9a76f0d937c8b53083dd`.
- Candidate: `06ebf26dd4b9d5fb922eb95c71a865592d94ae7c`.
- Review branch/worktree: `review/CTRL-02-atomic-persist-06ebf26dd4b9` at `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/reviews/CTRL-02-atomic-persist-06ebf26dd4b9`.
- Reserved write scope: this review record only.

Independent requirement reconstruction:
- Preserve the inherited `3041d095d4daebed2c9e05c63eacb18e668e37e3` atomic-write and JSONL recovery behavior: same-directory unique `create_new` staging, complete publication under concurrent writers, non-overwrite of an existing owned-looking staging artifact, staged-file sync before rename, Unix parent-directory sync after rename, and clearing wholly invalid or wholly partial JSONL before a later valid append.
- Close only the inherited bare-relative-target gap by normalizing its empty parent to `.` so a successful Unix rename reaches the parent-directory sync path.
- Do not claim newest-valid owned-artifact selection, stale-debris cleanup, corruption quarantine/metadata, writer coordination/rotation, or completion of issue 30, issue 58, `SH03-T02`, `SH03-T05`, or the full SH03 plan.

Primary evidence and changed-line review:
- Read the complete current master checklist, CTRL-01 recovery implementation/review evidence, the complete SH03 manifest, issues 30 and 58, the worker evidence, the complete candidate diff, the full current atomic implementation, the JSONL recovery implementation/tests, and the historical `1649c18b2c3d..3041d095d` diff for both production paths.
- The candidate is a direct child of the exact base. Its complete base diff contains only `crates/roko-fs/src/atomic.rs` and `tmp/status-quo/execution-evidence/CTRL-02-ATOMIC-PERSIST.md`; no manifest, status, index, lockfile, or unrelated production path changed.
- `crates/roko-cli/src/runner/persist.rs` has SHA-256 `7ef25d1875f40619fb5e3d13caab17be3cb61699c2b8ae31c9c48aedc2fbe50c` at `3041d095d`, the exact base, and the candidate. The base copy of `atomic.rs` likewise matches `3041d095d` byte-for-byte before this correction.
- `atomic_write_bytes` now obtains one concrete parent, creates it, exclusively creates a same-directory staging sibling, writes and `sync_all`s that file, renames it, and then `sync_all`s the concrete parent on Unix. The new helper maps a bare `state.json` parent to `.` and preserves `nested` for `nested/state.json`.
- Collision handling advances the process-global sequence without truncating the pre-existing artifact. Write/sync and rename failures remove only the staging file allocated by the current call; a post-publication parent-sync failure remains an error. There is no debris selection, cleanup, or quarantine behavior hidden in the change.
- The inherited JSONL code atomically replaces a wholly invalid complete line with an empty file, permits a subsequent valid append, and atomically clears a wholly partial first line with exact truncated-byte reporting. The candidate does not broaden those semantics.
- The worker evidence explicitly lists newest-owned recovery, startup ownership, stale cleanup, quarantine/metadata, writer coordination/rotation, issues 30/58, and full `SH03-T05` as non-goals; its final status is only `IMPLEMENTED_UNREVIEWED` for this bounded precursor.

Independent verification:
- `CARGO_TARGET_DIR=/private/tmp/roko-ctrl02-atomic-persist-review-target cargo test -p roko-fs atomic -- --test-threads=1`
  - Exit 0; 7 passed. Coverage includes unique owned sibling naming, `create_new` collision avoidance with debris preservation, eight concurrent complete-value publishers, no post-success staging residue, overwrites, parent creation, JSON round-trip, and bare-relative parent normalization.
- `CARGO_TARGET_DIR=/private/tmp/roko-ctrl02-atomic-persist-review-target cargo test -p roko-cli runner::persist -- --test-threads=1`
  - Exit 0; 8 passed. The wholly invalid file was cleared before a valid append and the wholly partial file was cleared with the expected 13-byte report. The command emitted only the pre-existing missing-docs warning while compiling the unrelated `plan_validation` test target.
- `cargo fmt --all -- --check`
  - Exit 0.
- `git diff --check bafcebb686d12bd83b0c9a76f0d937c8b53083dd..06ebf26dd4b9d5fb922eb95c71a865592d94ae7c`
  - Exit 0.
- Candidate/base and precursor ancestry checks, exact changed-path inspection, and blob SHA-256 comparisons
  - Exit 0; the exact base is the candidate merge-base and `3041d095d` is an ancestor of the base.

Adversarial checks and limitations:
- Traced the successful Unix path rather than inferring durability from target contents: file `sync_all` precedes rename, and parent `sync_all` follows every successful rename, with `.` supplied for a bare relative target.
- Confirmed eight writers can race without sharing staging state and that the final target is exactly one complete submitted value; confirmed a sequence-colliding owned-looking artifact retains its original bytes.
- Confirmed ordinary tests cannot simulate power loss or directly observe kernel persistence. Source ordering plus the focused filesystem tests is proportionate proof for this bounded precursor, not crash-point recovery proof.
- Confirmed issue 58 remains open for newest-valid artifact recovery, stale zero-byte cleanup, and ambiguous-corruption quarantine, while issue 30 remains open for broader JSONL coordination, growth, and consumption concerns.

Verdict:
- **ACCEPTED**
- Confidence: high.
- Required next action: merge this review commit into `status-quo/integration-status-quo-20260714T073140Z`, verify the candidate is an ancestor, rerun the two focused suites and formatting/diff checks on the integrated head, and update only the bounded CTRL-02 precursor attribution after that proof. Do not mark `SH03-T02`, `SH03-T05`, issues 30/58, or the full SH03 plan done from this evidence.
