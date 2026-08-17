# tiagent: Core Architecture Design

This document is the definitive technical reference for tiagent's architecture. It explains
every core abstraction from scratch --- the data model, the trait system, the execution loop,
the layered runtime, and the extension points --- so that a reader with no prior context can
understand how tiagent works and why it is designed the way it is.

If you have not read **01-vision-and-overview.md**, here is the one-sentence summary: tiagent
is a general-purpose coding agent harness --- an alternative to Claude Code, Codex, or Cursor ---
that improves itself through experience, with optional Celestia integration for shared
cross-agent learning.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [How This Architecture Serves You as a Developer](#how-this-architecture-serves-you-as-a-developer)
3. [The Signal Model](#2-the-signal-model)
4. [The Trait System (Six Verbs)](#3-the-trait-system-six-verbs)
5. [The Universal Loop](#4-the-universal-loop)
6. [Layered Architecture](#5-layered-architecture)
7. [Celestia Integration Points (Optional)](#6-celestia-integration-points)
8. [Model Agnosticism and Backend Dispatch](#7-model-agnosticism-and-backend-dispatch)
9. [Extension Points](#8-extension-points)
10. [Comparison with Monolithic Frameworks](#9-comparison-with-monolithic-frameworks)
11. [Putting It All Together](#10-putting-it-all-together)

---

## 1. Architecture Overview

tiagent's architecture follows a principle that can be stated in one sentence:

> **One noun, six verbs.**

The single noun is **Signal** --- every piece of data that flows through tiagent is a Signal.
A user prompt is a Signal. An LLM response is a Signal. A tool call is a Signal. A test
result is a Signal. A learning artifact published to Celestia is a Signal. There is no
separate "Message" type, no "Event" type, no "Task" type. Everything is a Signal with a kind
field that tells you what it represents.

The six verbs are **traits** (interfaces, in Rust terminology) that define operations on
Signals:

| Verb | Trait | What it does |
|------|-------|-------------|
| Store | `Substrate` | Persist and retrieve Signals (local filesystem, Celestia DA, or both) |
| Evaluate | `Scorer` | Assign a numeric score to a Signal (priority, complexity, relevance) |
| Validate | `Gate` | Check whether a Signal meets quality criteria (compile, test, lint) |
| Route | `Router` | Choose which model, backend, and execution path to use |
| Assemble | `Composer` | Build the prompt that gets sent to an LLM from templates, context, tools, and history |
| Authorize | `Policy` | Decide whether an action is allowed, based on safety rules and role permissions |

These six traits are the entire kernel of tiagent. Everything else --- the CLI, the HTTP API,
the Celestia integration, the learning system, the tool infrastructure --- is built on top of
these six traits operating on Signals.

### Why this design?

Most agent frameworks are organized around **nouns**: an Agent class, a Tool class, a Memory
class, a Planner class. This leads to a proliferation of types that must all be coordinated,
and it makes extension difficult because adding a new capability often requires touching
multiple noun classes.

tiagent inverts this. There is one noun (Signal) and behavior is defined by verb traits.
Adding a new capability means implementing an existing trait, not inventing a new type.
Swapping behavior means providing a different trait implementation, not restructuring the
data model.

This also means the system is composable at the trait level. You can mix and match:

- A `CelestiaSubstrate` (stores Signals on Celestia's DA layer) with a `LocalSubstrate`
  (stores Signals as local JSONL files) --- or use both via a `HybridSubstrate`.
- A `CascadeRouter` (selects models based on learned performance data) with a
  `FixedRouter` (always uses the same model) --- swap one line of configuration.
- A `StrictPolicy` (requires human approval for destructive actions) with a
  `PermissivePolicy` (allows everything) --- choose based on your deployment's risk
  tolerance.

The traits define the interfaces. Implementations are pluggable. Configuration selects which
implementations are active.

---

## How This Architecture Serves You as a Developer

Before diving into the details, here is what the architecture means in practice when you
use tiagent as a coding agent.

**The universal loop is what runs when you type `tiagent run "implement feature X"`.**
Your prompt enters the system as a Signal, gets scored for complexity, gets routed to the
right model, gets assembled into a rich prompt with context and tools, gets executed by the
LLM (including any tool calls like reading files, running commands, or editing code), gets
verified by automated quality checks, and gets persisted so the system can learn from it.
All eight stages happen automatically. You type a prompt; you get verified, tested output.

**The Signal DAG is how tiagent tracks what it did and learns from it.** Every action the
agent takes --- every prompt, every LLM response, every file edit, every test run --- is
recorded as an immutable, content-addressed Signal. These Signals form a directed acyclic
graph that captures the full causal history of every execution. When something goes wrong,
you can walk the DAG backward from any result to see exactly what happened and why. When
something goes right, the system extracts reusable patterns (playbooks) for future tasks.

**Gates are automated quality checks that run after every agent action.** After the LLM
generates code, tiagent automatically runs a pipeline of checks: Does it compile? Do the
tests pass? Does it pass linting? Is the diff reasonable? This happens without you asking
for it. If a gate fails, tiagent can retry with the same model, escalate to a stronger
model, or replan the approach entirely --- all configured, not manual.

**The Substrate trait means storage is pluggable.** By default, tiagent stores everything
as local JSONL files. That is the complete, fully functional default. No blockchain, no
external dependencies, no network calls for storage. If you later want to share learning
across agents or teams, you can enable the optional Celestia substrate --- but most
developers will never need to.

**The CascadeRouter learns which model works best for your tasks.** tiagent starts with
the cheapest model that is likely to succeed. If it fails, it escalates to a stronger
(more expensive) model. Over time, it builds a profile: "for test-writing tasks, Sonnet
succeeds 94% of the time, so skip Haiku for those." This routing data persists across
sessions. Your agent gets cheaper and more reliable the more you use it.

**This is what makes tiagent get better over time, unlike static tools.** Claude Code,
Codex, and Cursor execute your prompt and forget about it. tiagent records every execution
as an episode, learns from successes and failures, adapts its model routing and gate
thresholds, and uses past trajectories to inform future tasks. The hundredth task you run
benefits from the ninety-nine that came before it.

---

## 2. The Signal Model

### What is a Signal?

A Signal is the universal data atom in tiagent. Every piece of information that enters,
flows through, or exits the system is represented as a Signal. This includes:

- A user prompt ("deploy my rollup to Mocha testnet")
- An LLM response (the model's generated text)
- A tool call (e.g., "submit this blob to namespace X")
- A tool result (e.g., "blob submitted, height 12345, hash 0xabc...")
- A gate result (e.g., "compilation passed", "3 tests failed")
- An episode summary (a structured record of a complete agent execution)
- A learning artifact (updated routing weights, a new playbook)
- A coordination message between agents

### The Signal struct

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The universal data atom. Every piece of information in tiagent is a Signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    /// Content-addressed identity. Computed as SHA-256 of the serialized payload.
    /// This means two Signals with identical payloads always have the same id,
    /// regardless of when or where they were created.
    pub id: Hash,

    /// What type of data this Signal carries. Used for filtering, routing,
    /// and determining how to interpret the payload.
    pub kind: SignalKind,

    /// The actual data. Serialized as JSON for flexibility. The structure of
    /// the payload depends on the `kind` field.
    pub payload: serde_json::Value,

    /// Key-value metadata attached to the Signal. Always includes at least
    /// `created_at` (timestamp) and `source` (what created this Signal).
    /// May also include provenance chains, cost data, model identifiers, etc.
    pub metadata: HashMap<String, serde_json::Value>,

    /// Optional parent Signal hash. When present, this creates a directed edge
    /// in the Signal DAG. For example, a Response Signal's parent is the Prompt
    /// Signal that produced it. A GateResult Signal's parent is the Response
    /// Signal it validated.
    pub parent: Option<Hash>,

    /// Celestia namespace for DA submission. Determines which namespace this
    /// Signal will be published to when written to the DA layer. Agents,
    /// data types, and coordination groups each have their own namespaces.
    pub namespace: Option<Namespace>,
}
```

### Content addressing

Every Signal's `id` is a SHA-256 hash of its serialized payload. This has three important
consequences:

1. **Deduplication**: If the same data enters the system twice, it produces the same hash
   and can be deduplicated automatically. Two agents that independently generate the same
   tool call produce Signals with the same id.

2. **Integrity**: Given a Signal's id, you can verify that its payload has not been tampered
   with by recomputing the hash. This is particularly important for Signals retrieved from
   the DA layer, where you want to verify that what you read is what was originally written.

3. **Referencing**: Signals can reference each other by id without embedding the full content.
   A GateResult can reference the Response it validated by storing the Response's hash in its
   `parent` field. This creates a lightweight DAG (directed acyclic graph) of Signal
   relationships.

### Signal kinds

The `kind` field is an enum that categorizes Signals:

```rust
/// Categorizes what a Signal represents. The payload structure
/// varies by kind --- see documentation for each variant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SignalKind {
    // --- Input signals ---
    /// A user prompt or task description. Payload: { "text": "...", "context": {...} }
    Prompt,
    /// A scheduled or event-triggered task. Payload: { "trigger": "...", "params": {...} }
    Event,

    // --- Execution signals ---
    /// An LLM response. Payload: { "text": "...", "model": "...", "usage": {...} }
    Response,
    /// A request to invoke a tool. Payload: { "tool": "...", "args": {...} }
    ToolCall,
    /// The result of a tool invocation. Payload: { "tool": "...", "result": {...} }
    ToolResult,

    // --- Validation signals ---
    /// The result of a gate check. Payload: { "gate": "...", "passed": bool, "details": {...} }
    GateResult,

    // --- Learning signals ---
    /// A complete episode (structured trace of an agent execution).
    /// Payload: { "turns": [...], "score": f64, "duration_ms": u64 }
    Episode,
    /// A reusable strategy extracted from successful episodes.
    /// Payload: { "pattern": "...", "steps": [...], "success_rate": f64 }
    Playbook,
    /// Updated model routing weights. Payload: { "weights": {...} }
    RoutingUpdate,

    // --- Coordination signals ---
    /// A message between agents. Payload: { "from": "...", "to": "...", "body": {...} }
    Coordination,
    /// A proof of completed work. Payload: { "task": "...", "result_hash": "...", "attestation": {...} }
    WorkProof,

    // --- Meta signals ---
    /// An HDC fingerprint (compact behavioral signature).
    /// Payload: { "vector": [...], "dimensions": usize }
    Fingerprint,
    /// System-level state (config changes, lifecycle events).
    /// Payload varies.
    System,
}
```

### The Signal DAG

Signals form a **directed acyclic graph** (DAG) through their `parent` fields. This DAG
captures the causal relationships between all data in the system:

```
                    ┌─────────────────┐
                    │   Prompt        │  "deploy rollup to Mocha"
                    │   id: 0xa1b2... │
                    └────────┬────────┘
                             │ parent
                    ┌────────▼────────┐
                    │   Response      │  LLM generates deployment plan
                    │   id: 0xc3d4... │
                    └───┬─────────┬───┘
                        │         │ parent
               ┌────────▼───┐ ┌──▼───────────┐
               │  ToolCall   │ │  ToolCall     │
               │  "check     │ │  "submit      │
               │   config"   │ │   blob"       │
               │  id: 0xe5.. │ │  id: 0xf6..   │
               └────────┬───┘ └──┬────────────┘
                        │        │ parent
               ┌────────▼───┐ ┌──▼───────────┐
               │ ToolResult  │ │ ToolResult    │
               │ "config ok" │ │ "blob at      │
               │ id: 0x17..  │ │  height 999"  │
               └─────────────┘ └──┬───────────┘
                                  │ parent
                          ┌───────▼───────┐
                          │  GateResult   │  compilation passed
                          │  id: 0x28...  │
                          └───────┬───────┘
                                  │ parent
                          ┌───────▼───────┐
                          │  Episode      │  full execution trace
                          │  id: 0x39...  │
                          └───────────────┘
```

The DAG is useful for:

- **Auditing**: Walk the DAG backward from any result to see every step that produced it.
- **Debugging**: When a gate fails, follow the parent chain to find the tool call or
  response that caused the failure.
- **Learning**: Episode Signals reference the entire sub-DAG of their execution, making
  it possible to analyze complete trajectories.
- **Provenance on DA**: When Signals are published to Celestia, the DAG structure is
  preserved. Any agent can verify the provenance chain of a published result.

### Immutability

Signals are **immutable** once created. You never update a Signal; you create a new Signal
that references the old one. This is a deliberate design choice:

- It makes the DAG append-only, which aligns with Celestia's append-only DA layer.
- It eliminates an entire class of concurrency bugs (no mutable shared state).
- It enables content addressing (if Signals could change, their hashes would become
  invalid).
- It makes auditing trivial (the complete history is always preserved).

If you need to "correct" a Signal, you create a new Signal with the corrected payload and
add metadata indicating it supersedes the old one. The old Signal remains in the DAG for
auditability.

---

## 3. The Trait System (Six Verbs)

tiagent's behavior is defined by six traits. Each trait corresponds to one "verb" --- one
category of operation that the system performs on Signals. This section defines each trait
with its Rust interface, explains its role, and describes the concrete implementations that
tiagent ships with.

### 3.1 Substrate (store)

The `Substrate` trait defines how Signals are persisted and retrieved. It is the storage
layer of the system.

```rust
/// Persists and retrieves Signals. Implementations determine where
/// and how data is stored --- local filesystem, Celestia DA layer,
/// or a combination of both.
#[async_trait]
pub trait Substrate: Send + Sync {
    /// Write a Signal to storage. Returns the content-addressed hash
    /// that can be used to retrieve the Signal later.
    async fn write(&self, signal: &Signal) -> Result<Hash>;

    /// Read a Signal by its content-addressed hash. Returns None if
    /// the Signal is not found in this substrate.
    async fn read(&self, hash: &Hash) -> Result<Option<Signal>>;

    /// Query Signals by filter criteria. Filters can match on kind,
    /// metadata fields, time ranges, parent relationships, and more.
    async fn query(&self, filter: &SignalFilter) -> Result<Vec<Signal>>;

    /// Delete a Signal from local storage. Note: Signals published to
    /// Celestia's DA layer cannot be deleted --- this only affects the
    /// local cache/store.
    async fn delete(&self, hash: &Hash) -> Result<bool>;

    /// Check whether a Signal exists in this substrate without
    /// fetching its full content.
    async fn exists(&self, hash: &Hash) -> Result<bool>;
}
```

**Shipped implementations:**

| Implementation | Storage target | Use case |
|----------------|---------------|----------|
| `LocalSubstrate` | Local filesystem (JSONL files) and optional SQLite | **Default for all users.** Fast, zero-dependency, fully functional. Works offline. |
| `CelestiaSubstrate` | Celestia DA layer (blobs in namespaces) | **Optional.** For users who want shared cross-agent learning via the DA layer. |
| `HybridSubstrate` | Both local and Celestia | **Optional.** Fast local reads with DA-backed durability and sharing. |

Most developers will use `LocalSubstrate` exclusively. It stores Signals as JSONL files
in the `.tiagent/` directory and provides the same query, persistence, and learning
capabilities as any other substrate. No blockchain node, no network calls, no
configuration beyond the default.

`CelestiaSubstrate` is for users who want cross-agent learning: publishing execution
episodes to Celestia's DA layer so that other agents can retrieve and learn from them.
The `HybridSubstrate` combines both --- it writes locally first (for speed), then
asynchronously publishes to the DA layer (for sharing). These are opt-in via configuration.

### 3.2 Scorer (evaluate)

The `Scorer` trait assigns numeric scores to Signals. Scores are used throughout the system
for prioritization, filtering, and routing decisions.

```rust
/// Evaluates a Signal and produces a score between 0.0 and 1.0.
/// Higher scores indicate greater relevance, priority, or quality.
/// Multiple scorers can be composed --- their outputs are combined
/// using configurable aggregation (weighted average, max, min).
#[async_trait]
pub trait Scorer: Send + Sync {
    /// Evaluate a Signal and return a score in [0.0, 1.0].
    /// The meaning of the score depends on the scorer implementation:
    /// - PriorityScorer: how urgently this signal should be processed
    /// - ComplexityScorer: how difficult this task is (for model routing)
    /// - RelevanceScorer: how relevant this signal is to the current context
    /// - QualityScorer: how good this output is (for learning)
    async fn score(&self, signal: &Signal) -> Result<f64>;

    /// A human-readable name for this scorer, used in logging and
    /// debugging. Example: "complexity", "priority", "quality".
    fn name(&self) -> &str;
}
```

**Shipped implementations:**

| Implementation | What it measures | Used by |
|----------------|-----------------|---------|
| `ComplexityScorer` | Task difficulty (token count, tool requirements, domain specificity) | Router (to select appropriate model tier) |
| `PriorityScorer` | Urgency (deadline proximity, dependency criticality) | Orchestrator (to order task execution) |
| `QualityScorer` | Output quality (gate pass rate, test results, human feedback) | Learning system (to update routing weights) |
| `RelevanceScorer` | Contextual relevance (semantic similarity to current task) | Composer (to select which context to include) |

Scorers can be composed. For example, the Router might use a weighted combination of
complexity and priority scores to decide which model to use: a high-complexity, high-priority
task gets the strongest (most expensive) model, while a low-complexity, low-priority task
gets a fast, cheap model.

### 3.3 Gate (validate)

The `Gate` trait checks whether an output Signal meets quality criteria. Gates are the
verification layer that prevents bad outputs from propagating through the system.

```rust
/// Validates a Signal against quality criteria. Gates are organized
/// into a pipeline of "rungs" (levels), from basic structural checks
/// to deep semantic validation.
#[async_trait]
pub trait Gate: Send + Sync {
    /// Check whether a Signal passes this gate's criteria.
    /// Returns a GateResult containing pass/fail status, details
    /// about what was checked, and any diagnostic information.
    async fn check(
        &self,
        signal: &Signal,
        context: &GateContext,
    ) -> Result<GateResult>;

    /// A human-readable name for this gate. Example: "compile",
    /// "test", "lint", "diff-review".
    fn name(&self) -> &str;

    /// Which rung (level) this gate occupies in the pipeline.
    /// Lower rungs run first. If a lower rung fails, higher rungs
    /// are skipped (no point running tests if compilation fails).
    fn rung(&self) -> u8;
}

/// The result of a gate check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    /// The name of the gate that produced this result.
    pub gate: String,
    /// Whether the signal passed the gate's criteria.
    pub passed: bool,
    /// Human-readable explanation of the result.
    pub summary: String,
    /// Structured diagnostic details (compiler errors, test output, etc.).
    pub details: serde_json::Value,
    /// How long the check took.
    pub duration_ms: u64,
}
```

**The 7-rung gate pipeline:**

Gates are organized into seven rungs, executed in order. If a rung fails, subsequent rungs
are skipped. This avoids wasting time on expensive checks when cheap checks have already
found problems.

```
Rung 0: Parse        Does the output have valid structure?
                     (JSON parseable, expected fields present)
     │
     ▼
Rung 1: Compile      Does the generated code compile?
                     (cargo check, tsc --noEmit, go build)
     │
     ▼
Rung 2: Test         Do existing tests still pass?
                     (cargo test, npm test, go test)
     │
     ▼
Rung 3: Lint         Does the code pass linting rules?
                     (clippy, eslint, golint)
     │
     ▼
Rung 4: Diff         Is the diff reasonable?
                     (size limits, no accidental deletions, no secrets)
     │
     ▼
Rung 5: Semantic     Does the output match the intent?
                     (LLM-based review, requirement coverage check)
     │
     ▼
Rung 6: Human        Does a human approve?
                     (manual review gate, used for high-risk changes)
```

Not every task needs all seven rungs. The pipeline is configurable per-task:

- A code generation task might use rungs 0 through 5.
- A documentation task might use only rungs 0 and 4.
- A critical infrastructure change might require all seven, including human review.
- Gate thresholds adapt over time: if a particular rung consistently passes for a task type,
  its failure threshold is loosened (fewer false positives). If it consistently catches
  problems, its threshold is tightened. This adaptation uses exponential moving averages
  (EMA) and is persisted locally and to the DA layer.

### 3.4 Router (route)

The `Router` trait selects the execution path for a Signal --- which LLM model to use,
which backend to dispatch through, and which tool set to make available.

```rust
/// Selects the execution path for a Signal. The router considers
/// the signal's scores, the available backends, cost constraints,
/// and learned performance data to make routing decisions.
#[async_trait]
pub trait Router: Send + Sync {
    /// Given an input signal and the current routing context, select
    /// the best execution route. A Route specifies the model, backend,
    /// tool set, and any special configuration for this execution.
    async fn route(
        &self,
        signal: &Signal,
        context: &RoutingContext,
    ) -> Result<Route>;
}

/// A routing decision: which model, backend, and tools to use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    /// Which LLM model to use. Example: "claude-sonnet-4-20250514",
    /// "gpt-4o", "llama-3.1-70b".
    pub model: String,
    /// Which backend to dispatch through. Example: "claude-api",
    /// "openai-compat", "ollama".
    pub backend: String,
    /// Which tools to make available for this execution.
    /// None means "use the default tool set".
    pub tools: Option<Vec<String>>,
    /// Maximum tokens to generate.
    pub max_tokens: Option<u32>,
    /// Cost budget for this execution (in USD).
    pub cost_budget: Option<f64>,
    /// Optional metadata for the dispatch layer.
    pub params: HashMap<String, serde_json::Value>,
}
```

**Shipped implementations:**

| Implementation | Routing strategy | Use case |
|----------------|-----------------|----------|
| `CascadeRouter` | Uses learned performance weights to select from a cascade of models (strong, medium, fast). Starts with the cheapest model that historically succeeds for this task type; escalates to stronger models on failure. | Production default |
| `FixedRouter` | Always routes to the same model. Ignores scores and history. | Testing, development, cost-controlled deployments |
| `RoundRobinRouter` | Cycles through a list of models. Used for A/B testing prompt strategies across models. | Experimentation |

The `CascadeRouter` deserves more detail because it is the primary routing mechanism and a
key part of tiagent's self-improvement:

```
                     ┌──────────────────┐
                     │  Input Signal    │
                     │  + complexity    │
                     │    score         │
                     └────────┬─────────┘
                              │
                     ┌────────▼─────────┐
                     │ Check learned    │   Historical success rates
                     │ weights for this │   for each model on similar
                     │ task type        │   tasks (stored locally and
                     └────────┬─────────┘   on DA layer)
                              │
              ┌───────────────┼───────────────┐
              │               │               │
     ┌────────▼──────┐ ┌─────▼───────┐ ┌─────▼──────────┐
     │  Fast model   │ │ Mid model   │ │ Strong model    │
     │  (e.g. Haiku, │ │ (e.g.       │ │ (e.g. Opus,     │
     │   GPT-4o-mini)│ │  Sonnet,    │ │  GPT-4,         │
     │  $0.001/task  │ │  GPT-4o)    │ │  DeepSeek-R1)   │
     │               │ │ $0.01/task  │ │  $0.10/task      │
     └───────────────┘ └─────────────┘ └─────────────────┘
              │               │               │
              └───────────────┼───────────────┘
                              │
                     Selected based on:
                     1. Task complexity score
                     2. Learned success rates
                     3. Cost budget
                     4. Model availability
```

When a task completes, the Router records which model was used and whether the result
passed gates. Over time, this builds a profile: "for Celestia blob submission tasks,
Sonnet succeeds 95% of the time, so don't waste money on Opus for those." This data is
stored locally and published to the DA layer so new agents can start with community-learned
routing weights instead of the defaults.

### 3.5 Composer (assemble)

The `Composer` trait builds the prompt that gets sent to an LLM. This is not simple string
concatenation --- it assembles a multi-layer prompt from templates, contextual data, tool
definitions, conversation history, and learned strategies.

```rust
/// Assembles a complete prompt from multiple layers of context.
/// The composer is responsible for fitting everything within the
/// model's context window, prioritizing the most relevant information.
#[async_trait]
pub trait Composer: Send + Sync {
    /// Given a composition context (task, tools, history, knowledge),
    /// produce a complete prompt ready to send to an LLM backend.
    async fn compose(&self, context: &CompositionContext) -> Result<Prompt>;
}

/// A fully assembled prompt, ready for dispatch to an LLM backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    /// The system prompt (instructions, role, constraints).
    pub system: String,
    /// The conversation messages (user turns, assistant turns, tool results).
    pub messages: Vec<Message>,
    /// Tool definitions to make available to the model.
    pub tools: Vec<ToolDefinition>,
    /// Total estimated token count (for context window management).
    pub estimated_tokens: usize,
}
```

**The 9-layer prompt assembly:**

The Composer builds prompts by stacking nine layers, each contributing a different type of
context. Layers are prioritized so that if the context window is too small to fit everything,
lower-priority layers are trimmed or omitted first.

```
Layer 9 (lowest priority):  Agent personality / style guidelines
Layer 8:                    Background knowledge from DA layer (trajectory RAG)
Layer 7:                    Relevant playbooks from prior successful executions
Layer 6:                    Project-specific context (repo structure, conventions)
Layer 5:                    Task-specific context (related files, dependencies)
Layer 4:                    Conversation history (prior turns in this session)
Layer 3:                    Tool definitions and usage examples
Layer 2:                    Active constraints (safety rules, budget limits)
Layer 1 (highest priority): The current task description and instructions
```

Each layer has a token budget. The Composer measures the total and, if it exceeds the
model's context window, trims from the bottom (lowest priority) up. Critical information
(the task itself, safety constraints, tools) is never trimmed.

### 3.6 Policy (authorize)

The `Policy` trait enforces safety and authorization rules. It decides whether a given
action is allowed before it is executed.

```rust
/// Enforces safety constraints and authorization rules. Every action
/// that modifies state (tool calls, DA submissions, agent spawning)
/// passes through a Policy check before execution.
#[async_trait]
pub trait Policy: Send + Sync {
    /// Decide whether an action is authorized in the given context.
    /// Returns Allow, Deny (with reason), or Escalate (requires
    /// human approval).
    async fn authorize(
        &self,
        action: &Action,
        context: &PolicyContext,
    ) -> Result<PolicyDecision>;
}

/// The outcome of a policy check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyDecision {
    /// The action is allowed. Proceed.
    Allow,
    /// The action is denied. Do not proceed. Includes a human-readable
    /// reason that is surfaced to the agent and logged.
    Deny { reason: String },
    /// The action requires human approval before proceeding.
    /// The system will pause and wait for explicit authorization.
    Escalate { reason: String },
}
```

**Action classification:**

Policies classify actions into risk tiers:

| Tier | Examples | Default policy |
|------|----------|---------------|
| **Read** | Query DA layer, read file, list namespaces | Always allowed |
| **Write** | Submit blob, write file, update config | Allowed within budget and scope |
| **Destructive** | Delete data, overwrite production config, spend above budget | Requires escalation |
| **Privileged** | Spawn new agents, modify safety rules, access secrets | Requires explicit authorization |

**Safety contracts:**

Beyond basic risk tiers, Policies can enforce **safety contracts** --- structured YAML
documents that define exactly what an agent is and is not allowed to do:

```yaml
# Example safety contract for a Celestia deployment agent
agent: celestia-deployer
allowed_tools:
  - celestia_submit_blob
  - celestia_query_namespace
  - celestia_estimate_fee
  - file_read
  - shell_exec  # restricted to specific commands
denied_tools:
  - file_delete
  - shell_exec_unrestricted
max_cost_per_task_usd: 5.00
max_blobs_per_hour: 100
namespaces:
  writable:
    - "tiagent/v1/traces/*"
    - "tiagent/v1/learning/*"
  read_only:
    - "tiagent/v1/coord/*"
requires_human_approval:
  - any_action_on_mainnet
  - cost_exceeding_1_usd
```

When a safety contract YAML is present, the Policy trait enforces it strictly. When no
contract is provided, the system falls back to a permissive default that allows most
read and write operations but still blocks destructive actions.

---

## 4. The Universal Loop

Every agent execution in tiagent follows the same eight-stage loop. This loop is called
"universal" because it applies to every type of work --- a simple prompt/response, a
multi-step tool-using task, a complex plan execution, and even the self-improvement cycle
itself.

### The eight stages

```
query ──► score ──► route ──► compose ──► act ──► verify ──► persist ──► react
  │                                                                        │
  └────────────────────────────────────────────────────────────────────────┘
                              (loop / recursion)
```

Here is what happens at each stage, in detail:

### Stage 1: Query

**What happens**: An input Signal enters the system. This is the starting point of every
execution.

**Input sources**:
- A user types a prompt in the CLI: `tiagent run "deploy rollup to Mocha"`
- An HTTP API call arrives: `POST /api/run { "prompt": "..." }`
- A scheduled event fires: `cron: every 6 hours, check DA layer health`
- A "react" stage from a prior loop triggers a new query (recursion)
- An event subscription fires: "new blob in namespace X, process it"

**What the stage produces**: A Signal with `kind: Prompt` or `kind: Event`, containing
the input data and any initial context.

```rust
// Pseudocode for the query stage
let input_signal = Signal {
    id: hash_of(&payload),
    kind: SignalKind::Prompt,
    payload: json!({
        "text": "deploy rollup to Mocha testnet",
        "context": {
            "workspace": "/home/user/my-rollup",
            "chain": "celestia-mocha-4",
        }
    }),
    metadata: hashmap!{
        "created_at" => timestamp_now(),
        "source" => "cli",
    },
    parent: None,
    namespace: Some(Namespace::from("tiagent/v1/traces/agent-001")),
};
```

### Stage 2: Score

**What happens**: One or more `Scorer` implementations evaluate the input Signal. The
scores determine how the Signal is handled in subsequent stages.

**Scores computed**:
- **Complexity**: How difficult is this task? (Affects model selection in the Route stage.)
- **Priority**: How urgently should this be processed? (Affects ordering when multiple
  tasks are queued.)
- **Relevance**: How related is this to the agent's domain? (Used to decide whether to
  accept or delegate.)

**Example**: A prompt like "submit a 10-byte blob to namespace X" gets a low complexity
score (simple, well-defined task), so the Router can use a fast, cheap model. A prompt like
"design and implement a rollup bridge with fraud proofs" gets a high complexity score,
triggering the use of a stronger model.

```rust
// Pseudocode for the score stage
let complexity = complexity_scorer.score(&input_signal).await?;  // 0.2 (simple task)
let priority = priority_scorer.score(&input_signal).await?;      // 0.8 (user is waiting)
let scores = Scores { complexity, priority };
```

### Stage 3: Route

**What happens**: The `Router` uses the scores and its learned performance data to select
the execution path --- which model, which backend, and which tools.

**Decision factors**:
- Complexity score from the Score stage
- Historical success rates per model per task type (learned over time)
- Cost budget (configured per-agent or per-task)
- Model availability (is the preferred model's API up?)
- Prompt experiment state (if an A/B test is active, the experiment may override routing)

```rust
// Pseudocode for the route stage
let route = cascade_router.route(&input_signal, &routing_context).await?;
// route = Route {
//     model: "claude-sonnet-4-20250514",
//     backend: "claude-api",
//     tools: Some(vec!["celestia_submit_blob", "celestia_query_namespace", "file_read"]),
//     max_tokens: Some(4096),
//     cost_budget: Some(0.05),
//     params: {},
// }
```

### Stage 4: Compose

**What happens**: The `Composer` assembles the full prompt from multiple context layers.
This is where the 9-layer prompt assembly (described in section 3.5) executes.

**What gets assembled**:
- System prompt with role instructions, safety constraints, and agent personality
- Relevant context from the knowledge store (trajectory RAG from DA layer)
- Applicable playbooks from prior successful executions
- Tool definitions with schemas and usage examples
- Conversation history (if this is part of a multi-turn session)
- The task itself

```rust
// Pseudocode for the compose stage
let prompt = system_prompt_composer.compose(&CompositionContext {
    signal: &input_signal,
    route: &route,
    tools: &available_tools,
    history: &conversation_history,
    knowledge: &retrieved_trajectories,  // from DA layer
    playbooks: &matching_playbooks,
}).await?;
// prompt.estimated_tokens = 3847
// prompt.system.len() = 2100 chars (role + safety + context)
// prompt.tools.len() = 3 (celestia_submit_blob, celestia_query_namespace, file_read)
```

### Stage 5: Act

**What happens**: The composed prompt is dispatched to the selected LLM backend. The
model generates a response, which may include tool calls. If tool calls are present,
the tools are executed and results are fed back to the model in a tool loop that continues
until the model produces a final response with no further tool calls.

**The tool loop**:

```
                    ┌──────────────────┐
                    │  Send prompt to  │
                    │  LLM backend     │
                    └────────┬─────────┘
                             │
                    ┌────────▼─────────┐
                    │  Model response  │
                    └────────┬─────────┘
                             │
                   ┌─────────▼──────────┐
                   │ Contains tool      │
              ┌────┤ calls?             ├────┐
              │Yes └────────────────────┘ No │
              │                              │
     ┌────────▼────────┐            ┌────────▼─────────┐
     │ Execute tools   │            │ Final response   │
     │ (via MCP or     │            │ (proceed to      │
     │  built-in)      │            │  Verify stage)   │
     └────────┬────────┘            └──────────────────┘
              │
     ┌────────▼────────┐
     │ Feed results    │
     │ back to model   │──────────┐
     └─────────────────┘          │
              ▲                   │
              └───────────────────┘
              (loop until no more tool calls)
```

Each tool call goes through a Policy check before execution. If the Policy denies or
escalates, the tool call is blocked and the model is informed.

```rust
// Pseudocode for a single turn in the tool loop
let response = backend.dispatch(&prompt).await?;

for tool_call in response.tool_calls {
    // Policy check: is this tool call allowed?
    let decision = policy.authorize(
        &Action::ToolCall(tool_call.clone()),
        &policy_context,
    ).await?;

    match decision {
        PolicyDecision::Allow => {
            let result = tool_executor.execute(&tool_call).await?;
            // Feed result back to model for next turn
        }
        PolicyDecision::Deny { reason } => {
            // Inform model that tool call was blocked
        }
        PolicyDecision::Escalate { reason } => {
            // Pause and wait for human approval
        }
    }
}
```

### Stage 6: Verify

**What happens**: The agent's output is run through the gate pipeline. Each gate rung
checks a different quality dimension, from structural validity to semantic correctness.

**Gate execution**:

```rust
// Pseudocode for the verify stage
let gate_context = GateContext {
    task: &input_signal,
    response: &final_response,
    workspace: &workspace_path,
};

let mut gate_results = Vec::new();
for gate in gate_pipeline.sorted_by_rung() {
    let result = gate.check(&final_response, &gate_context).await?;
    gate_results.push(result.clone());

    if !result.passed {
        // Lower rung failed. Skip remaining rungs.
        // The failure is recorded and may trigger replanning (in the React stage).
        break;
    }
}
```

**What happens on failure**: If a gate fails, the system does not simply report an error.
Depending on configuration, it may:

1. **Retry** with the same model (transient failures).
2. **Escalate** to a stronger model (the task was too hard for the current model).
3. **Replan** by generating a revised approach based on the gate failure details
   (the strategy was wrong, not just the execution).

### Stage 7: Persist

**What happens**: All Signals produced during this execution (the input, response, tool
calls, tool results, gate results) are written to the Substrate. Depending on
configuration, this means local storage, Celestia's DA layer, or both.

```rust
// Pseudocode for the persist stage
// Write all signals produced during this execution
for signal in execution_signals.iter() {
    substrate.write(signal).await?;
}

// Construct an Episode signal that summarizes the full execution
let episode = Signal {
    id: hash_of(&episode_payload),
    kind: SignalKind::Episode,
    payload: json!({
        "turns": turn_count,
        "tool_calls": tool_call_count,
        "gate_results": gate_results,
        "model": route.model,
        "cost_usd": total_cost,
        "duration_ms": elapsed.as_millis(),
        "score": quality_score,
    }),
    metadata: hashmap!{
        "created_at" => timestamp_now(),
        "source" => "execution-loop",
    },
    parent: Some(input_signal.id.clone()),
    namespace: Some(Namespace::from("tiagent/v1/traces/agent-001")),
};
substrate.write(&episode).await?;
```

The Episode Signal is particularly important. It is a self-contained record of the full
execution that can be:

- Analyzed locally to update routing weights and gate thresholds.
- Published to Celestia's DA layer for other agents to learn from.
- Retrieved later via trajectory RAG when a similar task is encountered.
- Fingerprinted with HDC vectors for fast similarity matching.

### Stage 8: React

**What happens**: The system processes the results of the execution and triggers any
downstream effects.

**Downstream effects**:
- **Learning updates**: The cascade router's weights are updated based on whether the
  selected model succeeded or failed. Gate thresholds are adjusted via EMA. Prompt
  experiment results are recorded.
- **Event emission**: Other parts of the system (or external subscribers) are notified
  that a task completed, a gate failed, or a learning artifact was produced.
- **Recursive queries**: If the output of this loop triggers a new task (e.g., a gate
  failure triggers replanning, or a completed task unblocks a dependent task), a new
  input Signal is created and fed back into the Query stage.
- **DA publication**: If configured, the Episode and other learning artifacts are
  submitted to Celestia's DA layer for cross-agent sharing.

```rust
// Pseudocode for the react stage
// 1. Update learning state
cascade_router.record_outcome(&route, &gate_results).await?;
gate_thresholds.update_ema(&gate_results).await?;
if let Some(experiment) = active_experiment {
    experiment.record_result(&route.model, &gate_results).await?;
}

// 2. Emit events
event_bus.emit(Event::TaskCompleted {
    task_id: input_signal.id.clone(),
    success: all_gates_passed,
    episode_hash: episode.id.clone(),
}).await?;

// 3. Check for cascading work
if !all_gates_passed && config.replan_on_gate_failure {
    // Generate a revised plan based on the failure
    let replan_signal = build_gate_failure_replan(&input_signal, &gate_results);
    // Feed it back into the loop (recursion)
    universal_loop.execute(replan_signal).await?;
}

// 4. Publish to DA layer (if configured)
if config.publish_episodes_to_da {
    celestia_substrate.write(&episode).await?;
}
```

### Loop nesting and recursion

The universal loop can nest. The most common nesting patterns are:

1. **Tool calls within Act**: Each tool call is itself a mini-loop (query the tool,
   execute it, verify the result, persist the outcome).
2. **Replanning within React**: A gate failure triggers a new loop execution with a
   revised task.
3. **Orchestrated plans**: A plan with multiple tasks runs each task through the loop
   independently, with the orchestrator managing dependencies and parallelism.

```
Outer loop: Plan execution
├── Task 1: Universal loop (query → ... → react)
│   └── Tool loop: 3 tool calls (each with policy checks)
├── Task 2: Universal loop (query → ... → react)
│   └── Gate failure → Replan → Universal loop (recursive)
│       └── Tool loop: 2 tool calls
├── Task 3: (blocked on Task 1, runs after it completes)
│   └── Universal loop (query → ... → react)
└── React: Publish plan execution summary to DA
```

---

## 5. Layered Architecture

tiagent's runtime is organized into six layers. Each layer depends only on the layers
below it, never on the layers above. This strict layering makes the system testable
(lower layers can be tested in isolation) and extensible (upper layers can be replaced
without affecting lower layers).

```
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│   Layer 6: User Interface                                       │
│   CLI commands, HTTP API endpoints, Terminal UI                 │
│                                                                 │
│   Accepts user input, displays results, provides dashboards.    │
│   This is the only layer that interacts with humans directly.   │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   Layer 5: Coordination                                         │
│   Plan orchestrator, DAG executor, task scheduling              │
│                                                                 │
│   Manages multi-task plans: dependency ordering, parallel       │
│   execution, state persistence, resume-after-interruption.      │
│   Uses the Universal Loop (Layer 3) to execute individual       │
│   tasks.                                                        │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   Layer 4: Agent Dispatch                                       │
│   LLM backend abstraction, tool loop, MCP client, safety       │
│                                                                 │
│   Sends prompts to LLM backends, handles tool call loops,      │
│   manages MCP connections to external tool servers. This is     │
│   where the "Act" stage of the Universal Loop executes.         │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   Layer 3: Universal Loop                                       │
│   The 8-stage execution pipeline (query through react)          │
│                                                                 │
│   Orchestrates a single task execution by calling the verb      │
│   traits in order. This layer is the glue between all the       │
│   trait implementations.                                        │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   Layer 2: Verb Trait Implementations                           │
│   Concrete Scorers, Routers, Composers, Gates, Policies        │
│                                                                 │
│   The specific implementations of each verb trait. Pluggable    │
│   and configurable. Multiple implementations can coexist        │
│   (e.g., CelestiaSubstrate + FileSubstrate + HybridSubstrate). │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   Layer 1: Kernel                                               │
│   Signal struct, SignalKind enum, Hash type, 6 verb traits      │
│                                                                 │
│   The absolute minimum: data types and trait definitions.       │
│   This layer has NO dependencies on any other tiagent code.     │
│   It defines the contracts that everything else implements.     │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   Layer 0: Storage                                              │
│   FileSubstrate (local JSONL), CelestiaSubstrate (DA blobs)    │
│                                                                 │
│   The physical persistence layer. Reads and writes bytes.       │
│   CelestiaSubstrate uses lumina-node or RPC for DA access.     │
│   FileSubstrate uses the local filesystem.                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Dependency rules

- Layer 1 (Kernel) depends on nothing except `serde`, `serde_json`, `sha2`, and
  `async-trait`. It defines the trait interfaces and data types.
- Layer 0 (Storage) implements traits defined in Layer 1. It depends on `celestia-types`
  and `lumina-node` for DA access, or `tokio::fs` for local file access.
- Layer 2 (Implementations) implements traits defined in Layer 1 and may use Layer 0 for
  persistence.
- Layer 3 (Universal Loop) calls traits from Layer 1 and dispatches to Layer 4. It does
  not know which concrete implementations are active --- it operates on trait objects.
- Layer 4 (Agent Dispatch) handles LLM communication. It depends on HTTP client libraries
  and MCP SDK, but not on any specific model.
- Layer 5 (Coordination) uses Layer 3 to execute individual tasks within a multi-task plan.
- Layer 6 (User Interface) is the thin entry point: CLI parsing, HTTP routing, TUI
  rendering. It delegates all real work to lower layers.

### Deployment modes

The layered architecture supports two deployment modes. Both use the same code and the
same universal loop --- the only difference is which Substrate implementation is active at
Layer 0.

**Standalone mode** (default, no blockchain):
```
CLI / HTTP API / TUI                          Layer 6: User Interface
        │
Plan orchestrator, DAG executor               Layer 5: Coordination
        │
LLM backends, tool loop, MCP, safety         Layer 4: Agent Dispatch
        │
query → score → route → compose → act →      Layer 3: Universal Loop
verify → persist → react
        │
Scorers, Routers, Composers, Gates, Policies  Layer 2: Verb Implementations
        │
Signal, SignalKind, Hash, 6 verb traits       Layer 1: Kernel
        │
LocalSubstrate (JSONL files)                  Layer 0: Storage
```

This is a fully functional coding agent. All learning, routing, gate adaptation, and
episode recording works locally. No network dependencies beyond the LLM API calls.

**Network mode** (opt-in, adds shared learning):
```
Same stack as Standalone, plus:
        │
LocalSubstrate + CelestiaSubstrate           Layer 0: Storage (Hybrid)
```

Network mode adds DA-backed durability and cross-agent learning. Episodes, routing
weights, and playbooks are published to Celestia's DA layer so other agents can bootstrap
from community experience. Everything else in the stack is identical.

### Why this layering matters

1. **Testability**: Layers 0 through 3 can be tested with mock implementations. You can
   run the entire universal loop with an in-memory substrate and a mock LLM backend,
   verifying all the scoring, routing, composing, and gating logic without making a single
   network call.

2. **Flexibility**: Want to use a different DA layer? Implement the `Substrate` trait for
   that DA layer and swap it in at Layer 0. Want to add a new model provider? Implement the
   `Backend` trait and register it at Layer 4. No other layer needs to change.

3. **Minimal core**: Layer 1 (Kernel) is tiny --- perhaps 500 lines of Rust. Everything
   above it is optional. You can use tiagent's Signal model and trait system without using
   its CLI, HTTP API, or orchestrator.

---

## 6. Celestia Integration Points

> **This section is optional.** tiagent works as a fully functional coding agent without
> any Celestia integration. The `LocalSubstrate` provides complete storage, learning, and
> episode tracking with zero blockchain dependencies. The following describes what you gain
> by enabling the Celestia substrate: shared cross-agent learning, verifiable execution
> provenance, and a community knowledge layer that lets new agents bootstrap from the
> experience of others. If none of that applies to your use case, you can skip this section
> entirely.

When Celestia integration is enabled, tiagent treats the data availability (DA) layer as
an additional storage backend alongside local storage. This section describes exactly where
and how Celestia connects to the architecture.

### 6.1 Substrate: the DA storage interface

The `CelestiaSubstrate` is an implementation of the `Substrate` trait that reads and writes
Signals as blobs on Celestia's DA layer.

```rust
/// Stores Signals as blobs on Celestia's DA layer.
/// Each Signal is serialized to bytes, submitted to a namespace,
/// and referenced by its blob commitment + height.
pub struct CelestiaSubstrate {
    /// Connection to a Celestia node (light node or RPC).
    node: CelestiaClient,
    /// Default namespace for this agent's data.
    default_namespace: Namespace,
    /// Fee configuration (gas price, max fee per blob).
    fee_config: FeeConfig,
    /// Local cache to avoid re-fetching recently accessed blobs.
    cache: LruCache<Hash, Signal>,
}
```

**Write path**:
```
Signal ──► serialize to bytes ──► submit as blob to namespace ──► receive height + commitment
```

**Read path**:
```
Hash ──► check local cache ──► if miss, query DA by namespace + height ──► deserialize ──► Signal
```

### 6.2 Tiered storage

Not all data warrants DA-layer storage. tiagent uses a three-tier storage model:

```
┌──────────────────────────────┐
│  HOT: Local filesystem       │  Fast reads/writes, ephemeral
│  JSONL files in .tiagent/    │  Intermediate state, raw LLM output,
│  Latency: <1ms               │  secrets, large artifacts
├──────────────────────────────┤
│  WARM: Celestia DA layer     │  Shared, verifiable, append-only
│  Blobs in organized          │  Traces, episodes, learning artifacts,
│  namespaces                  │  coordination proofs, HDC fingerprints
│  Latency: ~1-10s             │  Cost: ~$0.07-$0.81 per MB
├──────────────────────────────┤
│  COLD: Permanent storage     │  Long-term archival (Arweave, Filecoin,
│  (optional, future)          │  or similar). For data that must survive
│  Latency: minutes-hours      │  Celestia's pruning window.
└──────────────────────────────┘
```

Promotion and demotion between tiers follows configurable rules:

- **Hot to Warm**: Episodes are promoted to DA after gate validation passes. Learning
  artifacts are promoted when updated. Coordination signals are promoted immediately.
- **Warm to Cold**: Signals older than the configured retention window are archived to
  permanent storage before Celestia prunes them (if cold storage is configured).
- **Warm to Hot**: When an agent needs a DA-stored Signal, it is fetched and cached locally
  for fast subsequent reads.

### 6.3 Namespace-per-agent partitioning

Celestia organizes blobs into namespaces. tiagent uses a structured namespace schema:

```
tiagent/v1/<data-type>/<agent-or-group-id>
```

| Namespace pattern | What it contains | Who writes | Who reads |
|-------------------|-----------------|------------|-----------|
| `tiagent/v1/traces/<agent-id>` | Execution traces (episodes) | The owning agent | Any agent (trajectory RAG) |
| `tiagent/v1/fingerprints/global` | HDC behavioral fingerprints | Any agent | Any agent (similarity search) |
| `tiagent/v1/learning/<agent-id>` | Routing weights, playbooks | The owning agent | Any agent (bootstrapping) |
| `tiagent/v1/coord/<group-id>` | Multi-agent coordination | Group members | Group members |

This namespace structure means:

- An agent's traces are isolated in its own namespace (easy to query, easy to audit).
- Cross-agent data (fingerprints, shared learning) lives in shared namespaces.
- Coordination data is scoped to the coordinating group.
- Namespace subscriptions enable real-time event-driven workflows: an agent can subscribe
  to new blobs in a coordination namespace and react immediately when a collaborator
  publishes something.

### 6.4 Light node embedding

tiagent can optionally embed a Celestia light node directly in the agent process using
`lumina-node`. This provides:

- **Direct DA verification**: The agent can verify data availability without trusting a
  remote full node.
- **Offline resilience**: The light node maintains a local view of the chain that can
  serve reads even when the network is temporarily unavailable.
- **Reduced latency**: Blob reads from a local light node are faster than RPC calls to a
  remote full node.

When a light node is not embedded (the default for lightweight deployments), the agent uses
RPC calls to a configured Celestia node endpoint.

---

## 7. Model Agnosticism and Backend Dispatch

tiagent does not hardcode any LLM provider. The agent dispatch layer treats all models
equally through a `Backend` trait.

Unlike Claude Code (locked to Claude), Codex (locked to GPT), or Cursor (primarily
GPT/Claude with limited configurability), tiagent treats all models as interchangeable
backends. Your cascade router will learn which model works best for which task type ---
Sonnet for routine code changes, Opus for complex architectural work, a local Llama model
for simple formatting tasks --- and route automatically based on accumulated performance
data. You are never locked into a single provider, and switching models requires no code
changes.

### The Backend trait

```rust
/// Dispatches a composed prompt to an LLM and returns the response.
/// Each backend implementation handles the specifics of one provider's
/// API: authentication, request format, streaming, error handling.
#[async_trait]
pub trait Backend: Send + Sync {
    /// Send a prompt to the model and return the complete response.
    /// The response includes the generated text, any tool calls the
    /// model wants to make, and usage statistics (tokens, cost).
    async fn dispatch(&self, prompt: &Prompt) -> Result<BackendResponse>;

    /// Stream a response token-by-token. Not all backends support
    /// streaming; those that do not should fall back to `dispatch`
    /// and emit the entire response as a single chunk.
    async fn stream(
        &self,
        prompt: &Prompt,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>>>>>;

    /// A human-readable name for this backend.
    /// Example: "claude-api", "openai-compat", "ollama".
    fn name(&self) -> &str;

    /// Which models this backend supports.
    /// Example: ["claude-sonnet-4-20250514", "claude-opus-4-0520"]
    fn supported_models(&self) -> &[String];
}

/// A response from an LLM backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendResponse {
    /// The generated text (if any). May be empty if the model
    /// responded entirely with tool calls.
    pub text: Option<String>,
    /// Tool calls the model wants to execute. The tool loop in
    /// the Act stage handles these.
    pub tool_calls: Vec<ToolCall>,
    /// Usage statistics for cost tracking and learning.
    pub usage: Usage,
    /// The model identifier that actually served this request
    /// (may differ from requested model due to fallbacks).
    pub model: String,
}
```

### Shipped backends

| Backend | Provider | Protocol | Models |
|---------|----------|----------|--------|
| `ClaudeApiBackend` | Anthropic | HTTPS (Messages API) | Claude Opus, Sonnet, Haiku |
| `ClaudeCliBackend` | Anthropic | Local CLI subprocess | Same models via `claude` CLI |
| `OpenAiCompatBackend` | Any OpenAI-compatible API | HTTPS | GPT-4o, GPT-4o-mini, Deepseek, Fireworks, Together, Groq |
| `OllamaBackend` | Local (Ollama) | HTTP (localhost) | Llama, Mistral, Phi, Gemma, any Ollama-supported model |
| `GeminiBackend` | Google | HTTPS (Generative AI API) | Gemini Pro, Flash, Ultra |

Adding a new backend means implementing the `Backend` trait --- typically 50-150 lines of
Rust, most of which is HTTP request/response formatting. The Backend never touches scoring,
routing, gating, or persistence. It only knows how to send a prompt and receive a response.

### How routing and backends interact

The `Router` selects a model name and backend name. The dispatch layer looks up the
corresponding `Backend` implementation and calls it:

```
Router decides: model="claude-sonnet-4-20250514", backend="claude-api"
         │
         ▼
Dispatch layer: look up "claude-api" in backend registry
         │
         ▼
ClaudeApiBackend.dispatch(prompt) ──► Anthropic Messages API ──► BackendResponse
```

This separation means the Router can make model selection decisions purely based on task
characteristics and learned performance data, without knowing anything about how the
backend communicates with the model. And backends can be added, removed, or updated without
affecting routing logic.

---

## 8. Extension Points

tiagent is designed to be extended in four primary ways. Each extension point has a clear
interface and does not require modifying core code.

### 8.1 Tool system via MCP

Tools are the mechanism by which agents affect the external world --- reading files,
submitting blobs, querying APIs, running commands. tiagent uses the **Model Context
Protocol (MCP)** as its tool interface.

**MCP overview**: MCP is an open protocol (originally developed by Anthropic) that defines
how LLMs connect to external tools and data sources. An MCP server exposes tools with typed
schemas. An MCP client connects to servers and makes tools available to the model.

tiagent is both an MCP client and an MCP server:

```
                    ┌──────────────────────────┐
                    │      tiagent agent       │
                    │                          │
                    │  ┌────────────────────┐  │
                    │  │  MCP Client        │  │  Connects to external
                    │  │  (consumes tools)  │──│──── MCP servers
                    │  └────────────────────┘  │    (GitHub, databases,
                    │                          │     custom tools)
                    │  ┌────────────────────┐  │
                    │  │  MCP Server        │  │  Exposes tiagent's tools
                    │  │  (provides tools)  │──│──── to external clients
                    │  └────────────────────┘  │    (Claude Desktop, Cursor,
                    │                          │     VS Code, other agents)
                    │  ┌────────────────────┐  │
                    │  │  Built-in tools    │  │  Celestia-specific tools
                    │  │  (Celestia suite)  │  │  (blob submit, namespace
                    │  └────────────────────┘  │   query, fee estimate, etc.)
                    └──────────────────────────┘
```

**As MCP client**: tiagent connects to any MCP-compatible tool server. This means agents
can use thousands of existing MCP tools (file systems, databases, GitHub, Slack, custom
servers) without any tiagent-specific integration work.

**As MCP server**: tiagent exposes its Celestia developer tools as an MCP server. This
means any MCP client --- Claude Desktop, Cursor, VS Code with Continue --- can use
tiagent's Celestia tools without running a tiagent agent. A developer can connect tiagent's
MCP server to their preferred LLM client and get Celestia blob submission, namespace
management, and chain queries as tools.

**Built-in Celestia tool suite**: tiagent ships with a set of Celestia-specific tools that
are available to agents by default:

| Tool | What it does |
|------|-------------|
| `celestia_submit_blob` | Submit data as a blob to a specified namespace |
| `celestia_get_blob` | Retrieve a blob by height and namespace |
| `celestia_query_namespace` | List blobs in a namespace within a height range |
| `celestia_estimate_fee` | Estimate the cost of submitting a blob |
| `celestia_get_balance` | Check the agent's TIA balance |
| `celestia_subscribe_namespace` | Subscribe to new blobs in a namespace (real-time) |
| `celestia_get_header` | Retrieve a block header by height |
| `celestia_das_status` | Check data availability sampling status |

### 8.2 Event system

tiagent has an internal event bus that decouples producers and consumers of system events.
Any component can emit events, and any component can subscribe to them.

**Built-in event types**:

```rust
pub enum Event {
    /// A task execution completed (successfully or not).
    TaskCompleted { task_id: Hash, success: bool, episode_hash: Hash },
    /// A gate check failed.
    GateFailed { gate: String, rung: u8, details: serde_json::Value },
    /// A new Signal was written to the DA layer.
    DaPublished { signal_hash: Hash, namespace: Namespace, height: u64 },
    /// Learning state was updated (routing weights, gate thresholds).
    LearningUpdated { component: String, details: serde_json::Value },
    /// A new blob appeared in a subscribed namespace.
    BlobReceived { namespace: Namespace, height: u64, size: usize },
    /// An agent was started or stopped.
    AgentLifecycle { agent_id: String, action: LifecycleAction },
}
```

**Use cases**:
- The learning system subscribes to `TaskCompleted` to update routing weights.
- The DA publisher subscribes to `TaskCompleted` to decide which episodes to publish.
- The orchestrator subscribes to `GateFailed` to trigger replanning.
- External integrations subscribe via webhooks or Server-Sent Events (SSE) to receive
  real-time notifications.

### 8.3 Plugin architecture

Custom verb trait implementations can be registered as plugins. This allows third parties
to extend tiagent without modifying core code:

- A custom `Scorer` that evaluates tasks using domain-specific criteria.
- A custom `Gate` that runs proprietary validation logic.
- A custom `Router` that integrates with an organization's model management platform.
- A custom `Substrate` that writes to a different DA layer (Avail, EigenDA, etc.).
- A custom `Policy` that enforces organization-specific safety rules.

Plugins implement the standard verb traits and are registered in configuration:

```toml
# tiagent.toml
[plugins.custom-scorer]
path = "./plugins/my-scorer"   # or a crate name

[plugins.custom-gate]
path = "./plugins/compliance-gate"
```

### 8.4 Webhooks and SSE

External systems can subscribe to tiagent events via:

- **Webhooks**: tiagent POSTs JSON payloads to configured URLs when events occur.
- **Server-Sent Events (SSE)**: External clients can connect to tiagent's HTTP API and
  receive a real-time stream of events.
- **WebSocket**: For bidirectional communication (monitoring dashboards, interactive
  control).

This enables integration with CI/CD pipelines, monitoring systems, chat platforms, and
custom dashboards without requiring code changes to tiagent.

---

## 9. Comparison with Monolithic Frameworks

Most existing agent frameworks are **monolithic**: they provide a single, opinionated
abstraction for building agents. tiagent takes a **composable** approach: it provides
small, well-defined traits that can be assembled in different ways.

### Structural comparison

| Dimension | tiagent | LangChain | CrewAI | AutoGen | Eliza |
|-----------|---------|-----------|--------|---------|-------|
| **Language** | Rust | Python | Python | Python | TypeScript |
| **Core abstraction** | Signal + 6 verb traits | Chain of LLM calls | Agent roles + tasks | Multi-agent conversations | Character-driven plugins |
| **Extension model** | Implement a trait | Subclass or wrap | Define roles + tools | Define agent behaviors | Write a plugin |
| **Storage** | Pluggable Substrate trait | Vector DBs (Pinecone, etc.) | In-memory | In-memory | SQLite + in-memory |
| **Model support** | Any (via Backend trait) | Any (via LLM abstraction) | OpenAI-focused | OpenAI-focused | OpenAI + Llama |
| **Verification** | 7-rung gate pipeline | None built-in | None built-in | None built-in | None built-in |
| **Self-improvement** | Built-in (cascade router, EMA gates, trajectory RAG) | None built-in | None built-in | None built-in | None built-in |
| **On-chain integration** | Native (Celestia DA) | Via tools/plugins | Via tools | Via tools | Token-level only |
| **Shared learning** | DA-backed cross-agent learning | None | None | None | None |
| **Safety model** | Policy trait + safety contracts | None structured | Basic role constraints | None structured | None |
| **Type safety** | Rust compile-time guarantees | Python runtime | Python runtime | Python runtime | TypeScript (partial) |

### Philosophy comparison

**Monolithic frameworks** (LangChain, CrewAI, AutoGen) make common cases easy at the cost of
flexibility:

- They provide high-level abstractions ("create an agent in 3 lines of code") but make it
  hard to customize individual components.
- They bundle many concerns (prompting, tool calling, memory, orchestration) into a single
  framework that is hard to use partially.
- They optimize for getting started quickly, not for production reliability or long-term
  maintainability.
- They have no built-in verification (no gate pipeline), no self-improvement (no learning
  loop), and no on-chain integration beyond basic RPC wrappers.

**tiagent** makes the common cases slightly harder (you need to understand Signals, traits,
and the universal loop) but provides much more control:

- Each verb trait can be swapped independently. You are not locked into a framework's
  opinion about how routing, composition, or validation should work.
- The Signal model provides a uniform, content-addressed data layer that works consistently
  across local and on-chain storage.
- The gate pipeline catches problems before they propagate.
- The learning loop improves performance over time without human intervention.
- The trait system means you can use tiagent's kernel (Signal + traits) without its CLI,
  HTTP API, or orchestrator.

### When to use what

| If you need... | Use |
|----------------|-----|
| A quick prototype with Python | LangChain or CrewAI |
| Multi-agent conversations | AutoGen |
| Social media bots with personality | Eliza |
| Production-grade agents with verification, self-improvement, and Celestia integration | tiagent |
| Agents that share what they learn through a verifiable DA layer | tiagent |
| A minimal Rust core you can embed in your own system | tiagent (kernel only) |

---

## 10. Putting It All Together

This section walks through two complete examples to show how all the architectural pieces
connect. The first is a pure software development task --- no blockchain involved. The
second shows how Celestia integration adds shared learning on top of the same loop.

### Example 1: Software development task

A developer runs:
```bash
tiagent run "Add input validation to the signup form --- email must be valid, password at least 8 chars"
```

**Stage 1: Query**

The CLI parses the command and creates an input Signal:
```
Signal {
    id: sha256("Add input validation to the signup form..."),
    kind: Prompt,
    payload: { "text": "Add input validation to the signup form..." },
    metadata: { "created_at": "2026-08-13T...", "source": "cli" },
    parent: None,
    namespace: None,
}
```

No namespace is set because this is standalone mode --- everything stays local.

**Stage 2: Score**

The ComplexityScorer evaluates the prompt:
- Multiple file edits, form validation logic, likely needs test updates -> complexity: 0.45
- Interactive CLI prompt, user is waiting -> priority: 0.9

**Stage 3: Route**

The CascadeRouter checks its learned weights:
- Complexity 0.45 is in the "mid model" range
- Historical data shows Sonnet succeeds on form-validation tasks 91% of the time
- Route: model="claude-sonnet-4-20250514", backend="claude-api", tools=["file_read", "file_write", "shell_exec", "grep"]

**Stage 4: Compose**

The SystemPromptComposer assembles a prompt:
- Layer 1 (task): The user's prompt text
- Layer 2 (constraints): "Do not delete existing tests. Run tests after changes."
- Layer 3 (tools): Schema definitions for file_read, file_write, shell_exec, grep
- Layer 5 (task context): The signup form file, existing validation code, test files
- Layer 7 (playbooks): A previously successful "add form validation" trajectory
- Total: ~4200 tokens

**Stage 5: Act**

The prompt is sent to Claude Sonnet. The model responds:

Turn 1: Model calls `file_read` with `{ "path": "src/components/SignupForm.tsx" }`
- Policy check: read action, allowed
- Tool returns the current form component source

Turn 2: Model calls `file_read` with `{ "path": "src/utils/validators.ts" }`
- Policy check: read action, allowed
- Tool returns existing validation utilities

Turn 3: Model calls `file_write` to update `src/utils/validators.ts` with email and password validators
- Policy check: write action, within scope, allowed

Turn 4: Model calls `file_write` to update `SignupForm.tsx` with validation calls and error display
- Policy check: write action, within scope, allowed

Turn 5: Model calls `file_write` to add tests in `src/utils/validators.test.ts`
- Policy check: write action, within scope, allowed

Turn 6: Model calls `shell_exec` with `{ "command": "npm test -- --watchAll=false" }`
- Policy check: shell action, allowed (non-destructive command)
- Tool returns: "Tests: 14 passed, 0 failed"

Turn 7: Model generates final response summarizing the changes.

**Stage 6: Verify**

The gate pipeline runs:
- Rung 0 (Parse): Response has valid structure -> PASS
- Rung 1 (Compile): `npx tsc --noEmit` succeeds -> PASS
- Rung 2 (Test): `npm test` passes (14/14) -> PASS
- Rung 3 (Lint): `npx eslint src/` passes -> PASS
- Rung 4 (Diff): 3 files changed, 87 lines added, no deletions of existing code, no secrets -> PASS

All gates pass.

**Stage 7: Persist**

All Signals from this execution are written to the local `LocalSubstrate`:
- Input Prompt, 5 ToolCall Signals, 5 ToolResult Signals, Response Signal, 5 GateResult Signals, Episode Signal
- Everything is stored as JSONL in `.tiagent/signals.jsonl`
- The Episode summary is also stored in `.tiagent/episodes.jsonl`

**Stage 8: React**

- CascadeRouter records: "Sonnet succeeded on a form-validation task" (weight update persisted locally)
- Gate thresholds updated via EMA (all passed, thresholds slightly loosened for this task type)
- Event emitted: `TaskCompleted { success: true }`
- No cascading work needed
- The user sees the response in their terminal

**What the user sees:**

```
$ tiagent run "Add input validation to the signup form --- email must be valid, password at least 8 chars"

[route] model=claude-sonnet-4-20250514 cost_estimate=$0.012
[tool]  file_read src/components/SignupForm.tsx
[tool]  file_read src/utils/validators.ts
[tool]  file_write src/utils/validators.ts (added email + password validators)
[tool]  file_write src/components/SignupForm.tsx (added validation + error display)
[tool]  file_write src/utils/validators.test.ts (added 6 test cases)
[tool]  shell_exec npm test -> 14 passed
[gate]  compile PASS | test PASS | lint PASS | diff PASS

Added email validation (RFC 5322 regex) and password validation (min 8 chars) to the
signup form. Error messages display inline below each field. Added 6 test cases covering
valid emails, invalid emails, short passwords, and edge cases. All 14 tests pass.

[episode] recorded, 7 turns, 8.2s, $0.011
```

No blockchain. No network calls except the LLM API. The episode is stored locally and will
inform future routing decisions and playbook retrieval for similar tasks.

---

### Example 2: Celestia blob submission (network mode)

This example shows the same loop with Celestia integration enabled, demonstrating
cross-agent learning.

### The task

A user runs:
```bash
tiagent run "Submit a 'hello world' blob to namespace 'test-ns' on Mocha testnet"
```

### What happens, stage by stage

**Stage 1: Query**

The CLI parses the command and creates an input Signal:
```
Signal {
    id: sha256("Submit a 'hello world' blob..."),
    kind: Prompt,
    payload: { "text": "Submit a 'hello world' blob to namespace 'test-ns' on Mocha testnet" },
    metadata: { "created_at": "2026-08-13T...", "source": "cli" },
    parent: None,
    namespace: Some("tiagent/v1/traces/agent-local"),
}
```

**Stage 2: Score**

The ComplexityScorer evaluates the prompt:
- Simple task (single tool call, well-defined parameters) -> complexity: 0.15
- The PriorityScorer notes this is an interactive CLI prompt -> priority: 0.9

**Stage 3: Route**

The CascadeRouter checks its learned weights:
- Complexity 0.15 is well within the "fast model" range
- Historical data shows Haiku succeeds on blob submission tasks 98% of the time
- Route: model="claude-haiku", backend="claude-api", tools=["celestia_submit_blob", "celestia_estimate_fee"]

**Stage 4: Compose**

The SystemPromptComposer assembles a prompt:
- Layer 1 (task): The user's prompt text
- Layer 2 (constraints): "You are on Mocha testnet. Max fee: 0.01 TIA."
- Layer 3 (tools): Schema definitions for celestia_submit_blob and celestia_estimate_fee
- Layer 7 (playbooks): A previously successful blob submission trajectory
- Total: ~1200 tokens (well within Haiku's context window)

**Stage 5: Act**

The prompt is sent to Claude Haiku. The model responds:

Turn 1: Model calls `celestia_estimate_fee` with `{ "size_bytes": 11, "namespace": "test-ns" }`
- Policy check: read-only action, allowed
- Tool returns: `{ "estimated_fee": "0.000001 TIA" }`

Turn 2: Model calls `celestia_submit_blob` with `{ "data": "hello world", "namespace": "test-ns" }`
- Policy check: write action, within budget, allowed
- Tool returns: `{ "height": 12345, "commitment": "0xabc...", "namespace": "test-ns" }`

Turn 3: Model generates final response:
"I have submitted a 'hello world' blob to namespace 'test-ns' on Mocha testnet.
The blob was included at height 12345 with commitment 0xabc..."

**Stage 6: Verify**

The gate pipeline runs:
- Rung 0 (Parse): Response is valid text with expected structure -> PASS
- Rung 4 (Diff): No code changes to review -> SKIP
- Remaining rungs: Not applicable to this task type -> SKIP

**Stage 7: Persist**

All Signals from this execution are written to the local FileSubstrate:
- Input Prompt Signal
- ToolCall Signal (estimate_fee)
- ToolResult Signal (fee estimate)
- ToolCall Signal (submit_blob)
- ToolResult Signal (blob submission result)
- Response Signal (final text)
- GateResult Signal (parse check passed)
- Episode Signal (complete execution summary)

The Episode is also queued for DA publication (it will be submitted to
`tiagent/v1/traces/agent-local` asynchronously).

**Stage 8: React**

- CascadeRouter records: "Haiku succeeded on a blob-submission task" (weight update)
- Event emitted: `TaskCompleted { success: true }`
- No cascading work needed (all gates passed, no dependent tasks)
- The user sees the response in their terminal

### What the user sees

```
$ tiagent run "Submit a 'hello world' blob to namespace 'test-ns' on Mocha testnet"

[route] model=claude-haiku cost_estimate=$0.001
[tool]  celestia_estimate_fee -> 0.000001 TIA
[tool]  celestia_submit_blob -> height=12345 commitment=0xabc...

I have submitted a 'hello world' blob to namespace 'test-ns' on Mocha testnet.
The blob was included at height 12345 with commitment 0xabc...

[episode] recorded, 3 turns, 0.4s, $0.0008
[da]      queued for publication to tiagent/v1/traces/agent-local
```

### What other agents can learn from this

Later, when another agent encounters a blob submission task, the trajectory RAG system can
retrieve this episode from the DA layer:

1. The new task's HDC fingerprint is compared against fingerprints in
   `tiagent/v1/fingerprints/global`.
2. A match is found: "blob submission to Mocha testnet."
3. The episode is retrieved from `tiagent/v1/traces/agent-local`.
4. The successful tool call sequence (estimate fee, then submit) is included as a playbook
   in the new agent's prompt via the Composer's Layer 7.
5. The new agent is more likely to succeed on its first attempt because it has a proven
   strategy to follow.

This is the core value proposition of tiagent's architecture: every execution makes every
future execution better, not just for the same agent, but for all agents on the network.

---

## Appendix: Glossary

| Term | Definition |
|------|-----------|
| **Signal** | The universal data type. Every piece of information in tiagent is a Signal: a content-addressed, typed, immutable datum with metadata. |
| **Verb trait** | One of the six core traits (Substrate, Scorer, Gate, Router, Composer, Policy) that define operations on Signals. |
| **Universal loop** | The 8-stage execution pipeline (query, score, route, compose, act, verify, persist, react) that every agent operation follows. |
| **Gate** | A validation check that determines whether an output meets quality criteria. Gates are organized into a 7-rung pipeline. |
| **Rung** | A level in the gate pipeline. Lower rungs (0-2) check basic structural and compilation requirements. Higher rungs (5-6) check semantic correctness and require human approval. |
| **Cascade router** | A Router implementation that selects models based on task complexity and learned performance data, starting with cheap models and escalating to expensive ones when needed. |
| **Episode** | A Signal that summarizes a complete agent execution: all turns, tool calls, gate results, model used, cost, and duration. |
| **Playbook** | A reusable strategy extracted from successful episodes. Included in prompts via the Composer to guide future executions. |
| **HDC fingerprint** | A Hyperdimensional Computing vector that compactly represents an agent's behavioral signature or a task's semantic identity. Used for fast similarity matching. |
| **DA layer** | Data Availability layer. In Celestia's architecture, this is the layer that stores blob data and makes it retrievable. tiagent uses it as shared agent memory. |
| **Namespace** | Celestia's partitioning mechanism for blobs. tiagent uses a structured namespace schema to organize agent data by type and purpose. |
| **Substrate** | The Verb trait for storage. Implementations include local filesystem (JSONL), Celestia DA (blobs), and hybrid (both). |
| **MCP** | Model Context Protocol. An open standard for connecting LLMs to tools and data sources. tiagent implements both MCP client (consumes tools) and MCP server (provides Celestia tools). |
| **Backend** | A trait implementation that handles communication with a specific LLM provider (Claude API, OpenAI-compatible, Ollama, etc.). |
| **Safety contract** | A YAML document that defines what an agent is and is not allowed to do: permitted tools, cost limits, namespace access, escalation rules. |
| **Trajectory RAG** | Retrieval-Augmented Generation using past execution trajectories (episodes) as context for new tasks. Episodes are retrieved from the DA layer based on HDC fingerprint similarity. |
