# CTRL-13 independent review

## Candidate and independence

- Verdict: **ACCEPTED**.
- Candidate: `fd637bdaf796fcf1f2da45be086e808927eb52af`.
- Exact assigned base and candidate parent:
  `bb5048f4c1d0e3f34155e89d39ccef46109c3b59`.
- Review branch/worktree: `review/CTRL-13-fd637bda` /
  `reviews/CTRL-13-fd637bda`.
- Reviewer assignment: independent CTRL-13 evidence and dependency-graph review;
  I did not author the candidate.
- Cumulative candidate diff: exactly the new 218-line
  `tmp/status-quo/execution-evidence/CTRL-13.md`; no source, test, manifest,
  index, status, or master path differs from the base.
- Integrated prerequisite merge commits named by the candidate were independently
  confirmed as ancestors of the base: CTRL-03 `4ae834b797fa`, CTRL-04
  `06e1d4404785`, CTRL-08 `515cbff5f715`, and CTRL-12 `1e478eaf1a6e`.

I reconstructed the requirement from the complete master, the candidate record,
the 93 source manifests, the sealed top-level index, the public parser and runtime
readiness code, and the relevant prior control evidence. I did not reuse the
worker's temporary script or generated index.

## Changed-line and production-path review

`git diff --check` passed. `git show --name-status` and the exact base-to-candidate
diff confirm that the candidate is evidence-only and contains no hidden generated
or executable change.

The evidence correctly distinguishes three different contracts:

1. every dependency string resolves to a node in the chosen 93-plan union;
2. readiness still requires completed task and task-level plan prerequisites; and
3. strict file prerequisites can warn even when plan IDs resolve.

The unchanged source trace supports those boundaries:

- `TasksFile::parse` deserializes task-level `depends_on_plan`; `TaskMeta` has no
  meta-level dependency field;
- `TaskDef::is_ready_with_plan_deps`, the orchestration trackers, and runner
  `TaskDag` require the declared plans to occur in `completed_plans`;
- `discover_plan_workflow_tasks` parses and schedules every discovered manifest
  without filtering `meta.status`, so a raw broad root would not itself exclude
  superseded plans.

The parser, validator, plan runner, and runner DAG paths are byte-identical from
the candidate's cited git `915d3c246` binary source through the candidate and the
then-current `f42df7d7a` integration source. The candidate therefore does not
overstate what its historical validation binary exercised.

## Independent manifest and graph reproduction

From the candidate worktree I ran an independent Python 3 `tomllib` census over
the sorted roots and all tracked TOML. It recomputed hashes from
`relative_path + NUL + raw_bytes + NUL`, checked unique plan/task IDs and
`meta.total`, resolved local and cross-plan references, and ran Tarjan SCC over
all 93 nodes including isolates.

```text
tracked TOMLs: 193; parse errors: 0
roots: top 32/210, backlog 55/614, self-heal 6/57
union: 93 unique plans, 881 tasks
task states: 33 done, 752 ready, 96 skipped
duplicate plan IDs: 0
duplicate task IDs within plan: 0
meta.total mismatches: 0
meta statuses: 90 ready, 3 superseded
meta.done mismatches: only status-quo-authoring-gaps (96 skipped terminal tasks)

same-plan references: 848; unresolved: 0
task plan references: 296
meta plan declarations: 2
all plan references: 298; unresolved: 0
unique plan edges: 135; cyclic SCCs: 0
```

The independently calculated root routing is also exact:

| Route | Raw references | Unique edges |
|---|---:|---:|
| backlog -> backlog | 268 | 111 |
| backlog -> self-heal | 5 | 3 |
| backlog -> top-level | 11 | 10 |
| self-heal -> self-heal | 14 | 11 |

The two meta declarations are exactly
`E46-github-workflow-integration -> E01-execution-engine` and
`E48-rate-limit-budgeting -> E01-execution-engine`. No top-level plan declares an
outgoing edge.

The byte-stream hashes exactly match the candidate:

```text
union:     ffcdbc95017beb0a340947d754d4328db713b8abd67f75b1bfdbd797762a64e4
top:       6bf80483d6103260f17d8d7e3a4bc923bd6ad24e7507fe2875722aa57005591a
backlog:   9dfea56c135a6bf12a01269a987b9d54d60ac8be11169cfbdc1a92db3d75dd54
self-heal: 26af4b091a5558920ec403d31774345e50b1cb2efaaebaf22aa8706087257811
```

## Public parser check

I temporarily added an uncommitted integration test that called the public
`roko_cli::task_parser::TasksFile::parse` API for every manifest. It independently
asserted the three root counts, unique parsed plan and within-plan task IDs,
`meta.total`, every local reference, and every task-level plan reference.

Final command, from the candidate worktree:

```sh
CARGO_TARGET_DIR=/Users/will/dev/nunchi/roko/agent-worktrees/status-quo-20260714T073140Z/integration/target \
  cargo test -p roko-cli --test ctrl13_review_parser -- --nocapture
```

Result: exit 0, one test passed:

```text
CTRL13_REVIEW_TASKSFILE_OK roots=32+55+6 plans=93 tasks=881 local_refs=848 task_plan_refs=296
test public_parser_resolves_the_complete_ctrl13_union ... ok
test result: ok. 1 passed; 0 failed
```

The temporary test emitted only the expected integration-test crate
`missing-docs` warning. It was removed and is absent from the review diff.

An earlier attempt used isolated
`CARGO_TARGET_DIR=/private/tmp/roko-ctrl13-parser-target`. It was interrupted with
SIGINT (exit 130) when the integration owner requested shared-target reuse; it is
not represented as passing. Its 3.0 GiB partial target was removed before the
final run.

## Strict validation and index sealing

I exported the immutable candidate with `git archive` to
`/private/tmp/roko-ctrl13-review-fd637bda`, changed the command working directory
to that disposable repository root, and ran the current repository CLI:

```sh
$BIN plan validate --strict tmp/status-quo/backlog/plans
$BIN plan validate --strict tmp/status-quo/self-heal/plans
$BIN plan validate --strict plans
```

Results:

```text
backlog:   exit 0; 0 diagnostics in 55 plans
self-heal: exit 0; 0 diagnostics in 6 plans
top-level: exit 1; 94 diagnostics in 32 plans
diagnostic codes: PLAN_031 only
architecture-core-queue: 93
self-dev-ux: 1
```

Inspection of every diagnostic confirms that `PLAN_031` describes a missing file
prerequisite. None names an unresolved plan ID, malformed dependency, parse error,
or cycle. This validates the candidate's explicit warning/completion boundary.

The tracked source index SHA-256 was
`7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44`
both before and after the corrected disposable-root run. Only the archive index
regenerated, to
`27c6a5e0c486c2485ffc3f973a77ff73bffdf68a85cfcd78a409195c48ca95a8`.

For completeness, my first archive invocation passed archive target paths while
remaining in the source worktree. The validator generates `plans/INDEX.md`
relative to its process working directory, so that invocation changed the review
worktree index to the generated 30-plan/144-task form. I did not use checkout or
reset: I restored the four-line sealed content with `apply_patch`, confirmed the
exact sealed hash and clean status, then reran correctly from inside a fresh
archive as recorded above. Both disposable archives were removed.

## Ledger and supersession adversarial checks

I derived the 120-task ledger from manifest IDs rather than the candidate prose:

```text
P08-P34: 27 plans, 115 tasks
architecture-defi-critical-path + e2e-smoke: 2 plans, 5 tasks
retained sealed ledger: 29 plans, 120 tasks
separate architecture-core-queue: 24 additional tasks in the resolution union
```

The caveat is accurate: CTRL-13 includes the recovered architecture queue for ID
resolution but does not silently expand or reconcile the sealed 120-task CTRL-15
ledger.

The only superseded plans are `self-dev-ux` (55 ready), `self-dev-extras` (11
ready), and `status-quo-authoring-gaps` (96 skipped). Removing those 162 tasks
leaves exactly 90 active plans and 719 tasks: 33 done and 686 ready. No one of the
135 graph edges enters or leaves a superseded node. The sealed index explicitly
excludes the first two, and the candidate correctly treats exclusion as an
execution-selection rule rather than a parser guarantee.

## Artifacts, limitation, and verdict

All reviewer-created archives, the temporary parser test, and the interrupted
isolated target were removed. Before this review commit, `git status --short`
showed only this review record and `plans/INDEX.md` retained its sealed hash.

The required shared-target parser run rebuilt
`integration/target/debug/roko` with candidate build provenance. At review end it
reported git `fd637bdaf` and SHA-256
`b1f3ef5a92dbad05ccf492c985bfcae2e13385df871597ef1858efecc2398ede`.
This untracked build artifact does not affect candidate truth, but the integration
owner must rebuild the binary from the final integrated HEAD before relying on
CTRL-11/release provenance; the coordinator was notified immediately.

**Verdict: ACCEPTED, high confidence.** The exact evidence-only candidate proves
zero unresolved local and plan IDs for the explicitly chosen union, preserves the
strict-file and completion caveats, and makes no premature CTRL-15 or task-completion
claim. No candidate correction remains.

Exact next action: merge this accepted review branch into the integration branch,
rerun the census/parser/strict and sealed-index proof on the resulting integration
commit, rebuild the integration binary at that head, and only then reconcile
CTRL-13's canonical status. This verdict alone is not `DONE`.
