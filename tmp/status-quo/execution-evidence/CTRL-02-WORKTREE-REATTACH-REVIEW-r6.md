# CTRL-02 worktree reattachment correction — independent review r6

- Task: `CTRL-02-WORKTREE-REATTACH-r6`
- Reviewer role: independent review; no candidate implementation, manifest, status,
  master, or integration edits
- Prior rejected candidate: `15c61ddd18ae0f76d817900a9aea6b000bdc3602`
- r6 implementation: `1c5d4dce90a0a040e68c4908b1f1c5b78fc00b34`
- Exact cumulative candidate reviewed: `6eb27dd88b2276ef67ed5ea34d61153108403168`
- Integrated prior rejections: `eed6bc786` (r1), `73be06879` (r2),
  `5065180bf` (r3), `21adc2085` (r4), `cceab1280` (r5)
- Verdict: **REJECTED**
- Confidence: high

## Accepted r6 corrections

Within one configured `worktrees_root`, r6 replaces r5's replaceable marker with a
substantially stronger append-only journal. The inspected production path:

- acquires a permanent no-follow `repository.lock` and holds its `flock` through
  create/ensure/discovery/remove/prune, recovery, registry reconciliation, and
  cleanup;
- acquires `<id>.claim` with `mkdirat`, retains root/claim descriptors, validates
  owner/type/mode/link count/size, and rechecks public root and claim inode identity;
- publishes create-new, UUID-namespaced phase files with complete identity and an
  exact predecessor digest, fsyncing the files and directories without renaming over
  a stable record;
- preserves every legacy `<id>.json` entry, including malformed and dangling forms;
- records old-or-absent branch identity and uses `git update-ref` compare-and-swap;
- fails restart closed at Prepared/Linked, independently reproves reciprocal Git
  identity/list/branch/tip/unlocked state at ResetComplete, and keeps the
  self-contained cleanup-safe record until the last unlink;
- retains the earlier cancellation-independent owner, process-containment,
  sanitized-Git, reciprocal-link, effective-identity, and fail-closed cleanup work.

The focused 62-test suite and full crate suite independently pass those ordinary
and same-root paths. These improvements do not close the repository-identity lock
gap below.

## Release-blocking finding: the repository lock is scoped to a configurable worktree root

`WorktreeConfig.worktrees_root` is a public independent path (`worktree.rs:240-258`).
`acquire_repository_mutation_lock` creates and locks
`<worktrees_root>/.roko-creation/repository.lock` (`worktree.rs:1723-1756`). It does
not derive the lock location from `repo_root` or the canonical Git common directory.
Consequently, two managers configured for the same repository but different valid
worktree roots open different lock inodes. They can concurrently mutate the same
branch namespace and `.git/worktrees` registry while both believe they hold the
repository-wide owner.

This is not a hostile same-UID pathname replacement. Both configurations use fresh,
ordinary directories through the public API. It also is not covered by the r6 tests:
`repository_flock_serializes_independent_manager_instances` clones the complete
configuration, and the subprocess test opens that same one lock pathname.

### Deterministic exact-tip reproduction

I temporarily added one reviewer-only unit test at exact tip `6eb27dd8`. It created
manager A, acquired A's mutation lock, then created manager B with the exact same
`repo_root`, base branch, budget, and TTL but a second `worktrees_root`. A worker
attempted B's mutation lock while A still held its owner. Correct repository-wide
behavior was for B to remain blocked until A released.

Command (debug information was disabled only to fit the host's constrained review
disk; production/test behavior was unchanged):

```text
CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 \
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
CARGO_PROFILE_DEV_SPLIT_DEBUGINFO=off CARGO_PROFILE_TEST_SPLIT_DEBUGINFO=off \
CARGO_TARGET_DIR=/private/tmp/ctrl02-r6-review-target \
cargo test -p roko-orchestrator --lib \
  worktree::tests::reviewer_repository_lock_is_shared_across_worktree_roots \
  -- --exact --nocapture
```

Result: exit 101; 0 passed, 1 failed, 530 filtered out, at the intended assertion:

```text
same-repository manager with a different worktree root bypassed the repository lock
```

The contender acquired and released its distinct lock before A was dropped. The
test joined its worker, both lock descriptors were released, and the temporary
repository was removed. The reviewer test was then removed. The restored source
blob is `89f67fa6d49535bf56ceeda1f41167a71009bb63`, exactly equal to
`6eb27dd8:crates/roko-orchestrator/src/worktree.rs`; `git diff --exit-code` passed.

## Required correction

Anchor the permanent mutation lock to canonical repository identity, not to the
caller-selected output root. The simplest correction is a secure, never-unlinked
lock under the sanitized, canonical Git common directory, opened and inode-validated
relative to a retained common-directory descriptor. An alternative must durably
bind one canonical `worktrees_root` to that same repository and reject every
differing configuration before side effects. Revalidate the repository/common-dir
identity while acquiring the lock.

Add deterministic same-repository/different-root regressions for independent manager
instances and a separate process. Under that boundary, exercise same/different ids
and create versus remove/prune, not only raw lock acquisition. If multiple roots are
supported, also prove that a second root cannot register a branch already checked
out in the first root.

Two review-contract gaps should be corrected with the same candidate:

- The implementation and tests use `#[cfg(unix)]`, while the evidence and intended
  contract say macOS/Linux with an unsupported-platform stub. Narrow the cfg to
  `target_os = "macos" | "linux"` (and its negation) or explicitly justify, test,
  and document support for every Unix target admitted by the code.
- Add explicit hard-linked-record, mixed-UUID, live-owner-at-empty/ResetComplete,
  and each-cleanup-unlink crash-prefix tests. The validators and terminal-last order
  look conservative in static review, but the current 62 tests do not exercise all
  of those named boundaries.

## Independent verification

After restoring exact candidate source:

- `cargo test -q -p roko-orchestrator worktree::tests --lib` — exit 0; 62 passed,
  468 filtered out. This includes r1-r5 cancellation/runtime, environment identity,
  marker replacement, branch CAS, journal corruption, terminal recovery, real Git
  reciprocity/list/health, and same-root lock regressions.
- `cargo test -q -p roko-orchestrator` — exit 0; 530 unit tests, 3 integration tests,
  and 2 doctests passed; 2 doctests intentionally ignored.
- `cargo check -q -p roko-agent -p roko-orchestrator --all-targets` — exit 0 with
  pre-existing test warning debt outside the r6 changed lines.
- `cargo clippy -q -p roko-agent -p roko-orchestrator --lib -- -D warnings` — exit 0.
- `cargo fmt --all -- --check` — exit 0.
- `cargo metadata --locked --no-deps --format-version 1` — exit 0.
- `git diff --check 15c61ddd..6eb27dd8`, exact ancestry, direct implementation
  parent, and four-path scope checks — exit 0.

The first fresh reviewer target exhausted the nearly full host disk before reaching
the test. I removed only that reviewer-owned 1.0 GiB partial target and reran with
debug information disabled; the deterministic reproduction and all commands above
then completed. No candidate file was changed to obtain a pass.

Manifest review is otherwise clean: `rustix 1.1.4` is a direct orchestrator
dependency with `fs` and `process`; that exact version was already present through
`tempfile`. `Cargo.lock` changes by one dependency edge under `roko-orchestrator` and
adds no package/version/checksum. The cumulative candidate changes only
`Cargo.lock`, `crates/roko-orchestrator/Cargo.toml`, `worktree.rs`, and the worker
evidence. No public manager method, handle, or snapshot schema changed.

## Verdict

**REJECTED** for exact candidate
`6eb27dd88b2276ef67ed5ea34d61153108403168`.

Do not integrate `1c5d4dce9` / `6eb27dd88` as the durable-claim correction. Submit
a fresh immutable candidate whose kernel owner is genuinely repository-wide across
all valid manager configurations, retains the accepted inode/journal/CAS/recovery
behavior, closes the platform/test-contract gaps above, and receives fresh
independent review.
