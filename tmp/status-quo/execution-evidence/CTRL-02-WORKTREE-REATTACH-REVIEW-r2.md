# CTRL-02 worktree reattachment correction — independent review r2

- Task: `CTRL-02-WORKTREE-REATTACH-r2`
- Reviewer role: independent review; no production, manifest, status, or master edits
- Rejected candidate: `4c5abf86067d801716d860294d1f19b5a4b85334`
- Corrected implementation: `38f89e6d79b63c17b1f0c4f3cf1a4227e63d7c2e`
- Exact cumulative candidate reviewed: `91a4f341087b68369e5b9695a25425708e6bc302`
- Candidate chain: `91a4f341` -> `38f89e6d` -> `4c5abf860` -> `da5e899b`
- Verdict: **REJECTED**

## Release-blocking finding

### Runtime shutdown abandons Git descendants after serialization ownership disappears

The correction fixes ordinary caller cancellation, but it does not satisfy the required task/child lifecycle under runtime shutdown. Public mutation methods move an `OwnedMutexGuard` into a Tokio task (`worktree.rs:263-273`, `383-392`, `484-492`). Dropping the runtime drops that task and therefore releases the operation mutex. The mutation helper only applies Tokio `kill_on_drop(true)` to the direct child and then awaits `wait_with_output` (`worktree.rs:935-950`). Tokio 1.51.1 documents kill-on-drop as a request to kill that child and process reaping as best effort; it does not supervise or terminate the child's process tree.

Git may execute hooks or helpers. A descendant can consequently remain alive and keep mutating after the direct Git child/task and the mutex ownership have been dropped. A later manager/runtime is then free to start another mutation. The candidate's statement that kill-on-drop is a sufficient runtime/task-abort safeguard and that no mutating child is abandoned (`CTRL-02-WORKTREE-REATTACH.md:28`) is not true for descendants. It also has no deterministic runtime-shutdown or descendant regression.

#### Exact-tip reproduction

I exported the exact candidate tip to a disposable directory and added a reviewer-only unit test; the candidate worktree was never modified. The test:

1. creates a real `WorktreeManager` and injects a Git wrapper through the existing test seam;
2. has the wrapper start a background shell descendant, with stdin/stdout/stderr detached, that records its PID, waits for a release file, then records a completion file;
3. starts `WorktreeManager::create` on a two-worker Tokio runtime and waits until both direct and descendant PID files exist;
4. drops the runtime without releasing the wrapper;
5. probes both PIDs, then explicitly releases the descendant and requires its completion marker so the reproduction itself leaves no running process.

Command:

```text
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/ctrl02-r2-target \
  cargo test -p roko-orchestrator \
  reviewer_repro_runtime_shutdown_leaves_git_descendant_alive \
  --lib -- --nocapture
```

Result: exit 0 for the vulnerability assertion, with:

```text
direct_alive_after_runtime_drop=true descendant_alive_after_runtime_drop=true
test worktree::tests::reviewer_repro_runtime_shutdown_leaves_git_descendant_alive ... ok
test result: ok. 1 passed; 0 failed
```

The direct PID still answering `kill -0` may represent a not-yet-reaped process, which is independently consistent with best-effort reaping. The descendant was demonstrably runnable rather than merely a zombie: after the runtime was gone it observed the reviewer release file and wrote the completion marker. The reviewer then waited for that marker. No test helper was added to the candidate.

This is release-blocking because it violates the assigned review criteria for descendants and background-task leakage on runtime shutdown and can reintroduce overlapping mutation outside the normal caller-abort path.

Required correction: supervise the entire mutation process tree independently of the Tokio task/runtime (for example, process-group/job containment plus a drop guard that terminates the group and provides deterministic reaping), add an adversarial runtime-shutdown regression with a long-lived descendant, and prove no descendant/direct child or Git lock survives before another mutation can enter. Reconcile disk/registry truth after an interrupted add/remove. A bounded nonclaim for unrelated managers or cross-process serialization does not cover children created by this manager's own Git invocation.

## Ordinary-cancellation assessment

The r1 rejection was valid: at `4c5abf860`, the caller owned the Tokio mutex guard and awaited `Command::output`, whose child continues by default when the future is dropped. Aborting that caller therefore released serialization while its Git child could remain active.

The r2 implementation does correct that ordinary in-runtime path:

- the owned guard and owned arguments move into a detached operation task before the public method awaits its join handle;
- cancellation while queued performs no mutation, while cancellation after acquisition leaves the operation task holding the guard through Git outcome and registry reconciliation;
- lock-internal helpers avoid recursive acquisition;
- remove retains the handle until Git succeeds and preserves it on spawn failure;
- no production panic/reentrancy path or same-manager-clone budget race was found in the reviewed diff.

The deterministic create/remove barriers and spawn-failure retry therefore pass, but they do not exercise operation-task or runtime shutdown and cannot discharge the finding above.

## Proportionate verification

After confirming the release blocker, the coordinator requested proportionate rejection evidence rather than unnecessary full gates.

- Candidate focused command: `CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/tmp/ctrl02-r2-target cargo test -p roko-orchestrator worktree --lib -- --nocapture`
  - exit 0; 39 passed, 0 failed; includes the two caller-abort barriers and remove-spawn retry.
  - Before this final run, `cargo clean -p roko-orchestrator --target-dir /tmp/ctrl02-r2-target` forced recompilation from the candidate path so the disposable reviewer harness could not contaminate the result.
- Exact worktree-module filter: `cargo test -p roko-orchestrator worktree::tests --lib -- --nocapture`
  - exit 0; 38 passed, 0 failed.
- `cargo fmt --all -- --check`: exit 0.
- `cargo metadata --locked --no-deps --format-version 1`: exit 0.
- `git diff --check 4c5abf860^..HEAD`: exit 0.
- Reserved dependency identity: no candidate diff in `Cargo.lock` or `crates/roko-orchestrator/Cargo.toml`; their `3041d095` and candidate blob IDs match respectively (`c9fb112b...` and `fb04eeaa...`).
- History: both `1649c18b2c3d2b3602bfe17398b0e1454a19c5ef` and `3041d095d4daebed2c9e05c63eacb18e668e37e3` are ancestors of the exact candidate.
- Cumulative scope from `4c5abf860^`: only `crates/roko-orchestrator/src/worktree.rs` and `tmp/status-quo/execution-evidence/CTRL-02-WORKTREE-REATTACH.md` before this review record.
- Full orchestrator and all-target checks were not rerun after the independently reproduced release blocker; the author-recorded passes are not disputed and cannot resolve process-tree lifecycle behavior.

## Bounded conclusions

Reattachment identity, unsafe-candidate preservation, cloned-manager serialization during a live runtime, remove retry, and max-live behavior are not rejected by this review. Separate managers/processes, subprocess deadlines, dirty-work ownership, runner resume, and the rest of SH02 remain correctly bounded nonclaims. The rejection is narrowly the manager-owned process tree and reconciliation boundary when the owned operation task/runtime is torn down.

Final status: **REJECTED; do not integrate `38f89e6d` / `91a4f341` as the cancellation correction.**
