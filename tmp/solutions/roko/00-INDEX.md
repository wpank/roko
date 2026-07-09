# Roko Solutions — Complete Specification Index

**71 documents, ~47K lines.** Single source of truth for implementation planning.

Compiled 2026-04-29. Covers: architecture, current state, subsystem audits, research synthesis,
UX vision, performance, gate evolution, GTM, and implementation plans — all grounded in source code.

---

## Core Analysis (01–06)

| # | Doc | Lines | What |
|---|-----|-------|------|
| 01 | [01-LESSONS-AND-APPROACHES.md](01-LESSONS-AND-APPROACHES.md) | ~556 | 7 solution approaches, mega-parity runner deep dive, speed/cost/reliability metrics, 16 architecture lessons, three-runtime problem matrix, innovations worth preserving |
| 02 | [02-ACP-AND-WORKFLOW-PATTERNS.md](02-ACP-AND-WORKFLOW-PATTERNS.md) | ~669 | ACP protocol spec (types.rs grounded), 16-transition state machine, workflow templates, 10 agent roles, session config, 9 limitations, convergence plan |
| 03 | [03-PROVIDER-AND-AGENT-AUDIT.md](03-PROVIDER-AND-AGENT-AUDIT.md) | ~823 | 7 provider adapters, tool dispatch pipeline, 30 built-in tools, CascadeRouter (3-stage bandit, 17-dim), multi-agent pools, config audit, 8 hardcoded model strings |
| 04 | [04-ORCHESTRATION-AND-GATES-AUDIT.md](04-ORCHESTRATION-AND-GATES-AUDIT.md) | ~653 | Three execution engines compared (orchestrate.rs vs runner v2 vs WorkflowEngine), 7-rung gate pipeline, DAG execution, 5 feedback loops, prompt assembly |
| 05 | [05-CURRENT-STATE-AND-GAPS.md](05-CURRENT-STATE-AND-GAPS.md) | ~641 | 8 anti-patterns with file paths, runner v2 gap table (13 features), config field load paths, "what done looks like" criteria, effort estimates |
| 06 | [06-IMPLEMENTATION-PLANS.md](06-IMPLEMENTATION-PLANS.md) | ~699 | 8 phased plans with per-task breakdowns, novel innovations (express gates, cumulative context, wave gates, VCG activation, LLM judge), priority matrix |

## Research & Synthesis (07–08)

| # | Doc | Lines | What |
|---|-----|-------|------|
| 07 | [07-RESEARCH-SYNTHESIS-1.md](07-RESEARCH-SYNTHESIS-1.md) | ~943 | 15 research docs → 14 themes: agent orchestration, cost optimization, evaluation, self-improvement, learning/memory, safety, event sourcing, HDC, DX, competitive landscape, collective intelligence, protocols, regulatory, marketplace |
| 08 | [08-RESEARCH-SYNTHESIS-2.md](08-RESEARCH-SYNTHESIS-2.md) | ~880 | 13 research docs → 16 themes: control plane positioning, bandit routing proofs, first-dollar markets (bioinformatics, smart-contract, FHIR), OSS playbooks, Linear protocol, Codex CLI analysis, Berlin/EU strategy, 90-day roadmap |

## Vision & Innovation (09–12)

| # | Doc | Lines | What |
|---|-----|-------|------|
| 09 | [09-UX-WORKFLOW-VISION.md](09-UX-WORKFLOW-VISION.md) | ~892 | **Context Packs** — 5-pass funnel (synthesize→architect→decompose→scope→execute), per-tier token budgets, auto-split algorithm, interactive steering, ACP integration, 6-phase implementation plan |
| 10 | [10-INNOVATIONS-AND-NEW-FEATURES.md](10-INNOVATIONS-AND-NEW-FEATURES.md) | ~1,518 | 10 innovations: agent memory, multi-agent collaboration, self-improving gates, adaptive context, speculative execution, agent debugging, cross-project learning, interactive steering, cost optimization, A2A interop. 76-day implementation estimate |
| 11 | [11-CURRENT-STATE-GROUND-TRUTH.md](11-CURRENT-STATE-GROUND-TRUTH.md) | ~618 | Source-verified audit: every CLI command (reality vs claims), 3 execution paths, gate status, learning artifacts (20+ files, what's consumed vs orphaned), 8 UX pain points, top 20 prioritized fixes |
| 12 | [12-ACP-MCP-DEEP-DIVE.md](12-ACP-MCP-DEEP-DIVE.md) | ~963 | ACP wire format + cognitive loop, 5 MCP crates, workflow pattern mapping, 11 protocol gaps, 5 novel extensions (parallel agents, MCP federation, progressive context, learning-informed MCP, A2A bridge), 6-phase plan |

## Performance (13-PERF-*)

| Doc | Lines | What |
|-----|-------|------|
| [13-PERF-BENCHMARK-RESULTS.md](13-PERF-BENCHMARK-RESULTS.md) | ~321 | Phase-by-phase timing for 3 models, connection latency by provider, inference latency (8 models), gate pipeline timing per rung, workflow template comparison, projected post-optimization |
| [13-PERF-BOTTLENECK-ANALYSIS.md](13-PERF-BOTTLENECK-ANALYSIS.md) | ~681 | 15 bottlenecks with cost/fix/code paths, 3-phase optimization projections, measurement methodology with tracing |
| [13-PERF-WARM-POOL-DESIGN.md](13-PERF-WARM-POOL-DESIGN.md) | ~650 | Three-tier pool (HOT/WARM/COLD), WarmDispatchPool implementation, connection reuse analysis, speculative execution extension |
| [13-PERF-OPTIMIZATION-PLAYBOOK.md](13-PERF-OPTIMIZATION-PLAYBOOK.md) | ~890 | **14 concrete optimizations** with code, test plans, rollback strategies. 4-week implementation timeline |
| [13-PERF-HAL-AND-AGENT-BENCHMARKS.md](13-PERF-HAL-AND-AGENT-BENCHMARKS.md) | ~835 | HAL (Princeton ICLR 2026), SWE-bench contamination crisis, FeatureBench, AgencyBench, reliability metrics, roko benchmark suite design |
| [13-PERF-HAL-BENCHMARK-INTEGRATION.md](13-PERF-HAL-BENCHMARK-INTEGRATION.md) | ~600 | HAL integration architecture, Python wrapper, roko-native benchmarking, multi-dimensional evaluation, CI integration |

## Visual Gate Evolution (14-GATE-VIZ-*)

10-document PRD set for evolving the gate pipeline into a visual evaluation system.

| Doc | Lines | What |
|-----|-------|------|
| [14-GATE-VIZ-00-System-Overview.md](14-GATE-VIZ-00-System-Overview.md) | ~494 | System overview, BridgeGateService integration, layer dependencies, design principles |
| [14-GATE-VIZ-01-Core-Abstractions.md](14-GATE-VIZ-01-Core-Abstractions.md) | ~1,831 | Criterion/Evidence/Judge traits, EvalService runtime, registry types, TOML authoring, LegacyCriterion bridge |
| [14-GATE-VIZ-02-Evidence-Collectors.md](14-GATE-VIZ-02-Evidence-Collectors.md) | ~1,582 | ProcessCollector, DiffCollector, HttpCollector + 3 novel: AstCollector (tree-sitter), SemanticDiffCollector, RuntimeTraceCollector |
| [14-GATE-VIZ-03-Criterion-Library.md](14-GATE-VIZ-03-Criterion-Library.md) | ~1,184 | Compile/test/lint/diff criteria + novel: StructuralCompleteness, Complexity, Substance, Coverage. Rung-to-criterion mapping |
| [14-GATE-VIZ-04-Judge-Methodology.md](14-GATE-VIZ-04-Judge-Methodology.md) | ~903 | LLM-as-judge patterns, CalibraEval debiasing, IPI/TOV metrics, RISE-Judge, Trust-or-Escalate, PanelJudgeOracle |
| [14-GATE-VIZ-05-Self-Improvement-Flywheel.md](14-GATE-VIZ-05-Self-Improvement-Flywheel.md) | ~995 | Gate→agent feedback circuit, RLAIF/RLSF integration, FeedbackService bridge, closed-loop model selection, cost economics |
| [14-GATE-VIZ-06-Community-Marketplace.md](14-GATE-VIZ-06-Community-Marketplace.md) | ~985 | Trust/reputation, profile inheritance, fork lineage, offline mode, governance, registry backend |
| [14-GATE-VIZ-07-Dashboard-Integration.md](14-GATE-VIZ-07-Dashboard-Integration.md) | ~803 | TUI widgets, web API endpoints, SSE events, CLI output format, unified data model |
| [14-GATE-VIZ-08-Migration-And-Orchestration.md](14-GATE-VIZ-08-Migration-And-Orchestration.md) | ~1,133 | 5-phase progressive migration, bridge adapters, orchestrator switchover, 9 PRs ~3,900 LOC |
| [14-GATE-VIZ-09-Research-Appendix.md](14-GATE-VIZ-09-Research-Appendix.md) | ~657 | 13 research topics current through April 2026, benchmark survey, bias analysis, glossary |

## UX & CLI (15-UX-*)

| Doc | Lines | What |
|-----|-------|------|
| [15-UX-AUDIT.md](15-UX-AUDIT.md) | ~468 | 4 user pain points with source evidence, workflow analysis (aggregate→funnel→execute), gap analysis |
| [15-UX-GOALS.md](15-UX-GOALS.md) | ~395 | 6 design principles, full workflow walkthrough with CLI commands, task TOML tiering (4 tiers), feature requirements P0-P3 |
| [15-UX-ISSUES.md](15-UX-ISSUES.md) | ~509 | 10 issues (4 critical): no aggregation, no funnel, no task validation, no auto-splitting. Each with fix plan |
| [15-UX-FEATURES.md](15-UX-FEATURES.md) | ~385 | 186 features inventoried (34 wired, 36 built, 5 designed, 111 not built). Smart context windowing (16), task splitting (21) |
| [15-UX-MORI-REFERENCE.md](15-UX-MORI-REFERENCE.md) | ~425 | Mori's ingestion/DAG/dispatch workflow, 5 things that didn't work, carry-forward vs improve decisions |
| [15-UX-PLAN.md](15-UX-PLAN.md) | ~584 | 7 phases (0-6), ~10,550 LOC total. Phase 0: `roko next`, run summary, dry-run. Phase 4: full aggregate+funnel |
| [15-UX-SYMPHONY-ANALYSIS.md](15-UX-SYMPHONY-ANALYSIS.md) | ~328 | Symphony comparison, 5 things to borrow, 5 things not to borrow |
| [15-UX-TRACKER-INTEGRATIONS.md](15-UX-TRACKER-INTEGRATIONS.md) | ~460 | Corpus-centric integration model, CorpusSource trait, TrackerAdapter trait, priority ranking |

## Prompt Assembly (16-PROMPT-*)

| Doc | Lines | What |
|-----|-------|------|
| [16-PROMPT-AUDIT.md](16-PROMPT-AUDIT.md) | ~437 | 9-layer builder, 9 "built but not connected" components, file inventory with LOC, "who uses what" matrix |
| [16-PROMPT-GOALS.md](16-PROMPT-GOALS.md) | ~340 | 8 core properties, model-aware windowing, progressive refinement (5-stage pipeline), measurement criteria |
| [16-PROMPT-ISSUES.md](16-PROMPT-ISSUES.md) | ~392 | 17 issues (3 critical): small models overloaded (ISS-01), BudgetPredictor never called (ISS-02), chat bypasses builder (ISS-03) |
| [16-PROMPT-PLAN.md](16-PROMPT-PLAN.md) | ~446 | 4 phases: wire ContextTier, progressive refinement, path convergence, advanced. Code snippets included |
| [16-PROMPT-CONTEXT-WINDOWING.md](16-PROMPT-CONTEXT-WINDOWING.md) | ~424 | **Model-size problem quantified**, per-tier composition strategy, section priority rankings, dynamic budget algorithm, cache alignment |
| [16-PROMPT-INNOVATIONS.md](16-PROMPT-INNOVATIONS.md) | ~538 | 10 techniques: structured prompting, adaptive CoT, hierarchical context, anti-pattern inoculation, metacognitive prompting, attention steering, context compression, multi-agent coordination, prompt versioning, A/B testing |

## Orchestration (17-ORCH-*)

| Doc | Lines | What |
|-----|-------|------|
| [17-ORCH-AUDIT.md](17-ORCH-AUDIT.md) | ~702 | 3 runtimes with LOC, import-level analysis of orchestrate.rs, state machine comparison, DAG execution, process supervision, affect modulation, 31-file inventory |
| [17-ORCH-GOALS.md](17-ORCH-GOALS.md) | ~381 | 5 primary + 4 secondary goals, 6 runner lessons, 6 novel patterns (speculative execution, adaptive parallelism, cost-aware scheduling, progressive refinement, tournament, wave gating) |
| [17-ORCH-ISSUES.md](17-ORCH-ISSUES.md) | ~497 | 21 issues (4 critical): serial default, god file, state machine mismatch, dispatch duplication |
| [17-ORCH-PLAN.md](17-ORCH-PLAN.md) | ~651 | 9 phases: parallel execution, cumulative context, feature extraction, failure recovery, speculative execution, adaptive parallelism, anti-pattern checks, cost-aware scheduling, retirement |
| [17-ORCH-PATTERNS.md](17-ORCH-PATTERNS.md) | ~550 | **Distilled patterns**: worktree isolation, wave gating (10x speedup), context handoff (4 patterns), failure recovery (4 patterns), scheduling (4 patterns), cost management, monitoring, conveyor belt pattern |

## Learning & Feedback (18-LEARN-*)

| Doc | Lines | What |
|-----|-------|------|
| [18-LEARN-AUDIT.md](18-LEARN-AUDIT.md) | ~452 | 3 crates (70+ modules), 11 components deep-dived: CascadeRouter (LinUCB 18-dim), FeedbackService, LearningRuntime (18-step pipeline), Conductor (7 actions, 19-dim), experiments, playbooks, anomaly detection, knowledge store, dream cycle |
| [18-LEARN-GOALS.md](18-LEARN-GOALS.md) | ~400 | 6 core properties, 4 novel approaches, knowledge integration, UX data feeds, measurable targets (30/90-day), 4 invariants |
| [18-LEARN-ISSUES.md](18-LEARN-ISSUES.md) | ~476 | 20 issues (3 critical): chat records nothing (I-01), ACP records only thresholds (I-02), full loop in dead code (I-03) |
| [18-LEARN-PLAN.md](18-LEARN-PLAN.md) | ~578 | 6 phases: universal feedback wiring, routing intelligence, learned intervention, knowledge integration, experiment automation, continuous optimization. 32 files, 17 test scenarios |

## Inference Dispatch (19-DISPATCH-*)

| Doc | Lines | What |
|-----|-------|------|
| [19-DISPATCH-AUDIT.md](19-DISPATCH-AUDIT.md) | ~497 | 7 backends, 15 LLM invocation paths, ModelCallService (2,143 LOC), 6-tier model selection, CascadeRouter (4-stage LinUCB), response parser duplication, anti-patterns |
| [19-DISPATCH-GOALS.md](19-DISPATCH-GOALS.md) | ~312 | One dispatch path, cascade learning, budget enforcement, provider health, multi-model racing. Provider landscape 2025-2026 (11 families) |
| [19-DISPATCH-ISSUES.md](19-DISPATCH-ISSUES.md) | ~474 | 17 issues (3 critical): CascadeRouter zero live callers (ISS-01), one-shot paths skip feedback (ISS-02), orchestrate.rs god object (ISS-03) |
| [19-DISPATCH-PLAN.md](19-DISPATCH-PLAN.md) | ~742 | 10 phases: wire CascadeRouter, episode logging, ACP migration, circuit breaker, budget enforcement, parser consolidation, env key elimination, orchestrate.rs decomposition, novel strategies, observability. 12-20 days |

## Gate Pipeline (20-GATE-*)

| Doc | Lines | What |
|-----|-------|------|
| [20-GATE-AUDIT.md](20-GATE-AUDIT.md) | ~641 | 16 gate implementations, 4 dispatch paths with feature matrix, adaptive thresholds (EMA/CUSUM/BOCPD), SPC, Hotelling, feedback classifier, composition wrappers, AP-1 through AP-10 mapping, 40-file inventory |
| [20-GATE-GOALS.md](20-GATE-GOALS.md) | ~430 | Unified dispatch, adaptive everywhere, LLM judge via CascadeRouter, custom gates from config, 6 novel gate types, process reward model, self-improving system (6 feedback loops) |
| [20-GATE-ISSUES.md](20-GATE-ISSUES.md) | ~454 | 15 issues (5 critical): 3 dispatch paths, stub verdicts passing, hardcoded judge model, feedback not available, ACP rung violation. AP mapping |
| [20-GATE-PLAN.md](20-GATE-PLAN.md) | ~856 | **8 phases**: converge dispatch, fix stubs + judge, wire adaptive intelligence, failure classification, process reward model, novel gate types, custom gates from config, gate events for UX |

## GTM & Market (21-GTM-*)

| Doc | Lines | What |
|-----|-------|------|
| [21-GTM-MOAT-ANALYSIS.md](21-GTM-MOAT-ANALYSIS.md) | varies | 5 moat layers, April 2026 market data ($6.8B), competitive comparison, gateway economics |
| [21-GTM-ADAPTER-MAP.md](21-GTM-ADAPTER-MAP.md) | varies | Subsystem generalization opportunities, competitive landscape table, 11 adapter analyses |
| [21-GTM-ADAPTER-PHILOSOPHY.md](21-GTM-ADAPTER-PHILOSOPHY.md) | varies | Adapter-first extensibility, vendor independence, 7 ecosystem flywheel patterns |
| [21-GTM-GATEWAY-ADAPTERS.md](21-GTM-GATEWAY-ADAPTERS.md) | varies | Gateway as adapter stack (8 layers), standalone market economics, OTel spans |
| [21-GTM-INTEGRATIONS.md](21-GTM-INTEGRATIONS.md) | varies | 90-day shipping sequence, 5 named chains, recipe.toml schema, Linear/Langfuse partnerships |
| [21-GTM-NEW-MARKETS.md](21-GTM-NEW-MARKETS.md) | varies | 24 integration categories, market sizing, prioritization matrix |
| [21-GTM-PITCH-INTELLIGENCE.md](21-GTM-PITCH-INTELLIGENCE.md) | varies | Investor thesis, 4-pillar differentiation, 3-tier commercial offering, VC map |
| [21-GTM-ECOSYSTEM-PATTERNS.md](21-GTM-ECOSYSTEM-PATTERNS.md) | varies | Plugin patterns from 12 platforms, lock-in thresholds, contributor funnel, MCP quality crisis |
| [21-GTM-ADVANCED-PATTERNS.md](21-GTM-ADVANCED-PATTERNS.md) | varies | 12 architectural patterns with competitive deltas and roko code examples |
| [21-GTM-SYNERGY-PATTERNS.md](21-GTM-SYNERGY-PATTERNS.md) | varies | 21 synergy patterns with April 2026 market data, composition map |

## Runner Operations (22)

| # | Doc | Lines | What |
|---|-----|-------|------|
| 22 | [22-RUNNER-LESSONS.md](22-RUNNER-LESSONS.md) | ~1,304 | Mega-parity runner: architecture, speed optimization (15-40min→1-5min), 7 failure modes with fixes, cherry-pick workflow, verification strategy, operational procedures, methodology (aggregation→funnel→execute pattern), broader patterns |

---

## Reading Order for Implementation Planning

**Quick orientation** (read first):
1. `11-CURRENT-STATE-GROUND-TRUTH.md` — what actually works today
2. `05-CURRENT-STATE-AND-GAPS.md` — the delta to close
3. `09-UX-WORKFLOW-VISION.md` — where we're going

**Per-subsystem deep dives** (read the AUDIT → ISSUES → PLAN for each):
- Orchestration: `17-ORCH-*`
- Inference dispatch: `19-DISPATCH-*`
- Gate pipeline: `20-GATE-*` + `14-GATE-VIZ-*`
- Prompt assembly: `16-PROMPT-*`
- Learning: `18-LEARN-*`
- UX/CLI: `15-UX-*`

**Cross-cutting**:
- `06-IMPLEMENTATION-PLANS.md` — prioritized task breakdowns
- `10-INNOVATIONS-AND-NEW-FEATURES.md` — forward-looking features
- `13-PERF-OPTIMIZATION-PLAYBOOK.md` — 14 concrete perf wins

**Research context** (reference as needed):
- `07-RESEARCH-SYNTHESIS-1.md`, `08-RESEARCH-SYNTHESIS-2.md`
- `13-PERF-HAL-AND-AGENT-BENCHMARKS.md`

## Key Numbers

| Metric | Value |
|--------|-------|
| Documents | 71 |
| Total lines | ~47,000 |
| Subsystems audited | 7 (orchestration, dispatch, gates, prompt, learning, UX, GTM) |
| Issues catalogued | ~130+ across all audits |
| Implementation tasks | ~200+ across all plans |
| Research papers referenced | 50+ |
| Novel innovations proposed | 10 major + dozens minor |
| Competitive products analyzed | 8 (Cursor, Codex CLI, Devin, Claude Code, Windsurf, Replit, Cody, Continue) |
