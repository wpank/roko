# Data Flow Diagrams

> **Verified against codebase: 2026-09-15.**
> ASCII diagrams showing how data moves through the system for key scenarios.
> Each diagram is annotated with the responsible crate, key types, and
> the file where the logic lives.

---

## Table of Contents

1. [Plan Execution (end-to-end)](#1-plan-execution-end-to-end)
2. [Single Task Dispatch (the core loop)](#2-single-task-dispatch-the-core-loop)
3. [Agent Dispatch Detail](#3-agent-dispatch-detail)
4. [Gate Validation Pipeline](#4-gate-validation-pipeline)
5. [Knowledge Query Flow](#5-knowledge-query-flow)
6. [Dream Consolidation Cycle](#6-dream-consolidation-cycle)
7. [Cascade Model Routing](#7-cascade-model-routing)
8. [HTTP Control Plane Request Flow](#8-http-control-plane-request-flow)
9. [State Persistence Layout](#9-state-persistence-layout)
10. [Signal Lifecycle](#10-signal-lifecycle)
11. [Prompt Assembly Detail](#11-prompt-assembly-detail)
12. [Affect State Computation](#12-affect-state-computation)

---

## 1. Plan Execution (end-to-end)

The complete flow when `roko plan run plans/` executes a multi-task plan.

```
                          roko plan run plans/
                                 |
                                 v
                    +------------------------+
                    |  Plan Loader            |  crates/roko-cli/src/runner/plan_loader.rs
                    |  Parse tasks.toml,      |
                    |  build task DAG         |  Output: Vec<Task> with dependency edges
                    +------------+-----------+
                                 |
                                 v
                    +------------------------+
                    | plan_to_graph()         |  crates/roko-graph/src/convert.rs
                    | Convert task DAG to     |
                    | Graph of Cells          |  Output: Graph<Cell> with typed edges
                    +------------+-----------+
                                 |
                                 v
                    +------------------------+
                    | GraphEngine.execute()   |  crates/roko-graph/src/engine.rs
                    | Topological sort        |  crates/roko-graph/src/topo.rs
                    | -> wave scheduling      |  Independent tasks grouped into waves
                    +------------+-----------+
                                 |
                    +------------+-----------+
                    |  For each wave:         |  Waves execute sequentially
                    |  parallel Cell dispatch  |  Cells within a wave run concurrently
                    +------+-----+-----------+
                           |     |
              +------------+     +------------+
              v                               v
     +-----------------+            +-----------------+
     | Task Cell A     |            | Task Cell B     |
     | (independent)   |            | (independent)   |
     +--------+--------+            +--------+--------+
              |                               |
              v                               v
     +--------+--------+            +--------+--------+
     | Per-task loop:   |            | Per-task loop:   |
     | ROUTE -> COMPOSE |            | (same steps)    |
     | -> MODULATE      |            |                 |
     | -> ACT -> VERIFY |            |                 |
     | -> PERSIST       |            |                 |
     | -> LEARN -> REACT|            |                 |
     +--------+--------+            +--------+--------+
              |                               |
              v                               v
     +--------+--------+            +--------+--------+
     | Checkpoint       |            | Checkpoint       |
     | .roko/state/     |            | .roko/state/     |
     | graph/<run-id>/  |            | graph/<run-id>/  |
     +--------+--------+            +--------+--------+
              |                               |
              +------------+--+---------------+
                           |
                           v
                    +------+------+
                    | Budget check |  crates/roko-graph/src/budget.rs
                    | USD limit?   |  Atomic reservation per task
                    +------+------+
                           |
                    +------+------+
                    | Next wave    |  Dependencies from completed
                    | (depends on  |  tasks now unblock
                    | completed)   |
                    +------+------+
                           |
                           v
                    +------+------+  (repeat until all waves done)
                    | Plan done    |
                    | Final state  |
                    | persisted    |  .roko/state/graph/<run-id>/final.json
                    +-------------+

Resume path (on crash or interruption):

  roko plan run plans/ --resume-plan
         |
         v
  +------+--------+
  | Snapshot Load  |  crates/roko-graph/src/snapshot.rs
  | Read last      |  Load completed task set from checkpoint
  | checkpoint     |  Graph fingerprint matching for exact resume
  +------+--------+
         |
         v
  Skip completed tasks, resume from next wave
```

---

## 2. Single Task Dispatch (the core loop)

What happens inside each task Cell. This is the most important data flow in the
system. Every arrow is a function call with real code behind it.

```
  Task definition (from plan's tasks.toml)
         |
         v
  +------+------+  Step 1: ROUTE
  | CascadeRouter|  crates/roko-learn/src/cascade_router.rs
  | Select model  |
  | based on:     |  Three stages based on observation count:
  |  - task role  |    Static (< 50 obs): config table lookup
  |  - complexity |    Confidence (50-200): pass rate + CI
  |  - history    |    UCB (200+): LinUCB contextual bandit
  |  - cost       |
  |  - health     |  Filter unhealthy providers from health registry
  +------+--------+
         |  Output: CascadeModel { primary, fallbacks, overflow_fallback, latency_sla }
         v
  +------+--------+  Step 2: COMPOSE
  | SystemPrompt   |  crates/roko-compose/src/system_prompt_builder.rs
  | Builder        |  Assemble 9-layer prompt under token budget:
  |                |
  |  L1: Role      |    "You are a senior Rust engineer..."
  |  L2: Convent.  |    "Use snake_case, thiserror, tokio..."
  |  L3: Domain    |    Crate map, project structure, active context
  |  L3c: Signals  |    Active pheromone/stigmergic guidance signals
  |  L4: Task      |    "Implement the rate limiter in crates/roko-serve/..."
  |  L4b: Feedback |    Gate errors from prior attempts (retry context)
  |  L5: Tools     |    "Available: Read, Write, Bash, Search..."
  |  L6: Playbooks |    Successful when/then patterns from PlaybookStore
  |  L7: Anti-pat  |    "Never call .unwrap() in library crates"
  |  L8: Affect    |    "Recent failures detected -- exercise extra caution"
  |                |
  |  Cache tiers:  |    L1-L2: stable (cacheable prefix)
  |                |    L3-L5: semi-stable (session-level cache)
  |                |    L6-L8: volatile (per-task, per-retry)
  +------+---------+
         |  Output: assembled system prompt (String, token-budgeted)
         v
  +------+---------+  Step 3: MODULATE
  | DaimonState     |  crates/roko-daimon/src/lib.rs
  | Compute         |
  | DispatchModul.  |  Read PAD state (pleasure/arousal/dominance)
  |                 |  Three timescales:
  |  Fast (emotion) |    Immediate reaction (minutes decay)
  |  Med (mood)     |    Running average (hours decay)
  |  Slow (temper.) |    Long-term tendency (days decay)
  |                 |
  |  Outputs:       |  temperature_delta: +/- adjustment
  |                 |  max_turns: tool loop iteration cap
  |                 |  exploration_rate: cascade exploration multiplier
  +------+---------+
         |  Output: DispatchModulation { temperature_delta, max_turns, exploration_rate }
         v
  +------+---------+  Step 4: ACT
  | AgentDispatcher |  crates/roko-agent/src/dispatcher/mod.rs
  | Send prompt to  |
  | LLM provider    |  12 backends: AnthropicApi, ClaudeCli, CodexCli,
  | Run tool loop   |    OpenAiCompat, CursorAcp, CursorCli, PerplexityApi,
  |  - read files   |    GeminiApi, GeminiCli, CerebrasApi, Hermes, OpenClaw
  |  - write code   |
  |  - run commands |  Safety layer enforces tool policies per role
  |                 |  AgentContract: allowlists intersect, denials win
  |                 |  Immune boundary screens all tool results (5-stage)
  +------+---------+
         |  Output: agent output (code changes, signals, tool results)
         v
  +------+---------+  Step 5: VERIFY
  | GatePipeline    |  crates/roko-gate/src/gate_pipeline.rs
  | 7-rung pipeline:|  crates/roko-cli/src/runner/gate_dispatch.rs
  |  R0: Compile    |  cargo check / tsc / go build
  |  R1: Lint       |  cargo clippy -D warnings / eslint
  |  R2: Test       |  cargo test --workspace
  |  R3: Symbol     |  Public API breakage check (diff analysis)
  |  R4: GenTest    |  Agent-written behavioral tests + VerifyChainGate
  |  R5: PropTest   |  proptest/quickcheck + FactCheckGate
  |  R6: Integration|  Full integration scenarios + LlmJudgeGate
  | Adaptive EMA    |  Thresholds auto-adjust per rung
  +------+---------+
         |  Output: Vec<Verdict> { passed, gate_name, details, score }
         v
  +------+---------+  Step 6: PERSIST
  | EpisodeLogger   |  crates/roko-learn/src/episode_logger.rs
  | episodes.jsonl  |  Record: task, model, cost, tokens, duration, verdicts
  |                 |
  | engrams.jsonl   |  crates/roko-fs/src/file_substrate.rs
  |                 |  Store agent output Signal in substrate
  |                 |
  | state/graph/    |  crates/roko-graph/src/snapshot.rs
  |                 |  Checkpoint execution state for resume
  +------+---------+
         |
         v
  +------+---------+  Step 7: LEARN
  | CascadeRouter   |  .feedback() updates model arm reward
  | DaimonState     |  .on_outcome() updates PAD affect state
  | KnowledgeStore  |  Consider creating knowledge entry (roko-neuro)
  | GateThresholds  |  Update adaptive EMA per rung
  | Efficiency      |  Write to .roko/learn/efficiency.jsonl
  +------+---------+
         |
         v
  +------+---------+  Step 8: REACT (on failure)
  | GateFailureRepl |  crates/roko-cli/src/runner/gate_dispatch.rs
  |                 |  Build revision plan from failure details
  | CascadeRouter   |  Record escalation outcome (if model was changed)
  | Conductor       |  crates/roko-conductor/src/lib.rs
  |  12 watchers:   |  Check for anomalies:
  |  - stuck agent  |    budget exhaustion, quality degradation,
  |  - circuit break|    latency spike, cost anomaly, stuck loops
  +------+---------+
```

---

## 3. Agent Dispatch Detail

The internal mechanics of Step 4 (ACT), showing the provider selection and tool loop.

```
  System prompt + user message + DispatchModulation
         |
         v
  +------+---------+
  | Provider Select |  Choose backend from CascadeModel
  | (roko-agent/    |  Try primary first, then fallback chain on failure
  |  dispatcher)    |  Context overflow? Use overflow_fallback model
  +------+---------+
         |
         v
  +------+---------+
  | SafetyLayer     |  crates/roko-agent/src/safety/
  | Pre-dispatch    |  Checks before any LLM call:
  | checks:         |   - Role authorization (does this role permit this op?)
  |                 |   - Tool allowlist intersection (AgentContract)
  |                 |   - Budget guard (enough budget remaining?)
  |                 |   - Sandbox policy (5 levels: None/Minimal/Standard/Strict/Air-gapped)
  |                 |   - Workspace root enforcement (no escaping project dir)
  +------+---------+
         |  (reject if any check fails -- fail closed)
         v
  +------+---------+
  | API Request     |  HTTP to Anthropic/OpenAI/Gemini/Cerebras
  | (provider       |  Or subprocess for CLI-based providers (Claude CLI, Codex CLI, etc.)
  |  adapter)       |  Includes MCP tool definitions in the request
  +------+---------+
         |
         v
  +------+---------+  TOOL LOOP (may iterate many times)
  | LLM Response    |<---------------------------------+
  |  text + tool    |                                  |
  |  calls          |                                  |
  +------+---------+                                   |
         |                                             |
    +----+----+                                        |
    |         |                                        |
    v         v                                        |
  text    tool_use                                     |
  only    request                                      |
    |         |                                        |
    |    +----v--------+                               |
    |    | ToolDispatch |  Route to handler:            |
    |    | (roko-agent) |   Read, Write, Bash, Search,  |
    |    |              |   WebFetch, MCP tools, etc.    |
    |    +----+---------+                               |
    |         |                                        |
    |    +----v--------+                               |
    |    | SafetyLayer  |  Per-tool enforcement:        |
    |    | per-tool     |   - Workspace-rooted paths    |
    |    | checks       |   - Cooldown between writes   |
    |    |              |   - Isolation (no cross-task)  |
    |    |              |   - Tool-specific denials      |
    |    +----+---------+                               |
    |         |                                        |
    |    +----v--------+                               |
    |    | Execute tool |  Run the actual operation:    |
    |    |              |   File read -> return content  |
    |    |              |   File write -> apply diff     |
    |    |              |   Shell cmd -> run & capture   |
    |    +----+---------+                               |
    |         |                                        |
    |    +----v--------+                               |
    |    | Immune Graph |  crates/roko-agent/src/immune_boundary.rs
    |    | (5-stage)    |  Screen tool result:          |
    |    |              |   1. Taint classification     |
    |    |              |      (is this from untrusted source?)
    |    |              |   2. Policy evaluation        |
    |    |              |      (does policy permit this?)
    |    |              |   3. Quarantine check         |
    |    |              |      (flagged for review?)    |
    |    |              |   4. Evidence recording       |
    |    |              |      (log for audit trail)    |
    |    |              |   5. Attestation              |
    |    |              |      (cryptographic proof)    |
    |    +----+---------+                               |
    |         |  Screened tool result                   |
    |         +-------------------------------------->+
    |                   (back to LLM with result)
    |
    v
  Final text output (agent done, tool loop exhausted or max_turns hit)
         |
         v
  Return to caller -> Step 5: VERIFY (gate pipeline)
```

---

## 4. Gate Validation Pipeline

How the 7-rung gate pipeline evaluates agent output. Cheap gates run first.

```
  Agent output (code changes in worktree)
         |
         v
  +------+--------+
  | Rung Selector  |  crates/roko-gate/src/rung_selector.rs
  | Select rungs   |  Based on:
  |                |   - Plan complexity band (simple/medium/complex)
  |                |   - Task type (impl, test, refactor, docs)
  |                |   - Config overrides ([gates] max_rung)
  |                |   - Cost budget (skip expensive rungs if low)
  +------+--------+
         |
         v
  Rung 0: COMPILE -----> cargo check / tsc / go build
         |                compile_errors.rs parses diagnostics
         | pass?           |
         |            fail: Verdict { passed: false,
         v                   gate: "compile", details: error_msg }
  Rung 1: LINT ---------> cargo clippy -D warnings
         |                clippy_gate.rs
         | pass?           |
         |            fail: collect warnings + errors
         v                 |
  Rung 2: TEST ---------> cargo test --workspace
         |                test_gate.rs
         | pass?           |
         |            fail: capture test output, identify failing tests
         v                 |
  Rung 3: SYMBOL -------> diff public API surface
         |                symbol_gate.rs
         |                Check for breaking changes in pub items
         | pass?           |
         v                 |
  Rung 4: GEN-TEST -----> run agent-written behavioral tests
         |                generated_test_gate.rs + verify_chain_gate.rs
         | pass?           |
         v                 |
  Rung 5: PROP-TEST ----> proptest / quickcheck
         |                property_test_gate.rs + fact_check.rs
         | pass?           |
         v                 |
  Rung 6: INTEGRATION --> full integration scenarios
         |                integration_gate.rs + llm_judge_gate.rs
         |                (LLM evaluates code quality -- most expensive gate)
         | pass?           |
         v                 v
  +------+--------+  +------+--------+
  | All passed     |  | Any failure   |
  | -> accept      |  | -> collect    |
  | Signal         |  | all verdicts  |
  +------+--------+  | -> feed back  |
         |           | to replan     |
         v           +------+--------+
  Proceed to                |
  PERSIST step              v
                      +-----+--------+
                      | Replan Logic  |  gate_dispatch.rs
                      |               |  build_gate_failure_plan_revision()
                      | Include:      |
                      |  - failed gate|  Which rung failed?
                      |  - error msg  |  What was the error?
                      |  - attempt #  |  How many retries so far?
                      |  - prior diff |  What did the agent change?
                      +------+--------+
                             |
                             v
                      Feed failure context into
                      next attempt (L4b: Feedback layer)


  Adaptive thresholds (EMA):
  +--------------------------------------------+
  | After each rung verdict:                    |
  |   threshold[rung] =                         |
  |     alpha * current_pass_rate +             |
  |     (1 - alpha) * threshold[rung]           |
  |                                             |
  |   alpha (learning rate) is configurable     |
  |   under [learning] in roko.toml             |
  |                                             |
  | When pass_rate > threshold:                 |
  |   Standards tighten (expect more passes)    |
  |                                             |
  | When pass_rate < threshold:                 |
  |   Standards relax (avoid infinite retries)  |
  |                                             |
  | Persisted to:                               |
  |   .roko/learn/gate-thresholds.json          |
  |                                             |
  | Inspectable via:                            |
  |   roko learn gates                          |
  +--------------------------------------------+

  The 19 gate types (13 rung-dispatched + 6 standalone):

  Rung-dispatched:
    R0: CompileGate
    R1: ClippyGate
    R2: TestGate
    R3: SymbolGate
    R4: GeneratedTestGate, VerifyChainGate
    R5: PropertyTestGate, FactCheckGate
    R6: IntegrationGate, LlmJudgeGate

  Standalone (invoked outside the rung pipeline):
    DiffGate          -- post-task diff analysis
    CodeExecutionGate -- sandboxed code execution
    ShellGate         -- arbitrary shell command verification
    BenchmarkRegressionGate -- performance benchmarks
    FormatCheckGate   -- code formatting (cargo fmt)
    SecurityScanGate  -- security scanning
```

---

## 5. Knowledge Query Flow

How `roko knowledge query "topic"` retrieves and ranks knowledge.

```
  roko knowledge query "rate limiting patterns"
         |
         v
  +------+--------+
  | Query Parser   |  crates/roko-neuro/src/context.rs
  | (roko-neuro)   |  Parse natural language query
  |                |  Extract: topic keywords, time range, entry type filter
  +------+--------+
         |
         v
  +------+--------+  Two-path retrieval:
  | KnowledgeStore |  crates/roko-neuro/src/lib.rs
  +------+---+----+
         |   |
    +----+   +----+
    v              v
  TEXT MATCH    HDC MATCH
  (keyword/     (semantic           crates/roko-neuro/src/hdc.rs
   regex)        similarity)
    |              |
    v              v
  +--------+  +--------+
  | Filter  |  | Filter  |
  | by type |  | by cos  |        Entry types: Fact, Insight, Heuristic,
  | by age  |  | distance|        Procedure, Constraint, Anti-Knowledge
  | by tier |  | < radius|
  +----+----+  +----+----+        Tiers: Transient, Working, Reference
       |            |
       +-----+------+
             |
             v
  +----------+---------+
  | Merge & Rank        |  crates/roko-neuro/src/context.rs (ContextAssembler)
  | (ContextAssembler)  |
  |  - Apply decay      |  weight = base_score * 2^(-age / half_life)
  |  - Score dimensions  |  relevance, recency, confidence
  |  - Tier priority     |  Reference > Working > Transient
  |  - Deduplicate       |  Content hash collision removal
  +----------+---------+
             |
             v
  +----------+---------+
  | Format Results      |
  |  - Entry type       |  "Fact: Rate limiting is best applied at..."
  |  - Confidence       |  0.87
  |  - Age / half-life  |  "Created 12 days ago, 90-day half-life"
  |  - Source lineage   |  "From episode E-347 (task: api-middleware)"
  |  - Tier             |  "Working"
  +----------+---------+
             |
             v
  Display to user (CLI output)
     OR
  Inject into system prompt Layer 8 (during task dispatch)
```

### Knowledge Entry Lifecycle

```
  New information source:
    - Agent discovery (code patterns found during execution)
    - Gate result (verified facts from compile/test outcomes)
    - User input (roko knowledge import, roko note)
    - Dream distillation (offline consolidation)
         |
         v
  +------+--------+
  | Admission      |  crates/roko-neuro/src/admission.rs
  | Control        |
  |  - Confidence  |  Does this meet the admission threshold?
  |    threshold?  |
  |  - Novel?      |  Is it a near-duplicate by HDC similarity?
  |  - Relevant?   |  Does it relate to the current project domain?
  +------+--------+
         |  (reject if duplicate or below threshold)
         v
  Tier: TRANSIENT  -- may be pruned aggressively (minutes to hours)
         |
         | gate-backed confirmation + context evidence
         | (a gate result validates this fact)
         v
  Tier: WORKING    -- retained during active task scope
         |            crates/roko-neuro/src/tier_progression.rs
         | multiple confirmations, sufficient age, sufficient accesses
         v
  Tier: REFERENCE  -- permanent, feeds future prompts

  Decay curves by entry type:
    Facts:       90-day half-life  (verified truths persist)
    Insights:    30-day half-life  (interpretations fade faster)
    Heuristics:  90-day half-life  (behavioral rules are durable)
    Procedures:  90-day half-life  (recipes are durable)
    Constraints: 90-day half-life  (invariants persist)
    Anti-knowledge: 30-day half-life (proven-false things fade)
```

---

## 6. Dream Consolidation Cycle

Offline learning that runs between work sessions via `roko knowledge dream run`.
This is how the system gets better over time: successful patterns are extracted,
stored, and reused.

```
  roko knowledge dream run
         |
         v
  +------+--------+
  | DreamRunner    |  crates/roko-dreams/src/runner.rs
  | Load recent    |  Read .roko/episodes.jsonl
  | episodes       |  Filter: only completed episodes since last dream
  +------+--------+
         |
         v
  +------+--------+  Phase 1: STAGING
  | StagingBuffer  |  crates/roko-dreams/src/staging.rs
  | Group by task  |  Buffer episodes for batch processing
  | shape          |  Group by task shape / domain
  |                |  Confidence staging: accumulate before processing
  +------+--------+
         |
         v
  +------+--------+  Phase 2: HYPNAGOGIA
  | HypnagogiaEng. |  crates/roko-dreams/src/hypnagogia.rs
  | Loosened assoc. |  Discovery of non-obvious connections:
  | discovery       |
  |  ThalamicGate  |    Filter noise (low-confidence episodes)
  |  ExecutiveLoop |    Relax constraints (consider wider matches)
  |  HomuncularObs |    Spot patterns (recurring shapes/outcomes)
  |  DaliInterrupt |    Surprise-driven attention (unexpected results)
  +------+--------+
         |
         v
  +------+--------+  Phase 3: IMAGINATION
  | Counterfactual |  crates/roko-dreams/src/imagination.rs
  | Analysis       |
  |                |  "What if we had used a different model?"
  |  CausalModel   |  "What if this anti-pattern was caught earlier?"
  |  -> synthesize |  "What if the task was decomposed differently?"
  |  _hypotheses() |
  +------+--------+
         |
         v
  +------+--------+  Phase 4: REHEARSAL
  | ThreatReplay   |  crates/roko-dreams/src/rehearsal.rs
  | (roko-dreams)  |  crates/roko-dreams/src/threat.rs
  |                |
  | enumerate      |  Replay failure scenarios:
  |  _threats()    |    - What failures could recur?
  | rehearse       |    - How would we detect them earlier?
  |  _threats()    |    - What warning entries should we create?
  | -> threat      |
  |  _warnings     |  Output: KnowledgeEntry warnings for future prompts
  +------+--------+
         |
         v
  +------+--------+  Phase 5: DISTILLATION
  | Knowledge      |  crates/roko-neuro/src/distiller.rs
  | Distillation   |  Extract durable entries from episode clusters:
  |                |
  |  Facts         |    Verified true statements ("this API returns 404
  |                |     when the resource does not exist")
  |  Insights      |    Interpretive observations ("smaller models
  |                |     struggle with async error handling")
  |  Heuristics    |    Behavioral rules ("always check the return type
  |                |     before adding error handling")
  |  Procedures    |    Step-by-step recipes ("to add a new gate:
  |                |     1. create gate struct, 2. impl Verify, 3. register")
  |  Constraints   |    Invariants ("roko-core must have no deps on agent")
  |  Anti-knowledge|    Proven-false ("wrapping in Arc<Mutex> does NOT
  |                |     fix the Send bound error for this type")
  +------+--------+
         |
         v
  +------+--------+  Phase 6: PLAYBOOK PROMOTION
  | PlaybookStore  |  crates/roko-learn/src/ (playbook modules)
  | (roko-learn)   |  Promote reliable success patterns:
  |                |
  |  When: task    |    "When the task involves adding an HTTP endpoint..."
  |  matches shape |
  |  Then: use     |    "Then: 1. add route, 2. add handler, 3. add test,
  |  this approach |     4. register in router"
  |  Confidence:   |    0.92 (based on 8/9 successful uses)
  +------+--------+
         |
         v
  +------+--------+  Phase 7: ROUTING ADVICE
  | DreamRouting   |  crates/roko-dreams/src/routing_advice.rs
  | Advice         |  Generate model routing recommendations:
  |                |
  |  Pattern       |    "For trait implementation tasks, claude-sonnet
  |  summaries     |     outperforms gpt-4o by 23%"
  |  Tier bias     |    "Reduce T2 usage for documentation tasks"
  |  adjustments   |
  |                |  Save to .roko/learn/dream-routing.json
  +------+--------+
         |
         v
  DreamCycleReport {
    episodes_processed: 47,
    knowledge_entries_created: 12,
    playbooks_promoted: 3,
    routing_advice_generated: true,
    threats_identified: 5,
  }
```

---

## 7. Cascade Model Routing

How the CascadeRouter selects which LLM model to use for a task.

```
  Task { role, complexity, domain, prior_attempts }
         |
         v
  +------+--------+  Determine stage based on observation count
  | Observation    |  crates/roko-learn/src/cascade_router.rs
  | Count Check    |
  +------+--------+
         |
    +----+----+----+
    v         v    v
  STATIC   CONFID. UCB
  (< 50    (50-   (200+
   obs)     200)    obs)
    |         |      |
    v         v      v
  +------+ +------+ +----------+
  |Config| |Pass  | |LinUCB    |  crates/roko-learn/src/model_router.rs
  |table:| |rate +| |contextual|
  |role->| |confid| |bandit:   |
  |model | |inter-| | features:|
  |      | |val   | | - role   |
  |      | |      | | - compl. |
  |      | |      | | - domain |
  |      | |      | | - cost   |
  |      | |      | | - tokens |
  +--+---+ +--+---+ +----+-----+
     |        |           |
     +--------+-----------+
              |
              v
  +------+--------+  Apply modifiers (in order):
  |                |
  | 1. Provider    |  Filter out unhealthy providers
  |    Health      |  (from persisted health registry)
  |                |
  | 2. Affect      |  DaimonState -> temperament_tier_shift
  |    Shift       |  After failures: bias toward more capable models
  |                |  After successes: allow cheaper models
  |                |
  | 3. Cost        |  Budget remaining -> cost_pressure_factor
  |    Pressure    |  Low budget: bias toward cheaper models
  |                |
  | 4. Knowledge   |  Neuro store -> knowledge_routing_advice
  |    Hint        |  Dream-generated model recommendations
  |                |
  | 5. Hysteresis  |  Avoid flip-flopping between models
  |                |  on consecutive tasks (stability band)
  |                |
  | 6. Context     |  If prompt > primary model's context window,
  |    Overflow    |  switch to overflow_fallback model
  +------+--------+
              |
              v
  CascadeModel {
    primary: "claude-sonnet-4-20250514",
    fallbacks: ["gpt-4o", "gemini-2.5-pro"],
    overflow_fallback: Some("gemini-2.5-pro"),
    latency_sla_ms: 30_000,
  }
```

---

## 8. HTTP Control Plane Request Flow

How an API request flows through `roko serve`.

```
  HTTP request (e.g., POST /api/plans/execute)
         |
         v
  +------+--------+
  | Axum Router    |  crates/roko-serve/src/routes/
  | (roko-serve)   |  ~376 canonical routes organized by subsystem:
  |                |   /health, /api/metrics, /api/plans, /api/prd,
  |                |   /api/agents, /api/knowledge, /api/learn,
  |                |   /api/config, /api/events, /api/feeds, etc.
  +------+--------+
         |
         v
  +------+--------+
  | Auth Middleware |  Serve auth middleware:
  |                |   - API key validation (Authorization header)
  |                |   - JWT verification (bearer tokens)
  |                |   - Worker callback auth (deployment-scoped tokens)
  |                |   - Optional: skip for health/metrics routes
  +------+--------+
         |
         v
  +------+--------+
  | Rate Limiting  |  Tower middleware layer
  | CORS           |  Configurable limits per route group
  | Compression    |  gzip/brotli for large responses
  +------+--------+
         |
         v
  +------+--------+
  | Route Handler  |  Business logic per endpoint:
  |                |   - Read/write state via StateHub (watch::Sender)
  |                |   - Dispatch plan execution (via RuntimeServices)
  |                |   - Query knowledge store (via KnowledgeStore)
  |                |   - Manage agents (via AgentDispatcher)
  |                |   - Configure system (via Config)
  +------+--------+
         |
    +----+----+
    v         v
  Sync      Async
  response  events
    |         |
    v         v
  JSON     SSE stream (/api/events/*)
  body     or WebSocket (/ws/*)
  (200,       |
   400,       v
   404,    DashboardEvent pushed via
   500)    StateHub watch::Sender -> TUI/SSE/WS consumers
```

---

## 9. State Persistence Layout

Where data lives on disk. All state is under `.roko/` in the project workspace.
No global database, no external server dependency.

```
  .roko/                            (gitignored -- local state only)
    |
    +-- engrams.jsonl              Signal log (append-only JSONL)
    |                              Every Signal ever created
    |                              Content-addressed, immutable lines
    |
    +-- episodes.jsonl             Episode log (agent turn records)
    |                              One record per task execution:
    |                              task, model, cost, tokens, duration,
    |                              gate verdicts, HDC fingerprint
    |
    +-- roko.toml                  Per-workspace config overrides
    +-- GAPS.md                    Canonical gap tracker
    |
    +-- state/
    |     +-- graph/               Graph engine checkpoints
    |     |     +-- <run-id>/      Per-plan-run state
    |     |           +-- wave-N.json     Completed wave state
    |     |           +-- final.json      Final plan state
    |     +-- state-snapshot.json  Legacy Runner-v2 snapshot (deprecated)
    |
    +-- learn/
    |     +-- cascade-router.json  Model routing state
    |     |                        Arm statistics, pass rates per model,
    |     |                        observation count, stage thresholds
    |     +-- gate-thresholds.json Adaptive gate thresholds
    |     |                        EMA per rung, learning rate (alpha)
    |     +-- efficiency.jsonl     Per-turn efficiency events
    |     |                        Cost, latency, tokens, model per turn
    |     +-- experiments/         A/B experiment assignments + results
    |     +-- dream-routing.json   Dream-generated routing advice
    |
    +-- knowledge/
    |     +-- entries/             Durable knowledge entries (by type)
    |     |     +-- facts/
    |     |     +-- insights/
    |     |     +-- heuristics/
    |     |     +-- procedures/
    |     |     +-- constraints/
    |     |     +-- anti-knowledge/
    |     +-- hdc/                 HDC fingerprint index
    |
    +-- prd/
    |     +-- ideas/               Raw work item ideas
    |     +-- drafts/              PRD drafts (markdown)
    |     +-- published/           Published PRDs (ready for planning)
    |
    +-- research/                  Research artifacts and citations
    |
    +-- archive/                   Cold storage for aged-out signals
```

---

## 10. Signal Lifecycle

How a Signal is born, lives, and (optionally) dies.

```
  Creation
    |
    v
  Signal::builder(Kind::AgentOutput)
    .body(Body::text("...code..."))
    .tag("task", "rate-limiter")
    .decay(Decay::HalfLife { half_life_ms: 7_776_000_000 })  // 90 days
    .build()
    |
    |  id = BLAKE3(kind | body | author | taint | lineage | tags)
    |  status = SignalStatus::Transient
    |  balance = 1.0
    v
  Store.put(signal) -> ContentHash
    |  Idempotent: same content = same hash = no duplicate
    |  Written to .roko/engrams.jsonl
    v
  Active life
    |  Queried via Store.query() or Store.query_similar() (HDC)
    |  Scored via Score.score(signal, ctx)
    |  Composed into prompts via Compose.compose(signals, budget, ...)
    |  Verified via Verify.verify(signal, ctx)
    |
    |  On each access: access_count += 1
    |  balance decays over time: balance *= 2^(-dt / half_life)
    |  balance refreshed on access (demurrage + reinforcement)
    |
    |  Tier progression (monotonic, never backward):
    |    Transient -> Working (gate confirmation + evidence)
    |    Working -> Consolidated (multiple confirmations + age)
    |    Consolidated -> Persistent (permanent, never auto-pruned)
    v
  Pruning (for low-tier signals)
    |  Store.prune(threshold, ctx)
    |  Signals with effective weight < threshold are removed
    |  Only Transient and Working tiers are prunable
    v
  Archival (for aged signals worth keeping)
    |  ColdStore.archive(signal)
    |  Signal moved from hot substrate to compressed archive
    |  ColdStore.thaw(id) to retrieve later
    v
  End of life
       Signal either persists in cold archive or is pruned
```

---

## 11. Prompt Assembly Detail

How the SystemPromptBuilder assembles the 9 layers into a single prompt.

```
  RoleSystemPromptSpec { role, task, config, context }
         |
         v
  +------+---------+
  | Token Budget    |  Calculate available tokens:
  | Calculator      |   model_context_window - reserved_for_output - safety_margin
  +------+---------+
         |
         v
  Layer 1: ROLE IDENTITY                               [Cache tier: SYSTEM]
    "You are a senior Rust engineer working on the roko project."
    Source: crates/roko-compose/src/templates/{role}.rs
    11 role templates: implementer, reviewer, researcher, refactorer,
    strategist, conductor, integration, scribe, quick, task_impl, common
         |
  Layer 2: CONVENTIONS                                  [Cache tier: SYSTEM]
    "Follow these conventions: snake_case, thiserror for errors,
     tokio for async, tracing for logging. Never use .unwrap() in
     library crates."
    Source: roko.toml [conventions] + workspace CLAUDE.md
         |
  Layer 3: DOMAIN CONTEXT                              [Cache tier: SESSION]
    Crate map, project structure, key types, dependency graph.
    Source: workspace analysis + config
         |
  Layer 3c: ACTIVE SIGNALS                             [Cache tier: SESSION]
    Active pheromone/stigmergic guidance signals from the substrate.
    "Warning: rate limiter middleware has a known race condition."
    Source: Store.query() for active pheromone-kind signals
         |
  Layer 4: TASK SPECIFICATION                           [Cache tier: TASK]
    "Implement rate limiting middleware in crates/roko-serve/src/
     middleware/rate_limit.rs. Use tower::RateLimit with a configurable
     requests-per-second limit."
    Source: task definition from plan
         |
  Layer 4b: GATE FEEDBACK (only on retry)              [Cache tier: DYNAMIC]
    "Previous attempt failed at Rung 2 (Test):
     test rate_limit::tests::test_concurrent_requests FAILED
     thread 'test_concurrent_requests' panicked at 'assertion failed:
     response.status() == 429'"
    Source: prior attempt verdicts
         |
  Layer 5: TOOL INSTRUCTIONS                           [Cache tier: SYSTEM]
    "Available tools: Read (read file contents), Write (write file),
     Bash (run shell command), Search (grep codebase), ..."
    Source: tool registry (roko-std StaticToolRegistry)
         |
  Layer 6: PLAYBOOKS                                   [Cache tier: TASK]
    "When implementing HTTP middleware:
     1. Create the middleware struct in src/middleware/
     2. Implement tower::Layer and tower::Service
     3. Add integration test in tests/
     4. Register in the router builder"
    Source: PlaybookStore (roko-learn) top matches for task shape
         |
  Layer 7: ANTI-PATTERNS                               [Cache tier: TASK]
    "Known mistakes to avoid:
     - Do NOT use .unwrap() -- use anyhow::Result
     - Do NOT import from roko-cli in library crates
     - Do NOT add fields to Signal without updating content_hash()"
    Source: knowledge store anti-knowledge entries + error patterns
         |
  Layer 8: AFFECT GUIDANCE                             [Cache tier: DYNAMIC]
    "Recent execution history: 2 consecutive failures on similar tasks.
     Exercise extra caution. Double-check error handling. Consider
     writing the test first before the implementation."
    Source: DaimonState PAD vector interpretation
         |
         v
  +------+---------+
  | Token Budget    |  Trim layers from bottom up if over budget:
  | Enforcement     |   L8 trimmed first (least critical)
  |                 |   L1 trimmed last (most critical)
  +------+---------+
         |
         v
  Final assembled system prompt (String)
  Sent to LLM provider in Step 4 (ACT)
```

---

## 12. Affect State Computation

How DaimonState computes the PAD vector and translates it to dispatch modulation.

```
  Task outcome (success or failure)
         |
         v
  +------+---------+
  | Prospect Value  |  crates/roko-daimon/src/lib.rs
  |                 |  prospect_value(pnl):
  |                 |    gains: x^0.88
  |                 |    losses: -2.25 * |x|^0.88
  |                 |  (Kahneman & Tversky prospect theory)
  +------+---------+
         |
         v
  +------+---------+
  | PAD Update      |  Three timescales, each with a PAD vector:
  |                 |
  | FAST (emotion)  |  decay: minutes
  |  P += delta_p   |  Immediate reaction to last outcome
  |  A += delta_a   |  P: success -> +, failure -> -
  |  D += delta_d   |  A: surprise -> +, expected -> 0
  |                 |  D: agency -> +, helpless -> -
  |                 |
  | MED (mood)      |  decay: hours
  |  P = ema(P, a)  |  Running average over recent session
  |  A = ema(A, a)  |  Smooths out individual outcome noise
  |  D = ema(D, a)  |
  |                 |
  | SLOW (temper.)  |  decay: days
  |  P = ema(P, a)  |  Long-term behavioral tendency
  |  A = ema(A, a)  |  Changes slowly over many sessions
  |  D = ema(D, a)  |
  +------+---------+
         |
         v
  +------+---------+  Interpret PAD octant:
  | Octant Mapping  |
  |                 |  P+A+D+ = Exuberant (confident exploration)
  |                 |  P+A-D+ = Relaxed (steady state, flow)
  |                 |  P-A+D- = Anxious (failures + no control)
  |                 |  P-A+D+ = Hostile (failures + fighting back)
  |                 |  P-A-D- = Bored (no signal, low engagement)
  |                 |  ... (8 octants total)
  +------+---------+
         |
         v
  +------+---------+
  | Dispatch        |  DispatchModulation:
  | Modulation      |
  |                 |  temperature_delta:
  |                 |    Frustration: -0.1 to -0.3 (more deterministic)
  |                 |    Flow: +0.0 to +0.1 (slightly more creative)
  |                 |    Anxiety: -0.2 (conservative)
  |                 |
  |                 |  max_turns:
  |                 |    Normal: 25 tool loop iterations
  |                 |    Fatigue: 15 (reduce wasted effort)
  |                 |    Flow: 30 (allow deeper exploration)
  |                 |
  |                 |  exploration_rate:
  |                 |    Frustration: 0.3x (stick with known-good models)
  |                 |    Flow: 1.5x (try cheaper or different models)
  |                 |    Default: 1.0x
  +------+---------+
         |
         v
  +------+---------+
  | Somatic Markers |  crates/roko-daimon/src/somatic_ta.rs
  |                 |  Situation-specific learned associations:
  |                 |    "This task shape + this model = bad outcome"
  |                 |  Stored in a KD-tree for fast PAD-space lookup
  |                 |  Bias the router's selection before it runs
  +------+---------+
         |
         v
  Output: DispatchModulation applied to Steps 1-4 of core loop
```
