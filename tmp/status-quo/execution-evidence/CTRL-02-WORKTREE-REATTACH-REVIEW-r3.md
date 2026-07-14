# CTRL-02 worktree reattachment correction — independent review r3

- Task: `CTRL-02-WORKTREE-REATTACH-r3`
- Reviewer role: independent review; no implementation, manifest, status, or master edits
- Prior rejected candidate: `91a4f341087b68369e5b9695a25425708e6bc302`
- r3 implementation: `312f445cd6c6c4f46b5e8c2ef09286b6d851b7d0`
- Exact cumulative candidate reviewed: `ee1ac3094779ccafeed85eb2dbb4f7c50135d4a5`
- Source/base: `91a4f341087b68369e5b9695a25425708e6bc302`
- Integrated prior rejections: `eed6bc786` (r1), `73be06879` (r2)
- Verdict: **REJECTED**

## Release-blocking finding

### A manager-created subprocess can escape observation before ownership cleanup

r3 correctly moves the operation and its `OwnedMutexGuard` to a caller-runtime-independent worker. It also starts the Git child in a new process group and periodically collects descendants. That does not constitute a complete ownership boundary, however. The implementation observes descendants by a process-tree snapshot followed by a 10 ms sleep (`worktree.rs:1140-1147`) and later proves absence only for the original process group and the PIDs captured in that vector (`worktree.rs:1255-1314`, `1317-1346`). A background subprocess that leaves the initial group and parent tree before a snapshot is absent from both sets.

Once the known set is reported absent, `cleanup_unproved` remains false and the independent worker drops the operation guard (`worktree.rs:1400-1407`). Disk/registry reconciliation can therefore complete and a later same-manager mutation can enter while a subprocess created by the just-cancelled Git invocation is still alive. This violates the master contract and the r2 correction requirement that manager-created descendants be absent before mutation ownership is released.

The candidate's included process-tree tests do not cover this boundary: their background subprocess remains in the wrapper's process group, so group cleanup succeeds without establishing complete descendant ownership.

## Exact-tip reproduction

I temporarily added one reviewer-only unit test at exact tip `ee1ac309`. The wrapper first completed the real canonical `git worktree add`, then created a background subprocess which left the initial process group and parent tree before the manager's next observation, recorded both PIDs, and kept the direct wrapper blocked. The test dropped the current-thread caller runtime only after the real Git side effect and both PID records existed.

Command:

```text
CARGO_TARGET_DIR=/private/tmp/ctrl02-r3-review-target \
  cargo test -p roko-orchestrator --lib \
  worktree::tests::runtime_shutdown_kills_daemonized_git_descendant_after_real_add \
  -- --exact --nocapture
```

Result: exit 101 after the intended assertion, with `0 passed; 1 failed; 508 filtered out`; the test itself completed in 0.78 s after a clean 3m11s build. At the assertion boundary it had proved all of the following simultaneously:

- the real worktree add had crossed the side-effect boundary;
- the direct wrapper was absent;
- the escaped background subprocess was still alive;
- `manager.operations.try_lock()` succeeded, so serialization ownership had been released;
- the completed worktree add had been reconciled into the in-memory registry;
- no repository Git lock remained.

The reviewer test then cleaned the background subprocess, restored the real Git executable, and successfully removed the reconciled temporary worktree before emitting its failing assertion. Thus the reproduction left no live process, worktree, or Git lock. The temporary test was removed with a named patch. The restored implementation blob is `acb8aed14864a144a583bc762b875cff9b1c35cb`, exactly equal to `ee1ac309:crates/roko-orchestrator/src/worktree.rs`; `git diff --exit-code ee1ac309 -- crates/roko-orchestrator/src/worktree.rs` passed.

## Required correction

Own the full subprocess lifetime at an OS-backed containment boundary that a child cannot leave before observation, and do not release the operation guard until that boundary proves empty and disk/registry reconciliation is complete. If complete descendant containment is not supportable on a platform, impose and enforce a rigorous trust boundary that prevents Git hooks/helpers/wrappers from creating background processes outside the owned boundary; documentation alone is insufficient. Add a regression which crosses a real create/remove side effect and demonstrates that a subprocess attempting to leave the initial group cannot survive before re-entry.

PID snapshots are also not an ownership proof: cached numeric PIDs can be reused before later probes or signals. Any correction using PID identities must bind them to stable process identity, not only a number.

## Scope, history, and hygiene

- `312f445cd` is directly based on rejected r2 tip `91a4f341`; `ee1ac309` is its evidence-only child.
- Cumulative r3 scope from `91a4f341..ee1ac309` is only `crates/roko-orchestrator/src/worktree.rs` and `tmp/status-quo/execution-evidence/CTRL-02-WORKTREE-REATTACH.md` (722 insertions, 118 deletions). The implementation commit changes only `worktree.rs`.
- Public method signatures and snapshot schema are unchanged. No runner, master, status, credential, deployment, manifest, or lockfile edit is present.
- `Cargo.lock` and `crates/roko-orchestrator/Cargo.toml` have no r3 diff.
- `cargo fmt --all -- --check`, `cargo metadata --locked --no-deps --format-version 1`, `git diff --check 91a4f341..ee1ac309`, dependency-file identity, and the base ancestry check all exited 0.
- The author's ordinary cancellation, same-group process-tree, crate, all-target, formatting, and metadata passes are not disputed. Further suite execution cannot resolve the independently reproduced lifecycle blocker and was not required after confirmation.

The review worktree contains no reviewer implementation change. Final status: **REJECTED; do not integrate `312f445cd` / `ee1ac309` as the runtime-ownership correction.**
