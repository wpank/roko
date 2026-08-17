# Design Patterns Catalog

This document catalogs the recurring design patterns used throughout tiagent,
a self-improving coding agent harness. Each pattern is presented with the
problem it solves, how it works, a structural sketch, its trade-offs, and a
concrete example from the codebase.

All patterns in this catalog apply to tiagent as a standalone coding agent.
Only Pattern 9 (Dual-Layer Storage) requires Celestia integration.

tiagent is built around a small number of composable abstractions. If you
understand the twelve patterns below, you understand how every subsystem in
the project fits together.

---

## 1. Signal DAG Pattern

> Works in standalone mode (no Celestia required)

### Problem

Agent workflows produce many artifacts: prompts, model responses, tool
outputs, gate verdicts, plan revisions. Without a unifying data model these
artifacts are scattered across log files, databases, and in-memory structs
with no way to trace how one piece of data led to another.

### Solution

Every piece of data in tiagent is a **Signal**. A Signal is:

- **Content-addressed**: its identifier is a hash of its contents.
- **Typed**: it carries a `kind` discriminator (e.g. `Prompt`, `Response`,
  `GateVerdict`, `EpisodeRecord`).
- **Immutable**: once created, a Signal never changes.
- **Linked**: it holds zero or more `parent` hashes pointing to the Signals
  that produced it.

Because each Signal references its parents by hash, the full history of any
artifact forms a directed acyclic graph (DAG). You can walk backwards from
any Signal to reconstruct the exact chain of inputs and decisions that
produced it.

### Structure

```
Signal {
    hash:       Blake3Hash,      // content-addressed ID
    kind:       SignalKind,      // Prompt | Response | GateVerdict | ...
    parents:    Vec<Blake3Hash>, // edges in the DAG
    payload:    serde_json::Value,
    created_at: DateTime<Utc>,
    metadata:   BTreeMap<String, String>,
}

         [Prompt A]
            |
            v
       [Response B]
          /    \
         v      v
  [ToolCall C]  [ToolCall D]
         \      /
          v    v
      [GateVerdict E]
```

### Trade-offs

- **Storage growth**: every intermediate value becomes a persisted object.
  Garbage collection (based on age or reachability) is necessary for
  long-running deployments.
- **Hash computation cost**: Blake3 is fast but not free. Batching Signal
  creation amortizes the overhead.
- **Rigidity**: once a Signal is written its hash is fixed. Correcting
  mistakes requires appending a new Signal that supersedes the old one rather
  than editing in place.

### Example

When an agent processes a task, the runtime creates a `Prompt` Signal whose
parents include the task definition Signal and any context Signals selected
by the Composer. The model response becomes a `Response` Signal whose sole
parent is the `Prompt`. If the response triggers tool calls, each call
produces its own Signal parented to the `Response`. The gate verdict at the
end parents all the tool-call Signals it evaluated. Walking the DAG from
the verdict backwards reconstructs the full execution trace -- tracking the
provenance of a code change through prompt, agent response, file edit, and
test result.

```rust
let prompt_signal = Signal::new(
    SignalKind::Prompt,
    serde_json::to_value(&assembled_prompt)?,
    vec![task_signal.hash, context_signal.hash],
);
substrate.write(prompt_signal).await?;
```

---

## 2. Verb Trait Pattern

> Works in standalone mode (no Celestia required)

### Problem

tiagent must run in multiple environments: local development with the
filesystem, CI with ephemeral containers, and production with database or
cloud storage backends. Hard-coding any single backend makes the system
brittle and untestable.

### Solution

Operations on data are expressed as **six verb traits**. Each trait defines
a small interface for one category of work:

| Trait      | Responsibility                              |
|------------|---------------------------------------------|
| Substrate  | Read and write Signals to a storage backend |
| Scorer     | Assign relevance scores to Signals          |
| Gate       | Validate outputs against quality criteria   |
| Router     | Choose which model or agent handles a task  |
| Composer   | Assemble prompts from context sources       |
| Policy     | Authorize or deny tool invocations          |

Implementations are swappable. The runtime depends on the trait, never on a
concrete struct.

### Structure

```rust
#[async_trait]
pub trait Substrate: Send + Sync {
    async fn read(&self, hash: &Blake3Hash) -> Result<Signal>;
    async fn write(&self, signal: Signal) -> Result<Blake3Hash>;
    async fn query(&self, filter: &SignalFilter) -> Result<Vec<Signal>>;
}

// Local filesystem implementation
pub struct FileSubstrate { root: PathBuf }

// Database-backed implementation
pub struct DbSubstrate { pool: SqlitePool }

// In-memory implementation for tests
pub struct MemSubstrate { signals: Arc<DashMap<Blake3Hash, Signal>> }
```

### Trade-offs

- **Indirection cost**: every storage call goes through dynamic dispatch
  (`dyn Substrate`). In practice the overhead is negligible compared to I/O
  latency.
- **Trait coherence**: adding a method to a verb trait is a breaking change
  for all implementations. The traits are intentionally kept small to reduce
  churn.
- **Discoverability**: newcomers must learn six trait names before the
  architecture makes sense. This document and the crate-level docs aim to
  close that gap.

### Example

Unit tests use `MemSubstrate` to avoid touching the filesystem:

```rust
#[tokio::test]
async fn gate_rejects_failing_compilation() {
    let substrate = MemSubstrate::default();
    let gate = CompileGate::new();
    let signal = Signal::new(SignalKind::CodePatch, json!({"diff": "..."}), vec![]);
    let hash = substrate.write(signal).await.unwrap();

    let verdict = gate.evaluate(&substrate, &hash).await.unwrap();
    assert!(!verdict.passed);
}
```

In production the same `CompileGate` is handed a `FileSubstrate` or
`DbSubstrate` and works identically.

---

## 3. Universal Loop Pattern

> Works in standalone mode (no Celestia required)

### Problem

Without a standard execution model, each feature invents its own control
flow. This leads to inconsistent error handling, missing logging, and
logic that cannot be reused across task types.

### Solution

Every agent operation -- whether it is answering a chat message, executing
a plan task, or running a research query -- follows the same eight-stage
loop:

```
query -> score -> route -> compose -> act -> verify -> persist -> react
```

Each stage maps to a verb trait method call:

| Stage    | Trait    | What happens                                       |
|----------|----------|----------------------------------------------------|
| query    | Substrate| Fetch relevant Signals from storage                |
| score    | Scorer   | Rank them by relevance to the current task          |
| route    | Router   | Pick the model/agent to handle this task            |
| compose  | Composer | Assemble the prompt from scored context             |
| act      | (agent)  | Send prompt to model, collect response + tool calls |
| verify   | Gate     | Run gate pipeline on the output                     |
| persist  | Substrate| Write result Signals to storage                     |
| react    | Policy   | Decide next action (continue, escalate, stop)       |

### Structure

```
  +-------+     +-------+     +-------+     +---------+
  | query | --> | score | --> | route | --> | compose |
  +-------+     +-------+     +-------+     +---------+
                                                 |
                                                 v
  +-------+     +---------+     +---------+     +-----+
  | react | <-- | persist | <-- | verify  | <-- | act |
  +-------+     +---------+     +---------+     +-----+
      |
      v
  [next iteration or stop]
```

### Trade-offs

- **Overhead for simple tasks**: a quick lookup still passes through all
  eight stages. In practice the stages short-circuit (e.g. scoring returns
  an empty list, gating auto-passes for read-only queries).
- **Debugging depth**: a bug in stage 5 may originate from a bad decision
  in stage 3. The Signal DAG helps trace across stages, but the indirection
  requires familiarity with the loop.
- **Rigidity**: genuinely novel execution patterns may not fit the eight
  stages cleanly. The `react` stage acts as an escape hatch by allowing
  arbitrary follow-up actions.

### Example

The `roko run "<prompt>"` command executes a single iteration of the
universal loop:

```rust
let signals = substrate.query(&task_filter).await?;
let ranked  = scorer.score(&signals, &task_context).await?;
let model   = router.select(&task_context).await?;
let prompt  = composer.assemble(&ranked, &task_context).await?;
let result  = agent.execute(&model, &prompt).await?;
let verdict = gate.evaluate(&substrate, &result.hash).await?;
substrate.write(result.into_signal()).await?;
policy.react(&verdict, &mut next_action).await?;
```

---

## 4. Cascade Router Pattern

> Works in standalone mode (no Celestia required)

### Problem

Large language models vary in cost, latency, and capability. Sending every
task to the most capable (and most expensive) model wastes budget. Sending
every task to the cheapest model produces low-quality results for hard
tasks.

### Solution

The Cascade Router starts each task with the cheapest viable model. If the
model fails (gate rejection, low confidence, error), the router escalates
to the next tier. Over time it learns which task categories succeed at
which tier and routes directly, skipping unnecessary attempts.

```
Tier 0: Haiku / small model    (cheapest, fastest)
Tier 1: Sonnet / medium model  (balanced)
Tier 2: Opus / large model     (most capable, most expensive)
```

Routing weights are persisted to disk so the router improves across
sessions.

### Structure

```rust
pub struct CascadeRouter {
    tiers: Vec<ModelTier>,
    weights: HashMap<TaskCategory, Vec<f64>>,  // category -> per-tier success rate
    persistence_path: PathBuf,
}

impl CascadeRouter {
    pub async fn select(&self, ctx: &TaskContext) -> Result<ModelId> {
        let category = ctx.classify();
        let tier_weights = self.weights.get(&category);

        // Pick highest-weighted tier that exceeds the confidence threshold
        for (i, tier) in self.tiers.iter().enumerate() {
            if tier_weights[i] >= self.confidence_threshold {
                return Ok(tier.model_id.clone());
            }
        }
        // Fallback: most capable tier
        Ok(self.tiers.last().unwrap().model_id.clone())
    }

    pub fn record_outcome(&mut self, category: TaskCategory, tier: usize, success: bool) {
        // EMA update: weight = alpha * outcome + (1 - alpha) * old_weight
        let alpha = 0.1;
        let outcome = if success { 1.0 } else { 0.0 };
        let w = &mut self.weights.entry(category).or_default()[tier];
        *w = alpha * outcome + (1.0 - alpha) * *w;
    }
}
```

### Trade-offs

- **Cold start**: a fresh deployment has no weight history. The router falls
  through to the most capable model until enough data accumulates. This is
  safe but expensive during the learning period.
- **Category granularity**: coarse categories (e.g. "code-generation") learn
  slowly. Fine-grained categories (e.g. "rust-trait-impl-with-generics")
  fragment the data. The default taxonomy balances breadth and depth.
- **Stale weights**: if model capabilities change (new release, fine-tune),
  old weights may route incorrectly. A decay factor on weights mitigates
  this.

### Example

Start with Claude Haiku for simple formatting tasks, escalate to Opus for
complex refactors. A documentation-writing task typically succeeds at Tier 0.
After a few successful runs the router assigns a high weight to Tier 0 for
the `documentation` category and stops trying higher tiers. A complex
refactor task fails at Tier 0 and Tier 1 before succeeding at Tier 2;
subsequent refactor tasks route directly to Tier 2.

---

## 5. Gate Pipeline Pattern

> Works in standalone mode (no Celestia required)

### Problem

Agent-generated code can compile but be incorrect, or pass tests but
violate style guidelines. A single pass/fail check is too coarse. Manual
review does not scale.

### Solution

The gate pipeline is a sequence of independent validation rungs, ordered
from cheap and fast to expensive and thorough:

```
Rung 0: Parse     — is the output syntactically valid?
Rung 1: Compile   — does it compile without errors?
Rung 2: Test      — do existing tests still pass?
Rung 3: Lint      — does clippy/eslint/etc. pass clean?
Rung 4: Diff      — is the diff size within bounds?
Rung 5: Semantic  — does an LLM judge confirm correctness?
Rung 6: Human     — manual sign-off (optional, policy-controlled)
```

Each rung runs independently and returns a `RungVerdict` (pass, fail,
skip). The pipeline aggregates verdicts into a final `GateVerdict`.

Thresholds are adaptive: an exponential moving average (EMA) of pass rates
per rung adjusts strictness over time. A rung that consistently passes
raises its threshold; one that consistently fails lowers it to avoid
blocking progress on flaky checks.

### Structure

```
Input Signal
    |
    v
 [Rung 0: Parse]  -----> pass/fail
    |
    v
 [Rung 1: Compile] ----> pass/fail
    |
    v
 [Rung 2: Test]  -------> pass/fail
    |
    v
 [Rung 3: Lint]  -------> pass/fail
    |
    v
 [Rung 4: Diff]  -------> pass/fail
    |
    v
 [Rung 5: Semantic] ----> pass/fail
    |
    v
 [Rung 6: Human]  ------> pass/fail/skip
    |
    v
 GateVerdict { passed: bool, rungs: Vec<RungVerdict> }
```

### Trade-offs

- **Latency**: running all rungs sequentially can take minutes for large
  code changes. Rungs 0-4 are parallelizable in principle but the current
  implementation runs them in order.
- **False positives**: the semantic rung (LLM judge) can reject correct code
  or approve incorrect code. Combining it with deterministic rungs reduces
  but does not eliminate this risk.
- **Threshold drift**: adaptive thresholds can degrade if a long run of bad
  outputs teaches the EMA to accept low quality. A floor threshold prevents
  this.

### Example

After an agent writes code, the pipeline automatically runs compile, test,
lint, and diff review. For example, an agent generates a Rust file and the
gate pipeline runs:

```rust
let rungs: Vec<Box<dyn Rung>> = vec![
    Box::new(ParseRung::new("rust")),
    Box::new(CompileRung::new(&workspace_root)),
    Box::new(TestRung::new(&workspace_root)),
    Box::new(ClippyRung::new(&workspace_root)),
    Box::new(DiffSizeRung::new(max_lines: 500)),
    Box::new(SemanticRung::new(&judge_model)),
];

let mut verdicts = Vec::new();
for rung in &rungs {
    let v = rung.evaluate(&output_signal).await?;
    verdicts.push(v);
    if v.failed() && rung.is_blocking() {
        break; // no point running later rungs
    }
}
let gate_verdict = GateVerdict::aggregate(&verdicts);
```

If the compile rung fails, the pipeline short-circuits and the agent
receives a replan prompt containing the compiler errors.

---

## 6. Snapshot-Resume Pattern

> Works in standalone mode (no Celestia required)

### Problem

Agent plan execution can take hours. Network failures, process crashes,
and machine restarts are inevitable. Without checkpointing, a crash means
starting the entire plan from scratch.

### Solution

The executor periodically serializes its state to a JSON snapshot file.
The snapshot contains everything needed to resume: which tasks completed,
which are pending, gate results, and signal hashes for all artifacts
produced so far.

On restart, passing `--resume <path>` loads the snapshot and picks up where
execution left off.

### Structure

```rust
#[derive(Serialize, Deserialize)]
pub struct ExecutorSnapshot {
    pub plan_id: String,
    pub created_at: DateTime<Utc>,
    pub tasks: Vec<TaskSnapshot>,
    pub completed_signals: Vec<Blake3Hash>,
    pub gate_results: Vec<GateResult>,
    pub current_task_index: Option<usize>,
}

#[derive(Serialize, Deserialize)]
pub struct TaskSnapshot {
    pub task_id: String,
    pub status: TaskStatus,  // Pending | Running | Completed | Failed | Skipped
    pub attempts: u32,
    pub last_error: Option<String>,
    pub output_signal: Option<Blake3Hash>,
}
```

```
  Normal run:
  [Task 1] -> [Task 2] -> [Task 3] -> ... -> [Task N]
                              |
                           CRASH
                              |
  Resume run:                 v
  (skip 1)    (skip 2)    [Task 3] -> ... -> [Task N]
```

### Trade-offs

- **Snapshot frequency**: writing after every task is safe but adds I/O
  overhead. Writing every N tasks risks losing up to N tasks of progress.
  The default is to snapshot after each task completes.
- **Schema evolution**: changing the snapshot struct requires a migration
  path for old snapshot files. A `version` field in the snapshot enables
  forward-compatible deserialization.
- **Non-determinism**: resuming may produce different results than a clean
  run if external state changed during the interruption (e.g. a dependency
  was updated). The snapshot records signal hashes so divergence can be
  detected.

### Example

```bash
# Start a plan run
tiagent plan run plans/refactor-auth/

# Process crashes after completing 7 of 12 tasks
# Snapshot written to .tiagent/state/executor.json

# Resume from where it stopped
tiagent plan run plans/refactor-auth/ --resume .tiagent/state/executor.json
# Tasks 1-7 are skipped, execution continues from task 8
```

---

## 7. Episode Logging Pattern

> Works in standalone mode (no Celestia required)

### Problem

Debugging agent behavior requires knowing exactly what happened: what
prompt was sent, what the model returned, which tools were called, what
those tools returned, how long each step took, and how much it cost.
Standard application logging is too unstructured for this.

### Solution

Every agent turn is recorded as an **Episode** -- a structured JSON object
capturing the full round-trip of a single model interaction. Episodes are
appended to a JSONL file (one JSON object per line), making the log both
human-readable and machine-parseable.

### Structure

```rust
#[derive(Serialize, Deserialize)]
pub struct Episode {
    pub id: Uuid,
    pub task_id: String,
    pub agent_id: String,
    pub model: String,
    pub timestamp: DateTime<Utc>,

    // Input
    pub system_prompt: String,
    pub user_prompt: String,
    pub context_signals: Vec<Blake3Hash>,

    // Output
    pub response_text: String,
    pub tool_calls: Vec<ToolCallRecord>,
    pub tool_results: Vec<ToolResultRecord>,

    // Metrics
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
    pub latency_ms: u64,

    // Provenance
    pub signal_hash: Blake3Hash,
    pub gate_verdict: Option<GateVerdict>,
    pub hdc_fingerprint: Option<Vec<f32>>,
}
```

```
.tiagent/episodes.jsonl:

{"id":"a1b2...","task_id":"T01","model":"sonnet","input_tokens":1200,...}
{"id":"c3d4...","task_id":"T01","model":"sonnet","input_tokens":800,...}
{"id":"e5f6...","task_id":"T02","model":"opus","input_tokens":3400,...}
```

### Trade-offs

- **Disk usage**: episodes include full prompt and response text. A busy
  agent can generate gigabytes of episode data per day. Rotation and
  archival (see Dual-Layer Storage) manage this.
- **Privacy**: episodes may contain sensitive data from tool calls. Access
  controls on the episode log and redaction filters are available but must
  be configured.
- **Append-only constraint**: JSONL cannot be updated in place. Correcting
  a logged episode requires appending a superseding record.

### Example

After every model call, the runtime appends an episode:

```rust
let episode = Episode {
    id: Uuid::new_v4(),
    task_id: current_task.id.clone(),
    agent_id: agent.id.clone(),
    model: selected_model.to_string(),
    timestamp: Utc::now(),
    system_prompt: assembled.system.clone(),
    user_prompt: assembled.user.clone(),
    context_signals: assembled.source_hashes.clone(),
    response_text: response.text.clone(),
    tool_calls: response.tool_calls.clone(),
    tool_results: tool_results.clone(),
    input_tokens: response.usage.input_tokens,
    output_tokens: response.usage.output_tokens,
    cost_usd: response.usage.cost_usd(),
    latency_ms: elapsed.as_millis() as u64,
    signal_hash: response_signal.hash,
    gate_verdict: None,  // filled in after gating
    hdc_fingerprint: None,  // filled in by neuro subsystem
};
episode_logger.append(&episode).await?;
```

Every tool call and model response is recorded so the agent can learn from
past coding sessions. These episodes serve multiple downstream consumers:
the dashboard displays them in real time, the learning subsystem uses them
to update cascade router weights, and the TraceCommons integration submits
anonymized versions to the shared dataset.

---

## 8. Context Bidding Pattern

> Works in standalone mode (no Celestia required)

### Problem

An agent's prompt has a finite token budget. Many sources want to inject
context: the task description, relevant code files, past episodes, research
notes, knowledge store entries, playbook instructions. Naively
concatenating everything overflows the context window and degrades model
performance.

### Solution

Each context source submits a **bid** declaring how much space it wants and
how important its content is. The Composer runs an allocation round,
distributing the available token budget across bidders by priority, then
assembles the prompt from the winning bids.

### Structure

```rust
pub struct ContextBid {
    pub source: BidderKind,       // Task | Neuro | Research | Playbook | Code
    pub priority: f64,            // 0.0 to 1.0, higher wins
    pub content: String,
    pub estimated_tokens: usize,
}

pub enum BidderKind {
    Task,       // the task description itself
    Neuro,      // knowledge store entries
    Research,   // research artifacts
    Playbook,   // reusable instruction sets
    Code,       // relevant source files
}

impl Composer {
    pub fn allocate(
        bids: &mut [ContextBid],
        budget: usize,
    ) -> Vec<&ContextBid> {
        bids.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap());
        let mut remaining = budget;
        let mut accepted = Vec::new();
        for bid in bids {
            if bid.estimated_tokens <= remaining {
                remaining -= bid.estimated_tokens;
                accepted.push(bid);
            }
        }
        accepted
    }
}
```

```
  Available budget: 8000 tokens

  Bid: Task description   (priority 1.0, 1200 tokens)  -> ACCEPTED
  Bid: Playbook rules     (priority 0.9, 800 tokens)   -> ACCEPTED
  Bid: Relevant code file (priority 0.7, 3000 tokens)  -> ACCEPTED
  Bid: Research notes     (priority 0.6, 2500 tokens)   -> ACCEPTED
  Bid: Knowledge entries  (priority 0.4, 4000 tokens)   -> REJECTED (over budget)

  Total allocated: 7500 / 8000 tokens
```

### Trade-offs

- **Greedy allocation**: the current algorithm is greedy by priority. A
  lower-priority bid that fits in the remaining space may be more useful
  than a higher-priority bid that was already accepted. A knapsack solver
  would optimize total value but adds complexity.
- **Token estimation accuracy**: bids estimate token counts before
  tokenization. If estimates are off, the assembled prompt may exceed the
  context window. A post-assembly token count with truncation handles this.
- **Static priorities**: bid priorities are currently set per source type.
  Task-specific priority tuning (e.g. "this refactor task benefits more from
  code context than research") is not yet implemented.

### Example

When dispatching an agent for a code-generation task, the runtime collects
bids from all registered `AttentionBidder` implementations:

```rust
let bidders: Vec<Box<dyn AttentionBidder>> = vec![
    Box::new(TaskBidder::new(&task)),
    Box::new(NeuroBidder::new(&knowledge_store, &task)),
    Box::new(ResearchBidder::new(&research_artifacts)),
    Box::new(PlaybookBidder::new(&playbook_store, &task)),
    Box::new(CodeBidder::new(&code_index, &task)),
];

let mut bids = Vec::new();
for bidder in &bidders {
    if let Some(bid) = bidder.bid(&task_context).await? {
        bids.push(bid);
    }
}

let accepted = composer.allocate(&mut bids, model.context_window - reserved);
let prompt = composer.assemble_from_bids(&accepted, &task_context);
```

---

## 9. Dual-Layer Storage Pattern

> Requires Celestia integration (optional)

### Problem

Agent data has different access patterns and durability requirements.
Active execution needs fast local reads and writes. Auditability requires
data to be shared and tamper-evident. Long-term archival needs permanence
beyond any single node's lifetime.

### Solution

tiagent uses three storage layers, each optimized for a different access
pattern:

| Layer | Backend     | Access     | Durability        | Latency  |
|-------|-------------|------------|-------------------|----------|
| Hot   | Local FS    | Read/write | Node lifetime     | < 1ms    |
| Warm  | Celestia DA | Append     | ~7-day DA window  | seconds  |
| Cold  | Arweave     | Append     | Permanent         | seconds  |

The key design decision is that the warm layer stores **commitments** (hashes
and metadata), not full payloads. Full data lives in the hot layer. This
keeps DA costs low while providing verifiability.

### Structure

```
  Hot Layer (local)                 Warm Layer (Celestia DA)
  +------------------+             +----------------------+
  | signals.jsonl    |  -- hash -> | commitment blob      |
  | episodes.jsonl   |  -- hash -> | commitment blob      |
  | state/*.json     |             | (hash + metadata)    |
  +------------------+             +----------------------+
                                            |
                                            | (periodic promotion)
                                            v
                                   Cold Layer (Arweave)
                                   +----------------------+
                                   | full archived data   |
                                   | (permanent)          |
                                   +----------------------+
```

```rust
pub struct DualLayerSubstrate {
    hot: FileSubstrate,
    warm: CelestiaSubstrate,
    cold: Option<ArweaveSubstrate>,
}

impl DualLayerSubstrate {
    pub async fn write_with_commitment(&self, signal: Signal) -> Result<Blake3Hash> {
        // 1. Write full signal to local storage
        let hash = self.hot.write(signal.clone()).await?;

        // 2. Submit commitment (hash + metadata) to Celestia DA
        let commitment = Commitment {
            signal_hash: hash,
            kind: signal.kind,
            timestamp: signal.created_at,
            payload_size: signal.payload.to_string().len(),
        };
        self.warm.submit_commitment(&commitment).await?;

        Ok(hash)
    }
}
```

### Trade-offs

- **Consistency window**: a Signal may exist in the hot layer before its
  commitment lands on Celestia (seconds of delay). Reads that require
  verified data must wait for commitment confirmation.
- **Recovery gap**: if the hot layer is lost before cold archival, only the
  commitments survive on Celestia. The data itself is gone. Replication
  across nodes or more frequent cold promotion mitigates this.
- **Cost management**: Celestia DA costs scale with blob count. Batching
  commitments (one blob per N signals) reduces cost but increases the
  consistency window.

### Example

After a plan run completes, the executor writes all result Signals to the
hot layer and submits a batch commitment to Celestia. The commitment blob
contains the Merkle root of all signal hashes, allowing any verifier to
check that a specific signal was part of the run. Weekly, a background job
promotes completed plan data from hot storage to Arweave for permanent
archival.

---

## 10. Effect Pipeline Pattern

> Works in standalone mode (no Celestia required)

### Problem

Agent side effects (file writes, shell commands, API calls, deployments)
are dangerous. An unchecked agent that directly executes
side effects can cause irreversible damage. Debugging what went wrong
requires knowing not just the outcome but the full lifecycle of each
attempted effect.

### Solution

Every side effect passes through a four-stage pipeline, where each stage
is a typed transition with explicit state:

```
Intent -> Claim -> Attempt -> Outcome
```

- **Intent**: the agent declares what it wants to do ("write file X with
  content Y").
- **Claim**: the Policy trait reviews the intent and either approves or
  rejects it. Approved intents become Claims.
- **Attempt**: the runtime executes the Claim. The execution itself is
  recorded regardless of success or failure.
- **Outcome**: the result (success with output, or failure with error) is
  captured as the final state.

Each stage is a Signal in the DAG, so the full lifecycle is traceable.

### Structure

```rust
pub enum EffectStage {
    Intent {
        tool_name: String,
        parameters: serde_json::Value,
        risk_tier: RiskTier,
    },
    Claim {
        intent_hash: Blake3Hash,
        approved_by: PolicyId,
        conditions: Vec<String>,  // e.g. "sandbox only", "dry run first"
    },
    Attempt {
        claim_hash: Blake3Hash,
        started_at: DateTime<Utc>,
    },
    Outcome {
        attempt_hash: Blake3Hash,
        success: bool,
        result: serde_json::Value,
        duration_ms: u64,
    },
}
```

```
  Agent wants to run `cargo test`:

  [Intent: run cargo test]
        |
        v  (Policy checks: tool is Safe tier, auto-approve)
  [Claim: approved by DefaultPolicy]
        |
        v  (Runtime executes)
  [Attempt: started at T0]
        |
        v  (Capture result)
  [Outcome: success, 47 tests passed, 3200ms]
```

### Trade-offs

- **Overhead for safe operations**: even a harmless `read_file` call
  passes through all four stages. Safe-tier tools use a fast path that
  collapses Intent+Claim+Attempt into a single step, but the Outcome is
  always recorded.
- **Blocking on approval**: Dangerous-tier effects require human
  confirmation at the Claim stage. This introduces latency but prevents
  catastrophic errors.
- **Rollback limitations**: not all effects are reversible. A sent HTTP
  request cannot be unsent. The pipeline records the Outcome for audit
  but cannot guarantee undo.

### Example

An agent attempts to run a destructive shell command. The Policy trait
classifies this as `RiskTier::Dangerous` and pauses for human confirmation:

```rust
let intent = EffectStage::Intent {
    tool_name: "run_shell".into(),
    parameters: json!({"command": "rm -rf build/ && make deploy"}),
    risk_tier: RiskTier::Dangerous,
};
let intent_signal = Signal::new(SignalKind::Effect, to_value(&intent)?, vec![]);
substrate.write(intent_signal.clone()).await?;

// Policy evaluates
let decision = policy.evaluate(&intent).await?;
match decision {
    PolicyDecision::Approve(conditions) => {
        let claim = EffectStage::Claim { /* ... */ };
        // proceed to Attempt
    }
    PolicyDecision::Deny(reason) => {
        // log denial, return error to agent
    }
    PolicyDecision::RequireHuman => {
        // pause execution, notify human via dashboard
    }
}
```

---

## 11. Push-Based Dashboard Pattern

> Works in standalone mode (no Celestia required)

### Problem

Monitoring agent progress by polling is wasteful and introduces latency.
An HTTP endpoint that returns the current state on every request creates
unnecessary load. Users expect real-time updates in the TUI, web
dashboard, and API consumers.

### Solution

State changes in the runtime emit events through `tokio::sync::watch`
channels. Consumers subscribe to the channel and receive updates the
instant they occur. No polling, no wasted requests.

The pattern supports multiple consumer types through a single event
stream:

- **TUI**: the ratatui dashboard subscribes and redraws on each event.
- **SSE**: the HTTP server streams events to browser clients.
- **WebSocket**: bidirectional connections for interactive dashboards.

### Structure

```rust
pub enum DashboardEvent {
    TaskStarted { task_id: String, agent_id: String },
    TaskCompleted { task_id: String, verdict: GateVerdict },
    TaskFailed { task_id: String, error: String },
    AgentSpawned { agent_id: String, model: String },
    AgentStopped { agent_id: String },
    MetricsUpdated { tokens: u64, cost: f64, elapsed: Duration },
    PlanProgress { completed: usize, total: usize },
}

pub struct StateHub {
    sender: watch::Sender<DashboardEvent>,
}

impl StateHub {
    pub fn emit(&self, event: DashboardEvent) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> watch::Receiver<DashboardEvent> {
        self.sender.subscribe()
    }
}
```

```
  Runtime                    StateHub                  Consumers
  +--------+                +---------+               +----------+
  | Runner | --emit()-----> | watch   | --subscribe-> | TUI      |
  | Agent  | --emit()-----> | channel | --subscribe-> | SSE      |
  | Gate   | --emit()-----> |         | --subscribe-> | WebSocket|
  +--------+                +---------+               +----------+
```

### Trade-offs

- **Backpressure**: `watch` channels keep only the latest value. If a
  consumer is slow, it misses intermediate events. For progress tracking
  this is acceptable (the consumer sees the current state). For event
  logs, a `broadcast` channel with bounded capacity is used instead.
- **Serialization cost**: events are cloned for each subscriber. Wrapping
  events in `Arc` reduces copying for large payloads.
- **Single-process limitation**: `watch` channels work within one process.
  Cross-process consumers (e.g. a separate web dashboard) connect via
  SSE or WebSocket, which the HTTP server bridges from the internal
  channel.

### Example

The TUI dashboard subscribes to the StateHub and redraws on every event:

```rust
let mut rx = state_hub.subscribe();
loop {
    tokio::select! {
        Ok(()) = rx.changed() => {
            let event = rx.borrow().clone();
            match event {
                DashboardEvent::PlanProgress { completed, total } => {
                    tui.update_progress_bar(completed, total);
                }
                DashboardEvent::TaskFailed { task_id, error } => {
                    tui.show_error(&task_id, &error);
                }
                _ => tui.refresh(),
            }
            tui.draw(&mut terminal)?;
        }
        _ = shutdown.recv() => break,
    }
}
```

---

## 12. Tool Safety Tier Pattern

> Works in standalone mode (no Celestia required)

### Problem

Agents invoke tools to interact with the outside world. Some tools are
harmless (reading a file), some modify local state (writing a file), and
some have irreversible external effects (deploying to production, calling
an external API). Treating all tools equally either blocks safe operations with
unnecessary confirmation prompts or lets dangerous operations execute
without oversight.

### Solution

Every tool is classified into one of three risk tiers:

| Tier       | Examples                          | Authorization     |
|------------|-----------------------------------|-------------------|
| Safe       | read_file, list_dir, search_code  | Auto-approved     |
| Moderate   | write_file, run_shell, edit_file  | Logged, auto-ok   |
| Dangerous  | run_deploy, rm_recursive, curl     | Human confirmation|

The Policy trait checks the tier before every tool invocation. Safe tools
execute immediately. Moderate tools execute but are logged for audit.
Dangerous tools pause execution and request human confirmation via the
dashboard or CLI prompt.

### Structure

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RiskTier {
    Safe,
    Moderate,
    Dangerous,
}

pub struct ToolManifest {
    pub name: String,
    pub description: String,
    pub tier: RiskTier,
    pub parameters: Vec<ToolParameter>,
}

#[async_trait]
pub trait Policy: Send + Sync {
    async fn authorize(
        &self,
        tool: &ToolManifest,
        params: &serde_json::Value,
        agent_context: &AgentContext,
    ) -> Result<PolicyDecision>;
}

pub enum PolicyDecision {
    Approve(Vec<Condition>),
    Deny(String),
    RequireHuman,
}
```

```
  Tool invocation request
         |
         v
  +------------------+
  | Policy::authorize|
  +------------------+
         |
    +----+----+--------+
    |         |        |
    v         v        v
  [Safe]  [Moderate] [Dangerous]
    |         |        |
    v         v        v
  Execute   Execute   Wait for
  silently  + log     human OK
    |         |        |
    v         v        v
  Return    Return    Execute
  result    result    + log
```

### Trade-offs

- **Tier assignment accuracy**: a tool miscategorized as Safe when it
  should be Dangerous creates a security gap. The default tier for unknown
  tools is Dangerous (fail-closed).
- **Human bottleneck**: too many Dangerous-tier tools slow down autonomous
  execution. Operators can override tiers in configuration for trusted
  environments (e.g. sandboxed CI).
- **Context-dependent risk**: `run_shell` with `ls` is Safe but
  `run_shell` with `rm -rf /` is Dangerous. Parameter-level risk analysis
  is planned but not yet implemented; the tier currently applies to the
  tool as a whole.

### Example

An agent calls `write_file` (Moderate tier) and `run_deploy`
(Dangerous tier) in sequence:

```rust
// write_file: Moderate tier, auto-approved with logging
let decision = policy.authorize(&write_file_tool, &params, &ctx).await?;
// decision = Approve([])
// -> execute immediately, log to audit trail

// run_deploy: Dangerous tier, requires human confirmation
let decision = policy.authorize(&deploy_tool, &params, &ctx).await?;
// decision = RequireHuman
// -> emit DashboardEvent::HumanApprovalRequired
// -> pause execution
// -> human clicks "Approve" in TUI
// -> execution continues
```

---

## Pattern Interactions

These patterns do not exist in isolation. A single agent turn exercises
many of them simultaneously:

1. The **Universal Loop** orchestrates the turn.
2. The **Cascade Router** picks the model.
3. **Context Bidding** assembles the prompt.
4. The model response triggers tool calls processed through the **Effect
   Pipeline** and **Tool Safety Tiers**.
5. Results pass through the **Gate Pipeline**.
6. Everything is recorded as **Signals** in the **DAG** and as
   **Episodes** in the log.
7. State is checkpointed via **Snapshot-Resume**.
8. Signals flow to **Dual-Layer Storage** for durability.
9. The **Push-Based Dashboard** reflects progress in real time.

**Example: a regular coding task.** An agent is asked to add input
validation to a REST handler. The **Cascade Router** (4) picks Sonnet
based on past success rates for the `code-generation` category. **Context
Bidding** (8) selects the handler source file, the project's validation
conventions from the knowledge store, and the task description. The model
response includes a `write_file` call, which the **Effect Pipeline** (10)
approves at Moderate tier via **Tool Safety** (12). The **Gate Pipeline**
(5) runs compile, test, and lint rungs against the result. Everything --
the prompt, the response, each tool call -- is captured as **Signals** in
the **DAG** (1) and as an **Episode** (7) in the log. The executor writes
a **Snapshot** (6) and the **Push-Based Dashboard** (11) updates the TUI
in real time. No Celestia or blockchain integration is involved.

Understanding these patterns and their interactions is the fastest path to
navigating the tiagent codebase.
