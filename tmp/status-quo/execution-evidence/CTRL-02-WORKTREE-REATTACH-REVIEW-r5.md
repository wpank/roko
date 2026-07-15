# CTRL-02 worktree reattachment correction — independent review r5

- Task: `CTRL-02-WORKTREE-REATTACH-r5`
- Reviewer role: independent review; no candidate implementation, manifest, status, master, or integration edits
- Prior rejected candidate: `16497357ff4613d1aa4d73e29a2acb7ffb08bfae`
- r5 implementation: `84d0c7d9972bb5e3212b78d0efd8fab32c8ca08d`
- Exact cumulative candidate reviewed: `15c61ddd18ae0f76d817900a9aea6b000bdc3602`
- Integrated prior rejections: `eed6bc786` (r1), `73be06879` (r2), `5065180bf` (r3), `21adc2085` (r4)
- Verdict: **REJECTED**
- Confidence: high

## Accepted r4 corrections

The r5 diff substantively addresses the three r4 findings in ordinary, non-racing execution:

- Identity, policy, common-directory, health, and mutation commands select the same resolved Git executable. Probe and mutation builders remove inherited and command-explicit `GIT_*` variables. The raw ambient-Git helpers are gone, and the included command-local spoof regression exercises the prior `GIT_DIR`/`GIT_WORK_TREE` bypass.
- Reattachment parses the candidate's regular `.git` pointer, rejects a symlink/non-directory administrative target, canonicalizes it as a direct child of the configured common directory's `worktrees` directory, requires a regular non-symlink reciprocal `gitdir` file resolving to the candidate `.git`, and only then performs sanitized Git identity/branch/tip probes.
- Initial `Prepared` publication uses a no-replace hard-link acquisition. Direct `create`/`create_for_plan` reject existing regular and dangling marker entries before their normal identity/mutation path. Legal phase and full marker-identity checks are present before transition and removal.
- The Unix containment predicate now rejects real root, effective root, and every real/effective UID mismatch before Linux capability validation. The r4 direct-child containment, cancellation-independent owner, cleanup reconciliation, and bounded nonclaims remain in place.

Those corrections close the reproduced r4 environment bypass and the ordinary outstanding-marker replacement path. They do not close the durable claim race below.

## Release-blocking finding: marker validation and replacement are not one atomic claim operation

`transition_creation_marker` reads and compares the stable marker (`worktree.rs:1409-1414`), writes a new temporary phase record, and then unconditionally renames it over the stable pathname (`worktree.rs:1415-1423`). Another manager or restart recovery actor can replace the stable pathname after the comparison but before the rename. The transition then overwrites a different durable claim even though the evidence says foreign/outstanding claims are never replaced. `remove_creation_marker_if_exact` has the same check-then-remove shape (`worktree.rs:1462-1474`). The shared in-memory mutex does not serialize another process and cannot survive restart.

### Deterministic exact-tip reproduction

I temporarily added one reviewer-only regression at exact tip `15c61ddd`. It allowed the transition to validate the expected `Prepared` bytes, replaced the stable pathname with a distinct foreign marker before the transition's replacement step, and then completed the transition. Correct behavior was an error with the foreign bytes unchanged.

Exact test result:

```text
worktree::tests::reviewer_transition_preserves_a_raced_foreign_claim
exit 101; 0 passed, 1 failed, 517 filtered out
panic: transition overwrote a raced foreign claim
```

The first identical invocation did not reach the test because its fresh reviewer target exhausted local disk space. I removed only reviewer-owned old r3/r4 build caches and reran the same test; the assertion above completed in 0.12 seconds. The test joined its helper thread before asserting, made no candidate-repository mutation, and left no child process or Git lock. Its disposable temporary directory was removed during unwind.

The reviewer test was then removed with a named patch. `crates/roko-orchestrator/src/worktree.rs` has blob `2f8fc620bc721e9b43e9abc80e72682e7baac938`, exactly equal to `15c61ddd:crates/roko-orchestrator/src/worktree.rs`; `git diff --exit-code 15c61ddd -- crates/roko-orchestrator/src/worktree.rs` passed.

## Required correction

Use an atomic per-id durable claim/transition mechanism that binds validation and replacement/removal to the same claimed inode or exclusive lock, serializes cooperating managers across processes, and preserves recoverable identity across restart. A transition must return an error and preserve bytes if the stable claim changed after validation. Apply the same guarantee to exact removal. Add deterministic regressions for both transition and removal at that boundary, then obtain fresh review of a new immutable tip.

## Scope and proportional verification

- `84d0c7d99` is a direct child of r4 tip `16497357f`; evidence-only `15c61ddd1` is its direct child.
- Cumulative r5 scope is only `crates/roko-agent/src/process/group.rs`, `crates/roko-orchestrator/src/worktree.rs`, and `tmp/status-quo/execution-evidence/CTRL-02-WORKTREE-REATTACH.md` (606 insertions, 176 deletions).
- Public manager methods, `WorktreeHandle`, and snapshot schema are unchanged. No runner, master/status, config, credential, deployment, manifest, lockfile, or external-service change is present.
- Manifest/lockfile identity, `git diff --check 16497357..15c61ddd`, exact ancestry, and source restoration passed.
- The author-recorded 6 process-group tests, 49 worktree tests, full orchestrator suite, all-target check, lib clippy, formatting, and metadata passes are consistent with the inspected ordinary paths. Per coordinator instruction, no broader suite was repeated after the deterministic release blocker.

Final status: **REJECTED; do not integrate `84d0c7d99` / `15c61ddd1` as the durable identity correction.**
