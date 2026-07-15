# Backlog and Roadmap State Audit

Audited: 2026-07-14

Scope: every one of the 102 files under `tmp/status-quo/backlog/`

Current repository HEAD: `1649c18b2c3d`
Backlog documents' stated baseline: mostly `5852c93c05` (2026-07-09/10)

> Historical baseline notice: the semantic-duplication finding and recovery-order
> item 4 below are addressed by the corrected CTRL-08 ownership contract in
> [`../backlog/17-OPERATIONAL-OWNERSHIP.md`](../backlog/17-OPERATIONAL-OWNERSHIP.md).
> Candidate `b9387fe6c3f42209a317a301302b027a6b882042` was independently rejected and is
> preserved with review `b0e21f69f427e738a7198f43ad5d827cf0b7c486`. Its replacement
> `ff6dc54afeccf4d06ebd95e476756d2383422205` was also rejected and is preserved with
> review `87461143496d405a0c3a0adffa9bfa2c278f1bc6`. Candidate
> `ec3ecf2f89f0dd74a6c5e973c9ea4c7185bec30e` was likewise rejected and preserved
> with review `ac3cfb8439bd4223663759360ad73a7b18461419`. The current correction was
> reconstructed from integration base `dd611500e7f9051fbdd3843cd20c5472efcfcbb7`.
> Counts and implementation states in this dated audit remain historical until their
> separately owned status reconciliation; this notice does not rewrite them.

## Verdict

The backlog is substantially **authored**, but barely **recorded as executed**.

- The executable layer is large and mostly well-shaped: 48 epic plans with 447 implementation
  tasks, six DOC plans with 71 reconciliation tasks, and one 96-task superseded authoring plan.
- Recorded implementation completion is **6/447 (1.34%)**. All six completed tasks are
  `E01-T01` through `E01-T06`. The remaining **441/447** are still marked `ready`.
- DOC reconciliation completion is **0/71**.
- The 96 old authoring-gap tasks are correctly `skipped`; they are provenance, not delivered
  implementation.
- All TOML files parse, every `[meta].total` and `[meta].done` matches its task records, task IDs
  are unique, same-plan dependencies resolve, and all non-skipped tasks have verify commands.
- This does **not** make the set execution-ready as a whole. A clean standalone backlog run has
  unresolved cross-plan names, the validator emits 23 warnings, several status/index documents
  contradict the manifests, and substantial semantic duplication remains.

The honest progress number is therefore **1.34% recorded implementation completion**, with an
important caveat: the manifests are behind the code. A read-only run of all 745 structural checks
on the 441 `ready` tasks found 50 tasks whose structural checks already pass and another 43 with
partial structural evidence. Those are candidates for reconciliation, not safe additions to the
done count; broad grep checks can false-positive and compile/test/acceptance evidence was not
complete for that set.

## Quantitative state

| Layer | Plans | Tasks | Done | Ready | Skipped | Recorded completion |
|---|---:|---:|---:|---:|---:|---:|
| E01-E18 status-quo work | 18 | 169 | 6 | 163 | 0 | 3.55% |
| E19-E45 v2 work | 27 | 243 | 0 | 243 | 0 | 0% |
| E46-E48 operational work | 3 | 35 | 0 | 35 | 0 | 0% |
| **Implementation total** | **48** | **447** | **6** | **441** | **0** | **1.34%** |
| DOC reconciliation | 6 | 71 | 0 | 71 | 0 | 0% |
| Superseded task authoring | 1 | 96 | 0 | 0 | 96 | intentionally retired |

`roko plan run tmp/status-quo/backlog/plans --dry-run --json` currently discovers 56 entries and
614 task records: 55 TOML plans plus the Markdown `plans/00-INDEX.md` as a zero-task entry. The
614 count includes 96 skipped provenance tasks; it must not be presented as remaining product
work.

## Validation and execution blockers

### Validator result

`target/debug/roko plan validate tmp/status-quo/backlog/plans` exits successfully in non-strict
mode but reports **23 diagnostics in 55 plans**. The same command with `--strict` exits **1**, so
the backlog is not strict-validation-clean:

- E06: missing decision output `decisions/E06-canonical-surface.md`.
- E09: missing decision output `decisions/E09-telemetry-lens-pipeline.md`.
- E11: missing recovered `plans/architecture-core-queue/tasks.toml`.
- E18: five missing intended outputs (`deny.yml`, `docker/roko.toml`, `docs-lint.yml`,
  `plan-validate.yml`, and `docs/v2/GITHUB-INTEGRATION.md`).
- E22: missing intended graph file `.roko/graphs/cognitive-loop.toml`.
- E26: fourteen warnings because the planned `roko-gateway` crate does not exist yet.

Most are legitimate creation targets, but the coverage document says there are only six warnings.
That assertion is stale.

### Cross-plan dependency defect

The live task DAG blocks a task until every `depends_on_plan` value appears in the completed-plan
set. The backlog contains **36 references to names that are neither backlog plan IDs nor top-level
plan IDs**, plus **11 references to top-level `plans/P*` plans that are outside this plan root**.
Consequently, a clean `--fresh` execution of only `tmp/status-quo/backlog/plans` can wait forever
even though validation exits zero.

The 36 missing-name references are concentrated in:

- `DOC-v1-cognition`: 26 short aliases such as `E01`, `E05`, and `E18` instead of canonical plan
  names.
- E06: three `E01` references.
- E07: four `E01` references.
- E10: `E01` and `E03`.
- E14: one case-mismatched `E01-EXECUTION-ENGINE` reference.

The 11 external references occur in E04, E07, E16, and E17 and target existing top-level P-plans.
Those dependencies need either a supported external-completion contract or migration into this
plan root. E11's missing `architecture-core-queue` is an additional intended prerequisite/output.

### Semantic duplication

The move from 149 E01-E18 tasks to 169 added 20 operational tasks, while E46-E48 separately added
35 operational tasks. The added E01/E08/E09/E14/E15/E17/E18 tasks overlap GitHub automation,
disk/worktree management, rate limiting, budgets, and operational documentation already owned by
E46-E48. The IDs are unique, but the work is not cleanly unique. Treat **447 as authored task
records, not 447 independent units of value**, until an ownership/dedup pass decides which epic is
canonical and turns the other records into dependencies or acceptance coverage.

## Documentation truth audit

Every Markdown document in the backlog is annotated below.

| Document | State | Annotation |
|---|---|---|
| `00-INDEX.md` | stale/mixed | Correct 447/71 headline and 48-epic inventory, but still says E01 has not landed, Graph is the default, runner-v2 is mandatory, and the current HEAD equals `5852c93c05`. It also calls the checklist/coverage 149-task artifacts although they now contain 447. |
| `01-TASK-EXECUTION-SCHEMA.md` | useful reference | Thorough schema reference; should be revalidated when the parser changes. It does not catch plan-root cross-dependency resolution. |
| `02-PLANS-RECONCILIATION.md` | historical | Valuable mapping to P08-P34, but anchored to old HEAD and describes pre-self-heal execution behavior. Use as provenance, not live status. |
| `03-WORK-BREAKDOWN-EPICS.md` | partial/historical | Sound E01-E18 work breakdown, but title/scope remains 18 epics/149 tasks and does not represent the 48-epic executable set. |
| `04-EXECUTION-READINESS.md` | obsolete diagnosis | Its Graph-default bootstrap diagnosis has been fixed. The risk list remains useful, but it is no longer a current gate report. |
| `05-MASTER-CHECKLIST.md` | coverage-good/status-wrong | Exhaustively lists 447 unique task IDs, but all 447 boxes are unchecked even though E01-T01..T06 are `done`; its legend says all tasks are ready. It cannot be used for progress reporting. |
| `06-EXECUTABLE-TASK-FILE-COVERAGE.md` | count-good/validation-stale | Correct 169+243+35=447 and 71 DOC counts. Its “six expected warnings” and “until E01-T01 lands” execution note are stale. |
| `07-SUBAGENT-TASK-AUTHORING-NOTES.md` | authoring provenance | Useful explanation of the initial 149-task materialization; it predates expansion to 447 and should not be read as full current coverage. |
| `08-SOURCE-CORPUS-PLAN-COVERAGE.md` | verified coverage, unexecuted | Its script still verifies 744 designated source docs, zero missing from ledgers/plans. All 71 resulting DOC tasks remain ready, so coverage means assigned, not reconciled. |
| `09-UNIFIED-ROADMAP.md` | useful strategy/stale counts | Good strategic sequencing and 47 unchecked exit outcomes. Some epic task counts (notably E46) predate final plan totals, and none of its outcomes are marked achieved. |
| `10-EPIC-DEPENDENCY-MATRIX.md` | conceptual only | Helpful human DAG, but it does not expose the executable `depends_on_plan` alias/external-root defect. Do not substitute it for DAG validation. |
| `11-EXECUTION-PLAYBOOK.md` | operational draft | Detailed procedures, but examples and gates inherit old status assumptions. Its seven checkboxes are all open. Reconcile after dependency names and canonical execution slices are fixed. |
| `12-MILESTONE-DEFINITIONS.md` | stale/internally inconsistent | Still states 389 implementation tasks and ~389 total, versus actual 447. All 46 milestone checkboxes are open. Useful acceptance language, unreliable totals/status. |
| `13-PLAN-AUDIT-E19-E30.md` | superseded audit | Says 115 tasks/zero blockers; current E19-E30 total is 118. It predates additions to E27 and E30 and says cross-plan deps are merely informal even though the runner enforces them. |
| `14-PLAN-AUDIT-E31-E42.md` | partly superseded | Correct 101-task scope. Its E36 self-dependency and E41 path issues have already been removed/fixed in TOML, but the document still reports them. E37's fragile structural check remains a quality warning. |
| `15-PLAN-AUDIT-E43-E48.md` | superseded findings | Its E47 bad context path and E48 noncanonical tiers have been corrected in TOML, while the document still reports them. Current validator evidence should replace its verdict. |
| `16-FINAL-GAP-ANALYSIS.md` | planning coverage, not implementation | Correctly motivated E46-E48, but still says Graph-default execution is broken and uses pre-remediation estimates. “Covered” means task coverage, not delivered features. |
| `GAP-REPORT-V3.md` | narrow historical evidence | A useful documentation-gap report against old HEAD. Its recommendation is about pack coverage, not current implementation completion, and should be rerun after code/document drift. |

## Epic prose audit

The 18 `epics/*.md` files are detailed source/problem statements for the original E01-E18 layer.
They are not live status records: none carries task completion state, all were authored against the
older baseline, and several no longer match executable plan cardinality.

| Epic document(s) | State | Annotation |
|---|---|---|
| `E01-EXECUTION-ENGINE.md` | stale | Describes 10 tasks and an unfixed Graph default; plan now has 16 tasks and T01-T06 are done. |
| `E02-STORAGE-CONVERGENCE.md` | planned | 12-task problem decomposition; manifest has 12 ready. |
| `E03-TYPE-CONSOLIDATION.md` | planned | Seven prose tasks match seven ready manifest tasks. |
| `E04-SECURITY-PERIMETER.md` | planned with latent-code signals | 19 manifest tasks ready; several structural checks already pass, so status reconciliation is needed before execution. |
| `E05-GATE-ADAPTIVITY-LIVE.md` | planned with latent-code signals | Eight ready; several structural checks already pass. Do not infer full acceptance from grep evidence. |
| `E06-COMPOSE-UNIFY.md` | planned/dependency-broken | Nine ready; three cross-plan dependencies use the noncanonical `E01` name. |
| `E07-LEARNING-KNOWLEDGE.md` | planned/dependency-broken | Ten ready; four `E01` aliases and one external P19 dependency need an execution contract. |
| `E08-CONDUCTOR-SUPERVISION.md` | stale count | Prose has seven tasks; manifest has nine after adding disk/worktree watchers that overlap E47. |
| `E09-OBSERVABILITY.md` | stale count | Prose has nine tasks; manifest has eleven after operational additions overlapping E47. |
| `E10-FRONTEND-CONTRACT.md` | planned/dependency-broken | Seven ready; two short cross-plan aliases are unresolved. |
| `E11-CHAIN-ISFR.md` | planned/missing prerequisite | Five ready; E11-T01 is explicitly meant to recover the absent architecture-core queue. |
| `E12-DEAD-CODE-CLEANUP.md` | planned/high-risk | Nine ready. Destructive/deletion tasks are correctly dependency-gated and must remain late. |
| `E13-SPEC-DEBT-V2.md` | planned/long-horizon | Three ready; appropriately not an early execution gate. |
| `E14-PROVIDERS-TOOLS.md` | stale count/dependency-broken | Prose has seven core tasks; manifest has twelve after rate/GitHub additions overlapping E46/E48, plus one case-mismatched plan dependency. |
| `E15-MCP-CONFIG.md` | stale count | Prose has six tasks; manifest has seven after GitHub auto-discovery, overlapping E46. |
| `E16-PRD-SELF-HOSTING.md` | externally gated | Two ready tasks, both dependent on P-plans outside this root. |
| `E17-ACP-COMPLETION.md` | stale count/externally gated | Prose has six core tasks; manifest has eight after budget/rate additions overlapping E48. Five external P-plan references need prior-state or root integration. |
| `E18-DOCS-CONFIG-OPS.md` | stale count | Prose has 13 core tasks; manifest has 15 after GitHub validation/docs additions overlapping E46. Five intended outputs remain validator warnings. |

## Exhaustive executable-plan ledger

“Missing” means a `depends_on_plan` value does not equal a plan ID in either this backlog root or
the repository's top-level `plans/`. “External” means it does exist in top-level `plans/`, but is
not loaded by a standalone run of this backlog root.

| Plan | Tasks | Done | Ready | Skipped | Cross-root issue |
|---|---:|---:|---:|---:|---|
| `DOC-status-quo-corpus` | 12 | 0 | 12 | 0 | — |
| `DOC-v1-cognition` | 7 | 0 | 7 | 0 | missing: 26 refs |
| `DOC-v1-ecosystem` | 10 | 0 | 10 | 0 | — |
| `DOC-v1-kernel` | 8 | 0 | 8 | 0 | — |
| `DOC-v2-core` | 10 | 0 | 10 | 0 | — |
| `DOC-v2-depth` | 24 | 0 | 24 | 0 | — |
| `E01-execution-engine` | 16 | 6 | 10 | 0 | — |
| `E02-STORAGE-CONVERGENCE` | 12 | 0 | 12 | 0 | — |
| `E03-type-consolidation` | 7 | 0 | 7 | 0 | — |
| `E04-security-perimeter` | 19 | 0 | 19 | 0 | external: 2 refs |
| `E05-gate-adaptivity-live` | 8 | 0 | 8 | 0 | — |
| `E06-COMPOSE-UNIFY` | 9 | 0 | 9 | 0 | missing: 3 refs |
| `E07-learning-knowledge` | 10 | 0 | 10 | 0 | missing: 4; external: 1 |
| `E08-conductor-supervision` | 9 | 0 | 9 | 0 | — |
| `E09-OBSERVABILITY` | 11 | 0 | 11 | 0 | — |
| `E10-FRONTEND-CONTRACT` | 7 | 0 | 7 | 0 | missing: 2 refs |
| `E11-chain-isfr` | 5 | 0 | 5 | 0 | missing intended architecture-core queue |
| `E12-DEAD-CODE-CLEANUP` | 9 | 0 | 9 | 0 | — |
| `E13-SPEC-DEBT-V2` | 3 | 0 | 3 | 0 | — |
| `E14-providers-tools` | 12 | 0 | 12 | 0 | missing: 1 ref |
| `E15-mcp-config` | 7 | 0 | 7 | 0 | — |
| `E16-prd-self-hosting-gaps` | 2 | 0 | 2 | 0 | external: 3 refs |
| `E17-acp-completion` | 8 | 0 | 8 | 0 | external: 5 refs |
| `E18-DOCS-CONFIG-OPS` | 15 | 0 | 15 | 0 | — |
| `E19-signal-protocol` | 10 | 0 | 10 | 0 | — |
| `E20-cell-unification` | 10 | 0 | 10 | 0 | — |
| `E21-graph-engine` | 10 | 0 | 10 | 0 | — |
| `E22-execution-runtime` | 10 | 0 | 10 | 0 | intended graph-file warning |
| `E23-agent-cognitive-autonomy` | 10 | 0 | 10 | 0 | — |
| `E24-memory-advanced` | 10 | 0 | 10 | 0 | — |
| `E25-learning-loops-advanced` | 10 | 0 | 10 | 0 | — |
| `E26-inference-gateway` | 12 | 0 | 12 | 0 | new-crate warnings |
| `E27-feeds-system` | 10 | 0 | 10 | 0 | audit doc still says 8 |
| `E28-groups-coordination` | 8 | 0 | 8 | 0 | — |
| `E29-connectivity-relay` | 9 | 0 | 9 | 0 | — |
| `E30-extension-system` | 9 | 0 | 9 | 0 | audit doc still says 8 |
| `E31-trigger-system` | 8 | 0 | 8 | 0 | — |
| `E32-tool-plugin-ecosystem` | 8 | 0 | 8 | 0 | — |
| `E33-telemetry-lens` | 9 | 0 | 9 | 0 | — |
| `E34-security-ifc` | 8 | 0 | 8 | 0 | — |
| `E35-auth-protocol` | 8 | 0 | 8 | 0 | — |
| `E36-payments` | 8 | 0 | 8 | 0 | old self-dependency already fixed |
| `E37-surfaces` | 9 | 0 | 9 | 0 | fragile verify remains |
| `E38-marketplace` | 9 | 0 | 9 | 0 | — |
| `E39-registries-identity` | 8 | 0 | 8 | 0 | — |
| `E40-arenas-evals` | 8 | 0 | 8 | 0 | old self-dependency already fixed |
| `E41-defi-products` | 8 | 0 | 8 | 0 | old bad path already fixed |
| `E42-config-evolution` | 8 | 0 | 8 | 0 | — |
| `E43-deployment-portability` | 8 | 0 | 8 | 0 | multiple structural checks already pass; reconcile status |
| `E44-cross-cut-functors` | 8 | 0 | 8 | 0 | — |
| `E45-orchestrator-mori-parity` | 10 | 0 | 10 | 0 | multiple structural checks already pass; reconcile status |
| `E46-github-workflow-integration` | 12 | 0 | 12 | 0 | overlaps expanded E01-E18 tasks |
| `E47-resource-disk-management` | 11 | 0 | 11 | 0 | overlaps expanded E01-E18 tasks; old audit path fixed |
| `E48-rate-limit-budgeting` | 12 | 0 | 12 | 0 | overlaps expanded E01-E18 tasks; 11/12 structural checks currently pass |
| `status-quo-authoring-gaps` | 96 | 0 | 0 | 96 | correctly superseded; exclude from remaining work |

## Remaining backlog files

| Files | State | Annotation |
|---|---|---|
| `exemplars/EX01-flip-engine-default.toml` | obsolete as exemplar outcome | The showcased change is already done; retain as schema example only and label it historical. |
| `exemplars/EX02-unify-signal-store.toml` | usable exemplar | Still an implementation example, not evidence E02 is done. |
| `exemplars/EX03-delete-orphan-statehub.toml` | usable/historical exemplar | Useful deletion-plan shape; ensure it does not duplicate a completed or re-scoped type-cleanup task. |
| `plans/00-INDEX.md` | materially stale | Lists only E01-E18 and 149 implementation tasks although 48 epic plans/447 tasks are present. |
| `references/PLANNING-METHODOLOGY.md` | sound reference | Methodology is useful background; it does not establish implementation truth. |
| `source-coverage/status-quo-corpus.md` | verified assignment ledger | 108 designated top-level pack docs are mapped. |
| `source-coverage/docs-v1-kernel.md` | verified assignment ledger | Its paths are present in both source ledger and DOC task layer. |
| `source-coverage/docs-v1-cognition.md` | coverage-good/execution-broken | Sources are mapped, but its DOC plan uses 26 unresolved short plan aliases. |
| `source-coverage/docs-v1-ecosystem.md` | verified assignment ledger | Source mapping passes; reconciliation work remains unexecuted. |
| `source-coverage/docs-v2-core.md` | verified assignment ledger | Source mapping passes; reconciliation work remains unexecuted. |
| `source-coverage/docs-v2-depth.md` | verified assignment ledger | Source mapping passes; reconciliation work remains unexecuted. |

## Recommended recovery order

1. **Create one authoritative status ledger.** Derive it from TOML, not checkboxes. Make the master
   checklist generated or remove its status semantics.
2. **Reconcile the 50 structurally-green ready tasks.** Run each task's full compile/test and
   acceptance suite; mark done only with evidence. E48, E43, E45, E04, and E05 have the largest
   visible status drift.
3. **Fix cross-plan identity before any broad run.** Canonicalize all short/case-mismatched E-plan
   names and define how external P-plan completion is imported. Add plan-root dependency linting.
4. **Deduplicate operational ownership.** Make E46/E47/E48 canonical, then convert overlapping
   additions in E01-E18 to dependencies/acceptance checks, or vice versa.
5. **Refresh the navigation/status documents.** Fix `00`, `05`, `06`, `plans/00`, `12`, and the
   three audit docs first. Archive obsolete Graph-default statements instead of leaving them as
   current instructions.
6. **Execute by safe slices.** Finish/reconcile E01, then security and honest-gate prerequisites,
   then storage/types/providers/MCP. Do not run E12 deletions or speculative E19-E45 work ahead of
   those foundations.
7. **Run DOC reconciliation after implementation ownership stabilizes.** Its assignment coverage
   is complete, but running it now risks authoring another parallel layer of duplicate work.

## Confidence

- **High**: file inventory, manifest counts/statuses, parser/schema checks, validator diagnostics,
  dry-run counts, source-corpus coverage, unresolved dependency-name counts, stale numeric claims.
- **Medium**: semantic-overlap assessment and structural evidence for latent implementation.
- **Low by design**: treating any `ready` task as fully implemented without its full verify and
  acceptance suite. This report deliberately does not inflate the done count.
