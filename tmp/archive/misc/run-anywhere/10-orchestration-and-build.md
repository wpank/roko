# Roko Orchestration & Build Engine: The Document Pipeline

> **Audience**: Technical leads, orchestrator designers, and developer experience architects
> **Scope**: Details the multi-agent `roko build` infrastructure, the Document Pipeline, the Unified DAG, and the Inference Gateway caching layer.

---

The bottleneck in AI-assisted development is not model quality. It is context. When using standard single-agent editors, context windows become overloaded and the agent makes decisions isolated from the rest of the workspace.

The **Roko Orchestration Engine** completely abandons the "chat" interface. Instead, it operates a deeply structured document pipeline that compresses context at every layer.

## The Document Hierarchy (Compression Pipeline)

The central idea is a deterministic hierarchy. Context is pre-computed offline using 32 enrichment scripts (via tree-sitter AST extraction, dependency analysis, etc.) rather than having an LLM waste tokens trying to "search" or "explore" at runtime.

```text
PRD (what you want)
  -> Plans (how to build it, in what order)
    -> Tasks (atomic units of work, file assignments, acceptance criteria)
      -> Briefs (pre-assembled context, budget-fitted to the agent's role)
        -> Prompts (what the agent actually sees)
```

An implementer agent does not read a 15-page PRD. It receives three paragraphs extracted directly from the PRD relevant only to its task, plus the Exact Type Signatures it needs to import, the file paths it must touch, and the precise test criteria (auto-derived from PRD invariants) that prove it worked.

### Five-Layer Orchestration

1. **Execution Layer / Worktree Isolation**: Every agent is assigned physical isolation via a Git worktree. When `roko build` runs, Agent A and Agent B operate on parallel branches. No shared mutable state.
2. **Context Layer**: Extracts specific subsets of the codebase per agent role. Implementers get task lists. Reviewers get completion summaries and diffs.
3. **Inference Gateway**: Acts as an LLM proxy. Three levels of caching (L3 Hash Cache, L2 Semantic Embedding Similarity, L1 Prefix KV Cache). Because the Roko pipelines are standardized, the prefix cache achieves up to a **91% hit rate**, resulting in 40-85% cheaper inference compared to raw API usage. 
4. **Agent Roles**: Roko contains 28 explicit developer roles (Strategist, Implementer, Architect, Auditor, Auto-fixer). Roles are model-agnostic. 
5. **Unified DAG Scheduler**: Instead of plan-level scheduling, Roko schedules at the Task Level. If Plan 3 Task 2 has no file conflicts with Plan 5 Task 1, they run simultaneously. This collapses wall-clock time completely.

## The Cybernetic Review Loop 

When a task completes, it does not hand the code to a human. 
1. **Compilation Gate**: It runs `cargo check` and `cargo test`.
2. **Failures**: Structured error traces feed directly back to the agent for targeted fixes.
3. **Success**: Spawns parallel Architect and Auditor agents to review the diff against the original Plan context. 
4. **Merge Queue**: A serialized merge queue respects dependency ordering, merging the parallel branches into a unified deployment branch.

## Real-World Efficiency Gains

By abandoning the single-chat loop and moving to this offline-enriched, DAG-scheduled architecture, Roko builds ship software significantly faster and cheaper than existing tools.

- **Cost**: 5-8x cheaper than sequential Claude Code execution due to the Inference Gateway and Prefix Caching.
- **Speed**: 4-12x faster wall-clock execution via Task-level parallelism running 6-20 agents simultaneously.
- **Quality**: Drastic defect reduction. Auto-generated tests and mock services are derived entirely from the PRD, catching missing conditionals before human review.

---

## The PRD-to-Execution Pipeline (End-to-End)

The complete flow from idea to merged code, every step a CLI command:

```bash
# 1. Capture a work item
roko prd idea "Wire SystemPromptBuilder into orchestrate.rs"

# 2. Draft a PRD from the idea (agent-assisted)
roko prd draft new "system-prompt-wiring"

# 3. Research the topic for context
roko research enhance-prd system-prompt-wiring

# 4. Generate implementation plan + tasks from the PRD
roko prd plan system-prompt-wiring    # Agent generates tasks.toml

# 5. Execute the plan (agents run tasks, gates validate, state persists)
roko plan run plans/

# 6. Resume if interrupted
roko plan run plans/ --resume .roko/state/executor.json
```

### Artifacts Generated Per Plan

| Artifact | Source | Content |
|---|---|---|
| `plan.md` | Human/agent | What to build, in what order |
| `tasks.toml` | Enrichment | Atomic units: file assignments, acceptance criteria, complexity band |
| `brief.md` | Enrichment | Pre-assembled context fitted to agent's role budget |
| `prd-extract.md` | Enrichment | Relevant PRD paragraphs (not the full document) |
| `verify-tasks.toml` | Enrichment | Verification steps per gate rung (symbol manifest, test stubs) |
| `review-tasks.toml` | Enrichment | Reviewer criteria and focus areas |
| `decomposition.md` | Enrichment | Step-by-step breakdown from Sonnet |
| `testing-backlog.md` | Enrichment | Test scenarios derived from PRD |
| `rubric.md` | Enrichment | Evaluation criteria for reviewers |
| `research.md` | Research agent | Pattern analysis, prior art, cross-plan context |
| `dependency-manifest.toml` | Enrichment | Full workspace dependency graph |
| `fixture-manifest.toml` | Enrichment | External services needed (EVM fork, mock HTTP) |
| `integration.md` | Enrichment | Cross-plan and cross-system notes |

### The Enrichment Pipeline (9 Steps, 2 Phases)

**Phase 1 (Sequential, Zero LLM Cost):**
1. PRD extraction (regex-based) → `prd-extract.md`
2. Brief generation (markdown parsing) → `brief.md`
3. Task generation (TOML from plan headings) → `tasks.toml`

**Phase 2 (Batchable via Batch API, 50% Discount):**
4. Verification task generation (Sonnet) → `verify-tasks.toml`
5. Review task generation (Sonnet) → `review-tasks.toml`
6. Step-by-step decomposition (Sonnet) → `decomposition.md`
7. Testing backlog (Sonnet) → `testing-backlog.md`
8. Review rubric & invariants (Haiku) → `rubric.md`
9. Scribe task list (Sonnet) → `scribe-tasks.toml`

**Cost**: $0.02-0.10 per plan enrichment. Typical 100-task build: **$31.50 total** vs $105 naive (67% savings).

### The Execution State Machine (14 Phases Per Task)

```
Plan → Enrich → Strategize → Implement → Compile Gate → Test Gate
  → Lint Gate → Symbol Gate → Generated Test Gate → Property Test Gate
  → Integration Gate → Review (Architect + Auditor + Scribe)
  → Merge Queue → Complete
```

With retry loops at each gate phase (max retries determined by adaptive thresholds), AutoFixer for syntactic errors, and MergeResolver for conflicts.

### The Unified Task DAG

Plans don't execute sequentially. ALL tasks from ALL plans are flattened into a single graph:

```rust
fn next_runnable(completed: &Set, in_flight: &Set) -> Vec<Task> {
    let blocked_files = files_touched_by(in_flight);
    dag.tasks()
        .filter(|t| !completed.contains(t))           // Not done
        .filter(|t| !in_flight.contains(t))            // Not running
        .filter(|t| t.deps().all(|d| completed.contains(d)))  // Deps met
        .filter(|t| !t.files().any(|f| blocked_files.contains(f)))  // No file conflicts
        .collect()
}
```

If Plan 3 Task 2 and Plan 5 Task 1 touch disjoint files, they run simultaneously — even across plans.

### Crash Recovery

Executor state snapshots to `.roko/state/executor.json` after every phase transition. On restart:
- Completed tasks: skipped
- In-flight tasks: restarted (processes are dead, worktree preserved)
- Merge checkpoint: if crash during merge, `git merge --abort` before restart
- Worktrees: never deleted (user may need them for inspection)

### Warm Agent Pre-Spawning

While an Implementer is running compile gates, Roko pre-spawns reviewer agents:

```
Implementer working → pre_spawn_warm(QuickReviewer)
Compile gate passes → promote_warm(QuickReviewer)  // instant start, no 5-15s cold start
Compile gate fails  → evict_warm(QuickReviewer)    // kill without wasting tokens
```

### Per-Role Prompt Budget Allocation

Different roles get different context proportions (characters, ~4 per token):

| Section | Implementer | Strategist | Architect | Scribe |
|---|---|---|---|---|
| Plan | 50K (25%) | 50K (30%) | 50K (25%) | 50K (25%) |
| PRD extract | 12K (20%) | 12K (20%) | 6K (15%) | 16K (25%) |
| Workspace map | 20K (10%) | 20K (20%) | 6K (15%) | 6K (10%) |
| Code context | 8K (10%) | 0 | 6K (8%) | 6K (10%) |
| Brief | 8K (10%) | 6K (5%) | 4K (10%) | 6K (10%) |
| Reviews | 3K (10%) | 3K (10%) | 3K (15%) | 3K (5%) |
| Skills | 8K (5%) | 4K (5%) | 4K (2%) | 4K (5%) |
| Instructions | 4K (5%) | 4K (5%) | 4K (5%) | 4K (5%) |

### The Learning Pack (Per-Task Context Injection)

Each task receives learned context from prior runs:

- **Playbook hints**: Rules that predicted outcomes in past builds (e.g., "always run `cargo check` before `cargo test` in this crate")
- **Research artifacts**: Researcher agent analysis per-plan
- **Dependency manifests**: External dependency requirements
- **Fixture manifests**: Test fixture setup (Anvil forks, mock HTTP servers)
- **Integration memos**: Cross-system notes from prior plans
- **Error patterns**: Common failure modes discovered by the conductor's watchers

### Context Injection into Worktree

Each agent's worktree receives pre-assembled context files:

```
context/in/
├── execution-pack.md          # Default: merged context
├── implementer-pack.md        # Role-specific pack
├── architect-pack.md
├── brief.md
├── prd2-extract.md
├── decomposition.md
├── verify-tasks.toml
├── learning.md                # Playbook + research + patterns
├── playbook.md
├── reflections.md             # Iteration memory from prior attempts
└── artifact-status.md         # Which artifacts are fresh vs stale
```

---

## Real-World Efficiency Gains (Expanded)

| Metric | Sequential (Cursor/Claude Code) | Roko Orchestration | Improvement |
|---|---|---|---|
| **Cost per task** | $2.00-8.00 (full context every time) | $0.30-2.00 (83% reduction + cache) | 5-8× cheaper |
| **Wall-clock time** | 8 hours (20 tasks × 24 min serial) | 45 min (12 agents parallel) | 4-12× faster |
| **First-pass quality** | ~40% (no verification, human catches errors) | ~65% (7-rung gate pipeline) | 1.6× higher |
| **Human review burden** | Review every change | Review only gate-passing changes | 60-80% reduction |
| **Learning** | None (every session starts fresh) | Cumulative (playbook, routing, skills) | Compounds over time |

---

## The Complexity Classifier

Not all tasks need the same pipeline. The classifier routes tasks to appropriate complexity bands:

### Classification Signals

| Signal | Measurement | Weight |
|---|---|---|
| Task count | Number of atomic tasks in the plan | 0.3 |
| Crates touched | How many crates modified | 0.25 |
| Cross-plan dependencies | Links to other plans | 0.2 |
| Prior failure rate | Historical pass rate for similar tasks | 0.15 |
| File count | Number of files estimated to change | 0.1 |

### Complexity Bands

| Band | Tasks | Crates | Dependencies | Model | Pipeline |
|---|---|---|---|---|---|
| **Trivial** | 1-2 | 1 | 0 | Haiku | Compile + Test only |
| **Simple** | 3-5 | 1 | ≤1 | Sonnet | + Symbol gate |
| **Standard** | 6-10 | 2-3 | ≤3 | Sonnet | + Generated tests + QuickReviewer |
| **Complex** | 10+ | 3+ | 3+ | Opus | Full 7-rung + 3 reviewers + Critic |

### The Escalation Ladder

If a task fails its gates, effective complexity is promoted one tier:
- Trivial fails → becomes Simple (adds Symbol gate)
- Simple fails twice → becomes Standard (adds generated tests)
- Standard fails twice → becomes Complex (full pipeline)

Escalation saturates at Complex — after that, failure is terminal.

---

## The Inference Gateway (Three-Layer Cache)

Between agents and LLM providers, the Inference Gateway provides three caching layers:

### L3: Hash Cache (Exact Match, <1ms)

SHA-256 hash of the full request. If exact match exists in LRU cache, return immediately. Zero cost, zero latency.

**Hit rate**: ~10-15% (identical prompts from retries or parallel same-role agents).

### L2: Semantic Cache (Cosine > 0.92, 5-20ms)

Compute embedding of the request. If a semantically similar request was made recently (cosine similarity > 0.92), return the cached response.

**Hit rate**: ~20-30% (similar but not identical requests, e.g., same task with slightly different context).

### L1: Prefix Cache (Provider-Side, 90% Discount)

Anthropic/OpenAI/GLM cache the KV state for shared prompt prefixes. The gateway ensures prefix stability via:
- BTreeMap serialization (deterministic key ordering)
- Cache layer markers (`<!-- roko:layer:N -->`)
- Stable section ordering (system prompt → workspace → plan → task)

**Hit rate**: 81-91% (most of the prompt is stable across requests for the same role).

### Combined Savings

```
Request arrives
  → L3 hash check (10% hit) → return cached ($0)
  → L2 semantic check (20% hit) → return cached ($0)
  → L1 prefix check (81% of remaining) → provider bills only suffix tokens
  → Full inference (9% of original requests)
```

**Effective cost**: ~$0.30-0.50 per task vs $2.00-8.00 without gateway.

---

## Technical Appendix: The Deep Architecture

For exhaustive technical mechanisms detailing how Roko achieves these constraints without crashing the repository, refer to:
* **[Parallel Execution & The Unified DAG](13-orchestration-parallel-execution.md)**: Details Git Worktree isolation, `sccache` multiplexing, and the file-conflict union-find matrices.
* **[Context Engineering Engine](14-orchestration-context-engine.md)**: Explains the 83% context compression, Tree-sitter AST, HDC chunk indexing, and prefix-caching BTreeMap strategy.
* **[Quality Gates & Verification](15-orchestration-quality-gates.md)**: Deconstructs the Compile/Test/Review sequence, structured error extraction, the `AutoFixer`, and automated Test Sidecars.

---

## The Agent Code Quality Patterns

### What Good Agent-Written Code Looks Like

Based on analysis of 6,300+ episodes, code written by agents that passes all gates consistently follows these patterns:

1. **Small, focused changes**: Agents that modify >10 files per task have 40% lower pass rates than those touching 2-3 files
2. **Test-adjacent edits**: Code changes accompanied by test updates pass 85% vs 65% without
3. **Existing pattern conformance**: Agents that match the codebase's existing naming/structuring conventions pass 20% more often
4. **Explicit imports**: Adding new `use` statements rather than relying on glob imports prevents 30% of compile failures
5. **Error handling**: Agents that propagate errors via `?` instead of `.unwrap()` produce code that passes the Auditor review 90% of the time

### The AGENTS.md Convention

Every project using roko should have an `AGENTS.md` (or `CLAUDE.md`) at the root:

```markdown
# Agent Instructions

## Coding Conventions
- Use `anyhow::Result` for error handling
- Deny `unsafe` code
- Run `cargo clippy` before committing
- Maximum 200 lines per function

## Project Structure
- All crates are in `crates/`
- Public API defined in each crate's `lib.rs`
- Tests in `tests/` directory (not inline)

## Known Pitfalls
- `roko-serve` has compilation errors on `workspace_dir` field
- Always import `tokio::sync::Mutex`, not `std::sync::Mutex` in async code
```

This file is loaded as Layer 1 (role-stable) in the prompt — it's in every agent's context, cached at 90% discount.

---

## The Configuration System (Layered)

### Loading Precedence (Highest → Lowest)

1. **CLI flags**: `--model claude-opus-4-6 --effort max`
2. **Environment variables**: `ROKO_MODEL=claude-opus-4-6`
3. **Project config**: `./roko.toml`
4. **Global config**: `~/.config/roko/config.toml`
5. **Built-in defaults**

Each layer only needs to specify overrides. Missing fields fall through to the next layer.

### Per-Plan Override

Individual plans can override global settings:

```toml
[plan_overrides."16-learning-loop"]
preferred_model = "claude-opus-4-6"
preferred_provider = "claude"
context_strategy = "inline_heavy"
complexity_band = "complex"
```

This lets complex plans use Opus while the rest of the codebase uses Sonnet — without global config changes.

---

## The Complexity Classifier (Detailed)

### Why Not All Tasks Need the Same Pipeline

Running the full 7-rung gate pipeline on a 3-line config change costs $1.50+ and takes 10+ minutes. Running only compile+test on a 200-line cross-crate refactor misses logic errors, naming issues, and API breakage. The complexity classifier exists to match pipeline intensity to task difficulty.

### Classification Signal Weights

The classifier computes a weighted score from five signals:

```
complexity_score = (task_count × 0.3) + (crates_touched × 0.25) +
                   (cross_plan_deps × 0.2) + (prior_failure_rate × 0.15) +
                   (file_count × 0.1)
```

Each signal is normalized to [0, 1] before weighting:

| Signal | Raw Value | Normalization | Weight | Rationale |
|---|---|---|---|---|
| Task count | Integer (1-50+) | min(tasks / 10, 1.0) | 0.3 | More tasks = more interactions = more failure modes |
| Crates touched | Integer (1-18) | min(crates / 3, 1.0) | 0.25 | Cross-crate changes require interface awareness |
| Cross-plan deps | Integer (0-10+) | min(deps / 3, 1.0) | 0.2 | Dependencies create ordering constraints and merge risk |
| Prior failure rate | Float (0.0-1.0) | 1.0 - historical_pass_rate | 0.15 | History predicts future; failing tasks need more verification |
| File count | Integer (1-100+) | min(files / 10, 1.0) | 0.1 | More files = larger diff = harder to review |

### The Four Complexity Bands

| Band | Score Range | Model | Gate Rungs | Review | Typical Cost |
|---|---|---|---|---|---|
| **Trivial** | 0.0 - 0.2 | Haiku | Compile + Test (rungs 0, 2) | None | $0.05-0.15 |
| **Simple** | 0.2 - 0.4 | Sonnet | + Lint + Symbol (rungs 0-3) | None | $0.15-0.50 |
| **Standard** | 0.4 - 0.7 | Sonnet | + GeneratedTest (rungs 0-4) | QuickReviewer | $0.50-2.00 |
| **Complex** | 0.7 - 1.0 | Opus | All 7 rungs (0-6) | 3 reviewers + Critic | $2.00-8.00 |

### The Escalation Ladder (With Saturation)

Failure promotes a task one tier. The rationale: if a task cannot pass at its current complexity band, it needs more verification (not less):

```
Attempt 1 (Trivial): compile + test only
  → Fails: test failure in edge case
Attempt 2 (Simple): + lint + symbol gate
  → Fails: missing public export
Attempt 3 (Standard): + generated tests + QuickReviewer
  → Fails: reviewer catches logic error
Attempt 4 (Complex): full 7-rung + 3 reviewers + Critic
  → Fails: terminal. No higher tier exists.
```

Escalation saturates at Complex. After that, failure is terminal and requires human intervention. The system does not retry indefinitely — unbounded retries waste budget without convergence.

### Affordance-Weighted Complexity

Task complexity is adjusted by the affordance score of the target files. Affordance measures how well-structured the code is for agent modification:

- **High affordance (> 0.7)**: Well-documented, small functions, clear interfaces, comprehensive tests. Agents succeed easily.
- **Medium affordance (0.3 - 0.7)**: Average codebase quality. Standard pipeline usually sufficient.
- **Low affordance (< 0.3)**: Large functions, sparse documentation, implicit contracts, no tests. Agents struggle.

When a Simple-classified task targets low-affordance code (score < 0.3), it is automatically promoted to Standard. The intuition: bad code needs more verification, regardless of how simple the task description appears.

```
effective_band = max(classifier_band, affordance_promotion(target_files))

fn affordance_promotion(files: &[FileInfo]) -> Band {
    let avg_affordance = files.iter().map(|f| f.affordance_score).sum::<f64>() / files.len() as f64;
    if avg_affordance < 0.3 { Band::Standard }
    else { Band::Trivial }  // No promotion needed
}
```

---

## The Configuration System (Extended)

### Loading Precedence (Detailed)

Settings cascade through five layers. Each layer only needs to specify overrides; missing fields fall through to the next layer:

```
Layer 1 (highest): CLI flags
  --model claude-opus-4-6 --effort max --max-agents 8

Layer 2: Environment variables
  ROKO_MODEL=claude-opus-4-6
  ROKO_MAX_AGENTS=12
  ROKO_GATE_TIMEOUT=300

Layer 3: Project config (./roko.toml)
  [agent]
  model = "claude-sonnet-4-20250514"
  provider = "claude"
  mcp_config = "mcp.json"

Layer 4: Global config (~/.config/roko/config.toml)
  [defaults]
  model = "claude-haiku-4-20250414"
  max_agents = 4

Layer 5 (lowest): Built-in defaults
  model = "claude-sonnet-4-20250514"
  max_agents = 4
  gate_timeout = 120
  max_retries = 3
```

Resolution example: if `--model` is passed on CLI, it wins. If not, `ROKO_MODEL` is checked. If not set, `roko.toml`'s `[agent].model` is used. And so on down the chain.

### Per-Plan Override Mechanism

Individual plans can override global settings for their execution scope:

```toml
# In roko.toml
[plan_overrides."16-learning-loop"]
preferred_model = "claude-opus-4-6"
preferred_provider = "claude"
context_strategy = "inline_heavy"
complexity_band = "complex"

[plan_overrides."42-rename-constants"]
preferred_model = "claude-haiku-4-20250414"
complexity_band = "trivial"
max_retries = 1
```

This lets complex plans (16-learning-loop) use Opus with full pipeline while trivial plans (42-rename-constants) use Haiku with minimal gates. No global config change needed. The override is scoped to a single plan and does not affect anything else.

### Overridable Fields

| Field | Type | Default | Description |
|---|---|---|---|
| `preferred_model` | String | "claude-sonnet-4-20250514" | LLM model for implementation |
| `preferred_provider` | String | "claude" | API provider |
| `context_strategy` | String | "balanced" | How context is assembled (inline_heavy, balanced, compressed) |
| `complexity_band` | String | auto | Force a specific complexity band (bypasses classifier) |
| `max_retries` | Integer | 3 | Maximum gate failure retries before terminal |
| `max_agents` | Integer | 4 | Concurrent agents for this plan |
| `gate_timeout` | Integer | 120 | Per-gate timeout in seconds |
| `review_mode` | String | auto | Force review mode (none, quick, full) |

### Environment Variable Naming Convention

All environment variables follow the pattern `ROKO_{SECTION}_{KEY}` with uppercase and underscores:

```bash
ROKO_MODEL=claude-opus-4-6                  # Top-level model
ROKO_AGENT_PROVIDER=claude               # [agent].provider
ROKO_GATE_TIMEOUT=300                    # [gate].timeout
ROKO_LEARN_EMA_ALPHA=0.15               # [learn].ema_alpha
ROKO_MAX_AGENTS=8                        # Top-level max_agents
```

---

## Agent Code Quality Patterns (Extended Analysis)

### What Good Agent-Written Code Looks Like

Based on analysis of 6,300+ episodes across 200+ plans, code written by agents that passes all gates consistently follows these patterns. These are not style preferences — they are statistically validated correlations between code characteristics and gate pass rates.

### Pattern 1: Small, Focused Changes

Agents touching >10 files per task have a **40% lower pass rate** than those touching 2-3 files.

Why: Large diffs are harder to verify (more surface area for errors), more likely to trigger file conflicts with concurrent agents, and more likely to introduce unintended side effects. The gate pipeline's per-file cost also increases linearly with diff size.

Mitigation: The enrichment pipeline's task decomposition step targets 2-5 files per task. Tasks that list >10 files are flagged for further splitting during `roko research enhance-tasks`.

### Pattern 2: Test-Adjacent Edits

Code changes accompanied by test updates pass **85% vs 65%** without test updates.

Why: Agents that update tests demonstrate understanding of the change's behavioral impact. Tests also catch regressions immediately during the Test Gate (Rung 2) rather than waiting for the Generated Test Gate (Rung 4).

Mitigation: The task template includes a `test_files` field. If non-empty, the agent's prompt explicitly instructs it to update the listed tests. The Scribe reviewer flags changes without test updates.

### Pattern 3: Existing Pattern Conformance

Matching codebase naming/structure conventions passes **20% more often** than introducing novel conventions.

Why: The Symbol Gate (Rung 3) checks for expected exports using patterns derived from the existing codebase. The Architect reviewer checks for interface coherence. Novel naming breaks both checks.

Mitigation: The `AGENTS.md` (or `CLAUDE.md`) file loaded as Layer 1 in every prompt establishes naming conventions. The enrichment pipeline also injects "similar code" examples from the same crate.

### Pattern 4: Explicit Imports

Adding explicit `use` statements instead of relying on glob imports prevents **30% of compile failures**.

Why: Glob imports (`use crate::*`) can shadow names, introduce ambiguity when new items are added, and make it unclear which module provides a given type. Explicit imports (`use crate::types::Signal`) are deterministic and survive codebase evolution.

Mitigation: Clippy's `wildcard_imports` lint is enabled by default in the Lint Gate (Rung 1). The `AGENTS.md` template includes "always use explicit imports" as a convention.

### Pattern 5: Error Handling

Agents that propagate errors via `?` instead of `.unwrap()` pass Auditor review **90% of the time**.

Why: `.unwrap()` panics on error, which is unacceptable in library code and most application code. The Auditor specifically checks for unwrap calls in non-test code. The Compile Gate itself does not catch this (code compiles fine), but the Auditor flags it as a correctness risk.

Mitigation: Clippy's `unwrap_used` lint is enabled for library code. The task template specifies `error_handling: "propagate"` by default. Agents receive explicit instructions to use `anyhow::Result` and `?` propagation.

### The AGENTS.md Convention (Detailed)

Every project using Roko should have an `AGENTS.md` (or `CLAUDE.md`) at the repository root. This file is loaded as Layer 1 (role-stable) in the SystemPromptBuilder's 6-layer architecture:

```
Layer 0: Base system prompt (model identity, general instructions)
Layer 1: AGENTS.md / CLAUDE.md (project-specific conventions)  ← cached at 90% discount
Layer 2: Role-specific instructions (Implementer, Reviewer, etc.)
Layer 3: Plan context (what to build)
Layer 4: Task context (specific unit of work)
Layer 5: Learned context (playbook hints, patterns, reflections)
```

Because Layer 1 is identical across all agents in the same project, it benefits from the Inference Gateway's prefix cache. With 81-91% prefix cache hit rates, the AGENTS.md content is effectively free after the first agent invocation.

Contents should include:
- **Coding conventions**: Error handling, import style, function size limits, testing strategy
- **Project structure**: Where crates live, how public APIs are defined, where tests go
- **Known pitfalls**: Compilation issues, async gotchas, dependency quirks
- **Naming conventions**: Type naming, module naming, variable naming patterns

The file should be concise (under 500 lines). Agents perform better with clear, actionable rules than with lengthy explanations. Each rule should be a directive ("Use `anyhow::Result`") not a discussion ("There are many ways to handle errors...").
