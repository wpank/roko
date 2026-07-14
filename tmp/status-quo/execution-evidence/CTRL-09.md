# CTRL-09 DOC-v2-core acceptance-roll-up evidence

## Candidate and scope

- Integrated base: `bb5048f4c1d0e3f34155e89d39ccef46109c3b59`
- Branch/worktree: `agent/CTRL-09-doc-v2-rollup-r2` /
  `workers/CTRL-09-r2`
- Reconstructed note source: the isolated, uncommitted two-file draft in
  `workers/CTRL-09` at old base `1cbca115`; the draft was treated as notes,
  not as a transferable commit.
- Changed control-plane paths:
  `tmp/status-quo/backlog/plans/DOC-v2-core/tasks.toml`,
  `tmp/status-quo/backlog/source-coverage/docs-v2-core.md`, and this evidence.
- Non-goals: product source, tests, manifests, lockfiles, master, shared indexes,
  task completion, integration edits, or validator-generated index drift.

Before reconstruction, the old draft's validator-generated `plans/INDEX.md`
change was restored through `apply_patch` to the sealed SHA-256
`7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44`.
The two deliberately reapplied note files have SHA-256 values
`35c2b939da233fc79c83ee79bc3c7875dbc509c10f25416c69c8ab7aa0b4976a`
and
`7722cd5384c164dc57005f8102ae54c39b2c145d078af461d83616e56cc20625`,
exactly matching the isolated notes after revalidation against the integrated
CTRL-08 ownership graph.

## Ownership disposition

DOC-v2-core is no longer a second product implementation stream. Its original
ten IDs remain ten `ready`, docs-domain, `scribe` acceptance roll-ups. They
write only their 34 disjoint `docs/v2/**/*.md` sources after the corresponding
runtime plans have merged:

| Roll-up | Documentation subject | Integrated product owners |
|---|---|---|
| `DOCV2-T01` | Signal and Cell | E03, E13, E19, E20, E29, E31, E33, E34 |
| `DOCV2-T02` | Graph, execution, orchestrator | E01, E05, E08, E21, E22, E45 |
| `DOCV2-T03` | Agent cognition | E06, E08, E17, E23, E44 |
| `DOCV2-T04` | Memory, learning, cross-cuts | E06, E07, E09, E24, E25, E44 |
| `DOCV2-T05` | Gateway, feeds, groups, connectivity | E14, E15, E26-E29, E36, E39 |
| `DOCV2-T06` | Extensions, triggers, tools | E04, E14, E15, E17, E30-E32 |
| `DOCV2-T07` | Telemetry, security, auth, payments, config | E02, E04, E09, E18, E33-E36, E42 |
| `DOCV2-T08` | Surfaces and marketplace | E10, E33, E36-E39 |
| `DOCV2-T09` | Registries, arenas, DeFi | E11, E39-E41 |
| `DOCV2-T10` | Index, deployment, roadmap, public guides | prior DOC roll-ups; E01, E10, E11, E16-E18, E43, E45 |

Every canonical E19-E45 plan is present in at least one scheduler-recognized
`depends_on_plan` list. Local edges serialize cross-document consistency and
make T10 the final public integration pass. Each task's acceptance is located at
task scope, source reconciliation distinguishes current behavior from normative
deferred targets, and no task gains authority to implement a missing capability.

Preservation comparison with the integrated base:

```text
task ID/order/status: unchanged
meta plan/total/done/status/max_parallel/skip_enrichment: unchanged
estimated total minutes: 4200 -> 1800
tasks: 10 ready; task-scoped nonempty acceptance: 10; nested verify acceptance: 0
```

No task is prematurely marked complete.

## Rust parser semantic proof

A temporary integration test called the public
`roko_cli::task_parser::TasksFile::parse` API against the actual candidate
manifest. It asserted the exact ordered IDs, unchanged meta/status, docs domain
and role, nonempty task-level acceptance, runtime owner dependencies, 34 unique
docs-only writers, and coverage of each E19-E45 plan:

```text
CARGO_TARGET_DIR=.../integration/target \
  cargo test -p roko-cli --test ctrl09_semantics -- --nocapture

running 1 test
test doc_v2_rollups_deserialize_with_task_scoped_acceptance_and_ownership ... ok
test result: ok. 1 passed; 0 failed
```

The temporary test was removed after the proof and is not candidate scope.

## Combined graph and source census

The census parses all top-level, backlog, and self-heal manifests as one
dependency universe and checks local references plus plan SCCs:

```text
plans=93 (32 top-level, 55 backlog, 6 self-heal)
tasks=881; statuses=33 done, 752 ready, 96 skipped
base task-level depends_on_plan references=296 occurrences / 133 unique runtime edges
candidate task-level depends_on_plan references=320 occurrences / 160 unique runtime edges
runtime edge derivation=133 base + 27 candidate additions - 0 removals = 160
meta.depends_on_plan references=2 occurrences / 2 unique edges
all-declared unique plan edges=162 (160 task-level runtime + 2 meta-only)
all-declared edge derivation=135 base + 27 candidate additions - 0 removals = 162
unresolved local references=0
unresolved plan references=0
cyclic strongly connected components=0
DOC IDs=10; ready=10; task acceptance=10; runtime dependencies=10
docs/v2 sources=34; context-covered=34; unique writers=34; ledger-covered=34
E19-E45 mapped=27/27
```

All 193 tracked TOML files also parse with `tomllib` and zero errors.

## Strict disposable validation and sealing

The integrated repository CLI was run from a disposable `git archive` of the
base with only the two candidate control-plane files overlaid:

```text
roko plan validate --strict tmp/status-quo/backlog/plans
0 diagnostics in 55 plans

roko plan validate --strict tmp/status-quo/self-heal/plans
0 diagnostics in 6 plans

roko plan validate --strict tmp/status-quo/backlog/plans/DOC-v2-core
0 diagnostics in 1 plan
```

An initial source-root invocation also reproduced the top-level root's 94
pre-existing `PLAN_031` prerequisite diagnostics, which are outside CTRL-09
and assigned to CTRL-13. Its generated index change was immediately restored
through `apply_patch`; subsequent validation stayed entirely in the disposable
archive. Final controls:

```text
git diff --check: exit 0
plans/INDEX.md SHA-256:
7ac5679f9ff7a32571ad0ed70e9914b579f12a5f22e4285f4804c20a19077b44
changed paths before evidence: exactly the two bounded DOC-v2 control files
```

## Review handoff

The candidate is ready for independent review. Review must reproduce the actual
`TasksFile` semantic parse, ownership/coverage census, combined graph/SCC
check, disposable strict roots, scope, sealed-index hash, and preservation
comparison. Integration and canonical status updates remain coordinator-owned.
