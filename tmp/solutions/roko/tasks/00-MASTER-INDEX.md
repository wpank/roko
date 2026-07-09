# 00 - Master Task Index

> Comprehensive index of all 19 task files in the roko implementation plan.
> Auto-generated from source files. Covers 735 tasks across 5 phases.
>
> **Master Plan**: `impl/00-MASTER-PLAN.md`
> **Generated**: 2026-04-29, branch `wp-arch2`

---

## Summary

| Metric | Value |
|--------|-------|
| Total tasks | 735 |
| Total files | 19 |
| Known effort (from 420 tasks) | 1780 hours |
| Estimated total effort | ~3114 hours |
| Anti-patterns catalogued | 91 |
| Quick wins (<=2h, no deps) | 46 |
| P0 (critical) tasks | 68 |
| P1 (architecture) tasks | 153 |
| P2 (features/UX) tasks | 156 |
| P3 (polish/GTM) tasks | 49 |
| Unstructured priority tasks | 289 |

**Note**: Files 06, 08, 10, 12, 13, 14, 17, 18 use unstructured metadata
(phase-level effort, LOC estimates, or no explicit priority/effort fields).
Their tasks are counted but marked with `??` priority and estimated effort.

---

## File Overview

| # | File | Domain | Tasks | Known Hours | Priorities | Phase |
|---|------|--------|-------|-------------|------------|-------|
| 01 | `01-STABILITY-AND-FIXES.md` | P0 panics, security holes, config crashes, learning wir | 78 | 206h | P0-P2 | 0 |
| 02 | `02-ORCHESTRATION.md` | Runtime convergence, orchestrate.rs decomposition, para | 28 | 119h | P0-P2 | 1-2 |
| 03 | `03-INFERENCE-DISPATCH.md` | Model selection unification, CascadeRouter, provider co | 38 | 136h | P0-P3 | 0-1 |
| 04 | `04-GATE-PIPELINE.md` | Gate convergence, GateService, config format unificatio | 27 | 73h | P0-P2 | 0-1 |
| 05 | `05-GATE-EVOLUTION.md` | LLM judge, express gates, wave gates, self-improving cr | 48 | 258h | P0-P3 | 2-3 |
| 06 | `06-PROMPT-ASSEMBLY.md` | Model-aware windowing, BudgetPredictor, section effecti | 33 | est. | mixed | 2 |
| 07 | `07-LEARNING-AND-FEEDBACK.md` | Episode logging, routing observations, threshold update | 27 | 88h | P0-P3 | 0 |
| 08 | `08-UX-AND-CLI.md` | Context Packs, streaming fix, plan grounding, CLI polis | 47 | est. | mixed | 0-2 |
| 09 | `09-ACP-AND-MCP.md` | ACP/MCP convergence, A2A bridge, MCP federation, protoc | 40 | 157h | P0-P3 | 2-4 |
| 10 | `10-PERFORMANCE.md` | Tracing, caching, gate modes, warm pool, PGO, HAL bench | 45 | est. | mixed | 3 |
| 11 | `11-INNOVATIONS.md` | Agent memory, multi-agent, speculative execution, cross | 65 | 492h | P0-P3 | 3-4 |
| 12 | `12-CODE-DEBT.md` | Dead code, duplicate elimination, module extraction, St | 37 | est. | mixed | 1-2 |
| 13 | `13-GTM-AND-INTEGRATIONS.md` | Adapter system, OTel, GitHub/Linear/Slack/Sentry, recip | 43 | est. | mixed | 4 |
| 14 | `14-RUNNER-PATTERNS.md` | Worktree isolation, wave gating, cumulative context, re | 30 | est. | mixed | 2-3 |
| 15 | `15-TESTING-AND-VERIFICATION.md` | HAL benchmarks, SWE-bench, CI integration, multi-dimens | 38 | 134h | P0-P2 | 4 |
| 16 | `16-CONFIG-AND-WIRING.md` | Config-runtime disconnects, env var bypass, built-but-n | 26 | est. | P1-P2 | 0-1 |
| 17 | `17-SAFETY-AND-SECURITY.md` | Permissions, AgentContract, NetworkPolicy, ScrubPolicy, | 21 | est. | mixed | 4 |
| 18 | `18-OBSERVABILITY.md` | Gateway events, regression detection, dry-run, checkpoi | 33 | est. | mixed | 3 |
| 19 | `19-CROSS-CUTTING.md` | StateHub unification, event schema, error handling, spe | 31 | 116h | P0-P10 | 1-2 |
| | **TOTAL** | | **735** | **1780h** known | | |

---

## Phase 0: Critical Fixes (Week 1-2)

**Goal**: Nothing crashes, nothing leaks secrets, config is respected,
the system records feedback on every run.

**Effort**: 6-10 person-days across 4 parallel tracks.

### Track A: P0 Bug Fixes (1 day)

| Task | Description | Effort | Depends | Source |
|------|------------|--------|---------|--------|
| 1.01 | Fix `roko config mcp` unreachable panic | 1h | none | 01 |
| 1.02 | Move share routes inside auth middleware | 1h | none | 01 |
| 1.03 | Auto-provision auth on cloud deploy | 2h | none | 01 |
| 1.04 | Add secret scrubbing to CLI Gist share path | 1h | none | 01 |
| 1.05 | Fix `acknowledge_public_risk` bypass | 1h | none | 01 |
| 1.06 | Remove `unsafe set_var` for --provider | 2h | none | 01 |
| 1.07 | Fix stub gate verdicts giving false PASS | 2h | none | 01 |
| 1.08 | Fix dual episode writes in `roko run` | 1h | none | 01 |
| 1.09 | Fix streaming events silently drained in chat | 2h | none | 01 |
| 1.10 | Fix `default_model` config being ignored | 2h | 1.09 | 01 |
| 4.1 | Add complexity and prior_failures to GateConfig | 1h | none | 04 |
| 4.2 | Add feedback/failure_classification to GateReport | 1h | none | 04 |
| 4.8 | Add Verdict::skip constructor, convert stub_verdict | 2h | none | 04 |
| 9.1 | Consolidate estimate_tokens into roko-core | 2h | none | 09 |
| 2.14 | Add Serialize/Deserialize to TaskStatus | 2h | none | 02 |
| 3.1 | Add load/save helpers for CascadeRouter | 2h | none | 03 |

### Track B: Learning Wiring (2-3 days)

| Task | Description | Effort | Depends | Source |
|------|------------|--------|---------|--------|
| 1.23 | Wire runner v2 AdaptiveThreshold observations | 1h | none | 01 |
| 1.24 | Wire runner v2 episode logging | 1h | none | 01 |
| 1.25 | Wire runner v2 section effectiveness updates | 1h | none | 01 |
| 1.26 | Wire runner v2 efficiency event recording | 1h | none | 01 |
| 1.12 | Wire feedback recording to `roko chat` | 2h | none | 01 |
| 7.01-7.06 | FeedbackService wiring (6 tasks) | 28h | varies | 07 |
| 16.11 | Wire CascadeRouter observations into runner v2 | -- | none | 16 |
| 16.12 | Add FeedbackService to `roko chat` path | -- | none | 16 |

### Track C: Config Consistency (2-3 days)

| Task | Description | Effort | Depends | Source |
|------|------------|--------|---------|--------|
| 16.1 | Make `auth_detect.rs` respect `roko.toml` providers | med | none | 16 |
| 16.2 | Fix `roko init` gate format to match runtime | sm | none | 16 |
| 16.10 | Route all CLI entry points through ServiceFactory | -- | 16.1 | 16 |
| 16.17 | Add config validation on load | -- | none | 16 |
| 16.18 | Normalize model aliases at load time | -- | none | 16 |
| 16.26 | Add config migration for gate format | -- | 16.2 | 16 |
| 3.22 | Remove hardcoded model string from auth_detect.rs | 2h | none | 03 |

### Track D: Streaming UX (1-2 days)

| Task | Description | Effort | Depends | Source |
|------|------------|--------|---------|--------|
| 16.15 | Wire streaming events to TUI in chat_inline.rs | -- | none | 16 |
| 1.32 | Fix model showing "-" in TUI for runner v2 | 1h | none | 01 |
| 1.34 | Fix `signals.jsonl` dead path | 1h | none | 01 |

---

## Phase 1: Architecture Convergence (Week 3-4)

**Goal**: One dispatch path, one gate pipeline, orchestrate.rs decomposed.
The system has a single source of truth for execution and learning.

**Effort**: 10-14 person-days across 4 parallel tracks.

### Track A: Runtime Convergence (3-4 days)

| Task | Description | Effort | Depends | Source |
|------|------------|--------|---------|--------|
| 2.01 | Extract OrchestrateCtx into standalone crate | 8h | none | 02 |
| 2.02 | Build RunnerV2 context handoff | 6h | 2.01 | 02 |
| 2.03 | Implement FeatureExtractor for orchestrate.rs | 6h | 2.02 | 02 |
| 2.04 | Port playbook injection to runner v2 | 4h | 2.03 | 02 |
| 2.05 | Port knowledge store queries to runner v2 | 4h | 2.03 | 02 |
| 2.06 | Port C-factor computation | 4h | 2.03 | 02 |
| 2.07 | Port crate familiarity tracking | 3h | 2.06 | 02 |
| 2.10-2.12 | Worktree integration (3 tasks) | 12h | varies | 02 |
| 16.3 | Thread `workflow.template` config into runner v2 | sm | none | 16 |
| 16.4 | Wire `learning.replan_on_gate_failure` into runner v2 | -- | none | 16 |

### Track B: orchestrate.rs Decomposition (3-4 days)

| Task | Description | Effort | Depends | Source |
|------|------------|--------|---------|--------|
| 2.15-2.18 | Module extraction (gate, routing, context, feedback) | 24h | 2.14 | 02 |
| 2.19 | Residual orchestrate.rs (state mgmt only) | 4h | 2.15-18 | 02 |
| 12.01-12.10 | Code debt: lint, duplication, dead code (10 tasks) | -- | varies | 12 |
| 19.01-19.05 | StateHub unification, event schema (5 tasks) | 30h | varies | 19 |

### Track C: Dispatch Unification (2-3 days)

| Task | Description | Effort | Depends | Source |
|------|------------|--------|---------|--------|
| 3.01-3.06 | CascadeRouter wiring, model selection (6 tasks) | 20h | varies | 03 |
| 3.07-3.10 | Stream parser consolidation (4 tasks) | 14h | varies | 03 |
| 3.22-3.26 | Hardcoded model removal (5 tasks) | 14h | varies | 03 |
| 16.9 | Remove `unsafe set_var` for provider override | -- | none | 16 |

### Track D: Gate Pipeline Convergence (2-3 days)

| Task | Description | Effort | Depends | Source |
|------|------------|--------|---------|--------|
| 4.03-4.07 | GateService wiring, config unification (5 tasks) | 20h | 4.01-02 | 04 |
| 4.09-4.15 | GateService integration into execution paths (7 tasks) | 24h | 4.03 | 04 |
| 4.21-4.27 | Gate events, metrics, persistence (7 tasks) | 18h | varies | 04 |

---

## Phase 2: UX Workflow and Features (Week 5-8)

**Goal**: Context Packs workflow end-to-end. Prompt assembly is model-aware.
Gate pipeline has semantic evaluation. Plans run in parallel.

**Effort**: 15-21 person-days across 5 tracks.

### Track A: Context Packs (5-7 days)

| Task | Description | Effort | Depends | Source |
|------|------------|--------|---------|--------|
| 8.01-8.10 | Context Pack data model, CLI, 5-pass funnel (10 tasks) | -- | Ph1 Track C | 08 |
| 8.11-8.20 | Pack synthesis, architecture, decomposition (10 tasks) | -- | 8.01-10 | 08 |

### Track B: Prompt Assembly Fixes (3-4 days)

| Task | Description | Effort | Depends | Source |
|------|------------|--------|---------|--------|
| 6.01-6.10 | ContextTier wiring, per-tier budgets, windowing (10 tasks) | -- | Ph1 Track C | 06 |
| 6.11-6.20 | BudgetPredictor, progressive refinement, chat path (10 tasks) | -- | 6.01-10 | 06 |
| 6.21-6.33 | VCG auction, section effectiveness, A/B testing (13 tasks) | -- | 6.11-20 | 06 |

### Track C: Gate Evolution (3-5 days)

| Task | Description | Effort | Depends | Source |
|------|------------|--------|---------|--------|
| 5.01-5.12 | roko-eval crate, typed evidence, criteria registry (12 tasks) | 62h | Ph1 Track D | 05 |
| 5.13-5.24 | LLM judge panels, express mode, wave gates (12 tasks) | 68h | 5.01-12 | 05 |
| 5.25-5.36 | Failure classification, feedback loop, dashboard (12 tasks) | 68h | 5.13-24 | 05 |
| 5.37-5.48 | Self-improving criteria, community marketplace (12 tasks) | 60h | 5.25-36 | 05 |

### Track D: Parallel Execution (2-3 days)

| Task | Description | Effort | Depends | Source |
|------|------------|--------|---------|--------|
| 2.20-2.25 | Parallel task execution, worktree isolation (6 tasks) | 24h | Ph1 Track A | 02 |
| 14.01-14.15 | Runner patterns: worktree, wave, cumulative context (15 tasks) | -- | 2.20 | 14 |

### Track E: Repo-Grounded Plan Generation (2 days)

| Task | Description | Effort | Depends | Source |
|------|------------|--------|---------|--------|
| 1.15 | Wire `build_repo_context` into plan generate | 2h | none | 01 |
| 16.16 | Wire `build_repo_context()` into plan generation | -- | none | 16 |
| 8.21-8.30 | Plan validation, grounding, splitting (10 tasks) | -- | none | 08 |

---

## Phase 3: Innovations and Learning Loops (Week 9-12)

**Goal**: System genuinely improves over time. Agent memory persists.
Multi-agent collaboration works. Self-improving gates evolve criteria.

**Effort**: 17-25 person-days across 5 tracks.

### Track A: Agent Memory and Continuity (5-7 days)

| Task | Description | Effort | Depends | Source |
|------|------------|--------|---------|--------|
| 11.01-11.13 | 3-tier memory system, agent continuity (13 tasks) | 100h | Ph1 Track A | 11 |

### Track B: Self-Improving Gates (3-5 days)

| Task | Description | Effort | Depends | Source |
|------|------------|--------|---------|--------|
| 5.37-5.48 | Gate criterion evolution, marketplace (12 tasks) | 60h | Ph2 Track C | 05 |
| 7.13-7.20 | Anomaly detection, conductor wiring (8 tasks) | 28h | varies | 07 |

### Track C: Multi-Agent Collaboration (4-6 days)

| Task | Description | Effort | Depends | Source |
|------|------------|--------|---------|--------|
| 11.14-11.30 | Composition operators, skill routing, speculative exec (17 tasks) | 140h | Ph1 Track C | 11 |

### Track D: Performance Optimization (3-4 days)

| Task | Description | Effort | Depends | Source |
|------|------------|--------|---------|--------|
| 10.01-10.45 | 14 optimizations: warm pool, express gates, caching (45 tasks) | -- | Ph1 | 10 |

### Track E: Observability (2-3 days)

| Task | Description | Effort | Depends | Source |
|------|------------|--------|---------|--------|
| 18.01-18.33 | Gateway events, regression detection, dry-run, checkpoints (33 tasks) | -- | Ph0 Track B | 18 |

---

## Phase 4: GTM, Integrations, and Ecosystem (Week 13+)

**Goal**: Roko is ready for external users. Integrations, benchmarks,
marketplace, and documentation are in place.

**Effort**: 16-23 person-days across 5 tracks.

### Track A: Integrations (5-7 days)

| Task | Description | Effort | Depends | Source |
|------|------------|--------|---------|--------|
| 13.01-13.43 | Adapter system, OTel, GitHub/Linear/Slack/Sentry (43 tasks) | -- | Ph2 | 13 |

### Track B: Benchmarking (3-5 days)

| Task | Description | Effort | Depends | Source |
|------|------------|--------|---------|--------|
| 15.01-15.38 | HAL benchmarks, SWE-bench, CI integration (38 tasks) | 134h | Ph2 | 15 |

### Track C: A2A Interop (3-4 days)

| Task | Description | Effort | Depends | Source |
|------|------------|--------|---------|--------|
| 9.20-9.40 | A2A bridge, Agent Cards, MCP federation (21 tasks) | 75h | Ph2 | 09 |

### Track D: Marketplace and Community (3-4 days)

| Task | Description | Effort | Depends | Source |
|------|------------|--------|---------|--------|
| 11.50-11.65 | Community gates, trust system, plugin registry (16 tasks) | 80h | Ph3 | 11 |
| 13.30-13.43 | Recipes, partnership integrations (14 tasks) | -- | Ph3 | 13 |

### Track E: Safety and Security Hardening (2-3 days)

| Task | Description | Effort | Depends | Source |
|------|------------|--------|---------|--------|
| 17.01-17.21 | Permissions, AgentContract, NetworkPolicy, audit (21 tasks) | -- | Ph2 | 17 |

---

## Cross-Cutting Concerns

These tasks span multiple phases and subsystems. They appear in file 19
(`19-CROSS-CUTTING.md`) and touch infrastructure shared across all components.

| Task | Description | Effort | Priority | Touches |
|------|------------|--------|----------|---------|
| 19.01 | Unify StateHub types (dual #[path] include) | 6h | P0 | serve, CLI, TUI |
| 19.02 | Converge DashboardEvent and DomainEvent | 4h | P1 | runtime, serve |
| 19.03 | Add ErrorKind coverage for missing subsystems | 2h | P7 | all crates |
| 19.04 | Replace anyhow at public crate boundaries | 8h | P1 | all crates |
| 19.05 | Replace 179 eprintln! calls with structured logging | 6h | P1 | all crates |
| 19.06 | Wire CancelToken propagation to chat/run/dispatch | 6h | P1 | CLI, agent |
| 19.07 | Wire GracefulShutdown into serve/daemon/ACP | 4h | P1 | serve, runtime |
| 19.08 | Add SIGTERM->SIGKILL escalation for stuck agents | 3h | P2 | runtime |
| 19.09 | Remove unsafe set_var for provider override | 2h | P2 | CLI |
| 19.10 | Add config unknown-key validation | 3h | P2 | core, CLI |
| 19.11-19.20 | Spec migration batches (10 tasks) | 40h | P3-P10 | all crates |
| 19.21-19.31 | Docker, API versioning, RPC errors (11 tasks) | 31h | P2-P8 | infra |

---

## Anti-Patterns Summary

**91 anti-patterns** catalogued across 10 files. Grouped by severity:

### Critical (system-level failures)

| ID | Anti-Pattern | File | Impact |
|----|-------------|------|--------|
| AP-GOD | 22K LOC god file (orchestrate.rs) | 02 | Unmaintainable, blocks all changes |
| AP-4DISP | Four separate dispatch implementations | 02 | Feature disparity, bugs |
| AP-NOROUTER | CascadeRouter has zero live callers | 03 | Router never learns from dispatch |
| AP-GODOBJ | 22K-line orchestrate.rs with dead PlanRunner | 03 | Dead code dominates codebase |
| AP-1 (04) | Stub gates that silently pass | 04 | False confidence in code quality |
| AP-6 (04) | Four separate gate dispatch paths | 04 | Inconsistent gate behavior |
| AP-BLIND | `roko chat` records zero learning signals | 07 | Chat sessions never contribute learning |
| AP-ACPBLIND | ACP records only gate thresholds | 07 | ACP dispatch is learning-blind |
| AP-DEADLOOP | Full learning loop only in dead code | 07 | System never learns |
| AP-COLD | Agents start cold every run | 11 | No memory of past sessions |
| AP-2HUB | #[path] include creates two StateHub types | 19 | Serve and CLI cannot share state |

### High (correctness or data loss)

| ID | Anti-Pattern | File | Impact |
|----|-------------|------|--------|
| AP-2SM | Two incompatible state machines | 02 | Conflicting execution semantics |
| AP-SERIAL | Serial default despite full DAG infra | 02 | Plans run 1x speed |
| AP-NOCHECK | No checkpoint for TaskScheduler state | 02 | Lost progress on crash |
| AP-4PARSE | 4 copies of stream-json parsing | 03 | Independent truncation bugs |
| AP-ENVKEY | Direct env var reads bypassing providers | 03 | No cost tracking, no rotation |
| AP-NOBUDGET (03) | BudgetCell default = unlimited spend | 03 | No budget enforcement |
| AP-BARESUBPROC | Command::new in ACP bypassing adapters | 03 | Untracked model usage |
| AP-NOHEALTH | ProviderHealthTracker never gates dispatch | 03 | Requests sent to down providers |
| AP-HARDCODE | 8 hardcoded model strings bypassing config | 03 | Config not respected |
| AP-5 (04) | Hardcoded LLM judge model | 04 | No model flexibility for judge |
| AP-7 (04) | Feedback only in orchestrate.rs | 04 | No gate feedback in live paths |
| AP-SUBPROCESS | Each gate spawns its own subprocess | 05 | Massive overhead per gate |
| AP-STUBJUDGE | StubJudgeGate always skips/fails | 05 | No semantic code review |
| AP-NOBUDGET (07) | No budget enforcement in any live path | 07 | Unbounded spend |
| AP-NOCONDUCTOR | No conductor intervention; retries hardcoded | 07 | Blind retry behavior |
| AP-IMPOVERISHED | Simplified routing context (9/18 zeroed) | 07 | CascadeRouter starved of data |
| AP-UNIFORM | Gate pipeline identical for all diffs | 11 | Trivial changes get full review |
| AP-NOCOST | No plan-level budget cap | 11 | Plans can spend unbounded |
| AP-UNBOUNDED | Knowledge query returns unbounded results | 09 | Context overflow |
| AP-ALLORNONE | Context gathered once with no refresh | 09 | Stale context across long tasks |
| AP-SERIAL (09) | Full template claims parallel but runs serial | 09 | False parallel execution |
| AP-ANYHOW | anyhow::Result at public crate boundaries | 19 | No structured error handling |
| AP-EPRINT | 179 eprintln! calls | 19 | No structured logging |
| AP-2EVENT | Two parallel event types for same occurrences | 19 | Duplicate event processing |
| AP-NOCANCEL | Chat/run/dispatch lack CancelToken | 19 | Cannot cancel operations |
| AP-NOSHUT | GracefulShutdown built but not wired | 19 | No clean shutdown |
| AP-1 (16) | auth_detect.rs ignores roko.toml providers | 16 | default_model has no effect |
| AP-2 (16) | roko init writes [[gate]] but runtime reads [gates] | 16 | Init gates silently discarded |
| AP-3 (16) | Direct env var reads bypass provider system | 16 | No cost tracking, no config |
| AP-6 (16) | CascadeRouter loaded but .observe() never called | 16 | Router never learns |
| AP-7 (16) | BudgetGuardrail never instantiated | 16 | No graduated enforcement |
| AP-BENCH-STUB | BenchmarkRegressionGate always passes | 15 | No regression detection |
| AP-NO-CONCURRENT | Learning artifacts not tested for concurrency | 15 | Data corruption risk |
| AP-NO-ROUNDTRIP | Learning artifacts not tested for persistence | 15 | Silent data loss |
| AP-NO-PARITY | Two gate paths never tested for equivalence | 15 | Silent behavior divergence |

### Medium (functionality gaps)

| ID | Anti-Pattern | File |
|----|-------------|------|
| AP-RUNG | Gate rung mapping duplicated | 02 |
| AP-AFFECT | Affect policy wired but only default used | 02 |
| AP-NOTHINK | UsageObservation missing thinking_tokens | 03 |
| AP-QUIRK | Per-provider boolean flags instead of quirks | 03 |
| AP-8 (04) | Built but unused gate features | 04 |
| AP-9 (04) | ACP runs clippy after test (wrong order) | 04 |
| AP-10 (04) | No cost tracking for LLM judge | 04 |
| AP-STRINGVERDICTS | Gate output is unstructured String | 05 |
| AP-NOEVIDENCE | Evidence produced and consumed inside gate | 05 |
| AP-NOFEEDBACK | Gate outcomes never feed back to agents | 05 |
| AP-DUAL | Dual episode writes in `roko run` | 07 |
| AP-NOANOMALY | Anomaly detector not wired | 07 |
| AP-NOHEALTH (07) | Provider health not connected to router | 07 |
| AP-NOSECTION | Section effectiveness collected but unused | 07 |
| AP-NODREAM | Dream cycle has no runtime trigger | 07 |
| AP-3TEMPLATES | Only 3 of 6 workflow templates implemented | 09 |
| AP-DUPETOKEN | estimate_tokens reimplemented 6 times | 09 |
| AP-MCPSYNC | MCP transport is synchronous only | 09 |
| AP-NOLEARN (09) | MCP tool outcomes not recorded | 09 |
| AP-ISOLATED | MCP servers cannot discover each other | 09 |
| AP-NOCARRY | Session does not track touched files | 09 |
| AP-NOLEARN (11) | force_backend not fed to CascadeRouter | 11 |
| AP-NEURO | KnowledgeStore not consulted for routing | 11 |
| AP-NODREAM (11) | Dream consolidation has no trigger | 11 |
| AP-SAMEFAM | Judge uses same model family as task agent | 11 |
| AP-VERBOSE | Tool outputs not truncated in context | 11 |
| AP-NOEXP | Experiment outcomes not fed to router | 11 |
| AP-NOESCAL | No SIGTERM->SIGKILL escalation | 19 |
| AP-NOVERSION | Zero API versioning on 85 routes | 19 |
| AP-SETVAR | unsafe set_var for provider override | 19 |
| AP-NOVALID | Config accepts unknown keys silently | 19 |
| AP-DOCKER | Single-stage Dockerfile, runs as root | 19 |
| AP-4 (16) | unsafe set_var for --provider flag | 16 |
| AP-5 (16) | ROKO_ACP_LEGACY env gate | 16 |
| AP-8 (16) | Hardcoded max_tokens differs per entry point | 16 |
| AP-COST-ZERO | BenchResult.cost_usd always 0.0 | 15 |
| AP-UNREACHABLE | unreachable!() in config dispatch | 15 |
| AP-NO-HARNESS | Test helpers are CLI-only | 15 |
| AP-SINGLE-RUN | Benchmark runs once, no consistency | 15 |

### Low (code quality)

| ID | Anti-Pattern | File |
|----|-------------|------|
| AP-DUP | GatePipeline / ComposedGatePipeline duplication | 04 |
| AP-SINGLEMODEL | LlmJudgeGate uses single oracle | 05 |
| AP-RUNGONLY | Adaptive thresholds per-rung not per-criterion | 05 |
| AP-SINGULAR | CFactorSummary is single scalar | 11 |
| AP-ENVAUTH | MCP auth via env vars only | 09 |
| AP-RPCINLINE | ACP/MCP use inline JSON-RPC error codes | 19 |

---

## Dependency Graph

```
Phase 0 (Critical Fixes) ── Week 1-2
  |
  +-- Track A: P0 Bugs .............. no deps, do first
  +-- Track B: Learning Wiring ...... no deps
  +-- Track C: Config Consistency ... no deps
  +-- Track D: Streaming UX ......... no deps
  |
  | All Phase 0 tracks are independent and parallelizable.
  |
  v
Phase 1 (Architecture Convergence) ── Week 3-4
  |
  +-- Track A: Runtime Convergence ... depends on Ph0-B (learning wired)
  +-- Track B: orchestrate.rs Decomp . depends on Ph1-A (features ported)
  +-- Track C: Dispatch Unification .. depends on Ph0-C (config consistent)
  +-- Track D: Gate Convergence ...... depends on Ph0-C (config formats)
  |
  | Track A blocks Track B. Tracks C and D are independent of A/B.
  |
  v
Phase 2 (UX Workflow + Features) ── Week 5-8
  |
  +-- Track A: Context Packs ......... depends on Ph1-C (unified dispatch)
  +-- Track B: Prompt Assembly ....... depends on Ph1-C (model resolution)
  +-- Track C: Gate Evolution ........ depends on Ph1-D (gate convergence)
  +-- Track D: Parallel Execution .... depends on Ph1-A (runtime converged)
  +-- Track E: Repo-Grounded Plans ... no deps beyond Phase 0
  |
  | Track E can start during Phase 1.
  |
  v
Phase 3 (Innovations + Learning) ── Week 9-12
  |
  +-- Track A: Agent Memory .......... depends on Ph1-A (learning wired)
  +-- Track B: Self-Improving Gates .. depends on Ph2-C (gate evolution)
  +-- Track C: Multi-Agent ........... depends on Ph1-C (dispatch unified)
  +-- Track D: Performance ........... depends on Ph1 (architecture stable)
  +-- Track E: Observability ......... depends on Ph0-B (learning exists)
  |
  | Track E can start during Phase 2.
  |
  v
Phase 4 (GTM + Ecosystem) ── Week 13+
  |
  +-- Track A: Integrations .......... depends on Ph2
  +-- Track B: Benchmarking .......... depends on Ph2
  +-- Track C: A2A Interop ........... depends on Ph2
  +-- Track D: Marketplace ........... depends on Ph3
  +-- Track E: Safety Hardening ...... depends on Ph2
  |
  All tracks are independent and parallelizable.
```

### Key Inter-File Dependencies

| Upstream | Downstream | Relationship |
|----------|-----------|-------------|
| 01 (Stability) | 02 (Orchestration) | P0 fixes before decomposition |
| 01 (Stability) | 07 (Learning) | Bug fixes before learning wiring |
| 03 (Dispatch) | 02 (Orchestration) | Unified dispatch before runtime convergence |
| 03 (Dispatch) | 06 (Prompt) | Model resolution before model-aware windowing |
| 04 (Gate Pipeline) | 05 (Gate Evolution) | Gate convergence before evolution |
| 07 (Learning) | 05 (Gate Evolution) | Learning wired before self-improving gates |
| 07 (Learning) | 11 (Innovations) | Learning infra before agent memory |
| 02 (Orchestration) | 14 (Runner) | Runtime converged before runner patterns |
| 16 (Config) | 03 (Dispatch) | Config consistency before dispatch unification |
| 16 (Config) | 04 (Gate Pipeline) | Config format before gate convergence |
| 19 (Cross-Cutting) | 12 (Code Debt) | StateHub unification before dead code removal |
| 12 (Code Debt) | 10 (Performance) | Clean code before optimization |

---

## Critical Path

The longest sequential dependency chain determines minimum calendar time:

```
Ph0 Track B (Learning Wiring)     Ph1 Track A (Runtime Convergence)
        2-3 days            --->          3-4 days
                                           |
                                           v
                                  Ph1 Track B (Decomposition)
                                         3-4 days
                                           |
                                           v
                                  Ph2 Track D (Parallel Execution)
                                         2-3 days

Total critical path: 10-14 days (Weeks 1-4)
```

Everything else can be parallelized around this chain. With 4 developers,
the theoretical minimum to Phase 2 entry is ~14 days.

### Parallelization Opportunities

| Phase | Tracks | Max Parallelism | Calendar Reduction |
|-------|--------|----------------|-------------------|
| Phase 0 | 4 independent tracks | 4-way | 6-10 days -> 2-3 days |
| Phase 1 | C+D independent of A+B | 2-way | 10-14 days -> 6-8 days |
| Phase 2 | E starts in Phase 1; A-D independent | 4-way | 15-21 days -> 5-7 days |
| Phase 3 | E starts in Phase 2; A-D independent | 4-way | 17-25 days -> 5-7 days |
| Phase 4 | All 5 tracks independent | 5-way | 16-23 days -> 4-5 days |

---

## Quick Wins

46 tasks with effort <= 2 hours and no dependencies. Sorted by priority,
then effort. Each can be done independently, in any order.

### P0 Quick Wins (8 tasks, ~11h)

| Task | Description | Effort |
|------|------------|--------|
| 1.01 | Fix `roko config mcp` unreachable panic | 1h |
| 1.02 | Move share routes inside auth middleware | 1h |
| 1.04 | Add secret scrubbing to CLI Gist share path | 1h |
| 1.05 | Fix `acknowledge_public_risk` bypass | 1h |
| 1.08 | Fix dual episode writes in `roko run` | 1h |
| 4.1 | Add complexity and prior_failures to GateConfig | 1h |
| 4.2 | Add feedback/failure_classification to GateReport | 1h |
| 1.03 | Auto-provision auth on cloud deploy | 2h |
| 1.06 | Remove `unsafe set_var` for --provider | 2h |
| 1.07 | Fix stub gate verdicts giving false PASS | 2h |
| 2.14 | Add Serialize/Deserialize to TaskStatus | 2h |
| 3.1 | Add load/save helpers for CascadeRouter | 2h |
| 4.8 | Add Verdict::skip constructor, convert stub_verdict | 2h |
| 9.1 | Consolidate estimate_tokens into roko-core | 2h |

### P1 Quick Wins (10 tasks, ~13h)

| Task | Description | Effort |
|------|------------|--------|
| 1.23 | Wire runner v2 AdaptiveThreshold observations | 1h |
| 1.24 | Wire runner v2 episode logging | 1h |
| 1.25 | Wire runner v2 section effectiveness updates | 1h |
| 1.26 | Wire runner v2 efficiency event recording | 1h |
| 1.30 | Fix ACP gate rung ordering (clippy before test) | 1h |
| 1.32 | Fix model showing "-" in TUI for runner v2 | 1h |
| 1.34 | Fix `signals.jsonl` dead path | 1h |
| 1.12 | Wire feedback recording to `roko chat` | 2h |
| 1.15 | Wire `build_repo_context` into plan generate | 2h |
| 2.8 | Export gate rung mapping from roko-gate | 2h |
| 3.7 | Extract shared truncation utility | 2h |
| 3.22 | Remove hardcoded model string from auth_detect.rs | 2h |
| 3.24 | Identify live exports from orchestrate.rs | 2h |

### P2 Quick Wins (17 tasks, ~20h)

| Task | Description | Effort |
|------|------------|--------|
| 1.37 | Export `rung_for_gate_name` from roko-gate | 0.5h |
| 1.45 | Wire domain profiles to AdaptiveThresholds | 1h |
| 1.48 | Wire regression detection alerting path | 1h |
| 1.54 | Make tool loop max iterations configurable | 1h |
| 1.57 | Wire knowledge candidate ingestion post-run | 1h |
| 1.66 | Wire VerdictPublisher to all gate dispatch paths | 1h |
| 1.68 | Wire StagingBuffer lightweight promotion | 1h |
| 1.70 | Add content-type-aware token counting ratios | 1h |
| 1.72 | Wire conversation compaction to `roko chat` | 1h |
| 4.21 | Define GateEvent enum | 1h |
| 1.36 | Normalize model aliases at load time | 2h |
| 1.58 | Fix `--share` without `--serve` producing dead URL | 2h |
| 3.17 | Add thinking_tokens to UsageObservation | 2h |

### P3 Quick Wins (4 tasks, ~8h)

| Task | Description | Effort |
|------|------------|--------|
| 3.37 | Add cache hit rate metrics to CacheCell | 2h |
| 3.38 | Add ToolLoop max iterations to ModelProfile config | 2h |
| 19.23 | Add API version header to roko serve | 2h |
| 19.27 | Improve Dockerfile with multi-stage caching | 2h |

---

## Effort Summary

### By Phase (from Master Plan)

| Phase | Scope | Effort | Calendar (1 dev) | Calendar (2 devs) |
|-------|-------|--------|-----------------|-------------------|
| Phase 0 | Critical fixes | 6-10 days | 2 weeks | 1 week |
| Phase 1 | Architecture | 10-14 days | 3 weeks | 2 weeks |
| Phase 2 | UX + Features | 15-21 days | 5 weeks | 3 weeks |
| Phase 3 | Innovations | 17-25 days | 6 weeks | 3 weeks |
| Phase 4 | GTM | 16-23 days | 6 weeks | 3 weeks |
| **Total** | | **64-93 days** | **22 weeks** | **12 weeks** |

### By File (from task-level estimates)

| File | Tasks | Known Hours | Avg/Task |
|------|-------|-------------|----------|
| 01-Stability & Fixes | 78 | 206h | 2.6h |
| 02-Orchestration | 28 | 119h | 4.2h |
| 03-Inference Dispatch | 38 | 136h | 3.6h |
| 04-Gate Pipeline | 27 | 73h | 2.7h |
| 05-Gate Evolution | 48 | 258h | 5.4h |
| 06-Prompt Assembly | 33 | (unstructured) | -- |
| 07-Learning & Feedback | 27 | 88h | 3.3h |
| 08-UX & CLI | 47 | (unstructured) | -- |
| 09-ACP & MCP | 40 | 157h | 3.9h |
| 10-Performance | 45 | (unstructured) | -- |
| 11-Innovations | 65 | 492h | 7.6h |
| 12-Code Debt | 37 | (unstructured) | -- |
| 13-GTM & Integrations | 43 | (unstructured) | -- |
| 14-Runner Patterns | 30 | (unstructured) | -- |
| 15-Testing & Verification | 38 | 134h | 3.5h |
| 16-Config & Wiring | 26 | (unstructured) | -- |
| 17-Safety & Security | 21 | (unstructured) | -- |
| 18-Observability | 33 | (unstructured) | -- |
| 19-Cross-Cutting | 31 | 116h | 3.7h |
| **Total** | **735** | **1780h** known | **4.2h** avg |

---

## Day 1 Priorities

Five fixes, ~5 hours of work, outsized impact:

| # | Fix | Time | Impact |
|---|-----|------|--------|
| 1 | Wire `ConfigCmd::Mcp` (kill crash) -- Task 1.01 | 30 min | P0 panic eliminated |
| 2 | Move share routes inside auth -- Task 1.02 | 30 min | Security hole closed |
| 3 | Fix model name "-" in TUI -- Task 1.32 | 1 hour | UX clarity |
| 4 | Add scrubbing to CLI Gist -- Task 1.04 | 1 hour | Secret leak closed |
| 5 | Forward streaming events to TUI -- Task 16.15 | 2 hours | Streaming UX |

Fixes 3 P0 issues, 1 security issue, and the most visible UX problem.
