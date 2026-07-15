# CTRL-02 worktree reattachment correction — independent review r4

- Task: `CTRL-02-WORKTREE-REATTACH-r4`
- Reviewer role: independent review; no implementation, manifest, status, master, or integration edits
- Prior rejected candidate: `ee1ac3094779ccafeed85eb2dbb4f7c50135d4a5`
- r4 implementation: `8dff7d32b93c1f2d5caebb03615e89d4d7cbc940`
- Exact cumulative candidate reviewed: `16497357ff4613d1aa4d73e29a2acb7ffb08bfae`
- Candidate parent/source: `8dff7d32b93c1f2d5caebb03615e89d4d7cbc940`
- Integrated prior rejections: `eed6bc786` (r1), `73be06879` (r2), `5065180bf` (r3)
- Verdict: **REJECTED**

## Release-blocking finding: reattachment identity is controlled by ambient Git variables

The mutation path strips inherited `GIT_*` variables (`worktree.rs:1679-1692`), but the reattachment probes do not use that boundary. `try_reattach_locked` invokes the free helpers `git_canonical_path`, `git_common_dir`, and `git_stdout` (`worktree.rs:1046-1086`). `git_stdout` launches the ambient string `git` with the inherited process environment unchanged (`worktree.rs:1821-1830`). The preceding `.git` check establishes only that a regular file exists; it neither parses that file nor proves reciprocal linkage to the configured repository (`worktree.rs:1034-1044`).

Consequently, inherited `GIT_DIR` and `GIT_WORK_TREE` values can make every Git probe report an attacker-selected ordinary directory as the configured repository's canonical worktree. Branch and tip checks then query the configured repository selected by `GIT_DIR`, not the candidate's dummy `.git` file. The candidate is registered even though real Git has no linked-worktree record for it. This violates the fail-closed identity requirement and can make later manager operations act on a path whose ownership was never established.

### Exact-tip reproduction

I temporarily added one reviewer-only unit test at exact tip `16497357`. It created a real temporary configured repository, put that repository on the expected `roko/plan/<id>` branch, created the exact canonical candidate directory with a regular `.git` file containing only `not a linked-worktree pointer`, and set inherited `GIT_DIR=<configured-repo>/.git` plus `GIT_WORK_TREE=<candidate>`. Production `discover_existing` returned the id and `manager.get(id)` returned a handle. A separate real `git worktree list --porcelain`, with both variables explicitly removed, proved that the candidate was absent from Git's worktree metadata.

Command:

```text
CARGO_INCREMENTAL=0 \
CARGO_TARGET_DIR=/private/tmp/ctrl02-r4-review-target \
  cargo test -p roko-orchestrator --lib \
  worktree::tests::reviewer_repro_git_environment_spoofs_reattach_identity \
  -- --exact --nocapture
```

Result: exit 0 with `1 passed; 0 failed; 511 filtered out`. The assertion that the invalid candidate was accepted therefore passed. The temporary test was removed with a named patch. The restored implementation blob is `bc31a30584aa545321ccc1d951547748878cabbe`, exactly equal to `16497357:crates/roko-orchestrator/src/worktree.rs`; `git diff --exit-code 16497357 -- crates/roko-orchestrator/src/worktree.rs` passed.

## Outstanding-marker correction

The evidence says the durable marker itself is create-new. The implementation only opens a uniquely named temporary file with `create_new`; it then renames that file onto the stable marker pathname (`worktree.rs:1291-1316`). On Unix, that rename replaces an existing marker. The public `create`/`create_locked` path does not reject an outstanding marker before choosing a new administrative directory and writing `Prepared` (`worktree.rs:400-445`).

After a restart following unproved cleanup, a direct `create` call can therefore destroy the only durable record of the original manager-owned administrative directory and replace it with a different identity. Subsequent rollback cannot reconstruct or safely remove the original object. Keeping an in-memory guard forever is not durable across process restart, so this contradicts the candidate's durable recovery and identity-preservation claim.

## Effective-root policy correction

`validate_no_descendant_context` rejects only real UID zero via `getuid` (`group.rs:19-31`). On supported non-Linux Unix systems there is no capability check. A process with nonzero real UID but effective UID zero, or otherwise mismatched real/effective identities, is therefore accepted despite the documented policy that privileged callers are rejected. The policy needs an explicit effective-identity check before the containment profile is treated as available.

## Required correction

Use the same resolved, policy-validated Git executable and sanitized environment for every identity probe as for mutations. Do not rely solely on Git output: parse the candidate `.git` pointer, constrain its administrative directory to the configured common Git directory, and verify the reciprocal administrative `gitdir` link before registration. Add the ambient-`GIT_DIR`/`GIT_WORK_TREE` regression above and prove rejection without altering the caller's environment globally.

Make initial `Prepared` marker acquisition exclusive at the final marker pathname and reject an existing marker before any new identity is allocated. Phase transitions must compare the existing complete marker identity and expected prior phase before an atomic replacement; they must never replace a foreign/outstanding claim. Add a restart-oriented regression that exercises direct `create`, not only `ensure_for_plan`, with an outstanding marker and proves the marker bytes and owned object remain unchanged.

Reject effective privilege and real/effective identity mismatches on every supported Unix platform before installing the no-descendant profile. Add platform-appropriate tests for the validation predicate.

## Accepted portions and verification

r4 does remove the r3 numeric-PID observation model. For an ordinary unprivileged caller, the direct child is retained, killed, and reaped, while the `RLIMIT_NPROC=0` profile prevents the tested shell from forking a background child. The ordinary manual linked-worktree create/remove/recreate path and the tested durable phase cases pass. Those improvements do not compensate for accepting an unowned path or losing an earlier durable claim.

Fresh reviewer commands at the restored exact candidate produced:

- `cargo test -q -p roko-agent process::group::tests --lib` — exit 0; 6 passed.
- `cargo test -q -p roko-orchestrator worktree::tests --lib` — exit 0; 43 passed.
- `cargo test -q -p roko-orchestrator` — exit 0; 511 unit and 3 integration tests passed; 2 doctests passed and 2 were intentionally ignored.
- `cargo check -p roko-agent -p roko-orchestrator --all-targets` — exit 0; pre-existing test warnings were emitted.
- `cargo clippy -q -p roko-agent -p roko-orchestrator --lib -- -D warnings` — exit 0.
- `cargo fmt --all -- --check` and `cargo metadata --locked --no-deps --format-version 1` — exit 0.
- `git diff --check 8dff7d32..16497357`, manifest/lockfile identity, implementation ancestry, and exact source restoration — exit 0.

Scope remains the three production files in implementation commit `8dff7d32b` plus its evidence-only child `16497357`; no manifest or lockfile changed. The review worktree contains no reviewer implementation change.

Final status: **REJECTED because the confirmed repository-identity environment bypass permits registration of an unowned path. Do not integrate `8dff7d32b` / `16497357` as the CTRL-02 correction. Produce a fresh immutable candidate addressing that bypass plus the marker and effective-identity policy corrections, then obtain fresh independent review.**
