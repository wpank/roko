# SH01-T06B2C1/B2C3A integration correction independent review

- **Verdict:** `ACCEPTED`
- **Candidate:** `bfe7b281abda9bb18b84364b6dcbf5f9b3e6693b`
- **Production/test commit:** `37d411ce204f50f00c1922670893a673565a7459`
- **Integration-native base:** `d0942fc63ef734017736294843e9112b78e8a656`
- **Historical candidate/source:** `fd4238a4c8fea5ee384787927734e79eb5dfa2ba` /
  `783d645c7e07a13be72f28914a1173aee122295e`
- **Historical reviewed base:** `739750232d54369a9f8712fdd093fca0ef8f3304`
- **Original precursor source:** `3041d095d4daebed2c9e05c63eacb18e668e37e3`
- **Current integration checked:** `dd611500e7f9051fbdd3843cd20c5472efcfcbb7`
- **Review branch:** `review/SH01-gate-bfe7b281`
- **Review date:** 2026-07-14

## Independent reconstruction

I read the complete master checklist, both worker evidence records, the historical
candidate and its base, the original precursor commit, the current correction diff,
current integration, and the unchanged gate-dispatch/timeout call sites. I inspected
every candidate change and independently compared the production blocks rather than
relying on the worker's selected ranges.

The candidate lineage is exact:

```text
d0942fc63ef734017736294843e9112b78e8a656
  -> 37d411ce204f50f00c1922670893a673565a7459
  -> bfe7b281abda9bb18b84364b6dcbf5f9b3e6693b
```

The base and source commit both use event-loop blob
`4573bb208fc57f6e72048b33d368c6d27dae8131` before the test addition. The source
commit changes only `crates/roko-cli/src/runner/event_loop.rs`, adding the 19-line
`plan_verify_uses_canonical_gate_test_timeout` regression. The candidate then adds
only its uniquely named 57-line evidence file. No production line, gate-dispatch
source, master, manifest, index, changelog, lockfile, configuration, or unrelated
precursor path is changed.

## Production-path verification

All four independent source comparisons exited zero with no diff:

```text
3041 RunGate block == historical 783d RunGate block
3041 RunVerify block == historical 783d RunVerify block
fd4238 RunGate block == integration-native d094 RunGate block
fd4238 RunVerify block == integration-native d094 RunVerify block
```

The unchanged integration-native RunGate path computes the exact effect and claims
its owner before calling `mark_gate_active`. A duplicate restores the prior
`AwaitingGate` owner, emits only the duplicate debug record, and returns. The
`dispatching gate pipeline` information event is after successful activation, so a
suppressed duplicate cannot emit a false dispatch record.

The unchanged RunVerify path binds
`plan_verify_rung = gate_dispatch::RUNG_PLAN_VERIFY` (`1000`) once and uses that
value for `gate_effect_key`, `new_gate_effect`, the persisted
`gate_dispatch_started` event, and `gate_timeout`. `gate_timeout` reserves compile
and lint indexes for their configured buckets and sends all other rungs, including
the plan-verify sentinel, to `gate_test()`. No `u32::MAX` or hard-coded rung `2`
remains in the RunVerify identity/timeout path.

The base-to-source diff is exactly 19 insertions and no deletions, all inside the
focused test. This proves that the current integration production blocks were kept
byte-for-byte and that no excluded `3041` precursor hunk was replayed.

## Historical conflict reproduction

The historical candidate remains valid attribution evidence but is correctly
superseded as a merge candidate. Independent reproduction gave:

```text
git merge-base 128dc950c 783d645c7
1649c18b2c3d2b3602bfe17398b0e1454a19c5ef

git merge-tree --write-tree 128dc950c 783d645c7
exit 1
```

It reported exactly the two documented paths:

1. `event_loop.rs` content conflict. Reconstructing the three-way file located its
   sole marker at the later `let Some(mut expiry) = owner_expiry(...)` integration
   change and `LostEffect` mutation, outside both intended gate hunks.
2. `SH01-T06A-C1-C2-CORRECTION.md` add/add conflict between the historical and
   integration-native deadline evidence.

The correction truthfully preserves current integration's lost-effect/deadline
implementation and canonical deadline evidence. It does not conceal or mechanically
resolve either conflict.

## Independent verification

Validation used a new isolated Cargo target with incremental compilation disabled,
two jobs, and debug information disabled. A temporary ignored RustEmbed `dist`
symlink pointed to the already-built integration asset directory only during the
commands. Results:

```text
cargo test -p roko-cli --lib \
  runner::event_loop::tests::plan_verify_uses_canonical_gate_test_timeout -- --exact
1 passed; 0 failed

cargo test -p roko-cli --lib runner::event_loop
47 passed; 0 failed

cargo test -p roko-cli --lib runner::gate_dispatch
9 passed; 0 failed

cargo test -p roko-cli --test runner_facades_e2e
3 passed; 0 failed

cargo check -p roko-cli --lib
passed

rustfmt --edition 2024 --check crates/roko-cli/src/runner/event_loop.rs
passed

cargo fmt --all -- --check
passed

git diff --check
passed
```

The temporary symlink was removed. The isolated target measured 2.3 GiB and was
removed with `cargo clean --target-dir`; 8,106 files / 2.2 GiB were reclaimed. The
review worktree was clean before creation of this review record, with no validation
artifact left behind.

## Integration compatibility

The correction source commit has the stated direct-base merge result:

```text
git merge-base d0942fc63 37d411ce2
d0942fc63ef734017736294843e9112b78e8a656

git merge-tree --write-tree d0942fc63 37d411ce2
81e3a995064fa9a7b79590308867d597e4f8d48f
```

The complete reviewed candidate is also mechanically compatible with current clean
integration:

```text
git merge-base dd611500e bfe7b281a
d0942fc63ef734017736294843e9112b78e8a656

git merge-tree --write-tree dd611500e bfe7b281a
4537f040a528f1f6d0c56529e5aec99413ad2780
```

Current integration changes since the correction base do not overlap either
candidate path.

## Verdict

`ACCEPTED`. Confidence is high. The candidate preserves the already-correct
production source, adds the missing canonical timeout regression, records the two
historical conflicts accurately, excludes unrelated precursor work, passes every
requested gate independently, and leaves no required correction before integration.
The next action is to merge this review branch in dependency order and rerun the
focused timeout/event-loop/gate checks on the resulting integration commit before
changing canonical status.
