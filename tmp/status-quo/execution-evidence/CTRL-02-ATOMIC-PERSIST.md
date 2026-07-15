# CTRL-02 atomic persistence precursor implementation evidence

Assignment:
- Plan: Wave 0 `CTRL-02`; bounded attribution of the atomic-write and JSONL-recovery portion of the July 14 precursor.
- Base SHA: `bafcebb686d12bd83b0c9a76f0d937c8b53083dd`.
- Branch/worktree: `agent/CTRL-02-atomic-persist` at `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/workers/CTRL-02-atomic-persist`.
- Integration branch: `status-quo/integration-status-quo-20260714T073140Z`.
- Reserved write scope: `crates/roko-fs/src/atomic.rs`, `crates/roko-cli/src/runner/persist.rs`, and this evidence record.

Requirement:
- Original defect or missing behavior: before precursor `3041d095d4daebed2c9e05c63eacb18e668e37e3`, atomic writers reused and truncated a predictable sibling, did not sync the staged file and parent rename as one durability sequence, and JSONL recovery left wholly invalid files in place. The precursor added unique exclusive staging, file sync, atomic rename, Unix parent sync, and destructive clearing only when no valid JSONL prefix exists. A remaining bounded edge treated a bare relative target's empty parent as no parent and therefore skipped the parent-directory sync.
- Acceptance requirements: attribute the inherited precursor exactly; prove concurrent writes publish only complete values; prove an existing owned-looking staging artifact is not overwritten; prove invalid and wholly partial JSONL files are cleared and accept a later valid append; and ensure both nested and bare relative targets reach a parent-directory sync path.
- Explicit non-goals: newest-valid artifact discovery, startup ownership decisions, stale staging cleanup, ambiguous corruption quarantine/metadata, JSONL writer coordination/rotation, or completion of `SH03-T05` and issues 30/58.
- Dependencies and their integration commits: the historical precursor is `3041d095d4daebed2c9e05c63eacb18e668e37e3`; current base already contains it as an ancestor. Wave 0 control-plane recovery was integrated by `1a385eb52c405e9471f0ad7e23cae9650c570290`.

Reproduction:
- Pre-fix command: `git diff 3041d095d4daebed2c9e05c63eacb18e668e37e3..bafcebb686d12bd83b0c9a76f0d937c8b53083dd -- crates/roko-fs/src/atomic.rs crates/roko-cli/src/runner/persist.rs`.
- Expected: no drift from the attributed precursor before this bounded correction.
- Actual: empty diff; both reserved production files were byte-identical to `3041d095d` at assignment base.
- Pre-fix source reproduction: `atomic_write_bytes(Path::new("state.json"), ...)` received `Path::parent() == Some("")`; the empty-parent filter converted that to `None`, so Unix `sync_parent_dir` returned without opening or syncing the current directory after rename.
- Expected: a bare relative target treats `.` as its containing directory and takes the same post-rename sync path as a nested/absolute target.
- Actual: the target contents were written, but durability of the directory entry was not requested.

Implementation:
- Design and invariants: normalize the target parent once. A missing or empty parent means `.`, while an explicit nested parent is preserved. Directory creation, same-directory exclusive staging, staged-file `sync_all`, rename, and Unix parent `sync_all` now all operate on that concrete parent path.
- Files/symbols changed: `atomic_write_bytes`, `parent_dir_for`, `sync_parent_dir`, and the focused `bare_relative_target_syncs_current_directory` regression in `crates/roko-fs/src/atomic.rs`. `persist.rs` remains byte-identical to the inherited precursor and supplies the existing wholly-invalid/partial JSONL regressions.
- Compatibility/migration: public APIs and staging-file names are unchanged; this only closes the relative-target durability gap.
- Failure/recovery/security behavior: staged write/sync and rename failures still clean only the newly owned staging file best-effort. Pre-existing debris remains untouched. A parent-sync failure is still returned after publication so callers cannot mistake uncertain durability for success.

Verification:
- Command: `CARGO_TARGET_DIR=/private/tmp/roko-ctrl02-atomic-persist-target cargo test -p roko-fs atomic -- --test-threads=1`.
- Exit/result: exit 0; 7 passed, including unique naming, exclusive collision avoidance, complete publication under eight concurrent writers, no staging residue after success, and bare-relative parent normalization.
- Command: `CARGO_TARGET_DIR=/private/tmp/roko-ctrl02-atomic-persist-target cargo test -p roko-cli runner::persist -- --test-threads=1`.
- Exit/result: exit 0; 8 passed. The wholly invalid file is atomically cleared, a later valid append parses, and a wholly partial file is cleared with exact truncated-byte reporting.
- Command: `cargo fmt --all -- --check` and `git diff --check`.
- Exit/result: both exit 0.
- Command: changed-path and history inspection with `git diff`, `git show 1649c18b2c3d..3041d095d -- <reserved production paths>`, and `git merge-base --is-ancestor 3041d095d HEAD`.
- Exit/result: the base's inherited bytes and semantics trace exactly to the precursor; ancestry exit 0; candidate changes remain within the three reserved paths.

Review readiness:
- Implementation commit: the source correction and this evidence form one atomic candidate commit; its immutable SHA is recorded in the coordinator/reviewer handoff.
- Diff scope reviewed: one private parent-normalization helper, its two call-site type adjustments, one focused unit regression, and this evidence record. `persist.rs` is deliberately unchanged.
- Known limitations: ordinary unit tests cannot simulate a power loss or directly observe the kernel's completed `fsync`; production ordering is verified by changed-line inspection and the focused filesystem tests. Crash-point recovery, artifact selection, cleanup, and quarantine remain assigned to `SH03-T02`/`SH03-T05`.
- Required reviewer focus: confirm the relative path maps to `.`, every successful Unix rename reaches parent `sync_all`, unique `create_new` staging and invalid-JSONL clearing remain intact, and no full-SH03 claim is implied.

Integration:
- Independent review `5626cd136908ab00884fac702c0ad6f708cc3969`
  accepted the exact candidate with no findings; integrated implementation and
  review commits are `20fae2713` and `9c015378a`.
- Post-merge atomic tests passed 7/7, runner persistence tests passed 8/8, the
  full TUI regression selection remained green at 245/245, and the roko-cli
  all-target check, format, and diff checks passed.
- Final status: `DONE` for this bounded CTRL-02 atomic/persist precursor only.
  Crash-point recovery, newest-owned artifact selection, stale cleanup,
  quarantine, issues 30/58, and `SH03-T05` remain open.
