# Innovations and New Features: Implementation Plan

> 65 tasks across 15 feature areas, organized into 4 phases by dependency
> ordering and compounding leverage. Each task includes file paths, concrete
> steps, and acceptance criteria that can be verified mechanically.
>
> Sources: `10-INNOVATIONS-AND-NEW-FEATURES.md`, `07-RESEARCH-SYNTHESIS-1.md`,
> `08-RESEARCH-SYNTHESIS-2.md`.
>
> Naming: tasks are numbered INNOV-NNN. Dependencies reference these IDs.

---

## Phase 1: Foundations (Memory, Context, Cost)

These deliver immediate value by wiring existing infrastructure into live
paths. No new architectural patterns required -- just connections between
built-but-disconnected subsystems.

---

### INNOV-001: Create MemoryLayer struct wrapping three memory tiers

**Files**:
- `crates/roko-learn/src/memory_layer.rs` (new)
- `crates/roko-learn/src/lib.rs` (add `pub mod memory_layer`)

**What**: Unified read interface over working memory (per-session), episodic
memory (EpisodeLogger), and durable knowledge (KnowledgeStore + PlaybookStore).

**Steps**:
1. Define `MemoryLayer` struct holding references to `EpisodeLogger`,
   `KnowledgeStore`, and `PlaybookStore`.
2. Define `MemoryInjection` struct with fields: `playbooks: Vec<PlaybookEntry>`,
   `anti_patterns: Vec<String>`, `relevant_episodes: Vec<EpisodeSummary>`.
3. Implement `MemoryLayer::new(roko_dir: &Path) -> Result<Self>` that loads
   all three stores.
4. Stub `query_for_task(&self, ctx: &TaskContext) -> Result<MemoryInjection>`
   (implemented in INNOV-002).
5. Add `pub mod memory_layer` to `crates/roko-learn/src/lib.rs`.

**Acceptance criteria**:
- `MemoryLayer::new()` succeeds on a `.roko/` directory with episode and
  knowledge data.
- Struct compiles and is importable from `roko_learn::memory_layer`.
- Unit test: construct MemoryLayer with empty stores, call `query_for_task`,
  get empty MemoryInjection.

**Dependencies**: None
**Effort**: 0.5 day

---

### INNOV-002: Implement memory retrieval with token budget

**File**: `crates/roko-learn/src/memory_layer.rs`

**What**: Implement `query_for_task()` with three-tier retrieval:
1. Exact match on task ID from EpisodeLogger (Tier 1)
2. HDC fingerprint similarity scan of recent episodes (Tier 2)
3. Semantic search by domain tags in KnowledgeStore (Tier 3)

**Steps**:
1. In `query_for_task()`, query EpisodeLogger for task ID matches.
2. If HDC fingerprint available on task context, scan recent episodes for
   cosine similarity > 0.7. Weight by recency (half-life decay, configurable).
3. Query KnowledgeStore by domain tags. Include anti-knowledge as "avoid"
   guidance in the injection.
4. Query PlaybookStore for playbooks matching task category.
5. Enforce 2K token budget: rank all results by relevance score, truncate
   to fit budget.
6. Return populated `MemoryInjection`.

**Acceptance criteria**:
- After 10+ recorded episodes, `query_for_task` returns the 3-5 most relevant
  items, not all items.
- Anti-knowledge entries appear in the `anti_patterns` field.
- Total token count of returned injection is <= 2048 tokens.
- Unit test: insert 20 episodes, query, verify budget constraint.

**Dependencies**: INNOV-001
**Effort**: 1 day

---

### INNOV-003: Wire MemoryLayer into SystemPromptBuilder

**Files**:
- `crates/roko-compose/src/prompt_assembly_service.rs`
- `crates/roko-cli/src/dispatch_v2.rs` or runner v2 dispatch path

**What**: Inject `MemoryInjection` into SystemPromptBuilder as layers 6
(Techniques: successful playbooks) and 7 (Anti-patterns: failure patterns +
anti-knowledge).

**Steps**:
1. In the dispatch path, call `memory_layer.query_for_task(&task_context)`.
2. Format playbooks as layer 6 content: brief summaries with confidence scores.
3. Format anti-patterns as layer 7 content: "AVOID: {pattern} (seen {count}
   times, last failure: {date})".
4. Pass formatted sections to `PromptAssemblyService` as additional layers.
5. Respect existing token budget: memory injection sections participate in
   the VCG auction if enabled, otherwise appended within budget.

**Acceptance criteria**:
- Run `roko plan run` on a plan where prior runs recorded episodes.
- Inspect the system prompt (via `--verbose` or episode log): layers 6/7
  contain memory-derived content.
- A task that previously failed sees the failure pattern in its anti-patterns
  section.

**Dependencies**: INNOV-002
**Effort**: 0.5 day

---

### INNOV-004: Wire memory update on task completion

**File**: `crates/roko-learn/src/runtime_feedback.rs`

**What**: After each task attempt, update the memory layer:
- Success: extract playbook, ingest knowledge at Transient tier
- Failure: store error pattern, ingest anti-knowledge
- Either: update episode with outcome + HDC fingerprint
- Confirmation: if new knowledge confirms existing, boost confidence

**Steps**:
1. In the post-task feedback handler, check task outcome.
2. On success: call `PlaybookStore::upsert()` with the successful approach.
   Call `KnowledgeStore::ingest()` with extracted facts at Transient tier.
3. On failure: call `KnowledgeStore::ingest_anti()` with error pattern.
4. On either: compute HDC fingerprint from task context + outcome, store
   on the episode.
5. If the ingested knowledge matches an existing entry (HDC similarity > 0.9),
   boost the existing entry's confidence instead of creating a duplicate.

**Acceptance criteria**:
- Run a plan with 5 tasks. 3 succeed, 2 fail.
- After run: KnowledgeStore has 3 new entries (successes) and 2 anti-knowledge
  entries (failures).
- PlaybookStore has at least 1 new playbook from a successful task.
- A second run on the same plan shows memory injection from first run's data.

**Dependencies**: INNOV-001
**Effort**: 0.5 day

---

### INNOV-005: Implement progressive disclosure context levels

**File**: `crates/roko-compose/src/prompt_assembly_service.rs`

**What**: Define four disclosure levels for context sections and implement
automatic level selection based on model context window size.

**Steps**:
1. Define `DisclosureLevel` enum: `Essential`, `Standard`, `Extended`, `Full`.
2. Tag each SystemPromptBuilder section with a disclosure level:
   - Essential: task description, tool instructions, critical constraints
   - Standard: + role context, code context, recent history
   - Extended: + knowledge injection, playbooks, full file contents
   - Full: + anti-patterns, experimental sections, verbose examples
3. Implement `select_disclosure_level(model_context_window: usize,
   content_tokens: usize) -> DisclosureLevel`:
   - If content fits in 30% of window: Full
   - If content fits in 50% of window: Extended
   - If content fits in 70% of window: Standard
   - Otherwise: Essential (with aggressive trimming)
4. Wire into the prompt assembly path: before assembling, compute total
   content tokens, select level, filter sections by level.

**Acceptance criteria**:
- Dispatching to a model with 8K context window (e.g., Cerebras 8B) produces
  a prompt at Essential or Standard level.
- Dispatching to Claude Opus (200K) produces a Full-level prompt.
- The section-level filtering is visible in verbose output.
- No prompt exceeds 70% of the model's context window.

**Dependencies**: None
**Effort**: 1 day

---

### INNOV-006: Define ModelContextProfile and calibration data

**Files**:
- `crates/roko-compose/src/context_profile.rs` (new)
- `crates/roko-compose/src/lib.rs` (add module)

**What**: Per-model profiles recording context window size, optimal context
range, and quality degradation curve.

**Steps**:
1. Define `ModelContextProfile` struct: `model_slug: String`,
   `context_window: usize`, `sweet_spot_range: (usize, usize)`,
   `degradation_threshold: usize`, `calibrated: bool`.
2. Implement `ModelContextProfile::default_for(slug: &str)` with known
   values for Claude (Opus/Sonnet/Haiku), GPT, Gemini, Cerebras, Ollama.
3. Implement serde for persistence to `.roko/learn/model-profiles/{slug}.json`.
4. Implement `ModelContextProfile::optimal_size(&self, content_tokens: usize)
   -> usize` that returns the ideal context size for this model given the
   available content.

**Acceptance criteria**:
- `ModelContextProfile::default_for("claude-sonnet-4")` returns a profile
  with context_window = 200_000 and a reasonable sweet spot range.
- Profile serializes to and deserializes from JSON.
- Unit test: `optimal_size` returns a value within the sweet spot range for
  moderate content sizes.

**Dependencies**: None
**Effort**: 0.5 day

---

### INNOV-007: Wire cost-aware Pareto routing into CascadeRouter

**Files**:
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-learn/src/budget.rs` (new or extend existing)

**What**: Modify CascadeRouter to consider cost-quality trade-offs using the
existing Pareto frontier data, not just quality.

**Steps**:
1. In `CascadeRouter::route()`, retrieve the Pareto frontier of non-dominated
   models (already computed but not used for cost).
2. Compute `budget_pressure = remaining_budget / remaining_tasks`.
3. Filter candidates to `expected_quality >= quality_floor` (configurable,
   default 0.7).
4. Among viable candidates, sort by cost-per-token ascending.
5. Return `ModelPlan { primary: cheapest_viable, escalation: next_tier,
   max_escalations: f(budget_pressure) }`.
6. Add `quality_floor` to CascadeRouter configuration.

**Acceptance criteria**:
- With a tight budget ($0.10/task), the router selects Haiku or Cerebras
  over Opus/Sonnet.
- With an unconstrained budget, the router selects the highest-quality model.
- After 50+ observations, dominated models (high cost, low quality) are not
  selected. Verify via `cascade-router.json`.

**Dependencies**: None
**Effort**: 1 day

---

### INNOV-008: Implement per-plan budget manager

**Files**:
- `crates/roko-learn/src/budget.rs` (new)
- `crates/roko-cli/src/main.rs` (add `--max-cost` flag)

**What**: Track cumulative cost per plan run and enforce hard caps.

**Steps**:
1. Define `BudgetManager` struct: `plan_budget_usd: f64`, `spent_usd: f64`,
   `remaining_tasks: usize`, `task_costs: HashMap<TaskId, f64>`.
2. Implement `budget_for_task(task: &Task) -> TaskBudget`:
   - `target_usd = avg_remaining * complexity_multiplier`
   - `hard_cap_usd = avg_remaining * 3.0`
   - `allow_escalation = spent < budget * 0.7`
3. Implement `record_cost(task_id, cost_usd)` and `is_exceeded() -> bool`.
4. Add `--max-cost <USD>` flag to `roko plan run` and `roko run` commands.
5. Wire BudgetManager into the dispatch path: before dispatching, check
   `is_exceeded()`. If true, halt with a clear error message showing
   spent vs budget.

**Acceptance criteria**:
- `roko plan run --max-cost 1.00` halts when cumulative cost reaches $1.00.
- Error message shows: "Budget exceeded: $1.02 spent of $1.00 budget."
- Budget state persists across `--resume` runs.
- Without `--max-cost`, no budget enforcement (backward compatible).

**Dependencies**: None
**Effort**: 1 day

---

### INNOV-009: Implement semantic cache with BLAKE3 exact match

**Files**:
- `crates/roko-learn/src/semantic_cache.rs` (new)
- `crates/roko-learn/src/lib.rs`

**What**: Before dispatching to an LLM, check if a semantically identical
prompt was already answered. Exact match via BLAKE3 hash of the prompt.

**Steps**:
1. Define `SemanticCache` struct with `exact: HashMap<[u8; 32], CachedResponse>`.
2. `CachedResponse`: `response: String`, `model: String`, `created_at: DateTime`,
   `ttl: Duration`, `task_category: TaskCategory`.
3. Implement `check(prompt: &str) -> Option<CachedResponse>`:
   - Compute BLAKE3 hash of prompt.
   - Look up in exact map. If found and not expired, return.
4. Implement `store(prompt: &str, response: &AgentResponse, ttl: Duration)`:
   - Only cache deterministic tasks (compile fixes, format, simple edits).
   - Never cache creative/architectural tasks.
5. Persist cache to `.roko/cache/semantic.json` with TTL-based eviction.
6. Wire into dispatch path: check cache before LLM call, store after.

**Acceptance criteria**:
- Run the same fix task twice. Second run hits exact cache, zero LLM cost.
- Second run response time < 100ms.
- Cache does not store creative/architectural task responses.
- Expired entries are evicted on next check.

**Dependencies**: None
**Effort**: 1.5 days

---

### INNOV-010: Add HDC fuzzy matching to semantic cache

**File**: `crates/roko-learn/src/semantic_cache.rs`

**What**: Extend semantic cache with HDC fingerprint fuzzy matching for
prompts that are similar but not identical (e.g., same fix with slightly
different line numbers).

**Steps**:
1. Add `fuzzy: Vec<(HdcVector, CachedResponse)>` to `SemanticCache`.
2. On cache miss for exact match, compute HDC fingerprint of prompt.
3. Scan fuzzy entries for cosine similarity > 0.95 (configurable threshold).
4. If match found, validate applicability: check that the cached response's
   code context overlaps with the current context (file paths, function names).
5. On store, also insert into fuzzy index.
6. Limit fuzzy index to 1000 entries (LRU eviction).

**Acceptance criteria**:
- A compile fix for `foo.rs:42` gets cached. A subsequent fix for `foo.rs:45`
  with the same error type hits the fuzzy cache.
- False positive rate < 5% on a test suite of 50 similar-but-different prompts.
- Fuzzy match latency < 10ms for 1000 entries.

**Dependencies**: INNOV-009
**Effort**: 1 day

---

### INNOV-011: Implement prompt compression pipeline

**File**: `crates/roko-compose/src/compressor.rs` (new)

**What**: Reduce token count while preserving semantic content, using three
strategies applied in sequence.

**Steps**:
1. Strategy 1: Strip cosmetic content -- redundant whitespace, code comments
   in examples, duplicate section headers. Regex-based, no LLM needed.
2. Strategy 2: Summarize long code blocks -- if a code block exceeds 200
   tokens, replace with function signature + docstring + "// ... N lines".
3. Strategy 3: Drop lowest-effectiveness sections -- if section_effect data
   is available (from learning), drop sections with the lowest measured lift
   first. Never drop task description or tool instructions.
4. Implement `compress(prompt: &str, target_tokens: usize) -> String`.
5. Wire into prompt assembly when the computed prompt exceeds the model's
   optimal context size (from ModelContextProfile).

**Acceptance criteria**:
- A 15K-token prompt compressed for an 8K-context model produces output
  <= 5.6K tokens (70% of window).
- Compressed prompt retains task description and tool instructions verbatim.
- Code blocks > 200 tokens are summarized to < 50 tokens.
- Unit test: compress a known prompt, verify output is valid and smaller.

**Dependencies**: INNOV-005, INNOV-006
**Effort**: 1 day

---

### INNOV-012: Add `roko learn costs` CLI command

**Files**:
- `crates/roko-cli/src/commands/learn.rs`
- `crates/roko-cli/src/main.rs`

**What**: CLI command showing per-task cost breakdown, cost-per-gate-pass,
and model cost distribution.

**Steps**:
1. Read `.roko/learn/efficiency.jsonl` for cost data.
2. Aggregate: total cost, per-task cost, per-model cost distribution.
3. Compute cost-per-gate-pass: total cost / number of gate passes.
4. Display as a formatted table with columns: Task, Model, Cost, Gate Pass,
   Cost/Pass.
5. Show model distribution as a simple bar chart (Unicode blocks).

**Acceptance criteria**:
- `roko learn costs` displays a table after at least one run with cost data.
- Per-model breakdown sums to total cost (within rounding).
- Cost-per-gate-pass is computed correctly.

**Dependencies**: None
**Effort**: 0.5 day

---

## Phase 2: Self-Improvement (Gates, Debugging, Steering)

These build on Phase 1 foundations. They require the memory layer and cost
infrastructure to be in place.

---

### INNOV-013: Create GateEvolver for failure-pattern-driven gate generation

**Files**:
- `crates/roko-gate/src/gate_evolver.rs` (new)
- `crates/roko-gate/src/lib.rs`

**What**: When specific failure patterns recur 3+ times, automatically
generate a targeted pre-flight check (ShellGate) for that pattern.

**Steps**:
1. Define `GateEvolver` struct holding a reference to `ErrorPatternStore`.
2. Implement `evolve_gates(&self, patterns: &[ErrorPattern]) -> Vec<GeneratedGate>`:
   - For each pattern with count >= 3, generate a ShellGate:
     - "unused import" -> `grep -rn "^use.*unused" {files}`
     - "missing semicolon" -> targeted syntax check
     - "type mismatch" -> focused `cargo check` on changed files only
3. Define `GeneratedGate`: `name: String`, `shell_command: String`,
   `target_pattern: String`, `created_from: ErrorPatternId`, `effectiveness: f64`.
4. Implement `should_retire(&self, gate: &GeneratedGate) -> bool`:
   - Retire if 3+ consecutive false positives.
5. Persist generated gates to `.roko/learn/gate-evolution.json`.

**Acceptance criteria**:
- After 5 runs with recurring "unused import" failures, a targeted grep-based
  gate exists in `gate-evolution.json`.
- Generated gate runs in < 100ms vs clippy's 3-8 seconds.
- A gate with 3+ consecutive false positives is marked `retired: true`.

**Dependencies**: None
**Effort**: 1.5 days

---

### INNOV-014: Wire generated gates into GateService runtime

**Files**:
- `crates/roko-gate/src/gate_service.rs`
- `crates/roko-core/src/config/` (gate config extension)

**What**: Load generated gates from `gate-evolution.json` and insert them
as pre-flight checks before the standard rung pipeline.

**Steps**:
1. In `GateService::run_gates()`, load generated gates from file.
2. Filter to non-retired gates whose target pattern matches the current diff.
3. Run matching generated gates as rung 0 (before compile).
4. If a generated gate catches the issue, skip the expensive standard rung.
5. Record generated gate outcomes for effectiveness tracking.

**Acceptance criteria**:
- A generated "unused import" gate fires before clippy and catches the issue.
- Gate report shows the generated gate ran at rung 0.
- If the generated gate passes but clippy later catches the same issue, the
  generated gate's effectiveness score decreases.

**Dependencies**: INNOV-013
**Effort**: 0.5 day

---

### INNOV-015: Implement DiffAnalyzer for rung relevance

**File**: `crates/roko-gate/src/diff_analyzer.rs` (new)

**What**: Analyze the diff to determine which gate rungs are relevant. A diff
touching only documentation files should skip compile, clippy, and test gates.

**Steps**:
1. Define `DiffAnalysis` struct: `files_changed: Vec<PathBuf>`,
   `categories: HashSet<FileCategory>`, `estimated_complexity: Complexity`.
2. `FileCategory` enum: `Source`, `Test`, `Documentation`, `Config`, `Build`.
3. Implement `analyze_diff(diff: &str) -> DiffAnalysis` using file extension
   and path heuristics.
4. Implement `relevant_rungs(analysis: &DiffAnalysis) -> Vec<RungId>`:
   - Documentation-only: format + diff only
   - Config-only: format + diff + validate
   - Test-only: format + test + diff
   - Source: all rungs
5. Wire into `GateService::run_gates()`: skip irrelevant rungs.

**Acceptance criteria**:
- A diff touching only `.md` files skips compile, clippy, and test gates.
- A diff touching only test files skips clippy but runs test and format.
- Gate report shows which rungs were skipped with reason "irrelevant to diff".

**Dependencies**: None
**Effort**: 1 day

---

### INNOV-016: Track gate effectiveness metrics

**Files**:
- `crates/roko-learn/src/gate_effectiveness.rs` (new)
- `crates/roko-learn/src/lib.rs`

**What**: Track precision, recall, and F1 per gate rung over time.

**Steps**:
1. Define `GateEffectiveness` struct: `rung_id`, `true_positives: u64`,
   `false_positives: u64`, `true_negatives: u64`, `false_negatives: u64`.
2. Compute precision = TP / (TP + FP), recall = TP / (TP + FN).
3. A "true positive" is: gate fails AND the issue was real (confirmed by
   human or autofix succeeding after addressing the flagged issue).
4. A "false positive" is: gate fails AND the fix attempt succeeds without
   addressing the flagged issue (the gate was wrong).
5. Persist to `.roko/learn/gate-effectiveness.json`.
6. Add `roko learn gates` CLI showing effectiveness report.

**Acceptance criteria**:
- After 20+ runs, `roko learn gates` shows precision/recall per rung.
- At least one gate with precision < 0.5 is flagged for review.
- Effectiveness data persists across runs.

**Dependencies**: None
**Effort**: 1 day

---

### INNOV-017: Define FailureKind taxonomy for agent debugging

**File**: `crates/roko-learn/src/failure_taxonomy.rs` (new)

**What**: Classify agent failures into a taxonomy for systematic debugging.

**Steps**:
1. Define `FailureKind` enum with variants:
   - `QualityFailure { gate_rung, error_hash, is_recurring }`
   - `ConvergenceFailure { iterations, repeated_error_hashes }`
   - `ResourceFailure { kind: Resource, used, limit }`
   - `ToolFailure { tool_name, error, is_permission }`
   - `ComprehensionFailure { evidence }`
2. Implement `classify(task_result: &TaskResult, episodes: &[Episode]) -> FailureKind`:
   - Check for repeated error hashes -> ConvergenceFailure
   - Check for budget/context exceeded -> ResourceFailure
   - Check for tool errors -> ToolFailure
   - Check for wrong-direction changes -> ComprehensionFailure
   - Default to QualityFailure
3. Add `is_recurring` check against past episodes.

**Acceptance criteria**:
- A task that fails 3 times with the same error is classified as
  ConvergenceFailure.
- A task that runs out of tokens is classified as ResourceFailure.
- A task where `bash` returns permission denied is classified as ToolFailure
  with `is_permission: true`.
- Unit tests for each variant.

**Dependencies**: None
**Effort**: 0.5 day

---

### INNOV-018: Implement debugger hypothesis generation

**Files**:
- `crates/roko-learn/src/debug_engine.rs` (new)
- `crates/roko-learn/src/lib.rs`

**What**: Given a classified failure, generate ranked hypotheses about the
root cause and propose configuration interventions.

**Steps**:
1. Define `Hypothesis` struct: `cause: String`, `confidence: f64`,
   `intervention: Intervention`, `evidence: Vec<String>`.
2. Define `Intervention` enum: `RouteToModel(String)`, `AddContext(String)`,
   `FixPermissions(RoleChange)`, `AdjustPrompt(SectionChange)`,
   `TuneGate(ThresholdChange)`.
3. Implement `generate_hypotheses(failure: &FailureKind, context: &TaskContext,
   history: &[Episode]) -> Vec<Hypothesis>`:
   - ConvergenceFailure -> ["missing context", "wrong model", "prompt interference"]
   - ResourceFailure -> ["context too large", "model too expensive"]
   - ToolFailure -> ["permission mismatch", "tool not available"]
   - QualityFailure -> ["wrong model tier", "missing relevant code"]
4. Rank hypotheses by: recurrence in history, similarity to past interventions
   that worked (from PlaybookStore).

**Acceptance criteria**:
- A ConvergenceFailure produces at least 3 ranked hypotheses.
- Hypotheses include actionable interventions, not just descriptions.
- A previously successful intervention ranks higher than novel ones.

**Dependencies**: INNOV-017
**Effort**: 1 day

---

### INNOV-019: Wire debugging into gate failure handler

**Files**:
- `crates/roko-runtime/src/workflow_engine.rs` or runner v2 gate handler
- `crates/roko-learn/src/debug_engine.rs`

**What**: When gate failures exhaust the autofix budget, invoke the debug
engine to classify the failure, generate hypotheses, apply the top
intervention, and retry.

**Steps**:
1. After autofix budget exhausted, call `classify()` on the failure.
2. Call `generate_hypotheses()` with the classified failure.
3. Apply the top hypothesis's intervention:
   - RouteToModel -> override model for retry
   - AddContext -> inject additional context into prompt
   - FixPermissions -> adjust role manifest for retry
   - AdjustPrompt -> modify section weights
4. Retry the task with the intervention applied.
5. If retry succeeds: record the intervention as a playbook entry.
6. If retry fails: try next hypothesis (up to 3 attempts).
7. If all hypotheses fail: generate a debug report and write to
   `.roko/debug/{task_id}.md`.

**Acceptance criteria**:
- A task that fails 3 times with "missing module" -> debug engine adds repo
  tree context -> retry succeeds.
- Successful intervention is saved as a playbook.
- Debug report is written for failures that exhaust all hypotheses.
- `roko debug <task-id>` displays the debug report.

**Dependencies**: INNOV-017, INNOV-018, INNOV-003
**Effort**: 1.5 days

---

### INNOV-020: Define SteeringAction primitives

**File**: `crates/roko-core/src/steering.rs` (new)

**What**: Define the core types for interactive steering of running agents.

**Steps**:
1. Define `SteeringAction` enum:
   - `Redirect { guidance: String, model_override: Option<String> }`
   - `Skip { reason: String }`
   - `Split { sub_tasks: Vec<TaskSpec> }`
   - `BudgetAdjust { remaining_budget_usd: f64, model_preference: Option<String> }`
   - `InjectContext { content: String, priority: ContextPriority }`
   - `ReviewVerdict { task_id: String, verdict: Verdict, notes: String }`
2. Define `ContextPriority` enum: `Override`, `Append`, `Background`.
3. Define `ConfidenceThresholds` struct:
   - `auto_proceed: f64` (default 0.85)
   - `suggest_review: f64` (default 0.50)
   - `require_approval: f64` (default 0.50)
4. Define `SteeringAuditEntry` for the audit trail.
5. Implement serde for all types.

**Acceptance criteria**:
- All types compile and serialize to/from JSON.
- Unit test: round-trip serialization for each SteeringAction variant.

**Dependencies**: None
**Effort**: 0.5 day

---

### INNOV-021: Implement steering channel

**File**: `crates/roko-runtime/src/steering.rs` (new)

**What**: A tokio mpsc channel that allows steering actions to be injected
into a running execution loop.

**Steps**:
1. Define `SteeringChannel` wrapping `(mpsc::Sender<SteeringAction>,
   mpsc::Receiver<SteeringAction>)`.
2. Implement `SteeringChannel::new(buffer: usize) -> (SteeringSender,
   SteeringReceiver)`.
3. In the workflow engine's main loop, poll the steering receiver at each
   iteration alongside the agent task.
4. On receiving a `SteeringAction`:
   - `Redirect` -> inject guidance into the agent's next prompt
   - `Skip` -> mark task as deferred, move to next
   - `BudgetAdjust` -> update BudgetManager
   - `InjectContext` -> append to current prompt context
5. Record every steering action to `.roko/steer/audit.jsonl`.

**Acceptance criteria**:
- Sending a `Redirect` action via the channel injects guidance into the next
  agent prompt iteration.
- Sending a `Skip` action stops the current task and moves to the next.
- Audit trail records all steering actions with timestamps.
- Channel is non-blocking: the execution loop continues if no steering action
  is pending.

**Dependencies**: INNOV-020
**Effort**: 1 day

---

### INNOV-022: Implement confidence scoring for tasks

**File**: `crates/roko-learn/src/confidence.rs` (new)

**What**: Compute confidence score for a task to determine whether to proceed
automatically, suggest review, or require approval.

**Steps**:
1. Define `ConfidenceScore` struct: `value: f64`, `components: Vec<(String, f64)>`.
2. Implement `compute_confidence(task: &Task, memory: &MemoryInjection,
   thresholds: &AdaptiveThresholds) -> ConfidenceScore`:
   - Component 1: task complexity vs model capability (from ModelContextProfile)
   - Component 2: similarity to past successes (from memory layer)
   - Component 3: expected gate pass probability (from adaptive thresholds)
   - Component 4: error pattern match (similar task failed before)
3. Weighted average of components (configurable weights).
4. Compare against `ConfidenceThresholds` to determine action:
   `AutoProceed`, `SuggestReview`, `RequireApproval`.

**Acceptance criteria**:
- A task with many similar past successes scores > 0.85.
- A task touching unfamiliar code with no history scores < 0.5.
- Confidence is logged per task in the episode data.

**Dependencies**: INNOV-002, INNOV-020
**Effort**: 1 day

---

### INNOV-023: Add HTTP steering endpoints

**File**: `crates/roko-serve/src/routes/steering.rs` (new)

**What**: REST API for steering running agents from external clients.

**Steps**:
1. `POST /api/steer/{task_id}` -- accept `SteeringAction` JSON body, send
   to steering channel.
2. `GET /api/confidence` -- return `Vec<ConfidenceReport>` for all active
   tasks.
3. `POST /api/approve/{task_id}` -- shorthand for `ReviewVerdict` with
   approve.
4. Wire into existing `roko-serve` router.
5. Return 404 if task_id is not active, 409 if task already completed.

**Acceptance criteria**:
- `POST /api/steer/task-07 { "action": "redirect", "guidance": "..." }`
  injects context into the running agent. Response includes agent state.
- `GET /api/confidence` returns confidence scores for active tasks.
- 401 returned for unauthenticated requests (when auth enabled).

**Dependencies**: INNOV-021, INNOV-022
**Effort**: 1 day

---

### INNOV-024: Add TUI steering panel (F8)

**File**: `crates/roko-cli/src/tui/modals/steering.rs` (new or extend)

**What**: TUI modal for interactive steering during `roko plan run`.

**Steps**:
1. Bind F8 to open the steering panel.
2. Panel shows: current task, confidence score, agent state.
3. Key bindings within panel:
   - `s`: redirect (text input for guidance)
   - `k`: skip current task
   - `b`: adjust budget (numeric input)
   - `c`: inject context (text input)
   - `Esc`: close panel
4. On action, send via `SteeringSender` to the execution loop.
5. Show confirmation: "Steering action applied: Redirect sent to task-07".

**Acceptance criteria**:
- F8 opens the steering panel during a running plan.
- Pressing `s` and typing guidance redirects the running agent.
- Panel shows current task confidence score.
- Esc closes without action.

**Dependencies**: INNOV-021, INNOV-022
**Effort**: 1.5 days

---

## Phase 3: Multi-Agent and Speculative Execution

These require parallel infrastructure and are higher complexity. They build
on the memory, cost, and steering foundations from Phases 1-2.

---

### INNOV-025: Implement competitive proposals (Best-of-N)

**Files**:
- `crates/roko-orchestrator/src/competitive.rs` (new)
- `crates/roko-orchestrator/src/lib.rs`

**What**: For a given task, spawn N agents with different models/approaches
in separate worktrees. Gate pipeline scores all N. Best wins.

**Steps**:
1. Define `CompetitiveRunner` struct with `proposal_count: usize` (default 3).
2. Implement `run_competitive(task: &Task, n: usize) -> Vec<ProposalResult>`:
   - Allocate N worktrees via WorktreeManager.
   - Spawn N agents concurrently (different models or different prompts).
   - Run gate pipeline on each completed proposal.
   - Rank by gate score. Select winner.
   - Clean up losing worktrees.
3. Wire into the dispatch path: if `--collaboration=competitive` is set,
   use CompetitiveRunner instead of single dispatch.
4. Track all proposals in the episode log with a `proposal_group` field.

**Acceptance criteria**:
- `roko plan run --collaboration=competitive --proposals=3` spawns 3
  implementers in separate worktrees.
- Gate pipeline scores all 3. Best wins. Others are cleaned up.
- TUI shows all proposals with scores.
- Episode log records all 3 proposals with the same `proposal_group`.

**Dependencies**: INNOV-008 (budget needed for cost control of N proposals)
**Effort**: 2 days

---

### INNOV-026: Implement swarm collaboration pattern

**Files**:
- `crates/roko-orchestrator/src/swarm.rs` (new)
- `crates/roko-orchestrator/src/lib.rs`

**What**: Concurrent implementer + reviewer with real-time message exchange
via a shared signal channel.

**Steps**:
1. Define `SwarmRunner` with `roles: Vec<AgentRole>`.
2. Implement shared message channel (tokio broadcast) for inter-agent
   communication.
3. Implementer agent runs in its worktree. Reviewer watches the diff stream.
4. If reviewer detects an issue mid-implementation, inject a `Redirect`
   steering action into the implementer's context.
5. Both agents share a signal bus: implementer emits progress signals,
   reviewer emits correction signals.
6. Task completes when implementer finishes AND reviewer approves.

**Acceptance criteria**:
- Swarm mode: reviewer catches a bug mid-implementation, implementer receives
  the correction before finishing. Verify via episode log timestamps.
- If reviewer never objects, task completes at normal speed (no overhead).
- Signal bus messages are recorded in the episode log.

**Dependencies**: INNOV-021 (steering channel for cross-agent messages)
**Effort**: 2 days

---

### INNOV-027: Implement specialist mode for multi-crate changes

**Files**:
- `crates/roko-orchestrator/src/specialist.rs` (new)
- `crates/roko-orchestrator/src/lib.rs`

**What**: For changes spanning multiple crates, spawn one specialist agent
per crate plus a merge coordinator.

**Steps**:
1. Define `SpecialistRunner` that analyzes the task to identify affected
   crates.
2. Spawn one specialist agent per crate, each in its own worktree.
3. Each specialist works only on its crate's files.
4. A merge coordinator agent reconciles cross-crate interface changes:
   - Collects all specialists' diffs.
   - Resolves conflicts (type mismatches across crate boundaries).
   - Produces the final merged diff.
5. Gate pipeline runs on the merged result.

**Acceptance criteria**:
- A 5-file cross-crate change spawns 3 specialists (one per crate) plus a
  merge agent.
- Total wall time < 1.5x single-agent time.
- Cross-crate type mismatches are resolved by the merge coordinator.

**Dependencies**: INNOV-025 (worktree infrastructure)
**Effort**: 2 days

---

### INNOV-028: Implement SpeculativeFixRunner for parallel gate fixes

**Files**:
- `crates/roko-orchestrator/src/speculative.rs` (new)
- `crates/roko-orchestrator/src/lib.rs`

**What**: On gate failure, spawn N fix agents in parallel with different
models/strategies. First to pass gates wins.

**Steps**:
1. Define `SpeculativeFixRunner` with `max_parallel_fixes: usize` (default 3).
2. Implement error complexity classifier:
   - Trivial (unused import, format) -> single haiku agent, no speculation.
   - Moderate (type mismatch, missing impl) -> 2 parallel: haiku + sonnet.
   - Complex (logic error, architectural) -> 3 parallel: sonnet x2 + opus.
3. On `GateFailed`, classify error and spawn parallel fix agents.
4. Use `CancelToken` from roko-runtime: first agent to pass gates cancels
   the others.
5. Feed failed attempt context as anti-pattern to surviving agents.
6. Track speculation outcomes for learning.

**Acceptance criteria**:
- A compile error fix spawns 2 parallel agents. The faster fix passes first,
  the other is cancelled. Total wall time < sequential retry.
- Speculation cost for 3 parallel haiku runs < 1 sonnet run.
- After 10+ speculative runs, the system learns which error categories
  benefit from speculation.

**Dependencies**: INNOV-008 (budget control), INNOV-017 (failure classification)
**Effort**: 2 days

---

### INNOV-029: Implement speculative prefetch for DAG tasks

**File**: `crates/roko-orchestrator/src/dag.rs`

**What**: While task N executes, speculatively prepare task N+1 by resolving
dependencies, pre-building the system prompt, and pre-spawning a warm agent.

**Steps**:
1. In the DAG executor's main loop, identify the next task(s) after the
   current batch.
2. For each candidate next task:
   - Resolve dependencies from the DAG.
   - Pre-build system prompt layers 1-3 (stable across tasks).
   - Pre-spawn a warm agent (connect to LLM, don't send prompt yet).
   - Pre-fetch code context for the candidate task.
3. If current task succeeds, hand off to the pre-warmed agent immediately.
4. If current task fails, discard the prefetch (context may have changed).
5. Add `--speculate` flag to `plan run` to enable speculative prefetch.

**Acceptance criteria**:
- DAG with 5 sequential tasks: speculative prefetch prepares task N+1 while
  N executes. Cold-start time for tasks 2-5 is < 200ms vs ~800ms without.
- Discarded prefetches do not leak resources (agents, worktrees).
- Prefetch is disabled by default; enabled with `--speculate`.

**Dependencies**: None
**Effort**: 1.5 days

---

### INNOV-030: Implement density-threshold gating for multi-agent

**File**: `crates/roko-orchestrator/src/dag.rs`

**What**: Before spawning a multi-agent plan, check that estimated agent
density exceeds rho_c = 0.23. If below, fall back to sequential single-agent.

**Steps**:
1. Define `agent_density(num_agents: usize, num_tasks: usize,
   interaction_edges: usize) -> f64`.
2. Before spawning parallel agents, compute density.
3. If density < 0.23, log warning and fall back to sequential execution.
4. Track density vs outcome in efficiency events for future calibration.
5. Add `multi_agent.density_threshold` to configuration.

**Acceptance criteria**:
- A plan with 2 agents and 20 tasks (density ~0.1) falls back to sequential.
- A plan with 5 agents and 10 tasks (density ~0.5) proceeds with multi-agent.
- Warning logged when density is below threshold.

**Dependencies**: None
**Effort**: 0.5 day

---

## Phase 4: Cross-Cutting, Safety, and Interoperability

These are strategic features that require the earlier phases to be stable.
They include cross-project learning, A2A interoperability, CaMeL safety,
and research-derived enhancements.

---

### INNOV-031: Implement cross-project global config directory

**Files**:
- `crates/roko-core/src/config/` (extend)

**What**: Create `~/.roko/` as the global config directory for cross-project
knowledge, model meta-knowledge, and shared configuration.

**Steps**:
1. Define `GlobalConfig` struct with paths for: `domains/`, `meta/`,
   `cache/`, `community/`.
2. Implement `GlobalConfig::ensure(home_dir: &Path) -> Result<Self>` that
   creates the directory structure.
3. Wire into CLI startup: ensure global dir exists before loading project
   config.
4. Add `global_dir` to the paths available in the runtime context.

**Acceptance criteria**:
- After running any `roko` command, `~/.roko/` exists with subdirectories.
- Global config is loadable from any project.
- Does not interfere with project-local `.roko/` directory.

**Dependencies**: None
**Effort**: 0.5 day

---

### INNOV-032: Implement domain detection for projects

**File**: `crates/roko-cli/src/repo_context.rs`

**What**: Detect the domain tags of the current project from file presence.

**Steps**:
1. Define `DomainTag` enum: `Rust`, `TypeScript`, `JavaScript`, `Python`,
   `Go`, `Blockchain`, `React`, `WebApp`, etc.
2. Implement `detect_domains(workdir: &Path) -> Vec<DomainTag>`:
   - `Cargo.toml` -> Rust
   - `package.json` -> JavaScript
   - `tsconfig.json` -> TypeScript
   - `pyproject.toml` or `requirements.txt` -> Python
   - `go.mod` -> Go
   - `foundry.toml` -> Blockchain
3. Cache the result for the session.
4. Make domain tags available to the dispatch path and memory layer.

**Acceptance criteria**:
- In the roko project (Rust), `detect_domains()` returns `[Rust]`.
- In a project with both `Cargo.toml` and `package.json`, returns both tags.
- Domain tags are logged in verbose output.

**Dependencies**: None
**Effort**: 0.5 day

---

### INNOV-033: Implement tiered KnowledgeStore for cross-project sharing

**Files**:
- `crates/roko-neuro/src/tiered_store.rs` (new)
- `crates/roko-neuro/src/lib.rs`

**What**: Multi-tier knowledge store wrapper that queries project-local,
domain-shared, and global meta-knowledge.

**Steps**:
1. Define `TieredKnowledgeStore` wrapping:
   - Tier 0: project-specific (`.roko/neuro/knowledge.jsonl`)
   - Tier 1: domain-specific (`~/.roko/domains/{domain}/knowledge.jsonl`)
   - Tier 2: model meta-knowledge (`~/.roko/meta/model-knowledge.jsonl`)
2. Implement `query(topic: &str, domains: &[DomainTag]) -> Vec<KnowledgeEntry>`:
   - Always include Tier 0 and Tier 2.
   - Include Tier 1 only if domain tags match.
   - Rank results by confidence, with Tier 0 having a small boost.
3. Implement conflict resolution: if entries conflict across tiers, use
   confidence score. If tied, prefer the more specific tier.

**Acceptance criteria**:
- Query from a Rust project returns Rust-specific Tier 1 knowledge.
- Query from a TypeScript project does NOT return Rust-specific knowledge.
- Model meta-knowledge (Tier 2) is available in all projects.
- Conflicting entries: higher-confidence entry wins.

**Dependencies**: INNOV-031, INNOV-032
**Effort**: 1.5 days

---

### INNOV-034: Implement knowledge tier promotion logic

**File**: `crates/roko-neuro/src/tiered_store.rs`

**What**: After a run completes, promote generalizable knowledge from
project-local to domain or global tiers.

**Steps**:
1. On run completion, scan new knowledge entries from Tier 0.
2. Filter: exclude entries containing project-specific paths, variable names,
   or secrets.
3. Classify remaining by tier:
   - About model/tool behavior -> Tier 2
   - About language/framework patterns -> Tier 1
   - Project-specific -> stay at Tier 0
4. Promote if:
   - Confidence > 0.8
   - Pattern matches domain tags (for Tier 1)
   - Not path-dependent
   - For Tier 2: confirmed across 2+ domains
5. Implement path/secret scrubbing: remove absolute paths, replace with
   placeholders, strip anything matching known secret patterns.

**Acceptance criteria**:
- After roko learns "Cerebras fails on async trait impls" in project A,
  the entry appears in `~/.roko/meta/model-knowledge.jsonl`.
- After roko learns a Rust-specific pattern in project A, it appears in
  `~/.roko/domains/rust/knowledge.jsonl`.
- Promoted entries contain no absolute paths or project-specific identifiers.
- An entry with confidence < 0.8 is not promoted.

**Dependencies**: INNOV-033
**Effort**: 1 day

---

### INNOV-035: Add `roko knowledge export/import` commands

**Files**:
- `crates/roko-cli/src/commands/learn.rs` or new subcommand
- `crates/roko-cli/src/main.rs`

**What**: Export scrubbed knowledge for sharing; import external knowledge.

**Steps**:
1. `roko knowledge export [--domain <tag>] [--tier <n>] -o <file>`:
   - Export entries matching filters.
   - Apply full scrubbing pipeline.
   - Output as JSON.
2. `roko knowledge import <file> [--tier <n>]`:
   - Validate entries.
   - Import at specified tier (default: Tier 1).
   - Merge with existing (boost confidence if duplicate).
3. `roko knowledge domains`:
   - List all domain stores with entry counts.

**Acceptance criteria**:
- `roko knowledge export` produces a JSON file with no absolute paths.
- `roko knowledge import` adds entries to the appropriate store.
- `roko knowledge domains` lists domains with counts.

**Dependencies**: INNOV-033, INNOV-034
**Effort**: 1 day

---

### INNOV-036: Define A2A core types

**File**: `crates/roko-core/src/a2a.rs` (new)

**What**: Rust types for A2A v1.0 protocol: AgentCard, Task, Artifact,
Message, Skill.

**Steps**:
1. Define `AgentCard` struct matching A2A spec:
   `name`, `description`, `url`, `version`, `capabilities`, `skills`,
   `authentication`.
2. Define `A2ASkill`: `id`, `name`, `description`, `input_modes`,
   `output_modes`.
3. Define `A2ATask`: `id`, `status`, `messages`, `artifacts`.
4. Define `A2AArtifact`: `name`, `content_type`, `data`.
5. Implement JSON-RPC 2.0 request/response types for A2A methods:
   `tasks/send`, `tasks/get`, `tasks/cancel`, `tasks/sendSubscribe`.
6. Implement serde for all types, validated against A2A v1.0 spec.

**Acceptance criteria**:
- All types serialize to JSON matching the A2A v1.0 spec.
- Round-trip test: serialize, deserialize, compare equality.
- AgentCard includes roko's 4 skills: code-implementation, code-review,
  gate-verification, knowledge-query.

**Dependencies**: None
**Effort**: 1 day

---

### INNOV-037: Implement A2A Agent Card endpoint

**File**: `crates/roko-serve/src/routes/a2a.rs` (new)

**What**: Serve the A2A Agent Card at `/.well-known/agent.json`.

**Steps**:
1. Implement `GET /.well-known/agent.json` route.
2. Generate AgentCard from roko-serve configuration:
   - Skills derived from configured agent roles.
   - URL from server bind address.
   - Authentication from configured auth scheme.
3. Add route to the serve router.

**Acceptance criteria**:
- `GET http://localhost:6677/.well-known/agent.json` returns a valid A2A
  Agent Card.
- Card includes 4 skills with correct descriptions.
- Card validates against A2A v1.0 JSON schema.

**Dependencies**: INNOV-036
**Effort**: 0.5 day

---

### INNOV-038: Implement A2A JSON-RPC endpoint (server)

**File**: `crates/roko-serve/src/routes/a2a.rs`

**What**: Handle incoming A2A task requests via JSON-RPC 2.0.

**Steps**:
1. Implement `POST /a2a` route accepting JSON-RPC 2.0 requests.
2. Dispatch based on method:
   - `tasks/send` -> create internal task, run agent, return result.
   - `tasks/get` -> return task status.
   - `tasks/cancel` -> cancel running task via CancelToken.
   - `tasks/sendSubscribe` -> create task with SSE updates.
3. Map A2A skills to internal AgentRole dispatch:
   - `code-implementation` -> Implementer role
   - `code-review` -> Reviewer role
   - `gate-verification` -> GateService direct call
   - `knowledge-query` -> KnowledgeStore query
4. Stream progress updates via SSE for `sendSubscribe`.
5. Return A2A-compliant task completion with artifacts.

**Acceptance criteria**:
- External A2A client sends `tasks/send` with code-implementation skill.
  Roko executes, runs gates, returns result.
- `tasks/cancel` correctly cancels a running agent.
- SSE stream shows progress updates for subscribed tasks.
- Invalid JSON-RPC returns proper error response.

**Dependencies**: INNOV-036, INNOV-037
**Effort**: 1.5 days

---

### INNOV-039: Implement A2A client for external agent delegation

**Files**:
- `crates/roko-agent/src/a2a_client.rs` (new)
- `crates/roko-agent/src/lib.rs`

**What**: Discover external agents via A2A Agent Card URLs and delegate
tasks to them.

**Steps**:
1. Define `A2AClient` struct with `reqwest::Client`.
2. Implement `discover(card_url: &str) -> Result<AgentCard>`:
   - Fetch Agent Card from URL.
   - Parse and validate.
   - Cache for session.
3. Implement `send_task(card: &AgentCard, skill_id: &str, input: &str)
   -> Result<A2ATask>`:
   - Construct JSON-RPC request.
   - Send to agent URL.
   - Poll for completion or subscribe via SSE.
4. Add `[a2a.agents]` config section to roko.toml:
   ```toml
   [a2a.agents]
   researcher = "https://research-agent.example.com/.well-known/agent.json"
   ```
5. At dispatch time, check if task domain matches an A2A agent's skills.
   If so, delegate via A2A. Fallback to local agent on failure.

**Acceptance criteria**:
- Configure an external agent in roko.toml. When a matching task appears,
  roko delegates via A2A.
- Delegation failure falls back to local agent.
- `roko agent discover <url>` displays the external agent's capabilities.

**Dependencies**: INNOV-036
**Effort**: 1.5 days

---

### INNOV-040: Implement CaMeL-style privileged/quarantined LLM split

**Files**:
- `crates/roko-agent/src/safety/` (extend existing safety module)

**What**: Separate LLM invocations into privileged (policy enforcement, gate
evaluation) and quarantined (untrusted input handling) trust domains.

**Steps**:
1. Define `TrustDomain` enum: `Privileged`, `Quarantined`.
2. In the dispatch path, tag each LLM call with its trust domain:
   - Quarantined: agent implementation, tool output processing, user message
     handling, web content processing.
   - Privileged: gate evaluation, policy enforcement, system prompt assembly.
3. Enforce: quarantined LLM's chain-of-thought never influences privileged
   LLM's decisions (reasoning-blind classifier pattern).
4. Privileged calls should use a different model lineage from quarantined
   calls when feasible (gate judge heterogeneity).
5. Log trust domain per LLM call in efficiency events.

**Acceptance criteria**:
- Gate evaluation calls are tagged as Privileged.
- Agent implementation calls are tagged as Quarantined.
- If the agent used Claude, the gate judge uses a different model family
  (when configured).
- Trust domain is visible in episode logs.

**Dependencies**: None
**Effort**: 1.5 days

---

### INNOV-041: Implement gate immutability from agent perspective

**File**: `crates/roko-gate/src/gate_service.rs`

**What**: Ensure gate definitions and evaluation logic live in a separate
trust domain from the agent code being evaluated. The agent cannot modify
gates.

**Steps**:
1. Load gate configs from a path that is NOT writable by agents:
   - System gates from the crate's compiled defaults.
   - User gates from `roko.toml` (loaded at startup, not re-read during run).
   - Generated gates from `.roko/learn/gate-evolution.json` (written only by
     the GateEvolver, not by agents).
2. During agent dispatch, do NOT pass gate config paths as tool-accessible
   files.
3. Validate gate config integrity: hash gate configs at startup, verify
   hash before each gate run.
4. Log any attempt to modify gate config paths via agent tools.

**Acceptance criteria**:
- An agent that attempts to write to gate config files via bash tool gets a
  permission error (if sandboxing enabled) or the modification is ignored.
- Gate config hash is verified before each gate run.
- Integrity violation is logged as a warning.

**Dependencies**: None
**Effort**: 1 day

---

### INNOV-042: Implement pheromone signal sanitization

**File**: `crates/roko-cli/src/orchestrate.rs` or signal handling path

**What**: Treat every inbound stigmergy pheromone from other agents as
untrusted input. Apply CaMeL information flow control.

**Steps**:
1. Before injecting pheromone signals into an agent's context, pass through
   a sanitization pipeline:
   - Strip any executable content (code blocks that look like tool calls).
   - Validate signal format against expected schema.
   - Truncate to maximum pheromone size (configurable, default 500 tokens).
2. Log sanitization events: what was stripped, from which source.
3. If a pheromone fails validation entirely, quarantine it and log a warning
   instead of injecting it.

**Acceptance criteria**:
- A pheromone containing a fake tool call is sanitized (tool call stripped).
- A pheromone exceeding 500 tokens is truncated.
- An invalid pheromone is quarantined, not injected.
- Sanitization events are logged.

**Dependencies**: None
**Effort**: 0.5 day

---

### INNOV-043: Add knowledge provenance tags

**File**: `crates/roko-neuro/src/knowledge_store.rs`

**What**: Tag knowledge entries with provenance: `extracted` (from code/docs),
`inferred` (from synthesis), `ambiguous` (sources disagree).

**Steps**:
1. Add `provenance: Provenance` field to `KnowledgeEntry`.
2. Define `Provenance` enum: `Extracted`, `Inferred`, `Ambiguous`.
3. Default all new entries to `Extracted`.
4. When the dream consolidation cycle synthesizes knowledge, tag as `Inferred`.
5. When two sources disagree on the same topic (HDC similarity > 0.9 but
   contradictory content), tag as `Ambiguous`.
6. Implement a lint pass: `lint_provenance() -> Vec<ProvenanceWarning>` that
   flags entries drifting from Extracted to Inferred without explicit
   acknowledgment.

**Acceptance criteria**:
- New knowledge entries from direct observation are tagged `Extracted`.
- Synthesized entries from dream cycle are tagged `Inferred`.
- `lint_provenance()` returns warnings for unacknowledged Inferred entries.
- Provenance is visible in `roko knowledge query` output.

**Dependencies**: None
**Effort**: 0.5 day

---

### INNOV-044: Wire RLAIF/RLSF pattern into learning loop

**Files**:
- `crates/roko-learn/src/feedback_service.rs`
- `crates/roko-learn/src/runtime_feedback.rs`

**What**: Use gate verdicts as reinforcement learning reward signals.
Successful gate passes are positive trajectories. Failed gates trigger
hindsight relabeling.

**Steps**:
1. After a gate passes, record the full trajectory (prompt, tool calls,
   output) as a positive training signal in the feedback service.
2. After a gate fails, apply AgentHER (Hindsight Experience Replay):
   ask "what sub-goals did this trajectory actually achieve?" and record
   those as positive episodes for sub-goals.
3. Store trajectory quality scores alongside episodes.
4. Feed trajectory quality into CascadeRouter observations:
   high-quality trajectories boost the model's routing weight.
5. Implement accumulate-only constraint: synthetic/relabeled data is always
   added to real data, never replaces it. Tag synthetic entries.

**Acceptance criteria**:
- After a gate pass, a positive trajectory is recorded with quality score.
- After a gate fail, at least one sub-goal is identified and recorded as
  a positive episode.
- CascadeRouter observations include trajectory quality.
- Synthetic entries are tagged and never replace real entries.

**Dependencies**: None
**Effort**: 1 day

---

### INNOV-045: Wire dream consolidation cron trigger

**Files**:
- `crates/roko-dreams/src/runner.rs`
- `crates/roko-cli/src/daemon.rs` or scheduled task infrastructure

**What**: The dream consolidation cycle is built but has no runtime trigger.
Wire it to run automatically on schedule or after N completed runs.

**Steps**:
1. Add `dream.schedule` to roko.toml config:
   ```toml
   [dream]
   schedule = "after_10_runs"  # or "daily" or "manual"
   ```
2. In the daemon or post-run hook, check if dream cycle should trigger:
   - Count completed runs since last dream cycle.
   - If count >= threshold, spawn dream cycle as a background task.
3. Dream cycle performs:
   a. Load recent episode data.
   b. Run hypnagogia (creative association across episodes).
   c. Run imagination (scenario generation from patterns).
   d. Run consolidation (knowledge compression, confidence boosting).
   e. Persist distilled knowledge back to the neuro store.
4. Log dream cycle outcomes: entries consolidated, entries pruned, new
   patterns discovered.

**Acceptance criteria**:
- After 10 completed runs, the dream cycle triggers automatically.
- Dream cycle output appears in `.roko/neuro/knowledge.jsonl` as new
  or updated entries.
- Dream cycle does not block the next run (runs in background).
- `roko knowledge dream run` still works for manual triggering.

**Dependencies**: None
**Effort**: 1 day

---

### INNOV-046: Enhance HDC fingerprinting for routing

**Files**:
- `crates/roko-primitives/` (HDC implementation)
- `crates/roko-learn/src/cascade_router.rs`

**What**: Use HDC fingerprints as routing keys in CascadeRouter. Each model
gets a capability fingerprint; each task gets a requirement fingerprint.
Route to the model with the lowest Hamming distance.

**Steps**:
1. Compute capability fingerprints per model from historical episode data:
   aggregate the HDC fingerprints of tasks where the model succeeded.
2. Compute requirement fingerprints per task from task context: task
   description, file paths, complexity indicators.
3. In CascadeRouter, add a routing stage: compute Hamming distance between
   task requirement fingerprint and each model's capability fingerprint.
4. Use HDC distance as a feature in the LinUCB context vector (add to the
   existing 17-dimensional vector).
5. Track HDC routing accuracy: did the HDC-selected model succeed?

**Acceptance criteria**:
- After 50+ episodes, models have non-trivial capability fingerprints.
- HDC routing selects the model whose capability profile best matches the
  task. Verify by comparing HDC-selected model to the actual winner.
- HDC distance is included in the LinUCB context vector.

**Dependencies**: None
**Effort**: 1 day

---

### INNOV-047: Add HDC consistency check for adversarial detection

**File**: `crates/roko-primitives/`

**What**: Bind HDC fingerprints to a hash of the skill/agent code. Flag
drift without corresponding code changes as potential adversarial
manipulation.

**Steps**:
1. When computing an HDC fingerprint for an agent/skill, also compute
   a BLAKE3 hash of the agent's source code or configuration.
2. Store `(hdc_fingerprint, code_hash)` pairs.
3. On subsequent runs, recompute both. If the HDC fingerprint has drifted
   (Hamming distance > threshold) but the code hash is unchanged, flag as
   `AdversarialDriftWarning`.
4. Log the warning with both fingerprints and hashes.
5. Configurable threshold (default: Hamming distance > 500 of 10240 bits).

**Acceptance criteria**:
- A stable agent/skill produces the same `(fingerprint, hash)` pair across
  runs.
- Manually corrupting the fingerprint triggers an AdversarialDriftWarning.
- Warning is logged but does not block execution (informational).

**Dependencies**: None
**Effort**: 0.5 day

---

### INNOV-048: Implement tool-output sanitization

**File**: `crates/roko-agent/src/tool_loop/result_msg.rs`

**What**: Truncate, filter, and validate tool outputs before including in
agent context. Addresses both cost (removing verbose output) and safety
(removing injected payloads).

**Steps**:
1. Define `ToolOutputSanitizer` with configurable max output size
   (default: 4096 tokens).
2. On each tool output:
   - Truncate to max size with "... (truncated, {N} tokens omitted)" suffix.
   - Strip ANSI escape codes.
   - Filter known injection patterns (tool calls embedded in output).
   - Validate UTF-8 encoding.
3. Log sanitization events when content is modified.
4. Make max size configurable per tool (some tools like `read_file` need
   larger output than `bash`).

**Acceptance criteria**:
- A `bash` tool output of 10K tokens is truncated to 4K with a truncation
  notice.
- ANSI codes are stripped from all tool outputs.
- A tool output containing a fake tool call has it sanitized.
- Sanitization is visible in verbose logging.

**Dependencies**: None
**Effort**: 0.5 day

---

### INNOV-049: Implement event log fork-from-checkpoint

**Files**:
- `crates/roko-runtime/src/workflow_engine.rs`
- `crates/roko-cli/src/main.rs`

**What**: When a gate fails, fork the execution from the last successful
checkpoint rather than restarting from scratch.

**Steps**:
1. Ensure each task completion writes a checkpoint to the event log with
   key `(run_id, task_id, attempt)`.
2. Add `--fork-from <task_id>` flag to `roko plan run`.
3. When `--fork-from` is specified, load the event log up to the named
   task's last successful checkpoint, replay state, and continue from there.
4. The fork creates a new `run_id` but shares the event log prefix with
   the original run.
5. Forked runs preserve all learning data from the original run.

**Acceptance criteria**:
- `roko plan run --fork-from task-05` starts execution from after task-05,
  with all state from tasks 01-05 loaded.
- The fork has a new run_id visible in logs.
- Learning data from the original run is available in the fork.

**Dependencies**: None
**Effort**: 1 day

---

### INNOV-050: Add payload size guards to event logger

**File**: `crates/roko-runtime/src/jsonl_logger.rs`

**What**: If an event exceeds a size threshold, offload the payload to a
separate file and store a reference in the event log.

**Steps**:
1. Define `MAX_INLINE_PAYLOAD_SIZE = 1_048_576` (1 MB).
2. Before writing an event, check payload size.
3. If > threshold:
   - Write payload to `.roko/events/payloads/{event_id}.json`.
   - Replace payload in event with `{ "$ref": "payloads/{event_id}.json" }`.
4. On event log read, resolve `$ref` references transparently.
5. Add payload GC: clean up unreferenced payload files older than 30 days.

**Acceptance criteria**:
- An event with a 5 MB tool output stores the output in a separate file.
- The event log entry contains a `$ref` instead of the full payload.
- Reading the event log resolves references transparently.
- Payload files older than 30 days are cleaned up.

**Dependencies**: None
**Effort**: 0.5 day

---

### INNOV-051: Add model-heterogeneity enforcement in gate judges

**File**: `crates/roko-gate/src/gate_service.rs`

**What**: Oracle gate rungs (4-6) must use a different model family from the
task agent. If the agent used Claude, the judge must use GPT or Gemini.

**Steps**:
1. In `enrich_rung_config()`, accept the agent's model slug.
2. Determine the agent's model family (Claude, GPT, Gemini, etc.).
3. For oracle rungs (4-6), select a judge model from a different family.
4. If no alternative model is configured, log a warning and proceed with
   the same family (degraded mode, not a hard failure).
5. Record judge model in the gate verdict for auditability.

**Acceptance criteria**:
- Agent uses Claude Sonnet -> oracle judge uses GPT or Gemini.
- If only Claude models are configured, a warning is logged.
- Judge model is visible in gate verdict logs.
- Preference leakage is reduced: measure judge agreement with ground truth
  before and after heterogeneity enforcement.

**Dependencies**: None
**Effort**: 0.5 day

---

### INNOV-052: Implement force_backend override learning (UX34)

**File**: `crates/roko-learn/src/cascade_router.rs`

**What**: When a user manually overrides the model with `force_backend`, the
CascadeRouter should learn from this preference signal.

**Steps**:
1. In the dispatch path, detect when `force_backend` is set.
2. Record the override as a strong observation: the user explicitly chose
   this model for this task type.
3. Weight override observations 3x compared to automatic observations
   (configurable multiplier).
4. After accumulating 5+ overrides for the same task category, adjust the
   CascadeRouter's static routing table to prefer the user's choice.
5. Add `roko learn tune routing --show-overrides` to display learned
   overrides.

**Acceptance criteria**:
- After 5 `--force-backend cerebras` overrides on "simple fix" tasks, the
  CascadeRouter routes "simple fix" tasks to Cerebras by default.
- Override learning is visible in `cascade-router.json` observations.
- `roko learn tune routing --show-overrides` lists learned preferences.

**Dependencies**: None
**Effort**: 0.5 day

---

### INNOV-053: Wire knowledge store consultation into CascadeRouter

**File**: `crates/roko-learn/src/cascade_router.rs`

**What**: Consult the neuro knowledge store for historical model performance
on similar tasks before routing.

**Steps**:
1. Before routing, query `KnowledgeStore` for entries about model performance
   on the current task's domain/type.
2. If knowledge exists (e.g., "opus handles architecture tasks well" at
   confidence 0.9), incorporate as a prior in the routing decision.
3. Knowledge query adds ~1ms latency (HDC similarity scan), acceptable.
4. If knowledge contradicts bandit observations, weight bandit observations
   higher (they are more recent and directly observed).

**Acceptance criteria**:
- If the knowledge store contains "cerebras fails on async Rust" with
  high confidence, CascadeRouter avoids cerebras for async Rust tasks.
- Knowledge consultation adds < 5ms to routing latency.
- Bandit observations override stale knowledge.

**Dependencies**: INNOV-001
**Effort**: 0.5 day

---

### INNOV-054: Implement OTel gen_ai.* span emission

**Files**:
- `crates/roko-runtime/src/otel_emitter.rs` (new)
- `crates/roko-runtime/src/lib.rs`
- `crates/roko-runtime/Cargo.toml` (add opentelemetry deps)

**What**: Emit OpenTelemetry spans alongside existing JSONL logging using
the gen_ai.* semantic conventions (v1.37+).

**Steps**:
1. Add `opentelemetry`, `opentelemetry-otlp`, `opentelemetry-sdk` to
   roko-runtime dependencies.
2. Define `OtelEmitter` struct wrapping a tracer provider.
3. On each agent dispatch, create a span with attributes:
   - `gen_ai.provider.name` (claude, openai, etc.)
   - `gen_ai.operation.name` (chat, execute_tool, etc.)
   - `gen_ai.conversation.id` (map to run_id)
   - `gen_ai.usage.input_tokens`, `gen_ai.usage.output_tokens`
   - `gen_ai.usage.cache_read.input_tokens`
4. On each gate evaluation, create a child span.
5. Emit to configurable OTLP endpoint (from roko.toml).
6. Feature-gate: only active when `[observability]` config is present.

**Acceptance criteria**:
- With `[observability] provider = "otlp-generic"` configured, OTel spans
  are emitted to the endpoint.
- Spans include all gen_ai.* attributes per v1.37+ spec.
- Without observability config, no OTel overhead (feature-gated).
- JSONL logging continues alongside OTel (not replaced).

**Dependencies**: None
**Effort**: 1.5 days

---

### INNOV-055: Add vendor-neutral observability config

**Files**:
- `crates/roko-core/src/config/serve.rs` (extend)
- `crates/roko-runtime/src/otel_emitter.rs`

**What**: Configurable observability backend via roko.toml.

**Steps**:
1. Add `[observability]` section to roko.toml schema:
   ```toml
   [observability]
   provider = "langfuse"  # or "phoenix" | "honeycomb" | "grafana" | "otlp-generic"
   endpoint = "https://..."
   protocol = "http/protobuf"  # or "grpc"
   api_key_env = "LANGFUSE_API_KEY"
   ```
2. In `OtelEmitter`, configure the OTLP exporter based on provider:
   - All providers use `opentelemetry-otlp` with different endpoints.
   - Provider-specific: set required headers (API key format varies).
3. Validate config at startup: check endpoint reachability, API key presence.

**Acceptance criteria**:
- Changing `provider` from `langfuse` to `honeycomb` requires only changing
  the config, not the code.
- Missing API key produces a clear error at startup.
- `roko config validate` checks observability config.

**Dependencies**: INNOV-054
**Effort**: 0.5 day

---

### INNOV-056: Emit gate results as structured compliance events

**File**: `crates/roko-gate/src/gate_service.rs`

**What**: Each gate result emits an OTel span with structured attributes
suitable for SIEM/GRC integration.

**Steps**:
1. After each gate rung completes, emit an OTel span with:
   - `gate.rung.id`, `gate.rung.name`
   - `gate.verdict` (pass/fail)
   - `gate.agent_id`, `gate.task_id`
   - `gate.evidence` (structured JSON of what was checked)
   - `gate.model_used` (for oracle rungs)
   - `gate.timestamp`
2. Use standard OTel event semantics so that SIEM tools can subscribe to
   gate events.
3. For Article 50 compliance: include `ai.provenance.model`,
   `ai.provenance.timestamp`, `ai.provenance.confidence` attributes.

**Acceptance criteria**:
- Gate results appear as OTel spans with all structured attributes.
- A SIEM tool (or OTel collector) can filter for `gate.verdict = fail`
  events.
- Compliance attributes satisfy Article 50 minimum requirements.

**Dependencies**: INNOV-054
**Effort**: 0.5 day

---

### INNOV-057: Wire experiment feedback into CascadeRouter

**File**: `crates/roko-learn/src/cascade_router.rs`

**What**: Connect experiment outcomes from ExperimentStore to CascadeRouter
routing weight updates.

**Steps**:
1. After an experiment arm concludes (success or failure), extract the
   model and prompt variant used.
2. Feed the outcome into CascadeRouter as an observation with the
   experiment context.
3. Winning experiment arms boost the associated model's routing weight.
4. Losing arms reduce the weight.
5. The bandit should promote winning prompt/model combinations automatically.

**Acceptance criteria**:
- An experiment with 3 arms (haiku, sonnet, opus) on the same task type:
  after 10 trials, the winning model has the highest routing weight.
- Experiment observations are visible in `cascade-router.json`.
- The experiment store and router are no longer operating independently.

**Dependencies**: None
**Effort**: 0.5 day

---

### INNOV-058: Implement Bayesian Model Reduction in dream cycle

**File**: `crates/roko-dreams/src/cycle.rs`

**What**: Apply AXIOM-style Bayesian Model Reduction to score and prune
knowledge entries during dream consolidation.

**Steps**:
1. During the consolidation phase, compute evidence for each knowledge
   entry: how many episodes support it vs contradict it.
2. Apply BMR: score candidate knowledge models from accumulated posteriors.
3. Prune low-evidence entries (evidence < threshold, configurable).
4. Merge near-duplicate entries (HDC similarity > 0.95): combine evidence,
   take the higher-confidence text.
5. Log pruning decisions: what was pruned, why, what was merged.

**Acceptance criteria**:
- After dream consolidation, knowledge store has fewer entries but higher
  average confidence.
- Entries with zero supporting episodes are pruned.
- Near-duplicate entries are merged.
- Pruning log shows what was removed and why.

**Dependencies**: INNOV-045
**Effort**: 1 day

---

### INNOV-059: Implement hindsight relabeling in dream cycle

**File**: `crates/roko-dreams/src/cycle.rs`

**What**: Apply AgentHER (Hindsight Experience Replay) to failed trajectories
during dream consolidation.

**Steps**:
1. Load failed episodes from the episode log.
2. For each failed episode, analyze the trajectory to identify sub-goals
   that were actually achieved (e.g., "correct file identified but wrong
   edit applied").
3. Create new positive episodes for those sub-goals with reduced scope.
4. Store relabeled episodes with `provenance: Inferred` and
   `source: hindsight_relabeling`.
5. Feed relabeled episodes into the learning loop as positive training data.

**Acceptance criteria**:
- A failed episode that correctly identified the right files produces a
  positive sub-goal episode for "file identification."
- Relabeled episodes are tagged as Inferred.
- The learning loop's positive trajectory count increases after dream
  consolidation.
- Reports +7-12% data efficiency on subsequent similar tasks (measure over
  20+ runs).

**Dependencies**: INNOV-045
**Effort**: 1 day

---

### INNOV-060: Implement CMP scoring for agent variants

**File**: `crates/roko-cli/src/orchestrate.rs` or `crates/roko-learn/`

**What**: Score agent variants by aggregate descendant performance
(Clade-Metaproductivity) rather than the variant's own output quality.

**Steps**:
1. Track agent lineage: which agent configuration produced which outcomes,
   and which configurations descended from which.
2. Define CMP score: average gate pass rate of all tasks dispatched by
   agents using this configuration AND all descendant configurations.
3. When evaluating which agent configuration to use, prefer configurations
   with higher CMP scores over those with higher individual scores.
4. Store CMP scores in `.roko/learn/agent-variants.json`.
5. Add `roko learn agents` CLI showing variant CMP scores.

**Acceptance criteria**:
- An agent configuration that produces good outcomes AND whose descendants
  also perform well has a higher CMP score than one that only performs well
  itself.
- CMP scores persist across runs.
- `roko learn agents` displays variant lineage with CMP scores.

**Dependencies**: None
**Effort**: 1 day

---

### INNOV-061: Add multi-dimensional collective intelligence measurement

**File**: `crates/roko-cli/src/orchestrate.rs`

**What**: Extend CFactorSummary to include synergy, redundancy, and unique
information components via Partial Information Decomposition (PID).

**Steps**:
1. Implement binary PID (Williams-Beer): decompose mutual information between
   two agent outputs into synergy (true collective), redundancy (wasted),
   and unique (specialization) components.
2. For n >= 3 agents, use pairwise PID (binary-only, per the mathematical
   limitation caveat).
3. Add `synergy`, `redundancy`, `unique_info` fields to `CFactorSummary`.
4. Gate multi-agent scaling on synergy threshold: if synergy < 0.1 (agents
   are redundant), recommend reducing agent count.
5. Log PID components in efficiency events.

**Acceptance criteria**:
- After a multi-agent run with 3 agents, CFactorSummary includes synergy,
  redundancy, and unique components.
- High-redundancy runs produce a recommendation to reduce agent count.
- PID components are visible in `roko learn all` output.

**Dependencies**: None
**Effort**: 1 day

---

### INNOV-062: Implement distributed causal discovery over episodes

**File**: `crates/roko-neuro/src/episode_completion.rs`

**What**: Apply DCILP-style causal discovery to identify genuine vs spurious
task dependencies from episode data.

**Steps**:
1. For each task, estimate its local Markov blanket from episode outcomes:
   which other tasks' outcomes statistically predict this task's success?
2. Merge local blankets into a global structural causal model (DAG).
3. Compare causal DAG with the declared dependency DAG in plans.
4. Flag spurious dependencies: tasks declared as dependent but with no
   causal relationship in the data.
5. Flag missing dependencies: tasks with causal relationships not declared
   in the plan.
6. Output recommendations: "task-07 does not actually depend on task-05
   (p = 0.02); consider parallelizing."

**Acceptance criteria**:
- After 20+ plan runs, the causal model identifies at least one spurious
  dependency (tasks that can be parallelized).
- Recommendations are actionable: include task IDs and p-values.
- The causal DAG is persisted to `.roko/learn/causal-model.json`.

**Dependencies**: None
**Effort**: 1.5 days

---

### INNOV-063: Add Article 50 compliance posture endpoint

**Files**:
- `crates/roko-serve/src/routes/compliance.rs` (new)
- `crates/roko-serve/src/routes/mod.rs`

**What**: HTTP endpoint reporting the system's EU AI Act Article 50
compliance status.

**Steps**:
1. `GET /api/compliance/article50` returns:
   - List of deployed agents with AI disclosure status.
   - Logging status: are events being recorded per Article 12(1)?
   - Log retention period (target: >= 6 months per Article 26(6)).
   - Provenance metadata status per agent output.
2. `GET /api/compliance/report` generates a compliance report suitable
   for audit submission.
3. Add `transparency_mode` flag to agent configuration: when enabled,
   agents must disclose AI nature at first contact.

**Acceptance criteria**:
- `GET /api/compliance/article50` returns a structured JSON report.
- Report includes logging status and retention period.
- `transparency_mode` flag is configurable in roko.toml.

**Dependencies**: INNOV-054
**Effort**: 0.5 day

---

### INNOV-064: Add C2PA-aligned metadata to agent outputs

**File**: `crates/roko-runtime/src/jsonl_logger.rs`

**What**: Attach provenance metadata to agent outputs satisfying the
Code of Practice multi-layered marking requirement.

**Steps**:
1. On each agent output, attach metadata fields:
   - `ai.generated: true`
   - `ai.model`: model slug
   - `ai.timestamp`: ISO 8601
   - `ai.agent_id`: agent identifier
   - `ai.confidence`: agent's confidence score (if available)
   - `ai.provenance_version`: "c2pa-draft-2026"
2. Include metadata in JSONL events.
3. Include metadata in A2A task artifacts.
4. Include metadata in shared run exports.

**Acceptance criteria**:
- Every agent output event in JSONL includes provenance metadata.
- Metadata fields match C2PA-aligned naming conventions.
- Shared run exports include provenance metadata.

**Dependencies**: None
**Effort**: 0.5 day

---

### INNOV-065: Add ERC-8004 identity fields to agent configuration

**File**: `crates/roko-core/src/config/`

**What**: Prepare agent configuration for on-chain identity integration
without requiring blockchain infrastructure.

**Steps**:
1. Add optional fields to agent config in roko.toml:
   ```toml
   [agent.identity]
   erc8004_id = "0x..."  # ERC-721 compatible identifier (optional)
   capabilities = ["code-implementation", "code-review"]
   reputation_tier = "verified"  # or "community" | "unverified"
   ```
2. If `erc8004_id` is set, include it in the A2A Agent Card.
3. If set, include it in the compliance report.
4. Validate format (Ethereum address format) but do not require on-chain
   verification (that is Phase 2+).

**Acceptance criteria**:
- Agent config accepts `[agent.identity]` section without error.
- ERC-8004 ID appears in A2A Agent Card when set.
- Invalid Ethereum address format produces a config validation error.
- Field is entirely optional; omitting it changes nothing.

**Dependencies**: INNOV-036
**Effort**: 0.5 day

---

## Summary

| Phase | Tasks | Effort (days) | Key Deliverables |
|-------|-------|---------------|-----------------|
| 1: Foundations | INNOV-001 to INNOV-012 | ~10 | Memory layer, context adaptation, cost optimization, semantic cache |
| 2: Self-Improvement | INNOV-013 to INNOV-024 | ~11.5 | Self-improving gates, agent debugging, interactive steering |
| 3: Multi-Agent | INNOV-025 to INNOV-030 | ~10 | Competitive proposals, swarm, speculative execution |
| 4: Cross-Cutting | INNOV-031 to INNOV-065 | ~26 | Cross-project learning, A2A, CaMeL safety, OTel, compliance, research features |
| **Total** | **65** | **~57.5** | |

### Dependency Graph (Critical Path)

```
INNOV-001 (MemoryLayer) -> INNOV-002 (Retrieval) -> INNOV-003 (Wire to prompt)
                                                  -> INNOV-022 (Confidence)
INNOV-001 -> INNOV-004 (Memory update)

INNOV-005 (Disclosure) + INNOV-006 (Profile) -> INNOV-011 (Compression)

INNOV-009 (Cache exact) -> INNOV-010 (Cache fuzzy)

INNOV-013 (GateEvolver) -> INNOV-014 (Wire gates)

INNOV-017 (Taxonomy) -> INNOV-018 (Hypotheses) -> INNOV-019 (Wire debug)

INNOV-020 (Steering types) -> INNOV-021 (Channel) -> INNOV-023 (HTTP)
                                                   -> INNOV-024 (TUI)
INNOV-020 -> INNOV-022 (Confidence) -> INNOV-023, INNOV-024

INNOV-031 (Global dir) + INNOV-032 (Domain) -> INNOV-033 (Tiered store)
  -> INNOV-034 (Promotion) -> INNOV-035 (Export/import)

INNOV-036 (A2A types) -> INNOV-037 (Card) -> INNOV-038 (Server)
INNOV-036 -> INNOV-039 (Client)

INNOV-054 (OTel) -> INNOV-055 (Config) -> INNOV-056 (Compliance events)
                                        -> INNOV-063 (Article 50)

INNOV-045 (Dream trigger) -> INNOV-058 (BMR) + INNOV-059 (Hindsight)
```

### Implementation Priority Order

**Immediate (P0, days 1-5)**:
INNOV-008 (Budget), INNOV-001 (Memory struct), INNOV-005 (Disclosure),
INNOV-007 (Pareto routing), INNOV-009 (Cache), INNOV-048 (Sanitize)

**Week 1-2 (P1)**:
INNOV-002 (Retrieval), INNOV-003 (Wire memory), INNOV-004 (Update memory),
INNOV-006 (Profile), INNOV-011 (Compress), INNOV-013 (GateEvolver),
INNOV-015 (DiffAnalyzer), INNOV-017 (Taxonomy), INNOV-051 (Judge heterogeneity),
INNOV-052 (UX34), INNOV-053 (Knowledge routing)

**Week 3-4 (P2)**:
INNOV-014 (Wire gates), INNOV-016 (Effectiveness), INNOV-018 (Hypotheses),
INNOV-019 (Debug), INNOV-020 (Steering types), INNOV-021 (Channel),
INNOV-022 (Confidence), INNOV-040 (CaMeL), INNOV-041 (Gate immutability),
INNOV-042 (Pheromone sanitize), INNOV-043 (Provenance), INNOV-044 (RLAIF),
INNOV-054 (OTel)

**Month 2 (P3)**:
INNOV-010 (Fuzzy cache), INNOV-023 (HTTP steer), INNOV-024 (TUI steer),
INNOV-025 (Competitive), INNOV-028 (Speculative fix), INNOV-029 (Prefetch),
INNOV-036 (A2A types), INNOV-037 (Card), INNOV-045 (Dream trigger),
INNOV-046 (HDC routing), INNOV-055 (OTel config), INNOV-056 (Compliance),
INNOV-057 (Experiment feedback)

**Month 3 (P4)**:
INNOV-026 (Swarm), INNOV-027 (Specialist), INNOV-030 (Density threshold),
INNOV-031-035 (Cross-project learning), INNOV-038-039 (A2A server/client),
INNOV-047 (HDC adversarial), INNOV-049-050 (Fork, payload guards),
INNOV-058-059 (BMR, hindsight), INNOV-060-062 (CMP, PID, causal),
INNOV-063-065 (Compliance, C2PA, ERC-8004)
