# Meta-Analysis: Existing Mori-Diffs and Gap Analysis Work

> Written 2026-08-19. Synthesizes all existing analysis across `tmp/mori-diffs/` (42 files),
> `.roko/GAPS.md`, and `tmp/backlog/` to prevent duplicate effort.

---

## 1. Corpus Summary

The existing analysis work spans three main locations:

| Location | Files | Total Size | Date Range | Purpose |
|---|---|---|---|---|
| `tmp/mori-diffs/` | 38 active + 4 archived | ~1.4 MB | 2026-04-26 to 2026-04-28 | Runner-v3 design, repo-wide audits, gap ledger |
| `.roko/GAPS.md` | 1 | ~64 KB | Last updated 2026-08-17 | Canonical gap tracker, resolution log |
| `tmp/backlog/` | ~60 specs + index | ~200 KB est. | Last updated 2026-08-18 | Actionable implementation specs |

### What was analyzed

The mori-diffs package is a comprehensive architecture audit covering every major subsystem.
It progressed through three phases:

**Phase 1 (files 00-13): Runner-v3 subsystem design.** These documents diagnosed specific
runner-v2 weaknesses and proposed runner-v3 solutions:

- 00-OVERVIEW: Architecture overview, module map, design principles, v2 deficiencies
- 01-AGENT-DISPATCH: Claude CLI hardcoding, no provider abstraction, static model hints
- 02-PLAN-EXECUTION: Sentinel-based DAG, no gate timeout, no concurrency limits, no merge queue
- 03-PERSISTENCE: Only executor.json saved, no version field, count-based tracking
- 04-LEARNING: CascadeRouter unused, no routing observations, hardcoded backend string
- 07-MIGRATION: Transition plan from v2 + orchestrate.rs
- 08-FILE-MAP: Every file that needs to be created/modified
- 09-COMPOSITION-AUCTION: VCG/density prompt assembly, section-effect tracking
- 10-DREAMS-CONSOLIDATION: Dream trigger vs consolidation gap
- 11-PARALLEL-MERGE: PlanMerger, GitMergeBackend, conflict evidence
- 12-AFFECT-ROUTING: Affect/knowledge/provider/calibration routing
- 13-KNOWLEDGE-LIFECYCLE: Knowledge admission, tier progression, live ingestor

**Phase 2 (files 16-28): Repo-wide architecture assessment.** These broadened from
runner-specific issues to codebase-wide structural problems:

- 16-INFRASTRUCTURE: Event source/subscription/conductor/prompt/experiment proof
- 17-ARCHITECTURE-REALITY-CHECK: The central diagnosis -- "spec/runtime honesty gap"
- 18-MASTER-AUDIT: "From scratch" redesign -- 6 hard boundaries
- 19-SELF-REVIEW-AND-PROOF: Scoring rubric, iteration log, evidence standards
- 20-RUNTIME-RECONCILIATION: Blueprint for replacing orchestrate.rs
- 21-FEATURE-PARITY-MATRIX: Mori vs Legacy vs Runner vs Target capabilities
- 22-STABILITY-PLAN: Crash/resume/process/merge/provider stability
- 23-HANDOFF-OPEN-ITEMS: Subsystem checklist handoff (8 subsystem areas)
- 24-DEFINITIVE-GAP-LIST: Historical gap taxonomy (superseded by 29)
- 25-CODE-ONLY-LEGACY-AUDIT: Legacy surface retirement checklist
- 26-REPOSITORY-WIDE-CODE-AUDIT: Marker classification, owner mapping
- 27-FILESYSTEM-RUNTIME-CI-AUDIT: Clean-clone invariants, provider matrix
- 28-FEATURE-MATRIX-DOGFOOD-UX-AUDIT: Dogfood/runtime/UX reconciliation

**Phase 3 (files 29-41): Deep subsystem audits.** These are the most thorough and
specific. Each has source-verified grep scans, implementation batches, proof schemas,
and archive gates:

- 29-CURRENT-RUNTIME-GAP-LEDGER: **Canonical priority board** (P0-P3 items)
- 30-ARCHITECTURAL-SIDE-EFFECT-AUDIT: Side-effect ownership firewall
- 31-REPOSITORY-WIDE-ARCHITECTURE-SCAN: Crate/file counts, subsystem owners
- 32-DEPENDENCY-LAYERING-AUDIT: L0-L6 layer model, crate graph problems
- 33-CONFIGURATION-PROVIDER-POLICY-AUDIT: Config/secret/policy resolution gaps
- 34-OBSERVABILITY-PROJECTION-QUERY-AUDIT: Event/projection/query/TUI proof
- 35-TASK-PROCESS-LIFECYCLE-AUDIT: Process/cancellation/shutdown/operation
- 36-WORKFLOW-ENTRYPOINT-ORCHESTRATION-AUDIT: One-shot/project workflow engine
- 37-WORKSPACE-LAYOUT-ARTIFACT-STORE-AUDIT: Storage/migration/artifact ownership
- 38-COGNITIVE-FEEDBACK-LOOP-AUDIT: Learning/knowledge/dreams closed loop
- 39-RUNNER-EXECUTION-POLICY-AUDIT: Runner state-machine/gate/retry/merge
- 40-SERVE-TUI-RUNTIME-ADAPTER-AUDIT: HTTP/TUI as thin adapters
- 41-INFERENCE-GATEWAY-MODEL-CALL-SERVICE-AUDIT: Unified model-call service

**Archived (verified/closed):** Files 05, 06, 14, 15 moved to `archive/2026-04-26-verified/`.

---

## 2. Central Conclusions Reached

### 2.1 The core diagnosis (from 17-ARCHITECTURE-REALITY-CHECK)

> "Roko is no longer suffering from dead-crate refactor theater; it is suffering from
> glue-layer centralization and status inflation."

The crate extraction from Mori is real -- roko-agent, roko-compose, roko-learn, roko-gate,
roko-neuro, roko-dreams are all substantive crates. The problem is that their integration
point remains "call this from CLI runtime glue" rather than composing through one
authoritative runtime spine.

### 2.2 The structural problem (from 18-MASTER-AUDIT)

The codebase had three orchestration shapes:
1. `roko-orchestrator` -- pure executor/state-machine layer
2. `roko-cli/src/runner/` -- active event-loop runtime for plan run
3. `roko-cli/src/orchestrate.rs` -- older but richer integration harness

This created split ownership for dispatch, prompt assembly, routing, knowledge, learning,
dashboard projection, and dream hooks.

### 2.3 The cognitive loop gap (from 38-COGNITIVE-FEEDBACK-LOOP-AUDIT)

> "Roko has many cognitive subsystems, but not one cognitive control plane."

Individual subsystems (learning, knowledge, dreams, affect, conductor) are substantial,
but the active execution paths do not form one closed observe-attribute-update-consolidate-
retrieve-act-prove loop.

### 2.4 The side-effect ownership problem (from 30-ARCHITECTURAL-SIDE-EFFECT-AUDIT)

The runner has feedback/dispatch/projection facades, but they are not exclusive ownership
boundaries. The runner still writes episodes, efficiency events, knowledge, router
observations directly. Dispatch is bypassed by dispatch_direct, chat inline, and one-shot
paths. Projections are bypassed by TUI/serve reading files directly.

### 2.5 The dependency layering problem (from 32-DEPENDENCY-LAYERING-AUDIT)

Application crates, domain crates, provider crates, runtime infrastructure, and UI/server
surfaces know too much about each other. `roko-core` depends upward on `roko-runtime`.
Domain crates depend on concrete agent/provider types. The target L0-L6 layer model was
defined but not enforced.

### 2.6 The inference gateway gap (from 41-INFERENCE-GATEWAY-MODEL-CALL-SERVICE-AUDIT)

Multiple model-call paths existed: runner dispatch, research commands, dream consolidation,
neuro distillation, web search tools, and HTTP gateway routes each made model calls through
their own paths. The target was a unified `ModelCallService` / `InferenceGateway`.

---

## 3. What Has Been Acted On Since the Analysis

The mori-diffs analysis was written primarily 2026-04-26 through 2026-04-28. Substantial
implementation work followed through 2026-08-17. The `.roko/GAPS.md` file tracks what was
resolved. Here is a summary of resolution status:

### 3.1 Resolved items (from GAPS.md)

These gaps identified by the mori-diffs analysis are now marked RESOLVED:

| Gap | Resolution Date |
|---|---|
| Provider outcome feedback and health routing | 2026-08 |
| loop_tick.rs architecture mismatch | 2026-08-16 |
| HTTP MCP tools retain executable clients | 2026-08-13 |
| Gate threshold flush interval configurable | 2026-08-13 |
| Workspace lock coverage | 2026-08-13 |
| GitHub workflow integration (E46 12/12) | 2026-08-14 |
| E18 docs, config, and operations (15/15) | 2026-08-14 |
| Workspace release verification | 2026-08-16 |
| Worker deployment callbacks authenticated | 2026-08-13 |
| Runner VCG feedback loop | 2026-08-16 |
| Tier progression after live knowledge ingestion | 2026-08-13 |
| Playbook selection wired at dispatch | 2026-08 |
| events.jsonl retention | 2026-08-13 |
| Resource and disk lifecycle (E47) | 2026-08-13 |
| Knowledge store consulted for model routing | 2026-08-13 |
| Config init/global model slug collision | 2026-08-13 |
| Scheduler deadlock after AgentCompleted | 2026-08-13 |
| Stale executor snapshot blocks fresh plan runs | 2026-08-13 |
| core.fsmonitor breaks worktree checkout | 2026-08-13 |
| Cascade router silently overrides configured model | 2026-08-14 |
| Dream automatic scheduling | 2026-08-15 |
| Deprecated rate-oracle vertical | 2026-08-13 |
| Serve RBAC and credential boundaries | 2026-08-15 |
| ACP mutation consent boundary | 2026-08-15 |
| Runtime gate dependency inversion | 2026-08-15 |
| roko-acp compile issues | 2026-08-13 |
| roko-orchestrator test failures | 2026-08-13 |

### 3.2 Epic completions addressing mori-diffs concerns

The 48-epic programme completed E01-E48 plus R01-R04. Key completions directly
addressing mori-diffs findings:

- **E26 (Inference gateway 12/12):** The `roko-gateway` crate now owns the nine-stage
  inference pipeline. This directly addresses the 41-INFERENCE-GATEWAY audit.
- **E25 (Advanced learning 10/10):** HDC consolidation, hindsight, playbook enrichment,
  runner-wired dispatch. Addresses 04-LEARNING and 38-COGNITIVE gaps.
- **E24 (Advanced memory 10/10):** Demurrage, falsifiers, HDC lookup, distillation.
  Addresses 13-KNOWLEDGE-LIFECYCLE gaps.
- **E34 (Security 8/8):** Trust-origin IFC, taint tracking, immune Graph, corrigibility.
  Addresses 15-SAFETY-EXTENSIONS (archived) gaps.
- **E23 (Cognitive autonomy 10/10):** Lifecycle type-state, vitality, EFE routing.
- **E33 (Telemetry 39/39 ingress):** All production event variants wired.
  Addresses 34-OBSERVABILITY gaps.
- **E42 (Config evolution 8/8):** Priority/provenance, migrations, profiles.
  Addresses 33-CONFIGURATION gaps.
- **E44 (Cross-cut functors 8/8):** Memory/Daimon/Dreams/Safety composition.
  Addresses parts of the 38-COGNITIVE closed-loop gap.

### 3.3 Key architecture changes since the analysis

- `orchestrate.rs` retired as the primary runtime. Runner-v2 is the active path.
- `dispatch/` module family created and wired into the runner.
- `runtime_feedback/` sinks exist for episodes, routing, knowledge, conductor, dreams.
- `projection/` and `runner/projection.rs` exist.
- Provider-neutral `AgentRuntimeEvent` defined in roko-agent.
- Claude stream parsing moved below roko-agent.
- `PlanMerger` and `GitMergeBackend` exist in runner/merge.rs.
- `TaskDag` exists in runner/task_dag.rs.
- Persist/resume infrastructure in runner/persist.rs and runner/resume.rs.
- `WorkflowEngine` facade created.
- Foundation services (ModelCallService, PromptAssemblyService, FeedbackService,
  GateService) created via arch-runner batches.

---

## 4. What Remains Open or Partial

### 4.1 GAPS.md open/partial items

| Gap | Status | Category |
|---|---|---|
| event_loop.rs is a ~23.1K-line god object | OPEN | Architecture |
| Prompt-experiment coverage across runtimes | PARTIAL | Learning |
| Cross-crate duplicate type families | PARTIAL | Architecture |
| `#[allow(dead_code)]` sites | HYGIENE/DEFERRED | Code quality |
| Immune system screening coverage | PARTIAL | Security |
| Graph Engine incomplete | PARTIAL | Runtime |

### 4.2 Backlog items tracking mori-diffs concerns

The `tmp/backlog/` has 106 specs (items 01-106 minus 4 removed as implemented).
The ones most directly related to mori-diffs findings:

**P0 (Critical):**
- 78: Efficiency gate_passed bug
- 86: Gate compile tool bypass
- 87: Task parser duplicate IDs

**P1 (High, directly from mori-diffs themes):**
- 60: Safety dispatch hardening (maps to 30-SIDE-EFFECT-AUDIT)
- 90: UX34 override learning isolation (maps to 38-COGNITIVE)
- 95: Config loader robustness (maps to 33-CONFIGURATION)
- 101: Async runtime anti-patterns (maps to 35-TASK-PROCESS)

**P2 (Medium, directly from mori-diffs themes):**
- 20: Event loop decomposition (maps to GAPS.md event_loop.rs god object)
- 42: Duplicate type consolidation (maps to cross-crate type families)
- 43: Clippy suppression removal (maps to dead_code hygiene)
- 47: ConfigLayer elimination (maps to 33-CONFIGURATION)
- 54: Graph engine runner-v2 parity (maps to Graph Engine PARTIAL)
- 55: AgentPool runtime integration (maps to 01-AGENT-DISPATCH warm pool)
- 61: Agent dispatch consolidation (maps to 01-AGENT-DISPATCH, 30-SIDE-EFFECT)
- 67: HDC prompt assembly wiring (maps to 09-COMPOSITION-AUCTION)
- 68: Budget pre-dispatch admission (maps to 02-PLAN-EXECUTION)
- 69: SSE parsing deduplication (maps to 34-OBSERVABILITY)
- 72: Pool architecture reconciliation (maps to 01-AGENT-DISPATCH)
- 80: Learning subsystem data quality (maps to 04-LEARNING, 38-COGNITIVE)
- 84: Cascade router task category (maps to 12-AFFECT-ROUTING)

---

## 5. Recurring Themes and Patterns

Five patterns recur across all the analysis work:

### 5.1 "Built but not wired" -- the dominant failure mode

This is the single most repeated finding. Every subsystem audit discovers substantial
code that exists in a crate but is not reachable from the live execution path. Examples:
CascadeRouter existed but was never consulted (04-LEARNING), warm pool concept existed
but was never instantiated (01-AGENT-DISPATCH), dream triggers existed but had no
production consumer (38-COGNITIVE), knowledge candidates were written but never read
(38-COGNITIVE). The epic programme addressed many of these, but the pattern persists
in Pool management and some projection surfaces.

### 5.2 "Multiple paths doing the same thing" -- ownership fragmentation

The second most repeated finding. Agent dispatch, prompt assembly, model calls, config
resolution, and state persistence each had 2-4 parallel paths doing roughly the same
thing with slight differences. The audits call this "split ownership" (18-MASTER-AUDIT),
"duplicated side effects" (30-SIDE-EFFECT-AUDIT), and "glue-layer centralization"
(17-ARCHITECTURE-REALITY-CHECK). The consolidation of orchestrate.rs into runner-v2
addressed the biggest instance, but serve routes and some CLI commands still have
independent dispatch paths.

### 5.3 "Status inflation" -- docs describing target state as current state

A persistent finding. The audits explicitly call out docs that "describe target
architecture using shipping language" (17-ARCHITECTURE-REALITY-CHECK), making the repo
"look more coherent on paper than it is in code." The 19-SELF-REVIEW-AND-PROOF file
establishes an explicit rubric for honest status reporting with the labels: proven,
wired-unproven, built-unrouted, legacy-only, open, stale-doc.

### 5.4 "Proof gap" -- claims without reproducible evidence

The audits repeatedly distinguish between "module exists" and "behavior is proven."
They define proof as: the active runner path owns the behavior AND a reproducible proof
command or artifact exists. Ten required proof artifacts were specified (00-OVERVIEW)
and none were generated. The stability plan (22) requires unit, integration, resume,
crash, and dogfood proof. The dogfood rerun (first run 2026-08-13) exposed real bugs
and has been partially addressed but never fully re-verified.

### 5.5 "God object" -- event_loop.rs as gravitational center

The event_loop.rs file was ~3K lines during the initial audit and is now ~23K lines.
It is the single largest technical debt item. Extraction has begun (gate_dispatch.rs,
persist.rs, snapshot_writer.rs, merge.rs, branch_cleanup.rs) but the core remains
a monolith that owns too many concerns. This is tracked in both GAPS.md (OPEN) and
backlog item 20 (P2, XL size estimated at 2-3 weeks).

---

## 6. What NOT to Duplicate

Any new analysis work should avoid re-covering these already-completed analyses:

### 6.1 Do not re-audit these subsystems from scratch

- **Provider dispatch architecture:** 01-AGENT-DISPATCH + 41-INFERENCE-GATEWAY
  comprehensively cover the pre-E26 state. E26 delivered the gateway crate.
- **Persistence/resume design:** 03-PERSISTENCE + 22-STABILITY-PLAN cover the
  design and hardening requirements.
- **Learning feedback loop design:** 04-LEARNING + 38-COGNITIVE-FEEDBACK cover the
  target cognitive spine. E25 delivered the learning loops.
- **Dependency layering target:** 32-DEPENDENCY-LAYERING defines the L0-L6 model.
- **Configuration/policy design:** 33-CONFIGURATION defines the target. E42 delivered
  config evolution.
- **Feature parity matrix:** 21-FEATURE-PARITY-MATRIX provides the Mori vs Roko
  capability grid. Many rows have been addressed by the epic programme.

### 6.2 Do not re-create these inventories

- Side-effect ownership inventory (30-ARCHITECTURAL-SIDE-EFFECT-AUDIT)
- Repository-wide file/crate scan (31-REPOSITORY-WIDE-ARCHITECTURE-SCAN)
- Config/env/secret source scan (33-CONFIGURATION scan counts)
- Cognitive subsystem pattern counts (38-COGNITIVE evidence scan)

### 6.3 Use existing tracking, do not create parallel trackers

- `.roko/GAPS.md` is the canonical gap tracker (resolution log for architectural items)
- `tmp/backlog/00-INDEX.md` is the canonical implementation spec index
- `tmp/mori-diffs/29-CURRENT-RUNTIME-GAP-LEDGER.md` is the mori-diffs priority board
- `tmp/mori-diffs/23-HANDOFF-OPEN-ITEMS.md` is the subsystem checklist handoff

---

## 7. Where New Work Should Focus

Based on what the existing analysis identifies as open but has not yet addressed:

### 7.1 Stale mori-diffs content

The mori-diffs documents are dated 2026-04-26 through 2026-04-28. Significant
implementation work happened between then and 2026-08-17 (48 epics completed,
124 plan tasks done). Many specific claims in the mori-diffs files are now stale.
The documents themselves note this with "2026-04-27 source correction" sections, but
these corrections are from April, not from August. A productive exercise would be to
update the feature parity matrix (21) and gap ledger (29) against August 2026 source
truth, but this should be a targeted update, not a ground-up re-audit.

### 7.2 TUI and UX comparison (the original goal)

The CONTEXT.md in `tmp/mori-old/` establishes that the original goal of the current
analysis round is to make roko's TUI match or exceed mori's quality. The mori-diffs
package covers architecture, runtime, persistence, learning, and dispatch in great
depth, but it has relatively little specific TUI/UX comparison. File
28-FEATURE-MATRIX-DOGFOOD-UX-AUDIT touches on this but from a runtime audit
perspective, not from a hand-verified UX quality perspective.

### 7.3 End-to-end dogfood verification

The first dogfood run (2026-08-13) exposed four blockers, all fixed. A clean live
full self-hosting rerun has not been completed. This is identified in CLAUDE.md as
priority item 19 and in GAPS.md as a remaining tranche.

### 7.4 Event_loop.rs decomposition

The single largest open architectural item. Backlog spec 20 defines the work but
it is sized XL (2-3 weeks). The god-object pattern is well-analyzed; what is needed
is execution, not more analysis.

### 7.5 Cross-surface projection agreement

TUI, HTTP API, CLI, and the converged frontend should project the same authoritative
durable state. This was identified by the audits (34-OBSERVABILITY, 40-SERVE-TUI)
and is listed as a remaining tranche in GAPS.md. StateHub baseline/overlay, cursor-atomic
SSE, and single-generation resume are explicitly partial.

---

## 8. Document Relationship Map

```
CANONICAL TRACKERS (use these, do not duplicate):
  .roko/GAPS.md .................. Resolution log, architectural gaps
  tmp/backlog/00-INDEX.md ........ Implementation specs index

MORI-DIFFS PRIORITY HIERARCHY (read in this order):
  29-CURRENT-RUNTIME-GAP-LEDGER .. Priority board (supersedes 24)
  30-ARCHITECTURAL-SIDE-EFFECT ... Side-effect ownership firewall
  31-REPOSITORY-WIDE-ARCH-SCAN .. Crate/file-level triage
  32-DEPENDENCY-LAYERING ......... L0-L6 layer model
  33-CONFIGURATION-PROVIDER ...... Config/secret/policy
  34-OBSERVABILITY-PROJECTION .... Event/projection/query
  35-TASK-PROCESS-LIFECYCLE ...... Process/cancellation/shutdown
  36-WORKFLOW-ENTRYPOINT ......... Workflow engine design
  37-WORKSPACE-LAYOUT ............ Storage/artifact ownership
  38-COGNITIVE-FEEDBACK-LOOP ..... Learning closed loop
  39-RUNNER-EXECUTION-POLICY ..... Runner state-machine
  40-SERVE-TUI-ADAPTER ........... HTTP/TUI convergence
  41-INFERENCE-GATEWAY ........... Model-call service

MORI-DIFFS SUBSYSTEM DETAIL (consult for specific areas):
  00-OVERVIEW .................... Runner v3 architecture entry point
  01-AGENT-DISPATCH .............. Provider dispatch design
  02-PLAN-EXECUTION .............. DAG/gate/retry/merge design
  03-PERSISTENCE ................. Snapshot/resume design
  04-LEARNING .................... Feedback loop design
  07-MIGRATION ................... orchestrate.rs transition
  08-FILE-MAP .................... Module ownership map
  09-COMPOSITION-AUCTION ......... VCG prompt assembly
  10-DREAMS-CONSOLIDATION ........ Dream trigger/consolidation
  11-PARALLEL-MERGE .............. Merge/warm-pool
  12-AFFECT-ROUTING .............. Affect/knowledge routing
  13-KNOWLEDGE-LIFECYCLE ......... Knowledge admission/progression

MORI-DIFFS CROSS-CUTTING (repo-wide assessment):
  16-INFRASTRUCTURE .............. Event sources, subscriptions
  17-ARCHITECTURE-REALITY-CHECK .. Central diagnosis
  18-MASTER-AUDIT ................ "From scratch" redesign
  19-SELF-REVIEW-AND-PROOF ....... Evidence standards
  20-RUNTIME-RECONCILIATION ...... orchestrate.rs replacement
  21-FEATURE-PARITY-MATRIX ....... Mori vs Roko capabilities
  22-STABILITY-PLAN .............. Crash/resume hardening
  23-HANDOFF-OPEN-ITEMS .......... Subsystem checklists
  25-CODE-ONLY-LEGACY-AUDIT ...... Legacy retirement
  26-REPOSITORY-WIDE-CODE-AUDIT .. Marker classification
  27-FILESYSTEM-RUNTIME-CI ....... Clean-clone, provider matrix
  28-FEATURE-MATRIX-DOGFOOD-UX ... Dogfood/UX audit

ARCHIVED:
  archive/2026-04-26-verified/05 . Prompt assembly (verified)
  archive/2026-04-26-verified/06 . Observability (verified)
  archive/2026-04-26-verified/14 . Failure/retry (verified)
  archive/2026-04-26-verified/15 . Safety/extensions (verified)

CURRENT WORK CONTEXT:
  tmp/mori-old/CONTEXT.md ........ Goals for current analysis round
  tmp/mori-old/MORI-TUI-SCREENSHOTS.md . Mori TUI reference
```
