# CTRL-05 implementation evidence

Assignment:

- Plan: Wave 0 `CTRL-05`; reconcile `E11-chain-isfr` task `E11-T01`
- Base SHA: `f576dedaaf7b45478b136802785bf2a9dfc11371`
- Branch/worktree: `agent/CTRL-05-architecture-queue` at `/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/workers/CTRL-05`
- Integration branch: `status-quo/integration-status-quo-20260714T073140Z`
- Reserved write scope: `tmp/status-quo/backlog/plans/E11-chain-isfr/tasks.toml` and this evidence file

## Requirement

Original defect: E11-T01 still said `plans/architecture-core-queue/` did not exist and required an ignored `.claude/worktrees/agent-*` file as its source. CTRL-01 has since recovered, independently reviewed, merged, and post-merge verified the canonical `plans/architecture-core-queue/tasks.toml`. The stale prerequisite therefore makes the prerequisite-aware validator reject E11 even though the canonical dependency and all three architecture-DeFi consumers are present.

Expected behavior:

- E11-T01 reads and verifies the tracked canonical recovery, not an ignored worktree.
- Its acceptance and verification prove the exact recovered SHA-256, the Q14 task anchor, all three `architecture-defi-critical-path` source references, and the exact merged CTRL-01 ancestry.
- Focused strict validation runs in a disposable root so validator-generated indexes/logs do not modify canonical files.
- E11-T01 remains `ready`; `[meta] total = 5`, `done = 0`, and all task statuses remain unchanged for coordinator transition after accepted integration.

Explicit non-goals: modifying `plans/architecture-core-queue/tasks.toml`, `plans/architecture-defi-critical-path/tasks.toml`, `plans/INDEX.md`, production code, another backlog manifest, the master checklist, or any canonical status. No placeholder or empty architecture queue may be created.

Dependency proof:

- CTRL-01 import implementation: `699df4e0ea34bddabc4516695d28d1bf41328774`
- CTRL-01 accepted independent review: `c19bd30160443759f96d8fef6149cc9b146a5bde`
- CTRL-01 recovery merge: `01c00546bc57a485ff53553d0fe53006afa8ed42`
- CTRL-01 coordinator closure/base: `f576dedaaf7b45478b136802785bf2a9dfc11371`

All three proof commits are ancestors of the assigned base. The implementation and review evidence on that base record the exact archive/source/import identity and post-merge verification.

## Reproduction

I copied the base E11 manifest to a disposable directory and ran the serialized prerequisite-aware CTRL-06 candidate binary (SHA-256 `1c3a80a77feec70bf1625cfc2f4b6fa623760fcc6a04b518ac38ae6ec24232b7`):

```text
warn PLAN_031 task 'E11-T01' requires prerequisite
'.claude/worktrees/agent-aefd7c48/plans/architecture-core-queue/tasks.toml'
which does not exist on disk and is not created by a declared dependency
1 diagnostics in 1 plan
exit 1
```

The canonical file was already present, tracked, non-empty, and had the reviewed hash. The only focused diagnostic was the obsolete ignored-worktree prerequisite.

## Implementation

Only E11-T01 metadata and its acceptance contract changed:

- Replaced the false absent-queue narrative with exact CTRL-01 implementation, review, merge, closure, and SHA-256 proof.
- Replaced the ignored `.claude` context path with the canonical recovered manifest, the complete architecture-DeFi consumer manifest, and the two merged CTRL-01 evidence records.
- Added task-level acceptance for tracked/non-empty byte identity, Q14/source-ref resolution, merged ancestry, isolated strict validation, and absence of placeholders.
- Replaced copy-oriented anti-patterns and MD5 language with rules forbidding ignored-worktree copying, recovered-source edits, placeholders, and premature status changes.
- Strengthened verification to assert Git tracking/non-empty content/exact SHA-256, parse both TOMLs and resolve exactly three DeFi parity rows to Q14, verify exact CTRL-01 ancestry, and run E11 strict validation in a disposable workspace root.

The disposable validator fixture exposes real repository prerequisites through symlinks but contains its own `plans/` and `.roko/` directories. Generated `plans/INDEX.md`, `.roko/INDEX.md`, PRD/research indexes, and logs therefore remain temporary and are removed by the command's trap.

No canonical queue bytes or consumer bytes changed. No directory or placeholder was added. E11 remains five tasks, zero done, five ready.

## Verification

Canonical recovery and status structure:

```text
python3 tomllib assertions
TOML_OK plan=E11-chain-isfr tasks=5 done=0 ready=5
T01_acceptance=4 T01_verify=4

git ls-files --error-unmatch plans/architecture-core-queue/tasks.toml
test -s plans/architecture-core-queue/tasks.toml
shasum -a 256 plans/architecture-core-queue/tasks.toml
3f90263abd24f1b937a882244e3c67290a1580bb11ebe724c6d399d0741d3fe5
```

Q14 and consumer resolution:

```text
tomllib parsed both canonical manifests
Q14_SOURCE_REFS_OK
anchor IDs found: 1
architecture-defi parity source_refs resolved: 3/3
```

Merged recovery ancestry:

```text
git merge-base --is-ancestor 699df4e0ea34bddabc4516695d28d1bf41328774 HEAD
git merge-base --is-ancestor c19bd30160443759f96d8fef6149cc9b146a5bde HEAD
git merge-base --is-ancestor 01c00546bc57a485ff53553d0fe53006afa8ed42 HEAD
RECOVERY_ANCESTRY_OK
```

Focused strict validation after the manifest correction, using the same serialized corrected CTRL-06 candidate while it was available:

```text
0 diagnostics in 1 plan
exit 0
```

That first post-fix invocation also reproduced the validator's known current-working-directory side effect by regenerating root `plans/INDEX.md` and ignored root `.roko` indexes/logs. I restored the index immediately to its exact tracked SHA-256 `7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44`, removed only those newly generated ignored files/directories, confirmed root `.roko` again contains only tracked `GAPS.md`, and changed the task command to run from a disposable workspace root. The final isolated fixture then reported:

```text
0 diagnostics in 1 plan
exit 0
generated regular files: disposable plans/INDEX.md and disposable .roko indexes/log only
source plans/INDEX.md after command:
7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44
```

Hygiene:

```text
git diff --check
exit 0

changed paths before evidence:
tmp/status-quo/backlog/plans/E11-chain-isfr/tasks.toml
```

## Review readiness

- Implementation commit: this scoped manifest/evidence commit; exact candidate SHA is reported in the worker handoff.
- Diff scope: only the assigned E11 manifest and this evidence file.
- Known limitation: the prerequisite-aware CTRL-06 validator candidate is not a dependency this worker may merge. Its available candidate binary was used for the focused before/after proof; the canonical command expects the rebuilt integrated `target/debug/roko` when the coordinator performs post-merge verification.
- Required reviewer focus: confirm no `.claude` prerequisite or absent-queue claim remains in E11-T01; independently reproduce the canonical hash, exact three Q14 source references, recovery ancestry, unchanged task/meta statuses, disposable validation behavior, and two-path diff scope.

Integration:

- Review evidence: pending independent review.
- Integration commit: pending.
- Post-merge verification: pending.
- Final status: implementation committed for review; E11-T01 deliberately remains `ready` until coordinator reconciliation.
