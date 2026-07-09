# Innovations and Net-New Features

Novel capabilities that go beyond fixing what exists. Each section proposes a
feature grounded in roko's current architecture, informed by 2025--2026
state-of-the-art research, and mapped to concrete implementation paths.

---

## 1. Agent Memory and Continuity

### Motivation

Roko agents today are stateless between invocations. Each `roko plan run`
spawns fresh agents that know nothing about the previous run's mistakes,
successes, or discoveries. The EpisodeLogger records what happened
(`.roko/episodes.jsonl`), the KnowledgeStore holds durable facts
(`.roko/neuro/knowledge.jsonl`), and the PlaybookStore captures reusable
sequences (`.roko/learn/playbooks/`), but none of these are consulted at
agent spawn time in the live dispatch path. The agent starts cold every time.

The 2026 consensus from Mem0, JetBrains Research, and the ActiveContext
framework (arxiv 2604.11462) is clear: vanilla RAG fails for agentic use
cases because agents need stateful persistence that recalls context on
demand, not just one-shot retrieval. Graph-based memory with conflict
detection (Mem0g's directed labeled knowledge graph) outperforms flat vector
stores on complex multi-hop questions by measurable margins.

Roko already has the building blocks -- episodic memory, knowledge store with
tier progression, HDC fingerprints for similarity matching, and anti-knowledge
for conflict detection. The innovation is wiring them into a unified memory
layer that agents consult and update at every turn.

### Design Sketch

**Three-tier agent memory**:

```
Tier 1: Working Memory (per-session)
  - Current task context, tool call history, partial results
  - Lives in tool loop state (`ToolLoopState` in `tool_loop/`)
  - Survives crash via checkpoint.rs (already built)

Tier 2: Episodic Memory (per-project, cross-session)
  - Past task attempts: what worked, what failed, error patterns
  - Indexed by HDC fingerprint for similarity-based retrieval
  - Source: EpisodeLogger + ErrorPatternStore + PlaybookStore

Tier 3: Semantic Memory (cross-project, durable)
  - Extracted knowledge entries with confidence decay
  - Anti-knowledge for "never do this" patterns
  - Source: KnowledgeStore (roko-neuro)
```

**Memory injection pipeline** (runs at dispatch time):

```
1. Hash current task context → HDC fingerprint
2. Query Tier 2: find top-K similar past episodes
   - Filter by outcome (success/failure)
   - Weight by recency (half-life decay)
3. Query Tier 3: semantic search by task domain tags
   - Include anti-knowledge as "avoid" guidance
4. Format as SystemPromptBuilder sections:
   - Layer 6 (Techniques): successful playbooks
   - Layer 7 (Anti-patterns): failure patterns + anti-knowledge
   - Layer 4 (Task context): relevant prior outputs
5. Token budget: cap at 2K tokens total across tiers
```

**Memory update pipeline** (runs at task completion):

```
1. Success → extract playbook, update SkillLibrary
2. Success → ingest knowledge entry at Transient tier
3. Failure → store error pattern, ingest anti-knowledge
4. Either → update episode with outcome + HDC fingerprint
5. Confirmation → if new knowledge confirms existing, boost confidence
```

### Implementation Plan

| Step | What | Where | Effort |
|------|------|-------|--------|
| 1 | Create `MemoryLayer` struct wrapping all 3 tiers | `crates/roko-learn/src/memory_layer.rs` | 1 day |
| 2 | Implement `query_for_task(task_context) -> MemoryInjection` | Same file | 1 day |
| 3 | Wire into `dispatch_agent_with()` in runner v2 | `crates/roko-cli/src/workflow_engine.rs` or runner v2 dispatch | 0.5 day |
| 4 | Wire into SystemPromptBuilder as layers 6/7 sections | `crates/roko-compose/src/prompt_assembly_service.rs` | 0.5 day |
| 5 | Implement memory update on task completion | `crates/roko-learn/src/runtime_feedback.rs` | 0.5 day |
| 6 | Add `roko learn memory query <topic>` CLI command | `crates/roko-cli/src/main.rs` (learn subcommand) | 0.5 day |
| 7 | Add TUI memory tab showing recent retrievals | `crates/roko-cli/src/tui/` | 1 day |

**Total effort**: ~5 days

### Acceptance Criteria

1. Run `roko plan run` on a plan where task 3 depends on patterns from task 1.
   Task 3's system prompt contains relevant playbook from task 1's success.
2. Run a plan where task 2 fails with a specific error. Re-run. The retry
   agent's prompt contains the error pattern as anti-guidance.
3. Run `roko learn memory query "gate failure patterns"` and get ranked results
   from the knowledge store with confidence scores.
4. After 10+ successful runs, verify that the memory layer surfaces the most
   relevant 3-5 playbooks (not all of them) within the 2K token budget.

---

## 2. Multi-Agent Collaboration

### Motivation

Roko's current multi-agent model is strictly sequential within a task: one
implementer, one reviewer, one auto-fixer, each waiting for the previous to
finish. The `MultiAgentPool` and `AgentPool` in `roko-agent` support parallel
dispatch and warm pools, but they are used only for different tasks in the DAG
-- never for multiple agents collaborating on the same task.

The 2026 landscape has shifted dramatically. Every major coding tool shipped
multi-agent capabilities within the same two-week window in February 2026.
Google's Agent2Agent (A2A) protocol, donated to the Linux Foundation, defines
a standard for agent interoperability. Research from AWS (Strands Agents),
OpenAI (Agents SDK), and the Swarms framework shows up to 70% higher success
rates on complex goals with multi-agent collaboration vs. single-agent
approaches.

The key insight from the swarm pattern is that peer agents working from
different perspectives and sharing findings produce emergent improvements that
no single agent achieves alone. For roko, this means an implementer and a
reviewer should not be strictly sequential -- the reviewer should be able to
intervene during implementation, and multiple implementers should be able to
propose competing approaches.

### Design Sketch

**Three collaboration patterns**:

```
Pattern 1: Competitive Proposals (Best-of-N)
  - N agents implement the same task independently (different worktrees)
  - Gate pipeline evaluates all N outputs
  - Best output (by gate score) wins
  - Use case: architectural decisions, complex refactors

Pattern 2: Peer Review Swarm
  - Implementer streams partial results via event bus
  - Reviewer monitors in real-time, can inject corrections early
  - Scribe generates documentation as implementation progresses
  - Use case: standard tasks, documentation-heavy work

Pattern 3: Specialist Decomposition
  - Conductor decomposes task into sub-problems
  - Specialist agents handle sub-problems in parallel
  - Merge agent synthesizes results
  - Use case: cross-crate changes, multi-file refactors
```

**Communication layer**:

```
AgentBridge {
  // Publish observations for peer agents
  fn publish(topic: &str, msg: AgentMessage) -> Result<()>;

  // Subscribe to peer observations
  fn subscribe(topic: &str) -> Receiver<AgentMessage>;

  // Request peer review of partial work
  fn request_review(artifact: &Path) -> ReviewVerdict;

  // Vote on a proposal
  fn vote(proposal_id: &str, verdict: Verdict) -> Result<()>;
}
```

This uses roko-runtime's existing `EventBus` (typed broadcast channel with
replay support) as the transport, avoiding any new infrastructure.

**Consensus mechanism for competitive proposals**:

```
Score(proposal) = w1 * gate_score
               + w2 * token_efficiency
               + w3 * diff_minimality
               + w4 * reviewer_confidence

If top_score - second_score < epsilon:
  Run tiebreaker: merge best parts of top 2
```

### Implementation Plan

| Step | What | Where | Effort |
|------|------|-------|--------|
| 1 | Define `AgentBridge` trait + `EventBus` impl | `crates/roko-agent/src/bridge.rs` | 1 day |
| 2 | Implement Best-of-N proposal runner | `crates/roko-orchestrator/src/competitive.rs` | 2 days |
| 3 | Wire worktree isolation per-proposal | `crates/roko-cli/src/orchestrate.rs` (WorktreeManager) | 1 day |
| 4 | Implement proposal scoring and selection | `crates/roko-gate/src/proposal_scorer.rs` | 1 day |
| 5 | Implement Peer Review Swarm mode | `crates/roko-orchestrator/src/swarm.rs` | 2 days |
| 6 | Add `--collaboration=competitive|swarm|specialist` flag | `crates/roko-cli/src/main.rs` | 0.5 day |
| 7 | Add ACP config for collaboration mode | `crates/roko-acp/src/types.rs` session config | 0.5 day |
| 8 | TUI visualization for multi-agent state | `crates/roko-cli/src/tui/` | 1 day |

**Total effort**: ~9 days

### Acceptance Criteria

1. `roko plan run --collaboration=competitive --proposals=3` spawns 3
   implementers in separate worktrees. Gate pipeline scores all 3. Best wins.
   TUI shows all proposals with scores.
2. Competitive mode on a refactor task produces measurably smaller diffs (by
   line count) than single-agent mode averaged over 5 runs.
3. Swarm mode: reviewer catches a bug mid-implementation, implementer receives
   the correction before finishing. Verify via episode log timestamps.
4. Specialist mode: a 5-file cross-crate change spawns 3 specialists (one per
   crate) plus a merge agent. Total wall time < 1.5x single-agent time.

---

## 3. Self-Improving Gates

### Motivation

Roko's 7-rung gate pipeline (compile, clippy, test, symbol, generated-test,
property-test, integration) runs the same checks regardless of what changed.
The `AdaptiveThresholds` in `roko-gate` implement EMA pass rates, CUSUM
change-point detection, SPC detectors, and Hotelling's T-squared for
multi-gate anomaly detection. These adjust retry budgets and suggest skipping
gates with long consecutive-pass streaks, but they do not learn *what* to
check or *how* to check it.

The key gap: when a specific failure pattern recurs (e.g., "imports added but
not sorted"), the gate pipeline keeps running the same broad checks instead
of adding a targeted check for that specific pattern. Meanwhile, the
`ErrorPatternStore` in `roko-learn` accumulates these patterns with full
context but never feeds them back into gate construction.

### Design Sketch

**Failure-pattern-driven gate generation**:

```
1. ErrorPatternStore accumulates patterns with:
   - error_hash, category, message, frequency, last_seen
   - affected_files, affected_crates, fix_patterns

2. GateEvolver analyzes accumulated patterns:
   - Cluster by category (compile, lint, test, format)
   - Identify high-frequency patterns (>3 occurrences)
   - Generate targeted checks:
     a) ShellGate commands for automatable checks
     b) GrepGate patterns for code-level checks
     c) LlmJudgeGate prompts for semantic checks

3. Generated gates inserted BEFORE standard rungs:
   - "Pre-flight" checks: fast, targeted, catch known issues
   - If pre-flight fails, skip expensive gates (save time)
   - If pre-flight passes, run standard pipeline (confidence boost)

4. Gate lifecycle:
   - Generated gates start with weight 0.5
   - True positives increase weight (useful check)
   - False positives decrease weight (noisy check)
   - Weight < 0.1 → auto-retire gate
   - Weight > 0.9 → promote to permanent rung
```

**Adaptive rung selection based on diff analysis**:

```
DiffAnalyzer {
  // Analyze git diff to determine which gates are relevant
  fn relevant_rungs(diff: &GitDiff) -> Vec<Rung> {
    let mut rungs = vec![Compile]; // always

    if diff.touches_cargo_toml() { rungs.push(Symbol); }
    if diff.touches_tests()      { rungs.push(Test); }
    if diff.touches_public_api() { rungs.push(Clippy); rungs.push(Integration); }
    if diff.adds_unsafe()        { rungs.push(Security); }
    if diff.line_count() > 200   { rungs.push(PropertyTest); }

    rungs
  }
}
```

**Gate effectiveness feedback loop**:

```
After each gate run:
  1. Record (rung, verdict, wall_time, diff_context) to gate-outcomes.jsonl
  2. If gate caught a real bug: increment true_positive counter
  3. If gate passed and downstream gate caught bug: increment false_negative
  4. If gate failed and fix was trivial/false alarm: increment false_positive
  5. Weekly: recompute precision/recall per rung
  6. Rungs with recall < 0.3: suggest retirement or replacement
  7. Rungs with precision < 0.5: suggest tightening or prompt revision
```

### Implementation Plan

| Step | What | Where | Effort |
|------|------|-------|--------|
| 1 | Create `GateEvolver` struct | `crates/roko-gate/src/gate_evolver.rs` | 1.5 days |
| 2 | Wire ErrorPatternStore read into evolver | `crates/roko-learn/src/error_pattern_store.rs` | 0.5 day |
| 3 | Implement ShellGate generation from patterns | `crates/roko-gate/src/gate_evolver.rs` | 1 day |
| 4 | Create `DiffAnalyzer` for rung relevance | `crates/roko-gate/src/diff_analyzer.rs` | 1 day |
| 5 | Wire diff analysis into `GateService::run_gates()` | `crates/roko-gate/src/gate_service.rs` | 0.5 day |
| 6 | Implement gate effectiveness tracking | `crates/roko-learn/src/gate_effectiveness.rs` | 1 day |
| 7 | Add `roko learn gates` CLI for gate health report | `crates/roko-cli/src/main.rs` | 0.5 day |
| 8 | Wire generated gates into GateConfig at runtime | `crates/roko-core/src/config/` | 1 day |

**Total effort**: ~7 days

### Acceptance Criteria

1. After 5 runs with recurring "unused import" failures, the gate pipeline
   automatically adds a targeted grep-based pre-flight check for unused
   imports. This check runs in <100ms vs. clippy's 3-8 seconds.
2. A diff touching only documentation files skips compile, clippy, and test
   gates entirely (only runs format + diff gates). Verify via gate report.
3. Gate effectiveness report shows precision/recall per rung after 20+ runs.
   At least one gate with precision < 0.5 is flagged for review.
4. Generated gates that produce 3+ consecutive false positives are
   automatically retired. Verify via `.roko/learn/gate-evolution.json`.

---

## 4. Adaptive Context Windows

### Motivation

Roko's SystemPromptBuilder assembles 9 layers of context, but it treats every
model the same: same token budgets, same section priorities, same pruning
strategy. In reality, models have wildly different effective context windows.
BenchLM.ai's 2026 comparison shows effective context can fall 99% below the
advertised maximum on complex tasks. A prompt that works well with Claude's
200K window may be catastrophically pruned for a Cerebras 8B model with an
8K effective window.

Anthropic's own context engineering guide (2026) identifies the core
challenge: context engineering is the discipline of building dynamic systems
that provide the right information at the right time within the LLM's context
window. The JetBrains Research team found that observation masking -- showing
agents only relevant observations while preserving action history -- is the
single most effective strategy for software engineering agents.

Roko already has the VCG auction mechanism (`vcg_allocate()` in
`roko-compose/src/auction.rs`) for allocating context budget across sections.
It also has section effectiveness tracking
(`roko-learn/src/section_effect.rs`) that measures which prompt sections
actually improve outcomes. The innovation is making these systems adaptive to
the specific model being used.

### Design Sketch

**Model-aware context profiles**:

```
ModelContextProfile {
  model_slug: String,
  effective_window: usize,     // measured, not advertised
  sweet_spot: Range<usize>,    // optimal token range for quality
  degradation_curve: Vec<(usize, f64)>,  // tokens → quality score
  section_affinities: HashMap<SectionKind, f64>,  // per-model
}
```

**Progressive disclosure strategy**:

```
Level 1: Essential (always included, ~500 tokens)
  - Role identity (layer 1)
  - Task description (layer 4, core only)
  - Tool instructions (layer 5, active tools only)

Level 2: Contextual (included if budget allows, ~1-2K tokens)
  - Domain conventions (layer 2)
  - Prior task outputs (layer 4b)
  - Top-1 playbook (layer 6)

Level 3: Enrichment (included for large-window models, ~2-4K tokens)
  - Gate feedback (layer 4b, full)
  - Multiple playbooks (layer 6)
  - Anti-patterns (layer 7)
  - Knowledge store results (layer 3)

Level 4: Deep context (opus/large-window only, ~4-8K tokens)
  - Full code context
  - Research citations
  - Affect guidance (layer 8)
  - Pheromone signals (layer 3c)
```

**Adaptive compaction**:

```
When token count exceeds model's effective window:
  1. Drop Level 4 sections first
  2. Summarize Level 3 sections (LLM-assisted if >2K tokens)
  3. Truncate Level 2 sections to key sentences
  4. Never touch Level 1 (essential)

When section effectiveness data available:
  - Rank sections by measured lift (section_effect.rs)
  - Drop lowest-lift sections first, regardless of level
  - Exception: never drop task description or tool instructions
```

**Per-model context optimization**:

```
1. On first use of a model, run calibration:
   - Send progressively longer contexts
   - Measure quality (gate pass rate) vs. context length
   - Record degradation curve

2. On subsequent uses, select optimal context size:
   - Target the "sweet spot" range
   - Avoid exceeding degradation threshold
   - Cache profile at .roko/learn/model-profiles/{slug}.json
```

### Implementation Plan

| Step | What | Where | Effort |
|------|------|-------|--------|
| 1 | Define `ModelContextProfile` struct | `crates/roko-compose/src/context_profile.rs` | 0.5 day |
| 2 | Implement progressive disclosure levels | `crates/roko-compose/src/prompt_assembly_service.rs` | 1 day |
| 3 | Wire section effectiveness into context sizing | `crates/roko-learn/src/section_effect.rs` → compose | 1 day |
| 4 | Implement model-aware VCG auction budgets | `crates/roko-compose/src/auction.rs` | 1 day |
| 5 | Build calibration runner | `crates/roko-learn/src/model_calibration.rs` | 1.5 days |
| 6 | Wire profiles into dispatch path | Runner v2 dispatch | 0.5 day |
| 7 | Add `roko config models calibrate <slug>` CLI | `crates/roko-cli/src/main.rs` | 0.5 day |
| 8 | Persist profiles to `.roko/learn/model-profiles/` | `crates/roko-learn/` | 0.5 day |

**Total effort**: ~6.5 days

### Acceptance Criteria

1. Dispatching to Cerebras 8B with a 15K-token prompt automatically compacts
   to <6K tokens using progressive disclosure. Gate pass rate does not
   decrease vs. manually trimmed prompt.
2. Dispatching to Claude Opus with the same task includes Level 4 context.
   Section effectiveness data shows which sections contributed to success.
3. After 20+ runs with section tracking, the context assembler drops
   consistently low-lift sections automatically. Token usage decreases by
   15-25% with no quality regression.
4. `roko config models calibrate sonnet` runs 5 test prompts at varying
   lengths, records degradation curve, and stores profile.

---

## 5. Speculative Execution

### Motivation

When a gate fails, roko's current retry loop is strictly sequential: fix,
re-gate, fix, re-gate, up to 5 iterations. Each iteration waits for the
previous to complete. For complex failures with multiple possible fixes, this
serial approach wastes time trying one fix at a time.

Research from 2025-2026 shows speculative execution for agents is a maturing
field. The "Speculative Actions" framework (arxiv 2510.04371) demonstrates up
to 55% accuracy in next-action prediction, translating to significant latency
reductions. Multi-model speculative execution -- running parallel predictions
with models of comparable capacity -- is now practical.

Roko's `ParallelExecutor` already tracks `speculative_executions` in its
snapshot format, and the `WorktreeManager` can isolate parallel attempts.
The innovation is bringing speculative execution to the fix-retry loop and
to plan-level strategy selection.

### Design Sketch

**Three speculative execution modes**:

```
Mode 1: Speculative Fix (gate failure recovery)
  - On gate failure, spawn N fix agents in parallel
  - Agent 1: minimal fix (haiku/fast model, targeted patch)
  - Agent 2: broader fix (sonnet, may touch related code)
  - Agent 3: architectural fix (opus, may restructure)
  - First to pass gates wins; kill others
  - Net effect: wall time of fastest fix, not sum of all attempts

Mode 2: Speculative Strategy (plan-level)
  - For complex tasks (>10 files), generate 2-3 strategies
  - Each strategy gets a lightweight probe:
    a) Estimate files to touch (code context)
    b) Estimate complexity (token count heuristic)
    c) Check for anti-knowledge conflicts
  - Best strategy (by probe score) gets full execution
  - Others kept as fallback if primary fails

Mode 3: Speculative Prefetch (pipeline-level)
  - While task N executes, speculatively prepare task N+1:
    a) Resolve dependencies from DAG
    b) Pre-build system prompt (layers 1-3 stable)
    c) Pre-spawn warm agent
    d) Pre-fetch code context
  - If task N succeeds, task N+1 starts with zero cold-start delay
  - If task N fails, discard prefetch (context may have changed)
```

**Speculative fix decision tree**:

```
On GateFailed(task, rung, error):
  complexity = classify_error(error)
  match complexity:
    Trivial (unused import, format) →
      Single haiku agent, no speculation
    Moderate (type mismatch, missing impl) →
      2 parallel agents: haiku (targeted) + sonnet (broader)
    Complex (logic error, architectural) →
      3 parallel agents: sonnet x2 (different approaches) + opus
    Unknown →
      2 parallel: sonnet (conservative) + sonnet (exploratory)
```

**Resource budgeting for speculation**:

```
SpeculationBudget {
  max_parallel_fixes: usize,      // default: 3
  max_speculation_cost: f64,       // USD cap per speculation round
  kill_on_first_success: bool,     // default: true
  reuse_failed_context: bool,      // feed failed attempt as anti-pattern
}

// Cost control: speculation cost capped at 2x single-attempt cost
// If 3 parallel haiku runs cost less than 1 sonnet run, always speculate
```

### Implementation Plan

| Step | What | Where | Effort |
|------|------|-------|--------|
| 1 | Create `SpeculativeFixRunner` | `crates/roko-orchestrator/src/speculative.rs` | 2 days |
| 2 | Implement parallel worktree dispatch for fixes | Wire into WorktreeManager | 1 day |
| 3 | Implement first-success-wins with cancellation | Use `CancelToken` from roko-runtime | 1 day |
| 4 | Wire error complexity classifier | `crates/roko-learn/src/error_enrichment.rs` | 0.5 day |
| 5 | Implement speculative prefetch for DAG tasks | `crates/roko-orchestrator/src/dag.rs` | 1.5 days |
| 6 | Add speculation budget controls to config | `crates/roko-core/src/config/` | 0.5 day |
| 7 | Add `--speculate` flag to `plan run` | `crates/roko-cli/src/main.rs` | 0.5 day |
| 8 | Track speculation outcomes in learning system | `crates/roko-learn/src/speculation_outcome.rs` | 1 day |

**Total effort**: ~8 days

### Acceptance Criteria

1. A compile error fix: `roko plan run --speculate` spawns 2 parallel fix
   agents. The faster fix passes gates first, the other is cancelled. Total
   wall time is less than sequential retry (measure).
2. A complex refactor task: speculative strategy generates 2 approaches, probe
   selects the one touching fewer files. The selected approach succeeds.
3. DAG with 5 sequential tasks: speculative prefetch prepares task N+1 while
   N executes. Cold-start time for tasks 2-5 is <200ms (vs. ~800ms without).
4. Speculation cost for a 3-parallel-haiku run is less than a single sonnet
   run. Verify via cost log.
5. After 10+ speculative runs, the system learns which error categories
   benefit from speculation (measured by time savings) and auto-enables
   speculation only for those categories.

---

## 6. Agent-Native Debugging

### Motivation

When a roko agent fails, the current debugging experience is manual: read the
episode log, decode the error, guess at the cause, manually craft a retry.
The `forensic_replay` module in `roko-learn` can reconstruct causal chains,
and the `error_enrichment` module can classify errors, but neither is exposed
as an interactive debugging workflow.

Microsoft Research's AgentRx framework (2026) establishes the principle of
systematic debugging for AI agents: decompose agent behavior into inspectable
components, identify which component failed, and apply targeted interventions.
The self-healing infrastructure pattern -- perceive, decide, remediate in
real-time -- is becoming the baseline for production agent systems.

Roko should be able to debug itself. When an agent fails, a debugger agent
should analyze the failure, propose a fix to the agent configuration (not just
the code), and optionally retry with the corrected configuration.

### Design Sketch

**Agent failure taxonomy**:

```
FailureKind {
  // The agent produced incorrect output
  QualityFailure {
    gate_rung: u8,
    error_hash: String,
    is_recurring: bool,
  },

  // The agent got stuck in a loop
  ConvergenceFailure {
    iterations: usize,
    repeated_error_hashes: Vec<String>,
  },

  // The agent ran out of context/budget
  ResourceFailure {
    kind: Resource,  // Tokens, Cost, Time, ContextWindow
    used: f64,
    limit: f64,
  },

  // The agent's tools didn't work
  ToolFailure {
    tool_name: String,
    error: String,
    is_permission: bool,
  },

  // The agent didn't understand the task
  ComprehensionFailure {
    evidence: String,  // from diff analysis or reviewer feedback
  },
}
```

**Debugger agent workflow**:

```
1. DIAGNOSE
   - Classify failure into taxonomy
   - Query ErrorPatternStore for matching patterns
   - Query episode log for similar past failures
   - Check if this is a regression (previously working)

2. HYPOTHESIZE
   - Generate ranked hypotheses:
     a) Wrong model for this task complexity
     b) Missing context (relevant file not in prompt)
     c) Tool permission mismatch (agent can't edit target files)
     d) Prompt section interference (conflicting instructions)
     e) Gate too strict / too lenient for this change type

3. INTERVENE
   - For each hypothesis, propose a configuration change:
     a) Route to different model → update CascadeRouter static table
     b) Add code context → adjust enrichment pipeline
     c) Fix permissions → update role manifest
     d) Adjust prompt → modify template or section weights
     e) Adjust gate → tune threshold or skip rung

4. RETRY
   - Apply intervention
   - Re-run failed task with corrected configuration
   - If success: record intervention as playbook
   - If failure: try next hypothesis

5. ESCALATE
   - If all hypotheses exhausted: generate human-readable report
   - Include: failure timeline, hypotheses tried, suggested manual fixes
   - Emit to TUI + write to .roko/debug/{task_id}.md
```

**Debug report format**:

```
## Debug Report: task-07 (implement-new-gate)
### Failure: ConvergenceFailure (3 iterations, same error)

**Root Cause**: Model sonnet repeatedly attempted to add a new file
to a module that doesn't exist. The module path in the task
description (`crates/roko-gate/src/custom/`) doesn't match the
actual structure (`crates/roko-gate/src/`).

**Intervention Applied**: Added repo context (file tree) to
system prompt layer 3. Re-routed to opus for architectural
understanding.

**Outcome**: Passed on retry (opus correctly identified the
module structure).

**Recommendation**: Add file tree context for all tasks touching
new modules. Consider making this default for Implementer role.
```

### Implementation Plan

| Step | What | Where | Effort |
|------|------|-------|--------|
| 1 | Define `FailureKind` taxonomy | `crates/roko-learn/src/failure_taxonomy.rs` | 0.5 day |
| 2 | Create `DebuggerAgent` that classifies failures | `crates/roko-agent/src/debugger.rs` | 1.5 days |
| 3 | Implement hypothesis generation | Same file | 1 day |
| 4 | Implement configuration intervention engine | `crates/roko-orchestrator/src/intervention.rs` | 1.5 days |
| 5 | Wire into gate failure handler in runner v2 | `crates/roko-runtime/src/workflow_engine.rs` | 1 day |
| 6 | Implement debug report generation | `crates/roko-learn/src/debug_report.rs` | 0.5 day |
| 7 | Add `roko debug <task-id>` CLI command | `crates/roko-cli/src/main.rs` | 0.5 day |
| 8 | Wire debug reports into TUI | `crates/roko-cli/src/tui/` | 1 day |
| 9 | Record successful interventions as playbooks | `crates/roko-learn/src/playbook.rs` | 0.5 day |

**Total effort**: ~8 days

### Acceptance Criteria

1. A task fails 3 times with the same error. The debugger agent auto-classifies
   as ConvergenceFailure, hypothesizes "missing context," adds repo tree to
   prompt, retries, and succeeds. Debug report written to `.roko/debug/`.
2. A task fails because the agent tries to use a disallowed tool (e.g., bash
   for a read-only role). Debugger identifies ToolFailure, proposes role
   permission fix, and generates a corrective manifest update.
3. `roko debug task-07` displays the debug report for a past failure, including
   failure timeline, hypotheses ranked by likelihood, and the intervention
   that worked (or all interventions tried if none worked).
4. Successful interventions appear as playbooks in `PlaybookStore` and are
   surfaced to future agents via memory injection (Innovation 1).

---

## 7. Cross-Project Learning

### Motivation

Roko's knowledge store is project-scoped: each `.roko/` directory is
independent. Knowledge learned in project A (e.g., "Cerebras struggles with
async Rust" or "this test framework needs --release for benchmarks") is
invisible to project B, even when the same user runs roko on both.

The 2026 trend toward "agent skills" as portable knowledge packages -- exemplified
by Oracle NetSuite's SuiteCloud Agent Skills and Anthropic's CLAUDE.md
conventions -- shows that cross-project knowledge transfer is becoming a
first-class concern. The key insight is that knowledge should be stratified:
some is universal (language patterns, model capabilities), some is
domain-specific (Rust async patterns, React hooks), and some is
project-specific (this repo's module structure).

Roko's `KnowledgeStore` already supports tiers (Transient, Stable, Core) and
confidence decay. The innovation is adding a fourth tier -- Universal -- that
syncs across projects and captures model/tooling meta-knowledge.

### Design Sketch

**Knowledge stratification**:

```
Tier 0: Project-specific (current behavior)
  - Module structure, crate dependencies, test patterns
  - Stored in .roko/neuro/knowledge.jsonl

Tier 1: Domain-specific (shared across similar projects)
  - Language idioms, framework patterns, common errors
  - Stored in ~/.roko/domains/{domain}/knowledge.jsonl
  - Domain tags: "rust", "typescript", "react", "blockchain"

Tier 2: Model meta-knowledge (shared across all projects)
  - Model capabilities: "opus handles architecture, sonnet handles edits"
  - Model failures: "cerebras can't do multi-file edits"
  - Tool effectiveness: "grep more reliable than glob for imports"
  - Stored in ~/.roko/meta/model-knowledge.jsonl

Tier 3: Universal (community-contributed, opt-in)
  - Best practices validated across many users
  - Published via roko knowledge sync (future)
  - Stored in ~/.roko/community/knowledge.jsonl
```

**Sync protocol**:

```
On project run completion:
  1. Scan episode outcomes for generalizable patterns
  2. Filter: exclude project-specific paths, variable names, secrets
  3. Classify remaining patterns by tier
  4. Promote Tier 0 → Tier 1 if:
     - Confidence > 0.8
     - Pattern matches domain tags
     - Not project-path-dependent
  5. Promote Tier 1 → Tier 2 if:
     - Pattern is about model/tool behavior (not code patterns)
     - Confirmed across 2+ domains
  6. Write to appropriate store

On project run start:
  1. Load Tier 0 (project-specific) -- always
  2. Load Tier 1 (domain) -- if domain tags match
  3. Load Tier 2 (meta) -- always
  4. Merge into MemoryLayer (Innovation 1)
  5. Conflicts resolved by confidence score
```

**Domain detection**:

```
fn detect_domain(workdir: &Path) -> Vec<DomainTag> {
  let mut tags = vec![];
  if exists("Cargo.toml")       { tags.push("rust"); }
  if exists("package.json")     { tags.push("javascript"); }
  if exists("tsconfig.json")    { tags.push("typescript"); }
  if exists("pyproject.toml")   { tags.push("python"); }
  if exists("foundry.toml")     { tags.push("blockchain"); }
  if exists(".react-*")         { tags.push("react"); }
  tags
}
```

### Implementation Plan

| Step | What | Where | Effort |
|------|------|-------|--------|
| 1 | Add `~/.roko/` global config directory | `crates/roko-core/src/config/` | 0.5 day |
| 2 | Implement domain detection | `crates/roko-cli/src/repo_context.rs` | 0.5 day |
| 3 | Create multi-tier KnowledgeStore wrapper | `crates/roko-neuro/src/tiered_store.rs` | 1.5 days |
| 4 | Implement Tier 0 → Tier 1 promotion logic | Same file | 1 day |
| 5 | Implement Tier 1 → Tier 2 promotion logic | Same file | 0.5 day |
| 6 | Implement secret/path scrubbing for promotion | `crates/roko-neuro/src/scrub.rs` | 1 day |
| 7 | Wire tiered store into MemoryLayer | `crates/roko-learn/src/memory_layer.rs` | 0.5 day |
| 8 | Add `roko knowledge export/import` commands | `crates/roko-cli/src/main.rs` | 1 day |
| 9 | Add `roko knowledge domains` for domain inspection | `crates/roko-cli/src/main.rs` | 0.5 day |

**Total effort**: ~7 days

### Acceptance Criteria

1. Run roko on project A (Rust). Agent learns "Cerebras fails on async
   trait impls." Start project B (Rust). Agent's prompt includes this
   knowledge from `~/.roko/domains/rust/knowledge.jsonl`.
2. Run roko on project C (TypeScript). Rust-specific knowledge is NOT
   injected (domain filter works).
3. Model meta-knowledge ("opus better for architecture tasks") is available
   in all projects regardless of domain.
4. `roko knowledge export` produces a scrubbed JSON file with no project
   paths, no secrets, no variable names -- only generalizable patterns.
5. A promoted knowledge entry that gets contradicted (anti-knowledge in a
   different project) has its confidence reduced across all tiers.

---

## 8. Interactive Steering

### Motivation

Roko's current human-in-the-loop model is binary: either the human approves
every action (blocking), or the agent runs fully autonomously (no oversight).
The `--approval` mode in runner v2 pauses at every phase transition, but
there is no way to steer a running agent without stopping it.

The 2026 human-in-the-loop literature identifies a critical scaling problem:
humans cannot meaningfully review AI decisions at machine speed. The EU AI
Act (Article 14) requires demonstrable human oversight that is trained,
measurable, and provable. The solution is not more review gates but smarter
intervention points: AI handles high-volume routine cases, humans focus on
low-confidence or exception cases.

Roko needs a "steering wheel, not a brake pedal" -- the ability to redirect
agents in real-time without stopping the pipeline.

### Design Sketch

**Steering primitives**:

```
SteeringAction {
  // Redirect the current agent's approach
  Redirect {
    guidance: String,       // injected as high-priority context
    model_override: Option<String>,  // switch model mid-task
  },

  // Skip the current task (mark as deferred)
  Skip {
    reason: String,
  },

  // Split the current task into sub-tasks
  Split {
    sub_tasks: Vec<TaskSpec>,
  },

  // Adjust budget for remaining tasks
  BudgetAdjust {
    remaining_budget_usd: f64,
    model_preference: Option<String>,
  },

  // Inject additional context mid-execution
  InjectContext {
    content: String,
    priority: ContextPriority,  // Override | Append | Background
  },

  // Approve or reject a pending review
  ReviewVerdict {
    task_id: String,
    verdict: Verdict,
    notes: String,
  },
}
```

**Confidence-based intervention requests**:

```
ConfidenceThresholds {
  auto_proceed: f64,     // > 0.85: agent proceeds without asking
  suggest_review: f64,   // 0.5-0.85: show in TUI, proceed unless vetoed
  require_approval: f64, // < 0.5: block until human approves
}

// Confidence computed from:
// - Task complexity vs. model capability
// - Similarity to past successes (memory layer)
// - Gate prediction (expected pass probability from adaptive thresholds)
// - Error pattern match (similar task failed before)
```

**Non-blocking intervention channel**:

```
TUI Keybindings:
  F8: Open steering panel
  s:  Redirect current task (inject guidance)
  k:  Skip current task
  b:  Adjust budget
  c:  Inject context
  r:  Override review verdict
  Esc: Close panel (no action)

HTTP API:
  POST /api/steer/{task_id}  { action: SteeringAction }
  GET  /api/confidence       { tasks: Vec<ConfidenceReport> }
  POST /api/approve/{task_id} { verdict: Verdict }
```

**Steering audit trail**:

```
Every steering action recorded to .roko/steer/audit.jsonl:
{
  "timestamp": "2026-04-29T15:30:00Z",
  "task_id": "task-07",
  "action": "Redirect",
  "guidance": "Use the existing gate infrastructure, don't create a new one",
  "agent_state_before": "implementing",
  "outcome": "task passed gates on next attempt"
}

// For EU AI Act compliance: demonstrable human oversight with timestamps
```

### Implementation Plan

| Step | What | Where | Effort |
|------|------|-------|--------|
| 1 | Define `SteeringAction` enum | `crates/roko-core/src/steering.rs` | 0.5 day |
| 2 | Implement steering channel (tokio mpsc) | `crates/roko-runtime/src/steering.rs` | 1 day |
| 3 | Wire steering into runner v2 execution loop | Runner v2 main loop | 1.5 days |
| 4 | Implement confidence scoring | `crates/roko-learn/src/confidence.rs` | 1 day |
| 5 | Add TUI steering panel (F8) | `crates/roko-cli/src/tui/modals/steering.rs` | 1.5 days |
| 6 | Add HTTP steering endpoints | `crates/roko-serve/src/routes/steering.rs` | 1 day |
| 7 | Implement steering audit trail | `crates/roko-learn/src/steering_audit.rs` | 0.5 day |
| 8 | Wire confidence thresholds into phase transitions | Runner v2 dispatch | 0.5 day |

**Total effort**: ~7.5 days

### Acceptance Criteria

1. During a `roko plan run`, press F8 in TUI. Steering panel opens. Type
   guidance text. The running agent's next tool loop iteration includes the
   guidance as a high-priority context section. Agent adjusts behavior.
2. A task with confidence < 0.5 (e.g., touching unfamiliar crate + no similar
   episodes) pauses in TUI with a "Review recommended" banner. Human can
   approve, redirect, or skip.
3. `POST /api/steer/task-07 { action: "redirect", guidance: "..." }` injects
   context into the running agent without stopping the pipeline. Response
   includes the agent's updated state.
4. After a run with 3 steering interventions, `.roko/steer/audit.jsonl`
   contains all 3 with timestamps, task state, and outcomes.
5. The confidence threshold auto-adjusts: after 50+ tasks where high-confidence
   tasks pass gates 95%+ of the time, the `auto_proceed` threshold tightens
   from 0.85 to 0.9 (learned calibration).

---

## 9. Cost Optimization

### Motivation

Enterprise LLM API spending doubled in six months to $8.4B by mid-2025, with
research showing 50-90% of inference costs can be eliminated with model
routing, semantic caching, and distillation. Roko's `CascadeRouter` already
implements a 3-stage routing strategy (static → confidence → LinUCB), but it
currently routes only on quality -- not on cost-quality trade-offs.

The 2025-2026 research on cascade routing shows that well-implemented
cascades achieve 87% cost reduction by ensuring expensive models handle only
the 10% of queries that truly require their capabilities. The PILOT framework
(Preference-prior Informed LinUCB) and LLM Shepherding both demonstrate
significant cost reductions while maintaining accuracy.

Roko has the infrastructure: `CostTable` for pricing, `CostsLog` for
tracking, `ParetoFrontier` for cost-quality analysis, and the cascade
router's Pareto frontier for down-weighting dominated models. The innovation
is activating these components together into a cost-aware routing and caching
system.

### Design Sketch

**Cost-aware cascade routing**:

```
CostAwareCascade {
  // Stage 1: Try cheapest model first
  fn route(task: &TaskContext) -> ModelPlan {
    let candidates = self.pareto_frontier.non_dominated();
    let budget_pressure = self.remaining_budget / self.remaining_tasks;

    // Sort by cost, filter by minimum quality threshold
    let cheapest_viable = candidates
      .filter(|m| m.expected_quality >= self.quality_floor)
      .min_by_key(|m| m.cost_per_token);

    ModelPlan {
      primary: cheapest_viable,
      escalation: self.next_tier(cheapest_viable),
      max_escalations: budget_pressure_to_max_escalations(budget_pressure),
    }
  }
}
```

**Semantic caching**:

```
SemanticCache {
  // Before dispatching to LLM, check if a similar prompt was already answered
  fn check(prompt_fingerprint: &[u8]) -> Option<CachedResponse> {
    // 1. Exact match on BLAKE3 hash (free)
    if let Some(hit) = self.exact.get(prompt_fingerprint) {
      return Some(hit);
    }

    // 2. Fuzzy match on HDC fingerprint (cosine similarity > 0.95)
    if let Some(hit) = self.fuzzy.nearest(prompt_fingerprint, 0.95) {
      // Validate: cached response still applicable?
      if self.validate_cache_hit(&hit, prompt_fingerprint) {
        return Some(hit);
      }
    }

    None
  }

  // After LLM response, cache if task is cacheable
  fn store(fingerprint: &[u8], response: &AgentResponse, ttl: Duration) {
    // Only cache deterministic tasks (compile fixes, format, simple edits)
    // Never cache creative/architectural tasks
    if response.task_category.is_deterministic() {
      self.exact.insert(fingerprint, response, ttl);
      self.fuzzy.insert(fingerprint, response);
    }
  }
}
```

**Token budget management**:

```
BudgetManager {
  plan_budget_usd: f64,
  spent_usd: f64,
  remaining_tasks: usize,
  task_costs: HashMap<TaskId, f64>,

  fn budget_for_task(&self, task: &Task) -> TaskBudget {
    let avg_remaining = (self.plan_budget_usd - self.spent_usd) / self.remaining_tasks;
    let complexity_multiplier = task.complexity.cost_multiplier();

    TaskBudget {
      target_usd: avg_remaining * complexity_multiplier,
      hard_cap_usd: avg_remaining * 3.0,  // never exceed 3x average
      model_ceiling: self.cheapest_model_above_quality_floor(),
      allow_escalation: self.spent_usd < self.plan_budget_usd * 0.7,
    }
  }
}
```

**Prompt compression**:

```
PromptCompressor {
  // Reduce token count while preserving semantic content
  fn compress(prompt: &str, target_tokens: usize) -> String {
    let current = count_tokens(prompt);
    if current <= target_tokens { return prompt.to_string(); }

    // Strategy 1: Remove redundant whitespace, comments
    let cleaned = strip_cosmetic(prompt);

    // Strategy 2: Summarize long code blocks
    let summarized = summarize_code_blocks(cleaned, target_tokens);

    // Strategy 3: Drop least-effective sections (from section_effect data)
    let trimmed = drop_low_lift_sections(summarized, target_tokens);

    trimmed
  }
}
```

### Implementation Plan

| Step | What | Where | Effort |
|------|------|-------|--------|
| 1 | Wire Pareto frontier into cascade router routing | `crates/roko-learn/src/cascade_router.rs` | 1 day |
| 2 | Implement cost-aware model selection | Same file + `crates/roko-learn/src/budget.rs` | 1 day |
| 3 | Implement semantic cache with BLAKE3 exact match | `crates/roko-learn/src/semantic_cache.rs` | 1.5 days |
| 4 | Add HDC fuzzy matching to semantic cache | Same file, use `roko-primitives` HDC | 1 day |
| 5 | Implement budget manager per-plan | `crates/roko-learn/src/budget.rs` | 1 day |
| 6 | Wire budget into dispatch path | Runner v2 dispatch | 0.5 day |
| 7 | Implement prompt compression pipeline | `crates/roko-compose/src/compressor.rs` | 1 day |
| 8 | Add `roko learn costs` CLI for cost analysis | `crates/roko-cli/src/main.rs` | 0.5 day |
| 9 | Add cost dashboard to TUI | `crates/roko-cli/src/tui/` | 1 day |
| 10 | Wire cost table from model config | `crates/roko-learn/src/cost_table.rs` | 0.5 day |

**Total effort**: ~9 days

### Acceptance Criteria

1. A 20-task plan with budget $5.00: the system routes 80%+ of tasks to cheap
   models (cerebras, haiku), escalating only complex tasks to sonnet/opus.
   Total cost is <$3.00 (vs. $8+ if all tasks use sonnet).
2. Semantic cache: run the same fix task twice. Second run hits exact cache,
   zero LLM cost, <100ms response time.
3. Budget exhaustion: when 70% of budget is spent with 50% of tasks remaining,
   the router automatically downshifts to cheaper models. No task is skipped.
4. `roko learn costs` shows per-task cost breakdown, cost-per-gate-pass, and
   model cost distribution pie chart.
5. After 50+ runs, the Pareto frontier correctly identifies dominated models
   (high cost, low quality) and stops routing to them. Verify via
   `cascade-router.json` observations.

---

## 10. Agent Protocol Interoperability (A2A)

### Motivation

Google's Agent2Agent (A2A) protocol, donated to the Linux Foundation in 2025
with 50+ technology partners, defines a standard for agent-to-agent
communication. MCP (Model Context Protocol) provides tools and context *to*
agents; A2A provides communication *between* agents. As the agent ecosystem
fragments across vendors and frameworks, interoperability becomes critical.

Roko already speaks MCP (via `agent.mcp_config` in roko.toml with
auto-discovery fallback) and has a per-agent HTTP sidecar
(`roko-agent-server`) with `/message`, `/stream`, and `/predictions`
endpoints. The innovation is making roko agents discoverable and callable via
A2A, enabling external agents to request roko's specialized capabilities
(code analysis, gate verification, knowledge queries) and enabling roko to
delegate to external agents.

### Design Sketch

**A2A Agent Card (discovery)**:

```json
{
  "name": "roko-code-agent",
  "description": "Autonomous coding agent with gate verification",
  "url": "https://localhost:6677/a2a",
  "version": "1.0",
  "capabilities": {
    "streaming": true,
    "pushNotifications": false
  },
  "skills": [
    {
      "id": "code-implementation",
      "name": "Code Implementation",
      "description": "Implement code changes with compile/test/lint verification",
      "inputModes": ["text"],
      "outputModes": ["text", "file"]
    },
    {
      "id": "code-review",
      "name": "Code Review",
      "description": "Review code changes with architectural analysis",
      "inputModes": ["text", "file"],
      "outputModes": ["text"]
    },
    {
      "id": "gate-verification",
      "name": "Gate Verification",
      "description": "Run compile, test, lint, format gates on a codebase",
      "inputModes": ["file"],
      "outputModes": ["text"]
    },
    {
      "id": "knowledge-query",
      "name": "Knowledge Query",
      "description": "Query project knowledge store for patterns and insights",
      "inputModes": ["text"],
      "outputModes": ["text"]
    }
  ],
  "authentication": {
    "schemes": ["bearer"]
  }
}
```

**A2A task lifecycle integration**:

```
External Agent → A2A Task Request → Roko
  1. Parse A2A task (JSON-RPC 2.0 over HTTP)
  2. Map A2A skill to internal AgentRole
  3. Create internal task in executor
  4. Stream progress via A2A SSE updates
  5. Return artifacts via A2A task completion

Roko → A2A Task Request → External Agent
  1. Discover external agent via Agent Card URL
  2. Map internal task to A2A skill request
  3. Monitor progress via SSE
  4. Integrate response into pipeline
```

**Roko as A2A server (exposing capabilities)**:

```
Routes added to roko-serve:
  GET  /.well-known/agent.json       → Agent Card
  POST /a2a                          → JSON-RPC endpoint
  GET  /a2a/tasks/{id}               → Task status + SSE stream

Supported A2A methods:
  tasks/send          → create and execute a task
  tasks/get           → get task status
  tasks/cancel        → cancel a running task
  tasks/sendSubscribe → create task with SSE updates
```

**Roko as A2A client (consuming external agents)**:

```
In roko.toml:
[a2a.agents]
researcher = "https://research-agent.example.com/.well-known/agent.json"
designer   = "https://design-agent.example.com/.well-known/agent.json"

// At dispatch time, if task domain matches an A2A agent's skills,
// delegate via A2A instead of spawning a local agent.
// Fallback: if A2A agent fails, retry with local agent.
```

### Implementation Plan

| Step | What | Where | Effort |
|------|------|-------|--------|
| 1 | Define A2A types (AgentCard, Task, Artifact, Message) | `crates/roko-core/src/a2a.rs` | 1 day |
| 2 | Implement Agent Card generation from config | `crates/roko-serve/src/routes/a2a.rs` | 0.5 day |
| 3 | Implement A2A JSON-RPC endpoint | Same file | 1.5 days |
| 4 | Map A2A skills to internal AgentRole dispatch | `crates/roko-serve/src/routes/a2a.rs` | 1 day |
| 5 | Implement A2A SSE streaming for task progress | Same file, use existing SSE infra | 1 day |
| 6 | Implement A2A client for external agent discovery | `crates/roko-agent/src/a2a_client.rs` | 1.5 days |
| 7 | Wire A2A delegation into dispatch path | Runner v2 or orchestrate dispatch | 1 day |
| 8 | Add `[a2a]` config section to roko.toml | `crates/roko-core/src/config/` | 0.5 day |
| 9 | Add `roko agent discover <url>` CLI | `crates/roko-cli/src/main.rs` | 0.5 day |

**Total effort**: ~8.5 days

### Acceptance Criteria

1. `GET http://localhost:6677/.well-known/agent.json` returns a valid A2A
   Agent Card with 4 skills listed.
2. An external A2A client can send a `tasks/send` request with a code
   implementation skill. Roko executes the task, runs gates, and returns
   the result via A2A task completion. SSE stream shows progress updates.
3. Configure an external research agent in roko.toml. When a research-domain
   task appears in a plan, roko delegates to the external agent via A2A.
   Result is integrated into the pipeline.
4. A2A task cancellation (`tasks/cancel`) correctly cancels the running agent
   via `CancelToken` and returns a cancelled status.
5. Authentication works: A2A requests without valid bearer token return 401.

---

## Cross-Cutting Concerns

### Observability for All Innovations

Every innovation above produces telemetry that feeds into roko's existing
learning infrastructure:

| Innovation | Data Produced | Consumed By |
|------------|---------------|-------------|
| Agent Memory | Retrieval relevance scores | Section effectiveness |
| Multi-Agent | Proposal comparison metrics | CascadeRouter |
| Self-Improving Gates | Gate precision/recall | Adaptive thresholds |
| Adaptive Context | Context size vs. quality | Model profiles |
| Speculative Execution | Speculation win rate | Error enrichment |
| Agent Debugging | Intervention success rate | PlaybookStore |
| Cross-Project Learning | Knowledge promotion rate | KnowledgeStore tiers |
| Interactive Steering | Steering frequency by confidence | Confidence calibration |
| Cost Optimization | Cost-per-gate-pass by model | Pareto frontier |
| A2A Interop | External agent success rate | Routing decisions |

### Incremental Adoption

Innovations are designed to be adopted independently:

```
Phase 1 (Immediate value, minimal risk):
  - Innovation 9 (Cost Optimization) -- saves money immediately
  - Innovation 4 (Adaptive Context) -- improves small-model performance
  - Innovation 1 (Agent Memory) -- leverages existing data

Phase 2 (Medium-term, moderate complexity):
  - Innovation 3 (Self-Improving Gates) -- learns from accumulated data
  - Innovation 6 (Agent Debugging) -- reduces manual intervention
  - Innovation 8 (Interactive Steering) -- adds human oversight

Phase 3 (Strategic, higher complexity):
  - Innovation 5 (Speculative Execution) -- requires parallel infra
  - Innovation 2 (Multi-Agent Collaboration) -- requires coordination
  - Innovation 7 (Cross-Project Learning) -- requires scrubbing/privacy
  - Innovation 10 (A2A Interop) -- requires ecosystem
```

### Dependency Graph

```
Innovation 1 (Memory) ─────────────────┐
  ↑                                     │
  │  Innovation 7 (Cross-Project) ──────┤
  │                                     │
Innovation 3 (Gates) ───────────────────┤
  ↑                                     │
  │  Innovation 6 (Debugging) uses 3 ───┤
  │                                     ↓
Innovation 4 (Context) ────────── Innovation 9 (Cost)
  ↑
  │  Innovation 5 (Speculative) uses 4
  │
Innovation 2 (Multi-Agent) ─── stands alone
Innovation 8 (Steering) ────── stands alone
Innovation 10 (A2A) ─────────── stands alone
```

### Total Estimated Effort

| Innovation | Days | Priority |
|------------|------|----------|
| 1. Agent Memory | 5 | P1 |
| 2. Multi-Agent | 9 | P3 |
| 3. Self-Improving Gates | 7 | P2 |
| 4. Adaptive Context | 6.5 | P1 |
| 5. Speculative Execution | 8 | P3 |
| 6. Agent Debugging | 8 | P2 |
| 7. Cross-Project Learning | 7 | P3 |
| 8. Interactive Steering | 7.5 | P2 |
| 9. Cost Optimization | 9 | P1 |
| 10. A2A Interop | 8.5 | P3 |
| **Total** | **76** | |

---

## Appendix A: What Roko Already Has That Others Don't

Before building new features, it is worth cataloguing the innovations already
present in roko that are absent or rare in competing systems:

### A.1 HDC Fingerprinting for Episode Memory

Hyperdimensional computing (HDC) vectors encode episodes as high-dimensional
binary vectors. Cosine similarity between fingerprints enables O(1) episode
retrieval without embedding models. No major coding agent does this.

**Where**: `crates/roko-learn/src/hdc_fingerprint.rs`,
`crates/roko-primitives/src/`

### A.2 Anti-Knowledge with Confidence Decay

The knowledge store explicitly tracks things that *don't* work, with
HDC-similarity-based conflict detection that warns, discounts, or rejects new
entries that conflict with existing anti-knowledge. Knowledge entries decay
over time (half-life model) and can be "resurrected" when re-confirmed.

**Where**: `crates/roko-neuro/src/knowledge_store.rs` (lines 59-67:
warn/discount/reject thresholds)

### A.3 VCG Auction for Context Allocation

Prompt section allocation uses a Vickrey-Clarke-Groves (VCG) auction --
a mechanism from economics that incentivizes truthful bidding. Context
bidders (knowledge, affect, per-subsystem) compete for token budget, and the
auction ensures Pareto-optimal allocation.

**Where**: `crates/roko-compose/src/auction.rs`

### A.4 17-Dimensional Contextual Bandit for Model Routing

The CascadeRouter's LinUCB stage uses a 17-dimensional context vector
including task tier, complexity, role hash, crate familiarity, conductor load,
daimon policy, and more. This is substantially richer than the model routing
in any publicly available agent framework.

**Where**: `crates/roko-learn/src/cascade_router.rs`

### A.5 Affect-Modulated Agent Dispatch

The Daimon subsystem (`roko-daimon`) applies somatic markers to dispatch
decisions -- an implementation of Damasio's somatic marker hypothesis from
neuroscience. Agent behavior is modulated by an "affect state" that reflects
the system's recent experience (frustration after failures, confidence after
successes).

**Where**: `crates/roko-daimon/`

### A.6 Pheromone Field for Inter-Task Communication

Gate verdicts propagate between tasks via a "pheromone field" -- a
bio-inspired communication mechanism where tasks leave traces that influence
subsequent tasks' behavior. This is analogous to ant colony optimization
where ants leave pheromone trails for other ants to follow.

**Where**: Referenced in `orchestrate.rs` pheromone sections, used via
SystemPromptBuilder layer 3c

### A.7 Dream Consolidation Cycle

The `roko-dreams` crate implements an offline consolidation cycle inspired by
sleep neuroscience: hypnagogia (creative association), imagination (scenario
generation), and consolidation (knowledge compression). This is not wired to
a cron trigger yet, but the engine is built and callable.

**Where**: `crates/roko-dreams/src/cycle.rs`, `crates/roko-dreams/src/runner.rs`

### A.8 Process Reward Model for Reasoning Verification

The gate pipeline includes a `ProcessReward` component that evaluates
step-level reasoning quality, not just final output quality. This aligns with
the 2025-2026 research on process reward models (PRMs) for improving LLM
reasoning.

**Where**: `crates/roko-gate/` (ProcessReward in gate infrastructure)

---

## Appendix B: Industry Context and Research References

### Agent-to-Agent Communication

The A2A protocol (Google, 2025, donated to Linux Foundation) defines
agent interoperability over HTTP/SSE/JSON-RPC. 50+ technology partners
including Atlassian, Salesforce, SAP, and ServiceNow. Complements MCP
(Anthropic) which provides tools/context to individual agents.

### Autonomous Coding Agents (2026 State)

Every major tool shipped multi-agent capabilities in February 2026. Claude
Code (Opus 4.7) is the most capable autonomous coding agent available.
Context engineering -- curating optimal token sets during inference -- has
replaced prompt engineering as the core skill.

### Multi-Agent Collaboration

Research shows 70% higher success rates on complex goals with multi-agent
collaboration vs. single-agent. Leading frameworks: LangGraph, CrewAI,
OpenAI Agents SDK, Google ADK. Swarm pattern with peer-to-peer information
exchange produces emergent improvements.

### Agent Memory Systems

Graph-based memory (Mem0g) outperforms flat vector stores on multi-hop
questions. The 2026 consensus: vanilla RAG fails for agentic use cases.
Three memory types formalized: episodic (what happened), semantic (what is
known), procedural (how to do things). Gartner predicts 40% of enterprise
apps will feature AI agents by 2026.

### Cost Optimization

Cascade routing achieves 87-98% cost reduction. PILOT (LinUCB with
preference priors), LLM Shepherding (2.8x cost reduction), and semantic
caching are the leading approaches. Enterprise spending doubled to $8.4B
in six months.

### Tool-Use Optimization

Speculative tool execution (arxiv 2510.04371) achieves 55% next-action
prediction accuracy. Tool caching (ToolCacheAgent) provides 1.69x latency
speedup. Multi-model speculative execution runs parallel predictions with
models of comparable capacity.

### Self-Healing Systems

AgentRx (Microsoft Research, 2026) establishes systematic debugging for
agents. Self-healing is becoming baseline for production agent systems.
Key requirement: confidence scoring and human escalation paths.

### Human-in-the-Loop Scaling

EU AI Act Article 14 requires demonstrable human oversight. Traditional
review models are collapsing at production scale. Solution: AI handles
routine cases, humans focus on low-confidence exceptions. 35% of
organizations plan to deploy AI agents in 2025, projected 86% by 2027.

### Context Window Management

ActiveContext (arxiv 2604.11462) reframes context management as active
sequential decision-making. JetBrains Research: observation masking is the
most effective strategy for software engineering agents. Effective context
often falls far below advertised maximum (up to 99% gap).

### Speculative Execution for Agents

Multi-model speculative execution runs parallel predictions. Distributed
Speculative Inference (DSI) is 1.29-1.92x faster. BanditSpec uses
multi-armed bandits to select speculative strategies dynamically.
