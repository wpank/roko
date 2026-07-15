# CTRL-08 r4 independent final review

- **Verdict:** `ACCEPTED`
- **Candidate:** `1e07967a348944a4a4a0a88395e1485ad076d94a`
- **Base:** `dd611500e7f9051fbdd3843cd20c5472efcfcbb7`
- **Prior rejected candidates/reviews:**
  `b9387fe6c3f42209a317a301302b027a6b882042` /
  `b0e21f69f427e738a7198f43ad5d827cf0b7c486`;
  `ff6dc54afeccf4d06ebd95e476756d2383422205` /
  `87461143496d405a0c3a0adffa9bfa2c278f1bc6`; and
  `ec3ecf2f89f0dd74a6c5e973c9ea4c7185bec30e` /
  `ac3cfb8439bd4223663759360ad73a7b18461419`
- **Current integration checked:** `fc831c5542950470808ed876a29f4f841e7cd936`
- **Review branch:** `review/CTRL-08-r4-1e07967a-final`
- **Review date:** 2026-07-14

## Independent method and scope

I read the complete master checklist; all three rejected candidate/review chains;
the r4 ownership matrix, evidence, dated audit, touched manifests, and epic prose;
and the relevant live parser, scheduler, provider, model-call, runner, limiter,
health-registry, and router code. I recreated the graph, reference, status, writer,
TOML, parser-semantic, strict-validation, scope, index, history, and merge checks
from the immutable candidate without using worker scripts or archives.

The candidate is the direct child of `dd611500e` and changes exactly 24 paths: 13
manifests, eight epic documents, one dated audit, one new ownership document, and
one worker evidence document. It changes no production source, test, master,
shared index, lockfile, or top-level plan index. `git diff --check` passes. The
candidate paths have no overlap with the changes from `dd611500e` through the
clean current integration state `fc831c554`.

## Rejected-finding reproduction and r4 disposition

All seven findings established by the first two rejected reviews remain corrected:

1. The E14/E48 plan direction is acyclic.
2. The acceptance roll-ups use scheduler-recognized dependencies, not informational
   metadata alone.
3. E02-T08 reviews the canonical StateHub boundary and does not create a second
   serve-filter implementation.
4. E08-T08 names the live React/ResourcesConfig supervision contract rather than a
   nonexistent Watcher API.
5. E01-T07 retains both SH02 task-isolation and E01-T14 plan-aggregation outcomes.
6. Provider health and rotation ownership now identify executable producers,
   consumers, and focused outcomes.
7. E47-T09 is the sole worktree-count metric producer using
   `WorktreeManager::active_count()`, while E08-T09 is a pure `React::decide`
   consumer; neither names the nonexistent Watcher/WatcherOutput API or substitutes
   disk budget for worktree count.

The two live-path defects from the third rejected review are also corrected:

- E14-T08 now maps the canonical outcome to both E48-T02 and E48-T03. E48-T02 owns
  configuration and the shared pool; E48-T03 requires a pooled limiter acquire on
  the actual legacy and runner-v2 model-call paths before every request. The roll-up
  explicitly rejects a dormant, process-global, per-agent, or ToolDispatcher-only
  substitute.
- E48-T05 names the real `ModelCallService`, legacy orchestration, runner-v2 event
  loop, runtime-feedback, registry, and cascade-router paths. It requires one
  runtime-scoped `Arc<ProviderHealthRegistry>` for outcome recording and routing.
  The live source confirms the existing trait-typed callback seam in
  `model_call_service.rs`, the provider execution surfaces, and the registry/router
  APIs. The contract forbids a production `roko-agent -> roko-learn` dependency;
  current manifests retain `roko-learn` only as an agent dev-dependency while
  `roko-learn` depends on `roko-agent`, so the specified callback avoids a reverse
  production edge. E14-T10 rolls up that same outcome rather than ToolDispatcher.
- E48-T11 is a zero-write `quick-reviewer` roll-up, depends locally on E48-T10, and
  maps its outcome to E48-T05. Its acceptance is task-scoped.

## Parser-semantic and scheduling proof

I added a temporary integration test that called the public
`roko_cli::task_parser::TasksFile::parse` API against the affected manifests. It
checked all 12 roll-ups and every task whose effective task-level acceptance changed
from the base. This produced a 20-task set, a strict superset of the handoff's 15
strengthened contracts. All 20 deserialize with nonempty `TaskDef.acceptance`, and
none has `acceptance` nested inside a `[[task.verify]]` table. All 12 roll-ups have
empty file ownership, role `quick-reviewer`, nonempty task acceptance, and a
scheduler dependency: 11 use a cross-plan dependency and E48-T11 correctly uses its
same-plan dependency on E48-T10.

```text
cargo test -p roko-cli --test ctrl08_r4_review_semantics -- --nocapture
running 1 test
test ctrl08_r4_acceptance_is_task_scoped_through_real_parser ... ok
test result: ok. 1 passed; 0 failed
```

The temporary test was removed after execution and is not part of review or
candidate scope.

## Independently reproduced release controls

```text
combined plans: 93 (32 top-level + 55 backlog + 6 self-heal)
unique meta/task plan edges: 135
unresolved local task references: 0
unresolved plan references: 0
cyclic strongly connected components: 0
changed manifests: 13
changed task ID/order/status drift: 0
changed meta plan/total/done/status drift: 0
backlog + self-heal statuses: 33 done, 542 ready, 96 skipped
acceptance roll-ups: 12
unordered ready-task same-file pairs in changed max_parallel > 1 plans: 0
tracked TOMLs parsed: 193; errors: 0
plans/INDEX.md SHA-256: 7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44
```

Using the integration-built repository CLI at git `915d3c246` against a disposable
archive of the immutable candidate reproduced:

```text
roko plan validate --strict tmp/status-quo/backlog/plans
0 diagnostics in 55 plans; exit 0

roko plan validate --strict tmp/status-quo/self-heal/plans
0 diagnostics in 6 plans; exit 0
```

Validation regeneration was confined to the disposable archive. The candidate
worktree retained the sealed source `plans/INDEX.md` hash above.

History and mechanical integration checks also pass:

```text
git merge-base dd611500e 1e07967a
dd611500e7f9051fbdd3843cd20c5472efcfcbb7

git merge-tree --write-tree dd611500e 1e07967a
c247f7322fef61985441eea5f09995f6b6410fbe

git merge-tree --write-tree fc831c554 1e07967a
35dd8c041a740b7b7f71d1c5930f0c74b4aa0867
```

The first merge tree equals the candidate tree. The second proves a conflict-free
merge into the newer integration state checked during this review.

## Verdict

`ACCEPTED`. Candidate `1e07967a348944a4a4a0a88395e1485ad076d94a`
satisfies the CTRL-08 r4 ownership/deduplication contract and corrects every prior
rejection finding with executable, parser-visible, graph-valid ownership records.
It is suitable for integration subject to the coordinator's normal merge and
post-merge verification contract.
