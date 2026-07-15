# Top-Level Document Audit — 2026-07-14

Scope: every Markdown file directly under `tmp/status-quo/`: numbered documents
`00`–`106` plus `DOC-MANIFEST.md` (108 files total). This report does not treat a
checklist, roadmap item, or completion-sounding prose as implementation proof.

Current code baseline: `main` at `1649c18b2c3d` (2026-07-12). Most of the pack
was verified at `5852c93c05` on 2026-07-08. The current baseline is 517 commits
later. Those commits include large artifact/archive moves, but also touch 40
production/test files: especially Runner v2 (15 files), `roko-gate` (6),
`roko-core` (2), `roko-runtime` (2), and `roko-graph` (1).

## Overall verdict

The top-level pack is a strong **July 8 evidence snapshot**, not a reliable
current completion ledger. Its subsystem maps, traces, and explicit file:line
evidence are unusually detailed, but the navigation layer has not been rolled
forward after implementation work. The most prominent claim in the pack is now
wrong:

- `roko plan run` now defaults to `runner-v2`
  (`crates/roko-cli/src/main.rs:1361`; changed in `996c7a08d`).
- `roko resume` now routes to `PlanEngine::RunnerV2`
  (`crates/roko-cli/src/main.rs:2699`; changed in `8bf311c2f`).
- The explicit Graph engine is still hollow: `TaskExecutorCell::default()` is
  `dry_run: true`, and the live branch still says “live dispatch not yet
  implemented” (`roko-graph/src/cells/task_executor.rs:32,86`).

Therefore the old P0 “the default path silently does no work” is fixed, while
the narrower debt “Graph lacks live execution/parity” remains. This distinction
invalidates the headline ordering in `00`, `01`, `12`, `13`, `24`, `29`, and
`95`, but does not invalidate most of their deeper architectural findings.

### Aggregate classification

| Primary class | Files | Meaning |
|---|---:|---|
| Analysis | 48 | Audits, censuses, and synthesized status claims |
| Implementation evidence | 20 | Direct code/runtime traces, route/schema/command/test ledgers |
| Source evidence | 12 | Archaeology, provenance, generated manifests, source crosswalks |
| Proposal/spec | 11 | Roadmaps, decisions, proof criteria, migration/deletion plans |
| Mixed | 11 | Navigation or files combining evidence and prescriptions |
| Obsolete/stale | 6 | Explicitly superseded first-pass material |
| **Total** | **108** | All top-level files accounted for |

These classes describe the document’s role, not whether its recommendations
were implemented. Across the corpus there are 33 files that still assert or
rely on the old Graph default and 15 that still describe resume as hardcoded to
Graph. Ninety-six files name the old `5852c93c` baseline explicitly.

## Evidence checks against current HEAD

| Claim family | Current result | Consequence for pack |
|---|---|---|
| Default execution engine | **Fixed**: Clap default is `runner-v2` | Old headline/P0 and user guidance are stale |
| `roko resume` engine | **Fixed**: routes to Runner v2 | Old “snapshot discarded” P0 is stale |
| Explicit Graph execution | **Still incomplete**: task executor defaults to dry-run; live branch is a stub | Engine-parity work remains open |
| Deployment clean checkout | **Still blocked**: root `roko.toml` is absent/untracked while Docker expects it | `09`/`77` blocker remains credible |
| Compose serve command | **Still stale**: `docker/docker-compose.yml:82` uses removed `--listen` | Deploy debt remains open |
| Test inventory | **Drifted**: current direct attribute census is 10,088; docs report 9,968 and 10,062 in different places | `10`, `16`, and `74` must be regenerated together |
| Runner module inventory | **Drifted**: 21 top-level Rust files now; docs say 19 or 20 | Runner family census needs refresh |
| Pack manifest | **Stale**: says backlog dirs are empty; they now contain epics/plans/coverage docs | `DOC-MANIFEST` no longer describes the tree |

No current full-workspace test run is claimed by this audit. “Substantiated”
below means current code or repository structure directly supports the named
claim, not that every behavior was executed end to end.

## Exhaustive per-file ledger

Disposition vocabulary: **current spot-check** = central claim checked at
current HEAD; **baseline-bound** = useful evidence at `5852c93c`, no full
re-verification here; **needs refresh** = a current contradiction is known;
**historical** = intentionally about an older source; **superseded** = another
top-level file is already designated as canonical.

### 00–29 — navigation, whole-system status, and source reconciliation

| File | Class | Status claims and current substantiation |
|---|---|---|
| `00-INDEX.md` | Mixed | **Needs refresh.** Pack navigation is useful, but its load-bearing Graph-default claim is false and its inventory predates the populated backlog/self-heal trees. |
| `01-EXECUTIVE-SUMMARY.md` | Mixed | **Needs refresh.** Half-migrated verdict remains plausible; “single dominant issue = default Graph no-op” and broken-resume claims are resolved. Security claims were not changed in the post-baseline source set and remain baseline evidence. |
| `02-SPEC-EVOLUTION.md` | Mixed | **Needs refresh.** Valuable spec history; its correction banner still states the obsolete Graph default. Treat coverage percentages/checkmarks as July 8 analysis. |
| `03-CRATE-AUDIT.md` | Analysis | **Baseline-bound.** Strong per-package census, but current changes touch core/runner/gate/runtime and exact LOC/test/caller claims are no longer current. |
| `04-NAMING-MIGRATION.md` | Mixed | **Baseline-bound.** Evidence-backed noun migration plus a proposal; no proof that unchecked rename items were subsequently completed. |
| `05-ARCHITECTURE-REALITY.md` | Obsolete/stale | **Superseded** by `13`/`36` by its own banner and manifest; also carries Graph-default framing. Keep only as historical narrative. |
| `06-WIRING-STATUS.md` | Analysis | **Needs refresh.** Its built-but-unwired method is sound, but it explicitly calls Graph the default and Runner v2 opt-in. |
| `07-MIGRATION-CHECKLIST.md` | Obsolete/stale | **Superseded** by `12`/`24`/`27`; 38 unchecked boxes are plans, not evidence of remaining current work. |
| `08-TECH-DEBT.md` | Obsolete/stale | **Superseded** first-pass debt list; its correction banner’s two P0s (default Graph and resume) have landed. |
| `09-DEPLOYMENT-STATUS.md` | Obsolete/stale | **Superseded** by `58`/`77`, but its checked blockers remain current: missing tracked `roko.toml` and compose `--listen`. |
| `10-TESTING-STATUS.md` | Obsolete/stale | **Superseded** by `16`/`74`; counts are now inconsistent with both current tree and sibling docs. |
| `11-DEPENDENCY-GRAPH.md` | Analysis | **Baseline-bound.** Manifest-parsed layer audit is good source evidence; rerun before using exact dependency counts because affected crates changed. |
| `12-ROADMAP.md` | Proposal/spec | **Needs re-baseline.** P0.1/P0.2 are implemented; all 43 boxes remain unchecked, proving the file was not used as a live completion ledger. |
| `13-CURRENT-STATE-MATRIX.md` | Analysis | **Needs refresh.** Rows for default execution and resume are resolved; deep subsystem rows remain useful baseline claims. |
| `14-V1-COVERAGE.md` | Analysis | **Needs refresh.** The ~72%/~60% estimates are judgment, not executable proof; engine-default/resume reasoning is obsolete. |
| `15-V2-COVERAGE.md` | Analysis | **Needs refresh.** Graph parity gaps remain, but the claim that the v2 execution contract is the default no-op no longer holds. Summary should continue to defer to `85` for detail. |
| `16-CODEBASE-INVENTORY.md` | Implementation evidence | **Needs regeneration.** Shell-computed baseline census is reproducible, but tests/modules/LOC have moved since July 8. |
| `17-TMP-SOURCE-RANKING.md` | Source evidence | **Historical/current-purpose.** Useful authority ranking of source material; not implementation status and should not be counted as completed work. |
| `18-V2-DEPTH-COVERAGE.md` | Analysis | **Baseline-bound.** Exhaustive spec-to-code analysis, but percentages are editorial estimates and current runner/gate work is not incorporated. |
| `19-DOC-DRIFT-REGISTER.md` | Analysis | **Needs refresh.** It correctly records July 8 drift but itself repeats the now-fixed Graph-default problem; maintained docs must be compared anew. |
| `20-TMP-NEWEST.md` | Source evidence | **Historical.** Valuable reconciliation of May–June design material; status dispositions are baseline-bound. |
| `21-TMP-MAY-BATCH.md` | Source evidence | **Historical with stale disposition.** Its design archaeology remains valid; “Graph is CLI default” no longer does. |
| `22-TMP-LEGACY.md` | Source evidence | **Historical.** Intentionally inventories May 1 archive material; do not treat checkboxes as live backlog without re-triage. |
| `23-TASKRUNNER-MIGRATION-STATUS.md` | Mixed | **Needs refresh.** Runner lineage is valuable; default-engine and current file-count claims have moved. |
| `24-OPEN-ISSUE-LEDGER.md` | Mixed | **Needs reconciliation.** At least the first two P0 rows are fixed; the ledger has no per-row implementation linkage/status transition. |
| `25-PROOF-GATES.md` | Proposal/spec | **Useful but mostly unexecuted.** Seventy-two unchecked gates define proof; they do not prove absence or completion. Engine gates need expected-result updates. |
| `26-CANONICAL-DECISIONS.md` | Proposal/spec | **Baseline decision queue.** Recommendations are not ratifications; current commits implicitly chose Runner v2 for default/resume but the decision record was not updated. |
| `27-IMPLEMENTATION-BACKLOG.md` | Proposal/spec | **Needs reconciliation.** Actionable work breakdown, but no completion state and old default/resume tasks remain described as open. |
| `28-DEFINITION-OF-DONE.md` | Proposal/spec | **Current as policy.** Strong criteria (default path + durable state + test + docs); it intentionally contains no evidence that those criteria were met. |
| `29-RISK-REGISTER.md` | Mixed | **Needs refresh.** Default synthetic-success and resume risks are mitigated on the default Runner-v2 path; explicit Graph risk remains. |

### 30–59 — subsystem audits and operational ledgers

| File | Class | Status claims and current substantiation |
|---|---|---|
| `30-CORE-SIGNAL.md` | Analysis | **Baseline-bound.** Naming/storage split analysis remains useful; two core files changed and exact census/line claims need rerun. |
| `31-GRAPH-CELLS-ENGINE.md` | Analysis | **Partly current.** Graph task executor is still a dry-run/live stub, directly substantiated; “default engine” wording is stale. |
| `32-EVENTS-BUS-STATEHUB.md` | Analysis | **Baseline-bound.** Serve was untouched post-baseline; runner/runtime integration may alter cross-path conclusions. |
| `33-AGENT-SAFETY.md` | Analysis | **Baseline-bound.** No evidence in this audit of a completed safety migration; one agent file changed, so exact provider-path conclusions need review. |
| `34-COMPOSE-PROMPTS.md` | Analysis | **Current spot-check.** Unusually, this file says Runner v2 is the no-flag path; that now matches current code and contradicts most sibling docs. Broader prompt-stack claims remain baseline-bound. |
| `35-GATES-VERIFICATION.md` | Analysis | **Needs refresh.** Six gate files and substantial runner gate ownership code changed after baseline. Do not rely on exact “stub/live” rows without rerun. |
| `36-ORCHESTRATION-RUNNERS.md` | Analysis | **Needs refresh.** Three-generation architecture remains, but selection/default and runner internals changed materially. |
| `37-RUNNER-V2-AND-GRAPH.md` | Analysis | **Partly superseded** by `95`; strategic recommendation effectively landed for default/resume, Graph parity remains open. |
| `38-AGENT-PROVIDERS-TOOLS.md` | Analysis | **Baseline-bound.** Provider/tool defects need targeted current verification; not proof that downstream tasks fixed them. |
| `39-NEURO-KNOWLEDGE.md` | Analysis | **Baseline-bound.** No post-baseline crate changes detected in its primary scope; integration claims can still drift through runner changes. |
| `40-LEARNING-TELEMETRY.md` | Analysis | **Baseline-bound.** Primary crate unchanged; runner telemetry and persistence changed, so end-to-end conclusions need refresh. |
| `41-DREAMS.md` | Analysis | **Needs refresh.** Its “not live default because Graph” rationale is obsolete; Runner v2 dream wiring is now on the default path. Remaining trigger/parity gaps may still hold. |
| `42-CHAIN-REGISTRIES-ISFR.md` | Analysis | **Baseline-bound.** Primary crate/apps unchanged; current usage/caller counts were not rerun. |
| `43-SURFACES-DEMO-UX.md` | Analysis | **Needs refresh.** Demo sources are unchanged, but plan-run UX conclusions inherit obsolete engine-default assumptions. |
| `44-AGENT-SERVER.md` | Analysis | **Baseline-bound.** Primary server unchanged post-baseline; still not an executed endpoint proof. |
| `45-CLI-SURFACE.md` | Analysis | **Needs refresh.** CLI changed heavily (26 files); command/default/resume tables are no longer authoritative. |
| `46-SERVE-HTTP-REALTIME.md` | Analysis | **Baseline-bound/high confidence for local structure.** Serve source was untouched post-baseline; cross-runtime behavior still needs integration proof. |
| `47-FOUNDATION-TYPES-REDESIGN.md` | Mixed | **Baseline-bound.** Combines duplicate-type audit and target design; two checked boxes do not establish full consolidation. |
| `48-MCP-CRATES.md` | Analysis | **Baseline-bound.** Primary crates unchanged; trust/integration conclusions remain evidence, not implementation completion. |
| `49-INDEX-LANG.md` | Analysis | **Baseline-bound.** Primary crates unchanged; duplicate HDC/caller claims should be mechanically rerun before deletion/refactor. |
| `50-QUALITY-CI-RELEASE.md` | Analysis | **Baseline-bound.** CI policy/gap overview, not proof that workflows passed or release readiness was achieved. |
| `51-ACP.md` | Analysis | **Baseline-bound.** Primary crate unchanged; permission/safety integration remains a baseline finding. |
| `52-PLUGIN-EXTENSIONS.md` | Analysis | **Baseline-bound.** Mixed checkboxes reflect doc audit progress and feature gaps; nine checked items are not whole-system completion. |
| `53-OBSERVABILITY.md` | Analysis | **Baseline-bound.** Runner telemetry changed substantially, so exact event/projection claims need a current trace. |
| `54-PER-CRATE-MIGRATION-CHECKLIST.md` | Proposal/spec | **Unreconciled.** 104 open boxes are a target-state list, not 104 confirmed current defects. |
| `55-DATA-DIR.md` | Obsolete/stale | **Superseded** by `60`; retain only as a lightweight path index. |
| `56-DAIMON.md` | Analysis | **Needs refresh.** “dark on Graph default” is no longer relevant to default execution; Runner-v2 integration claims may have strengthened. |
| `57-CONFIG.md` | Analysis | **Baseline-bound.** Config source changed in two core files; exact preflight/default tables need rerun. |
| `58-JOBS-DEPLOY.md` | Analysis | **Current spot-check for blockers.** Deployment shape is baseline evidence; missing `roko.toml` and stale compose flag remain. |
| `59-API-ROUTE-LEDGER.md` | Implementation evidence | **Baseline-bound/high confidence.** Serve route source was untouched; route existence is not endpoint correctness or auth proof. |

### 60–84 — cross-cutting implementation ledgers and maintenance plans

| File | Class | Status claims and current substantiation |
|---|---|---|
| `60-STATE-PERSISTENCE-LEDGER.md` | Implementation evidence | **Needs targeted refresh.** Direct writer→reader evidence is strong; runner persistence changed after baseline, so runner rows/atomicity conclusions moved. |
| `61-CONFIG-ENV-MATRIX.md` | Implementation evidence | **Baseline-bound.** Useful source map; config files changed and current env/config precedence was not rerun. |
| `62-CLI-COMMAND-LEDGER.md` | Implementation evidence | **Needs regeneration.** CLI changed materially; default/resume rows are known stale even if command count remains close. |
| `63-DELETE-ARCHIVE-PLAN.md` | Proposal/spec | **Current as safety policy.** Explicitly does not authorize deletion; no removal should be inferred as done. |
| `64-PARITY-TEST-MATRIX.md` | Proposal/spec | **Unexecuted target matrix.** Twelve open checkboxes specify missing proof; current test existence/results need separate census. |
| `65-DOCS-CONVERGENCE-PLAN.md` | Proposal/spec | **Needs re-baseline.** Good layering model; example drift list still says resume hardcodes Graph. |
| `66-FRONTEND-API-PARITY.md` | Implementation evidence | **Baseline-bound/high confidence.** Frontend and serve trees were untouched post-baseline; route existence still does not prove behavior. |
| `67-TMP-FEEDBACK-2-CROSSWALK.md` | Source evidence | **Historical plus baseline dispositions.** The 35-issue source map is valuable; FIXED/PARTIAL/OPEN states were not rolled forward after runner work. |
| `68-SELF-DEVELOPING-CROSSWALK.md` | Source evidence | **Needs disposition refresh.** Source archaeology is valid; “develop inherits Graph default” is obsolete. |
| `69-RESIDUAL-AUDIT-TRACKER.md` | Source evidence | **Historical.** Continuation map from a branch/worktree audit, explicitly not current truth. |
| `70-RELAY-PROTOCOL-FREEZE.md` | Mixed | **Baseline-bound.** Combines protocol evidence and freeze checklist; no post-baseline relay/app changes detected, but checks are not release proof. |
| `71-CI-RELEASE-PROOF-GAPS.md` | Implementation evidence | **Baseline-bound.** Workflow-by-workflow code inspection is useful; no claim here that current CI was executed. |
| `72-SOURCE-DOC-COVERAGE-LEDGER.md` | Source evidence | **Needs regeneration.** Coverage counts describe source conversion, not implementation; pack/backlog growth makes totals stale. |
| `73-EXAMPLES-PLANS-GRAPHS.md` | Implementation evidence | **Baseline-bound.** Loader/schema checks are concrete; explicit Graph still stubbed, but assets should be reparsed at current HEAD. |
| `74-TEST-AND-PROOF-INVENTORY.md` | Implementation evidence | **Needs regeneration.** Its 10,062 attribute count differs from `10` and current direct census (10,088); volume remains distinct from proof. |
| `75-SECURITY-AUTH-SCOPE-MATRIX.md` | Implementation evidence | **Baseline-bound/high priority.** Serve/apps unchanged, so code findings likely persist; must still be proven with adversarial integration tests. |
| `76-DATA-CONTRACTS-SCHEMAS.md` | Implementation evidence | **Baseline-bound.** Direct schema mapping is useful; current duplicate/conversion census not rerun after core/runtime changes. |
| `77-OPERATIONS-DEPLOY-RUNBOOK.md` | Mixed | **Current spot-check for named blockers.** Missing root config and obsolete compose flag remain; no clean-checkout deploy was executed here. |
| `78-V2-DEPTH-RESEARCH-PROMPT-LEDGER.md` | Source evidence | **Current as provenance.** Correctly fences research prompts from specification/implementation truth; incidental Graph-default references need update. |
| `79-REFERENCE-PROVENANCE-LEDGER.md` | Source evidence | **Current as provenance.** Bibliography ownership, not implementation status. |
| `80-SOURCE-DOC-MANIFEST.md` | Source evidence | **Needs regeneration.** Generated source inventory is dated July 7 and cannot prove implementation coverage. |
| `81-ROOT-DOCS-REWRITE-QUEUE.md` | Proposal/spec | **Needs refresh.** Useful rewrite workflow, but queued drifts and priority ordering include resolved execution issues. |
| `82-COMMAND-EXAMPLE-DRIFT-LEDGER.md` | Implementation evidence | **Needs refresh.** Current CLI default/resume changed; old replacement guidance can now mislead in the opposite direction. |
| `83-ENV-VAR-MANIFEST.md` | Source evidence | **Baseline-bound.** Generated env-read census should be rerun after config/core changes; it is not behavior proof. |
| `84-STATUS-PACK-MAINTENANCE.md` | Proposal/spec | **Good policy, not followed fully.** It explains regeneration, but generated files and status claims were not refreshed after E01/self-heal work. |

### 85–106 and manifest — coverage detail, subsystem families, and traces

| File | Class | Status claims and current substantiation |
|---|---|---|
| `85-V2-COVERAGE-KERNEL.md` | Analysis | **Needs refresh.** Graph implementation gaps remain, but all “Graph default” and broken-resume conclusions are obsolete. |
| `86-V2-COVERAGE-PLATFORM.md` | Analysis | **Baseline-bound.** Primary platform sources mostly unchanged; status percentages remain editorial, not execution proof. |
| `87-V2-COVERAGE-ECOSYSTEM.md` | Analysis | **Needs refresh.** Graph is no longer the CLI default; other ecosystem status rows remain baseline evidence. |
| `88-CONDUCTOR.md` | Analysis | **Baseline-bound.** Primary crate unchanged; runner changed heavily, so claims about what was/was not ported need a fresh caller trace. |
| `89-PRIMITIVES-HDC.md` | Analysis | **Baseline-bound.** Primary crate unchanged; exact consumer/feature activation counts were not rerun. |
| `90-RUNTIME-FS-STD.md` | Analysis | **Needs targeted refresh.** Runtime and runner changed; Graph-default/orchestrate-default wording is inconsistent with current selection. Atomic-storage findings may also have moved in later work. |
| `91-PRD-RESEARCH.md` | Analysis | **Needs refresh.** Its claimed CLI chain break at `plan run` is fixed by Runner-v2 default; research/provider and serve auto-plan defects require separate current checks. |
| `92-RUNNER-V2-MODULE-FAMILY.md` | Analysis | **Needs regeneration.** Runner is now default/resume target and has 21 top-level files, not 19; 15 runner files changed. |
| `93-ROKO-DEMO.md` | Analysis | **Baseline-bound/high confidence.** Demo crate unchanged; being compiled/self-contained is not a current smoke-test result. |
| `94-FEED-AGENTS-FLEET.md` | Analysis | **Baseline-bound/high confidence.** Serve feed sources unchanged; live data quality still needs runtime proof. |
| `95-ENGINE-DRIFT.md` | Analysis | **Needs major rewrite.** Excellent historical diagnosis, but its title/headline/default/resume P0 are resolved. Keep three-engine/parity analysis; demote Graph from “default” to explicit experimental path. |
| `96-TRACE-RUNNER-V2-EXECUTION.md` | Implementation evidence | **Needs retrace.** Runner is now the default path and 15 files changed, including lifecycle/ownership/persistence; old hop/line map is not current. |
| `97-TRACE-SERVE-LIFECYCLE.md` | Implementation evidence | **Baseline-bound/high confidence.** Serve source unchanged; its checked trace items show audit completion, not feature completeness. |
| `98-TRACE-SELF-HOSTING-LOOP.md` | Implementation evidence | **Needs major retrace.** The chain no longer falls into Graph by default; explicit Graph still behaves as documented. |
| `99-TRACE-AGENT-TURN.md` | Implementation evidence | **Baseline-bound.** Checked boxes mean trace claims were inspected, not that security/provider gaps were fixed. One agent file changed. |
| `100-TRACE-ACP-SESSION.md` | Implementation evidence | **Baseline-bound.** ACP source unchanged; open checklist items and architectural permission finding remain unproven at runtime here. |
| `101-TRACE-GATE-PIPELINE.md` | Implementation evidence | **Needs retrace.** Six gate files plus runner gate ownership changed; old exact hop/status map is unsafe to treat as current. |
| `102-SPEC-DEBT-LEDGER.md` | Analysis | **Baseline-bound.** Useful canonical roll-up, but current implementation commits were not reconciled into its concept statuses. |
| `103-DUPLICATE-TYPES-CENSUS.md` | Analysis | **Baseline-bound.** Six checked items attest census method, not consolidation; core/runtime changed and conversion census needs rerun. |
| `104-DEAD-CODE-AND-FACADE-CENSUS.md` | Analysis | **Needs refresh.** Current changes can alter caller/dead-code results; incidental Graph-default claims are stale. |
| `105-FRONTEND-DEMO-APP.md` | Implementation evidence | **Baseline-bound/high confidence.** Frontend/serve source unchanged; route mismatches likely persist, but no browser/API run was performed here. |
| `106-APPS-MIRAGE-RELAY-WATCHER.md` | Implementation evidence | **Baseline-bound/high confidence.** Apps/serve source unchanged; ten checked items mean file audit completeness, not operational deployment. |
| `DOC-MANIFEST.md` | Mixed | **Needs regeneration.** Good taxonomy/consolidation proposal, but says backlog directories are empty and reports a July 9 tree that no longer exists. |

## Cross-document contradictions and organization debt

1. **The navigation layer is not derived from implementation state.** E01 commits
   changed the two highest-priority behaviors, yet `00`, `01`, `12`, `13`, `24`,
   `29`, and `95` still present them as current P0s.
2. **One document already contradicts the old consensus.** `34-COMPOSE-PROMPTS`
   says no-flag execution is Runner v2; that statement now matches current code,
   while 33 sibling files still say Graph.
3. **Checkbox semantics are ambiguous.** In proposal docs, unchecked means “proof
   desired”; in traces, checked means “this audit step was performed”; in coverage
   docs, checkboxes may describe implementation. They cannot be summed as progress.
4. **Counts have multiple owners.** Test totals differ between `10` and `74`, then
   both drift from current HEAD. Runner file counts differ between `23`, `92`, and
   the current tree. Generated facts need one script/output owner.
5. **Canonical and historical documents are interleaved.** Explicitly superseded
   `05`, `07`, `08`, `09`, `10`, and `55` remain in the primary numeric sequence
   without a machine-readable lifecycle status.
6. **Evidence and plans are mixed at the same level.** A reader can easily infer
   that detailed roadmap prose means implementation occurred. `backlog/` and
   `self-heal/` now contain actual execution state, but the top-level pack does not
   roll those statuses back into its issue rows.
7. **The manifest is descriptive, not authoritative.** Its “backlog empty” entry
   proves it must be generated from the filesystem/task schemas, not maintained as
   prose.
8. **File:line evidence ages quickly.** The baseline-to-current runner changes are
   large enough that line references in traces `96` and `101` are no longer safe,
   even when the conceptual finding survives.

## Recommended next actions

1. Create one machine-readable status registry keyed by stable issue/task ID with
   `state = proposed|ready|running|verified|superseded`, `proof`, `verified_commit`,
   and `supersedes`. Generate summaries and checkboxes from it.
2. Immediately re-baseline `00`, `01`, `12`, `13`, `24`, `29`, and `95` against
   E01: mark default-engine and resume items verified, retain explicit Graph parity
   as a separate open item.
3. Retrace the code areas actually changed: `35`, `45`, `53`, `60`, `62`, `90`,
   `92`, `96`, and `101`. Do not spend equal effort re-auditing untouched demo/app
   sources.
4. Move or banner the six superseded files. Keep history, but remove them from the
   default reading path.
5. Regenerate `16`, `72`, `74`, `80`, `83`, and `DOC-MANIFEST` from scripts and
   make CI fail when generated inventories drift.
6. Link each top-level issue row to the executable backlog/self-heal task(s), the
   landing commit(s), and proof output. Without that bridge, “done” commits and the
   status pack will continue to disagree.
7. Run the narrow proof suite for E01/default/resume, then a full workspace quality
   pass. Code inspection proves route selection; it does not by itself prove a real
   provider run, durable resume, or cross-surface parity.

## Bottom line

The documents demonstrate that the July 8 audit work was broad and methodical.
They do **not** show that the total migration/debt programme is complete. They
also understate progress in the one area that was subsequently implemented:
the default and resume execution routes. Use this corpus as evidence and design
history, use executable task manifests/commits/tests as completion truth, and
reconcile the two before assigning any overall percentage.
