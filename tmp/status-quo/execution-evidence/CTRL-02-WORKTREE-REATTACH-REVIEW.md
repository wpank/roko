# CTRL-02 worktree-reattach precursor independent review

- Verdict: **REJECTED**
- Exact candidate: `4c5abf86067d801716d860294d1f19b5a4b85334`
- Candidate parent/base: `da5e899b68afd0edc4384d3974d93186f7057a76`
- Historical precursor: `3041d095d4daebed2c9e05c63eacb18e668e37e3`
- Historical parent: `1649c18b2c3d2b3602bfe17398b0e1454a19c5ef`
- Review branch/worktree: `review/CTRL-02-worktree-reattach-4c5abf86` /
  `reviews/CTRL-02-worktree-reattach-4c5abf86`
- Confidence: high

## Independent scope and requirement reconstruction

I read the complete master, the complete SH02 manifest, issues 05, 13, 20,
48, 49, 50, 51, and 53, the self-heal audit and SH02 changelog boundaries,
the worker evidence, the complete candidate diff, the full current worktree
manager and its production call sites, the relevant Cargo metadata, and the
exact historical `1649c18b2..3041d095d` worktree diff.

The candidate is a direct child of its stated base and changes exactly:

- `crates/roko-orchestrator/src/worktree.rs`;
- `tmp/status-quo/execution-evidence/CTRL-02-WORKTREE-REATTACH.md`.

It changes no Cargo manifest or lockfile, runner/resume/persist/merge path,
master, self-heal manifest, issue status, index, or ownership ledger. Both
historical commits are ancestors of the candidate. The assigned base's
`Cargo.lock`, orchestrator manifest, and worktree source were byte-unchanged
from `3041d095d` before this correction.

The bounded acceptance target is safe same-process reattachment and operation
reservation for clones of one manager. It is not task-attempt isolation,
immutable gate input, durable task commit, dirty-work ownership/quarantine,
PID/workspace-lock recovery, merge rollback, E47, or completion of any full
SH02 task. Separate managers and separate processes remain explicitly outside
the mutex's claim.

## Accepted portions of the candidate

The ordinary completed-operation behavior is strong:

1. Clones share one fair Tokio operation mutex. `create`, `ensure_for_plan`,
   `discover_existing`, `remove`, and `prune` take it, while the internal
   create helper prevents recursive acquisition by `ensure_for_plan`.
   Concurrent duplicate ensure creates one worktree, and concurrent creates
   cannot race the `max_live` check within one shared manager.
2. An absent path is distinguished from every present-but-unproved path.
   Present invalid candidates return the typed `ReattachRejected` error from
   `ensure_for_plan` and are not removed or replaced; discovery skips and logs
   them without mutation.
3. Reattachment requires a non-symlink exact canonical child, a regular `.git`
   file, a matching canonical Git top-level, the configured canonical common
   directory, the exact non-detached `roko/plan/<id>` branch, and equality
   between `HEAD` and the exact `refs/heads/roko/plan/<id>` tip.
4. Already-tracked handles are re-proved before ensure returns them. A stale
   snapshot path or branch cannot be substituted for the canonical candidate,
   and duplicate snapshot ids are rejected rather than overwritten.
5. Reattached creation time is derived from directory mtime and clamped to
   now; successful ensure updates activity. Existing create/remove/health,
   stale-lock, reclaim, and prune tests remain green. The added public error
   variant compiles at every in-repository call site.

These properties close the worker's ordinary duplicate-create reproduction
and fail-closed identity gaps, but they do not cure the cancellation defect
below.

## Release-blocking finding

### High: cancellation releases the reservation while Git continues mutating

The worker evidence claims a cancellation-safe operation guard, but every Git
subprocess is still awaited through `tokio::process::Command::output()` without
`kill_on_drop(true)` or an independently owned operation task:

- create: `worktree.rs:248-293`;
- prune: `worktree.rs:703-710`;
- remove helper: `worktree.rs:857-863`;
- reattachment probes: `worktree.rs:900-908`.

Tokio 1.51.1 is the resolved local dependency. Its checked source explicitly
states that a spawned process continues after the `Child` or output future is
dropped by default, initializes `kill_on_drop` to `false`, and kills on drop
only when that option is enabled. Cancelling a task awaiting any command above
drops the local async-mutex guard and the output future together. The Git child
may therefore continue after another clone acquires the purported exclusive
operation reservation. A cancelled `git worktree add`, remove, or prune can
overlap a later ensure/create/remove/prune and recreate the exact check/use and
Git-metadata races the mutex is meant to exclude.

`remove` has an additional deterministic state-loss window independent of how
quickly Git exits. At lines 434-444 it removes the handle from `active` before
awaiting `git_remove`. The error path restores the handle, but cancellation is
not an error return and executes no restoration. The manager can thus report
the worktree untracked while the child is still removing it, or permanently
lose the handle if the child fails after the caller is cancelled.

No candidate test cancels create, ensure, remove, or prune at a controlled
subprocess barrier. The passing concurrency tests let every Git process finish,
so they cannot establish the stated cancellation-safety contract.

Required correction: make the entire reserved operation and its state
reconciliation outlive cancellation of the public caller. One robust design is
an owned background operation that retains the mutex guard through child exit
and registry commit/restore; merely dropping the caller must not expose a
second mutation while the first Git process is live. If kill-on-drop is chosen
instead, the implementation must also prove deterministic reconciliation of
partially applied Git state and preserve the removed handle until successful
completion. Add barrier-controlled regressions that cancel at least create and
remove after the child starts, then prove no overlapping Git mutation, no lost
registry entry, no orphaned branch/worktree metadata, and successful retry.
Re-run the existing duplicate ensure, discover/ensure, budget, identity, and
full compatibility gates. Preserve the separate-process nonclaim.

## Independent verification

Focused real-Git suite:

```text
CARGO_TARGET_DIR=.../integration/target \
  cargo test -p roko-orchestrator worktree --lib -- --nocapture

exit 0; 36 passed; 0 failed; 467 filtered out
```

This independently passed exact reattachment, wrong branch, detached HEAD,
foreign common directory, missing metadata, symlink rejection, tracked stale
identity, duplicate snapshots, concurrent ensure/discover, one Git
registration, max-live contention, timestamps, create/remove, health,
stale-lock, reclaim, and prune coverage.

Full affected crate:

```text
CARGO_TARGET_DIR=.../integration/target cargo test -p roko-orchestrator

exit 0; 503 unit tests passed; 3 lifecycle integration tests passed;
2 doctests passed and 2 were intentionally ignored
```

Affected call-site compilation:

```text
CARGO_TARGET_DIR=.../integration/target \
  cargo check -p roko-orchestrator -p roko-cli --all-targets

exit 0
```

Warnings are pre-existing in orchestrator resource-budget/merge-queue tests
and `roko-cli/tests/plan_validation.rs`, outside the candidate.

Additional gates:

```text
cargo metadata --locked --no-deps --format-version 1   exit 0
cargo fmt --all -- --check                             exit 0
git diff --check <base>..<candidate>                   exit 0
candidate path allow-list                              exact two paths
candidate parent                                      exact assigned base
1649c18b2 and 3041d095d ancestor checks               pass
manifest/lock candidate diff                           empty
review worktree before this evidence                   clean
```

The historical source diff plus the new ordinary concurrency regression are
sufficient to corroborate the pre-fix duplicate-create race; rebuilding an
entire historical workspace would not address the candidate's distinct,
statically proved cancellation failure.

## Verdict and next action

**REJECTED** for exact candidate
`4c5abf86067d801716d860294d1f19b5a4b85334`.

Do not merge it as accepted work. Submit a fresh immutable candidate that
retains the accepted identity, typed-rejection, snapshot, timestamp, and
ordinary concurrency behavior; makes create/remove/prune mutation ownership
survive caller cancellation through complete Git/state reconciliation; adds
deterministic cancellation regressions; corrects the worker evidence's
cancellation-safety claim; and receives fresh independent review. Do not mark
SH02-T01/T02/T03/T04/T05/T06, issue 53 dirty recovery, E47, or CTRL-02 done from
this bounded review.
