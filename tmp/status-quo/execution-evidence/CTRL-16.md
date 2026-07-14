# CTRL-16 implementation-order reconciliation evidence

## Assignment

- Plan: `tmp/status-quo/MASTER-EXECUTION-CHECKLIST.md`, Wave 0 `CTRL-16`.
- Original r1 base: `3a4e57f02efa47f3106f54969799c34486b3ed7b`.
- Corrected-candidate base: `7f5221e9da762f51d2ab4056f0989b49de76bdea`;
  content-equivalent r1 replay: `bc11d75a84d1d4d90ad1cf988f41d97346c45c1e`.
- R3 correction parent: `a9ac1f25e99ad80805b5dc266d3b626632291afe`.
- R4 correction parent: `9044be08f5e732dc376bd0c3c7bd35f55742e5ab`.
- Branch/worktree: `agent/CTRL-16-r2` / `workers/CTRL-16-r2`.
- Integration branch: `status-quo/integration-status-quo-20260714T073140Z`.
- Reserved scope: `plans/_meta/IMPLEMENTATION_ORDER.md`,
  `tmp/status-quo/73-EXAMPLES-PLANS-GRAPHS.md`,
  `tmp/status-quo/backlog/02-PLANS-RECONCILIATION.md`,
  `scripts/demo-knowledge-feedback.sh`, and this evidence.
- Candidate identity: the exact immutable cumulative candidate SHA is supplied
  by the fresh independent review record (a commit cannot contain its own SHA).

## Requirement

Original defect: the imported implementation-order file still grouped
`dry-run-flag`, `live-demo-phase1`, and `live-demo-phase2` with the runnable
standalone queues even though Git history shows that `7899494d` deleted all
three manifests. At the same time, its terse architecture statement did not
record that CTRL-01/CTRL-05 had recovered and verified the real 24-task
`architecture-core-queue`. Two current-looking status-quo documents retained
baseline plan counts and side-queue language without an exact current
disposition for the removed roots.

The r1 independent review (`CTRL-16-REVIEW.md`, rejected in integration commit
`b59e497e7`) found three remaining defects: the tracked demo script still
dispatched both absent live-demo roots, the disposition omitted partial source
residue from `236686c7`, and this evidence retained literal candidate-SHA
template tokens. The corrected candidate must close all three without reviving
or marking any deleted task complete.

The r2 independent review (`CTRL-16-REVIEW-r2.md`, rejected in integration
commit `da5e899b6`) accepted those three corrections but reproduced one remaining
fail-closed defect: if a deleted root was an outward symlink to an external
directory, canonicalizing the configured state path erased the forbidden
lexical prefix and allowed the simulation to write through that symlink. R3
must reject both lexical and canonical routes before any source check, directory
creation, or write while continuing to allow an independently configured
external state directory.

The r3 independent review (`CTRL-16-REVIEW-r3.md`, rejected in integration
commit `35b1eb455`) accepted the outward-symlink correction but found two release
blockers. On the case-insensitive APFS host, a mixed-case spelling of a removed
root bypassed case-sensitive string containment and created the same physical
lowercase root. The quoted Python heredoc inside command substitution also did
not parse with stock macOS `/bin/bash` 3.2.57. R4 must close both defects while
preserving every accepted r1-r3 boundary and ordinary external-state support.

Expected behavior:

- every root presented as runnable has a tracked, non-empty, parseable
  `plans/<root>/tasks.toml`;
- the recovered architecture queue remains a separate 24-task executable plan,
  and the three architecture-DeFi parity rows continue to resolve to its Q14;
- the deleted dry-run and live-demo roots are explicitly historical,
  non-runnable, absent from the index, and not recreated;
- every active script path refuses the deleted live-demo roots; the retained
  simulation is deterministic, no-network, and can write to isolated state;
  normalized relative/absolute paths, non-existing descendants, repository
  aliases, case-insensitive spellings, and inward/outward/chained symlinks cannot
  bypass that refusal; both stock macOS Bash 3.2 and PATH Bash can parse and run
  the advertised modes;
- history and semantic boundaries remain clear: related execution-honesty work
  is not mislabeled as a task-for-task dry-run replacement, and `e2e-smoke` is
  not mislabeled as equivalent to greeting/farewell demo tasks;
- no task/status/count, manifest, generated index, ownership ledger, production
  Rust source, or lockfile changes.

Dependencies: CTRL-01's canonical import (`699df4e0e`), accepted review
(`c19bd3016`), and merge (`01c00546b`); CTRL-05's architecture reconciliation
and accepted proof; CTRL-15's corrected ownership/index integration; and the r1
rejection integrated at `b59e497e7`. The r2 and r3 rejections are integrated at
`da5e899b6` and `35b1eb455`. The r2 base also contains the separately accepted
workspace-lock precursor and does not change CTRL-16 semantics.

Explicit non-goals: implementing a dry-run feature; executing or changing any
task; manufacturing absent directories; changing manifests, the master,
`plans/INDEX.md`, `EXECUTION-OWNERSHIP.md`, production code, Cargo metadata, or
remote/external state; or rewriting historical baseline bodies as though they
were authored against the current tree.

## Reproduction and history proof

At the base, `plans/_meta/IMPLEMENTATION_ORDER.md` said all four names were
standalone side/demo queues and directed phase 1 before phase 2. Filesystem and
Git evidence disagreed:

```text
tracked non-empty current roots:
  architecture-core-queue         ready  24 tasks
  architecture-defi-critical-path ready   3 tasks
  e2e-smoke                       ready   2 tasks

absent roots:
  dry-run-flag
  live-demo-phase1
  live-demo-phase2
```

`git show --name-status 7899494d --` records deletion of all three absent
manifests. Their parent versions contain ten proposed workflow dry-run tasks,
two synthetic greeting tasks, and two synthetic farewell tasks respectively.
The same commit also deleted the architecture queue, but CTRL-01 recovered its
manifest byte-identically from five sealed sources and the historical Git blob;
CTRL-05 then verified Q14 and its three DeFi consumers. The current architecture
manifest is tracked, non-empty, and has SHA-256
`3f90263abd24f1b937a882244e3c67290a1580bb11ebe724c6d399d0741d3fe5`.

R1's documentation-only boundary was incomplete. The tracked executable
`scripts/demo-knowledge-feedback.sh --live` still called
`roko plan run plans/live-demo-phase1/` and then `live-demo-phase2/`, even
though neither manifest exists. Commit `236686c7` also introduced source bytes
from the deleted proposals:

- `crates/roko-cli/src/dry_run.rs`, exported by `roko-cli`, defines only
  `DryRunGate` and `DryRunPreview`. Its module prose claims `roko run
  --dry-run`, but the current `Command::Run` has no flag, `WorkflowRunConfig`
  has no field, and no `build_dry_run_preview`, workflow early exit,
  `dry_run_no_model_call_invoked`, or `dry_run_phase_list_*` test exists.
- `crates/roko-std/src/greeting.rs` defines only `format_greeting`.
  `roko-std/src/lib.rs` does not export `greeting`, no greeting test exists,
  and neither `format_farewell` nor its test exists.

These are partial inherited artifacts, not task completion, cancellation, an
accepted replacement, or a task-level supersession.

## Implementation

- Made current count and authority boundaries explicit in the implementation
  order: generated index for counts, master for dependency order, and ownership
  ledger for the sealed 120-task mapping.
- Kept all current primary-queue names and order unchanged.
- Documented the exact current architecture-core, architecture-DeFi, and
  `e2e-smoke` roots with task counts and dependency semantics.
- Moved the three absent names into a historical-removal table keyed to
  `7899494d`, with explicit non-equivalence/supersession boundaries and a rule
  against execution or placeholder recreation.
- Recorded the exact surviving structs/function and every missing wiring/test
  boundary from the rejected review, so future work reuses rather than
  duplicates residue without treating it as accepted completion.
- Replaced the script's advertised live dispatcher with an immediate fail-closed
  `--live` result before state creation. Default mode now writes fixed simulated
  episodes to `ROKO_DEMO_STATE_DIR` (or `.roko` by explicit default), performs
  no Cargo build/network/model/plan call, uses no deleted-root task IDs, and
  points to the current `EpisodeSink` and prompt-builder production anchors.
  It also rejects either removed plan root (or a descendant) as a configured
  state directory, so no script mode can recreate or mutate those roots.
- Closed the r2 outward-symlink bypass before any repository `cd`, source-anchor
  check, `mkdir`, or state write. The guard now compares both a normalized
  lexical configured path and its fully canonical path with the two physical
  removed-root names. It additionally projects suffixes that name the repository
  through a symlink alias back onto the physical repository without resolving
  the suffix, preserving the outward-root check for repo aliases. It deliberately
  does not canonicalize the forbidden roots themselves, so an ordinary external
  state directory is not rejected merely because a removed-root symlink happens
  to target the same external tree.
- Closed the r3 APFS alias bypass by applying Python `casefold()` after lexical
  normalization to every configured lexical, canonical, removed-root, and
  repository-projected comparison. Repository-prefix identity is case-folded as
  well. This is deliberately conservative for the two removed ASCII names: a
  mixed-case nonexistent leaf, an existing lowercase fixture reached through an
  uppercase spelling, or a mixed-case symlink chain is rejected before `mkdir`,
  while a genuinely independent external directory remains outside both roots.
- Replaced the Python heredoc nested inside command substitution with an inline
  `python3 -c` program whose quoting is accepted by macOS Bash 3.2.57. The guard
  output and exit-status contract is unchanged; both stock and PATH Bash now
  parse and execute help, fail-closed, rejection, and simulation paths.
- Clarified that the already-complete W01/P06/P07 names are historical labels,
  not current runnable roots.
- Added narrowly scoped current-control notices to the two baseline inventory
  documents; their dated bodies remain preserved for provenance.

No plan or task semantics changed. The only executable behavior change is the
demo script's safe removal of an impossible live path; it neither implements
nor claims the old demo tasks. The operational safety property is now complete:
absent plan names cannot be mistaken for valid inputs through either the
implementation-order document or this active script.

## Verification

The candidate verification contract is:

```text
1. Python tomllib parses every tracked TOML: 193/193, zero errors.
2. Current top-level manifest census: 32 manifests; 30 ready executable plans
   with 144 tasks and two superseded plans with 66 excluded tasks.
3. Runnable-root assertions: architecture-core-queue,
   architecture-defi-critical-path, and e2e-smoke are tracked/non-empty and
   parseable; their task counts are 24/3/2.
4. Historical-root assertions: dry-run-flag, live-demo-phase1, and
   live-demo-phase2 have no current tasks.toml and are not index rows.
5. Q14 resolution: exactly one Q14 task and exactly three DeFi source_ref
   consumers, all resolving to it.
6. Disposable strict validation: backlog 0 diagnostics/55 plans, self-heal
   0 diagnostics/6 plans.
7. A disposable `plans` generator run reproduces tracked plans/INDEX.md exactly:
   SHA-256 27c6a5e0c486c2485ffc3f973a77ff73bffdf68a85cfcd78a409195c48ca95a8.
8. Source plans/INDEX.md is byte-unchanged and `git diff --check` passes.
9. `/bin/bash -n` (3.2.57) and PATH `bash -n` (5.3.3) pass; under both versions,
   `--help` is truthful, `--live` and an unknown option exit 2 before plan
   or state mutation, a mixed-case removed-root case exits 2, and default simulation
   succeeds against isolated state without network/build/model/plan execution
   and emits exactly two valid JSON records.
10. A disposable adversarial matrix checks exact and descendant deleted roots,
    lexical `..`, an external symlink into a removed root, a removed-root outward
    symlink, the same outward route through explicit repository and `/tmp` to
    `/private/tmp` aliases, chained symlinks, mixed-case exact/descendant/chained
    paths, and a mixed-case route to a pre-existing physical lowercase fixture.
    Every rejected case exits 2 with no target/repository mutation; normal
    external state succeeds under both Bash versions.
11. No active script command passes either absent root to `roko plan run`; the
    cumulative corrected scope is exactly the five reserved paths.
```

Exact command output is recorded below after running those checks on the final
candidate tree.

```text
TOML_ROOT_Q14_OK tracked=193 manifests=32 executable=30/144 superseded=2/66 runnable=24/3/2 absent=3 anchor=1 consumers=3
HISTORY_BLOBS_OK dry_run=10 live1=2 live2=2 architecture=24
RESIDUE_BOUNDARY_OK dry_run_types=2 export=1 run_flag=0 runtime_field=0 builder=0 named_tests=0 greeting=1 std_export=0 greeting_test=0 farewell=0
MARKDOWN_LINKS_OK checked=16

backlog strict:  exit 0; 0 diagnostics in 55 plans
self-heal strict: exit 0; 0 diagnostics in 6 plans
top-level strict generator run: exit 1; 94 diagnostics in 32 plans
top-level diagnostic census: 94 PLAN_031 and no other PLAN code
generated index SHA-256: 27c6a5e0c486c2485ffc3f973a77ff73bffdf68a85cfcd78a409195c48ca95a8
source index before/after: 27c6a5e0c486c2485ffc3f973a77ff73bffdf68a85cfcd78a409195c48ca95a8
SCRIPT_BASH_MATRIX_OK system=3.2.57 path=5.3.3 syntax=0/0 help=0/0 live=2/2 unknown=2/2 mixed_reject=2/2 repository_unchanged=yes
SCRIPT_STATE_GUARD_MATRIX_OK case_mode=case_insensitive absolute_exact=2 relative_descendant=2 lexical_dotdot=2 inward_symlink=2 chained_inward=2 outward_removed_root=2 repo_alias_outward=2 tmp_private_alias=2 mixed_exact=2 mixed_descendant=2 mixed_physical_fixture=2 mixed_chain=2 mixed_repo_alias_outward=2 rejected_mutation=0
SCRIPT_SIMULATION_OK system_exits=0/0 path_exits=0/0 deterministic_sha=ed729b8bf452ba56c3b7bdb61090ddf75ef6038d27bba337e7cc4b21df35a01e no_network_build_model_or_plan records=2
SCRIPT_PLAN_TREE_UNCHANGED sha=f370c9003b47faa597a2a4c24cf26f926c20891d2d3e6f1481755db85179bba8
active absent-root plan-run scan: zero matches
cumulative replay/correction scope: exactly five reserved paths
git diff --check: exit 0
```

The top-level `PLAN_031` result is the existing bounded census of intended
future architecture-source outputs, not a parse, root-resolution, dependency,
or index error. All validation/generator invocations ran from fresh `git
archive` trees under `/tmp`; their generated indexes and `.roko` records were
removed with those temporary trees. The source index stayed unchanged. The
validator binary was the integrated binary reporting Git `7303d2f87`; CTRL-15
changed only control-plane documents, so its parser/generator behavior is the
reviewed behavior used for the current 30/144 index.

The r4 path matrix likewise ran only in fresh disposable archive copies on the
case-insensitive APFS host. It pre-created symlinks and one empty physical
lowercase removed-root fixture, compared rejected plan trees and external targets
before and after every invocation, and removed every archive, fixture, external
target, symlink, log, and simulated state directory afterward. All thirteen
guard cases exited 2 without mutation. Both Bash versions ran normal external
state twice, produced the same two-record digest, and left the plan-tree digest
unchanged. The recorded plan-tree digest uses the sorted relative-path/file-byte
aggregate from the prior matrix; the r4 per-case before/after tar digests likewise
matched exactly.

## Review readiness

- Candidate implementation identity: exact immutable cumulative SHA supplied
  by the fresh independent review record.
- Diff scope: exactly the five reserved documentation/script/evidence paths.
- Known limitations: the historical dry-run proposal has no equivalent current
  task-level owner. This change records that gap truthfully; it does not invent
  a supersession or authorize feature work.
- Required reviewer focus: reconstruct the three deleted blobs from Git,
  challenge every current/historical mapping, verify all named runnable roots,
  reproduce Q14 resolution and residue absence/presence checks, exercise all
  script modes with stock macOS Bash 3.2 and PATH Bash, adversarially repeat the
  case-folded lexical/canonical, repo-alias, mixed-case, inward/outward/chained
  symlink matrix, reproduce TOML/strict/index gates, and confirm no status,
  count, manifest, ownership, or generated-index change.

## Integration

- R1 review: `tmp/status-quo/execution-evidence/CTRL-16-REVIEW.md`, verdict
  `REJECTED`, integrated as `b59e497e7`; all three findings are addressed above.
- R2 review: `tmp/status-quo/execution-evidence/CTRL-16-REVIEW-r2.md`, verdict
  `REJECTED`, integrated as `da5e899b6`; the outward-symlink finding is addressed
  above without weakening ordinary external-state support.
- R3 review: `tmp/status-quo/execution-evidence/CTRL-16-REVIEW-r3.md`, verdict
  `REJECTED`, integrated as `35b1eb455`; the case-insensitive APFS bypass and
  stock Bash 3.2 parse failure are addressed above.
- Fresh r4 review evidence: pending independent review.
- Integration commit: pending.
- Post-merge commands/results: pending integration-owner verification.
- Final status: `IMPLEMENTED_UNREVIEWED`.
