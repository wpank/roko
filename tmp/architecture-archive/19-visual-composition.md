# Visual composition and authoring system

> Part of the [Roko Architecture Specification](00-INDEX.md).
> Depends on: [Dashboard Architecture](15-dashboard.md), [Agent Runtime](02-agent-runtime.md), [Extensions](03-extensions.md).

---

## Design philosophy

The authoring system treats every object in the platform as a typed composition of primitives. There are no special-case configuration blobs. An agent is a composition of a domain, extensions, gates, and model preferences. An arena is a composition of a task source, scoring function, and leaderboard rules. A plan is a composition of tasks, dependencies, and checkpoints.

This follows the same principle as a DAW (digital audio workstation). A DAW has a small number of primitive track types -- audio, MIDI, bus, send. Every song is a composition of those primitives. The DAW never needs a "song type" dropdown because the primitives compose into whatever the user needs.

The authoring system works the same way with 12 primitive object types (per dashboard PRD 23, superseding the 10-primitive vocabulary).

### Primitive object types

| # | Type | What it represents |
|---|------|--------------------|
| 1 | Agent | A configured runtime with archetype (domain + tool profiles + gate pipelines + model preferences), extensions, budget |
| 2 | Extension | A modular behavior unit across three tiers: Pi-compatible (JS/TS), Roko-enhanced (JS/TS + heartbeat), Roko-native (Rust, 22 hooks, 8 layers) |
| 3 | Connector | External system I/O adapter: chain RPC, exchange API, MCP server, database, webhook |
| 4 | Gate | A verification step (pre-action permission or post-action validation): shell command, Rust function, chain simulation, risk check |
| 5 | Feed | A continuous data stream: price feeds, block events, CI status, file changes, webhook streams |
| 6 | Recipe | A composable data transformation pipeline: indicator chains, P&L attribution, HDC encoding, scoring |
| 7 | Knowledge Entry | A typed entry in the durable knowledge store (Insight, Heuristic, Warning, CausalLink, etc.) |
| 8 | Arena | A competitive evaluation environment with task source, scoring, and leaderboard |
| 9 | Eval | A measurement against ground truth, never LLM-graded |
| 10 | Signal | A coordination event published to PulseBus (renamed from Pheromone at product layer) |
| 11 | Group | A coordinated subset of agents with shared state and governance |
| 12 | Bounty | A posted task with reward, escrow, and acceptance criteria |

Three supporting object types build on top of these: **Plan** (a DAG of tasks that reference agents, gates, and connectors), **Template** (a reusable snapshot of any object type), and **Generator** (an agent or function that produces instances of a given object type).

> **Migration from 10 to 12 primitives.** Domain is no longer standalone -- it became the `archetype` field on Agent. Connector, Feed, and Recipe are new additions. Pheromone was renamed to Signal at the product layer. See PRD 23 for the full migration path and backward compatibility guarantees.

### Composition over configuration

Every authoring surface in the dashboard composes these primitives. Creating an agent means selecting an archetype (which pre-fills domain, extensions, gates, and model preferences), adjusting the selection, attaching connectors and feeds, and setting a budget. Creating an arena means composing a task source, gate configuration, scoring function, and leaderboard rules. No surface has a freeform configuration textarea. Every field maps to a typed primitive.

### Progressive disclosure

Simple views come first. The Agent Composer starts with "pick a template." Users who want control drill into domain selection, extension toggles, gate pipeline editing, model routing, and budget configuration. Each level reveals more of the underlying composition without forcing users through it.

### Draft/deploy separation

Authoring is free. Deploying costs tokens, gas, or both. Users iterate on drafts without cost. Drafts auto-save. Deployment is an explicit action with a cost estimate shown before confirmation. This separation means users can experiment freely, keep multiple drafts, and only commit resources when ready.

---

## Plan mutation protocol

This is the headline feature. The user talks to an agent in a floating chat drawer. The agent interprets intent, generates structured mutations, and pushes them to the plan canvas. The canvas animates the changes in real time.

### Mutation types

```rust
/// A single atomic change to a plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum PlanMutation {
    /// Insert a new task into the plan.
    AddTask {
        task: TaskSpec,
        /// Place after this task. None means append to the end.
        after: Option<TaskId>,
    },
    /// Remove a task and all its dependency edges.
    RemoveTask {
        id: TaskId,
    },
    /// Patch fields on an existing task.
    UpdateTask {
        id: TaskId,
        patch: TaskPatch,
    },
    /// Add a dependency edge: `from` must complete before `to` starts.
    AddDependency {
        from: TaskId,
        to: TaskId,
    },
    /// Remove a dependency edge.
    RemoveDependency {
        from: TaskId,
        to: TaskId,
    },
    /// Reorder tasks. The vec represents the new ordering.
    Reorder {
        task_ids: Vec<TaskId>,
    },
    /// Group tasks into a parallel execution lane.
    SetParallel {
        task_ids: Vec<TaskId>,
    },
    /// Insert a manual checkpoint (human review gate) after a task.
    AddCheckpoint {
        after: TaskId,
        name: String,
    },
    /// Update plan-level metadata (name, description, error policy).
    UpdatePlanMeta {
        patch: PlanMetaPatch,
    },
}
```

### Supporting types

```rust
pub type TaskId = String;
pub type PlanId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSpec {
    pub id: TaskId,
    pub title: String,
    pub description: String,
    pub agent_profile: Option<String>,  // "research", "coding", "review"
    pub model: Option<String>,          // "claude-opus-4-6", "claude-sonnet-4-6"
    pub repo: Option<String>,           // "nunchi/roko"
    pub depends_on: Vec<TaskId>,
    pub files: Vec<String>,
    pub est_minutes: Option<u32>,
    pub budget_usd: Option<f64>,
    pub gate_pipeline: Option<Vec<String>>,  // gate IDs to run after completion
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<TaskId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub est_minutes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlanMetaPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_handling: Option<ErrorPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorPolicy {
    /// Stop the pipeline on first failure.
    StopOnFailure,
    /// Skip the failed task and continue with tasks that don't depend on it.
    SkipAndContinue,
    /// Retry the failed task up to N times before stopping.
    Retry { max_attempts: u32 },
    /// Pause and wait for human intervention.
    PauseOnFailure,
}
```

### Conversation-as-plan-editor: the full flow

**Step 1: user opens a session.** The plan canvas loads (empty or showing their last saved draft). A floating chat drawer docks to the right side of the viewport. The drawer is draggable, resizable, and can be undocked into a floating window.

**Step 2: user sends a message.**

```
POST /api/plans/{id}/chat
Content-Type: application/json

{
  "message": "I need to add auth to the roko API. JWT validation, middleware, the whole thing."
}
```

**Step 3: the backend dispatches to an LLM.** The plan chat endpoint builds a system prompt that includes the current plan state, the plan mutation schema, and instructions to respond with both natural language and structured mutations. The LLM call uses the configured model (defaults to `claude-sonnet-4-6` for plan editing, overridable per session).

**Step 4: the backend returns mutations alongside the reply.**

```json
{
  "reply": "I'll break this into four tasks. Research existing auth patterns first, then implement middleware, add tests, and update docs.",
  "mutations": [
    {
      "op": "add_task",
      "task": {
        "id": "t1",
        "title": "Research auth patterns",
        "description": "Analyze existing auth patterns in the codebase and recommend an approach",
        "agent_profile": "research",
        "repo": "nunchi/roko",
        "est_minutes": 5
      },
      "after": null
    },
    {
      "op": "add_task",
      "task": {
        "id": "t2",
        "title": "Implement JWT middleware",
        "description": "Add JWT validation middleware to the axum router",
        "agent_profile": "coding",
        "repo": "nunchi/roko",
        "depends_on": ["t1"],
        "est_minutes": 15
      },
      "after": "t1"
    },
    {
      "op": "add_task",
      "task": {
        "id": "t3",
        "title": "Write integration tests",
        "agent_profile": "coding",
        "repo": "nunchi/roko",
        "depends_on": ["t2"],
        "est_minutes": 8
      },
      "after": "t2"
    },
    {
      "op": "add_task",
      "task": {
        "id": "t4",
        "title": "Update API docs",
        "agent_profile": "coding",
        "repo": "nunchi/roko",
        "depends_on": ["t2"],
        "est_minutes": 5
      },
      "after": "t2"
    }
  ],
  "plan_state": { "task_count": 4, "est_total_minutes": 33 },
  "cost_estimate": {
    "total_usd": 1.20,
    "time_estimate_mins": 33,
    "confidence": 0.65
  }
}
```

The dashboard receives this response, applies the mutations to its local plan state, and animates them on the canvas. Four cards fly in, arrange themselves with dependency arrows, and the cost estimate appears in the footer.

**Step 5: user iterates.**

```
POST /api/plans/{id}/chat

{
  "message": "Split the middleware into two parallel tasks -- one for API routes and one for WebSocket. Use opus for the research."
}
```

Response:

```json
{
  "reply": "Done. Two middleware tasks now run in parallel. Research upgraded to opus.",
  "mutations": [
    { "op": "update_task", "id": "t1", "patch": { "model": "claude-opus-4-6" } },
    { "op": "remove_task", "id": "t2" },
    {
      "op": "add_task",
      "task": {
        "id": "t2a",
        "title": "API auth middleware",
        "description": "JWT validation for /api/* routes",
        "agent_profile": "coding",
        "repo": "nunchi/roko",
        "depends_on": ["t1"],
        "est_minutes": 12
      },
      "after": "t1"
    },
    {
      "op": "add_task",
      "task": {
        "id": "t2b",
        "title": "WebSocket auth middleware",
        "description": "JWT validation for /ws connections",
        "agent_profile": "coding",
        "repo": "nunchi/roko",
        "depends_on": ["t1"],
        "est_minutes": 10
      },
      "after": "t1"
    },
    { "op": "set_parallel", "task_ids": ["t2a", "t2b"] },
    { "op": "add_dependency", "from": "t2a", "to": "t3" },
    { "op": "add_dependency", "from": "t2b", "to": "t3" },
    { "op": "add_dependency", "from": "t2a", "to": "t4" },
    { "op": "add_dependency", "from": "t2b", "to": "t4" }
  ],
  "cost_estimate": {
    "total_usd": 1.80,
    "time_estimate_mins": 40,
    "confidence": 0.7
  }
}
```

The canvas animates: t2 dissolves, t2a and t2b scale in side by side, dependency arrows reroute from the old single path to fan-out/fan-in, and the cost estimate updates in the footer.

This loop continues for as many turns as the user needs. Each turn is cheap (one LLM call for plan editing, not agent execution). The user never edits TOML or YAML. They talk. The plan responds.

### Mutation application rules

The backend validates mutations before persisting:

1. `AddTask` with a duplicate `id` is rejected.
2. `RemoveTask` for a non-existent `id` is rejected.
3. `AddDependency` that would create a cycle is rejected (topological sort check).
4. `SetParallel` tasks must share at least one common predecessor.
5. Rejected mutations return in a `rejected` array with reasons. Valid mutations in the same batch still apply.

```json
{
  "reply": "...",
  "mutations": [ ... ],
  "rejected": [
    { "op": "add_dependency", "from": "t3", "to": "t1", "reason": "would create cycle: t1 -> t2a -> t3 -> t1" }
  ]
}
```

### Chat endpoint contract

```
POST /api/plans/{id}/chat
```

Request:

```json
{
  "message": "string (required)",
  "context": {
    "selected_tasks": ["t2a"],
    "viewport": "lane_view"
  }
}
```

The optional `context` field tells the agent what the user is looking at. If the user has selected a task on the canvas, the agent knows to focus edits there. The `viewport` hint (card_stack, lane_view, node_graph) lets the agent tailor its mutation strategy -- for example, using `set_parallel` only when the user can see lanes.

Response:

```json
{
  "reply": "string",
  "mutations": [ PlanMutation, ... ],
  "rejected": [ { "op": "...", "reason": "..." }, ... ],
  "plan_state": {
    "task_count": 5,
    "dependency_count": 6,
    "parallel_groups": 1,
    "est_total_minutes": 40
  },
  "cost_estimate": {
    "total_usd": 1.80,
    "per_task": [
      { "task_id": "t1", "model": "claude-opus-4-6", "estimated_tokens": 8000, "estimated_usd": 0.40 },
      { "task_id": "t2a", "model": "claude-sonnet-4-6", "estimated_tokens": 5000, "estimated_usd": 0.15 }
    ],
    "time_estimate_mins": 40,
    "confidence": 0.7,
    "breakdown": {
      "inference": 1.20,
      "feeds": 0.10,
      "gas": 0.50
    }
  }
}
```

---

## Plan states and lifecycle

A plan moves through five states. The transitions are explicit API calls, not automatic.

```
         chat           run            pause           resume
Draft ---------> Draft ------> Executing ------> Paused -------> Executing
  ^                              |                                   |
  |                              |                                   |
  |                              +----> Completed                    +----> Completed
  |                              |                                   |
  |                              +----> Failed                       +----> Failed
  |                                                                  |
  +------ (revise remaining) <--- Paused ----(chat)---> Paused ------+
```

| State | Editable | Agents running | Costs accumulating |
|-------|----------|----------------|--------------------|
| Draft | Yes | No | No (only chat model costs for plan editing) |
| Executing | No (frozen) | Yes | Yes |
| Paused | Remaining tasks editable | No (all stopped) | No |
| Completed | No | No | No |
| Failed | No | No | No |

### Run

```
POST /api/plans/{id}/run
```

Request: empty body (or optional overrides).

```json
{
  "budget_override_usd": 5.00,
  "dry_run": false
}
```

Response:

```json
{
  "execution_id": "exec-a1b2c3",
  "plan_id": "plan-xyz",
  "status": "executing",
  "snapshot_id": "snap-001",
  "agents_spawned": 1,
  "next_task": "t1"
}
```

What happens internally:

1. Plan state freezes. A snapshot is written to `.roko/state/plan-{id}-snap-{n}.json`.
2. The orchestrator builds a DAG from the plan's tasks and dependencies.
3. Tasks with no predecessors are dispatched first.
4. As each task completes and passes its gate pipeline, dependent tasks become eligible.
5. Events stream to the dashboard via WebSocket: `plan.task_started`, `plan.task_completed`, `plan.gate_result`, `plan.agent_output`.

### Pause

```
POST /api/plans/{id}/pause
```

Response:

```json
{
  "execution_id": "exec-a1b2c3",
  "status": "paused",
  "completed_tasks": ["t1"],
  "paused_tasks": ["t2a"],
  "remaining_tasks": ["t2b", "t3", "t4"],
  "cost_so_far_usd": 0.55,
  "snapshot_id": "snap-002"
}
```

What happens:

1. All running agents receive a graceful stop signal. Current work is abandoned (agents checkpoint their state if possible).
2. A pause snapshot is written, recording which tasks completed, which were in progress, and which are pending.
3. The plan transitions to Paused. The dashboard shows a frost overlay.
4. The chat drawer reopens. The user can now talk to revise remaining tasks -- the mutations apply only to tasks not yet completed.

### Resume

```
POST /api/plans/{id}/resume
```

Response:

```json
{
  "execution_id": "exec-a1b2c3",
  "status": "executing",
  "resuming_from": "snap-002",
  "remaining_tasks": ["t2a", "t2b", "t3", "t4"],
  "agents_spawning": 2
}
```

Agents respawn for remaining tasks. Completed tasks stay completed. The DAG picks up where it left off. Tasks that were in progress when paused restart from scratch (not from mid-execution state -- agent checkpointing is best-effort and not guaranteed).

---

## Three visual abstraction levels

The backend serves the same plan data model. The dashboard renders at three levels of visual complexity. Users switch between them freely. The backend does not care which view is active.

### Plan data model

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSpec {
    pub id: PlanId,
    pub name: String,
    pub description: String,
    pub status: PlanStatus,
    pub tasks: Vec<TaskSpec>,
    pub dependencies: Vec<(TaskId, TaskId)>,  // (from, to) -- "from" blocks "to"
    pub checkpoints: Vec<Checkpoint>,
    pub parallel_groups: Vec<Vec<TaskId>>,
    pub error_handling: ErrorPolicy,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Draft,
    Executing { execution_id: String },
    Paused { snapshot_id: String },
    Completed { execution_id: String, duration_secs: u64 },
    Failed { execution_id: String, reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub name: String,
    pub after_task: TaskId,
    pub requires_approval: bool,
}
```

### Level 1: card stack

A vertical list of task cards. Drag to reorder. Each card shows title, agent profile, model, estimated time, and status. Dependencies are implicit from ordering -- tasks appear top to bottom in execution order. Parallel tasks are indicated with a subtle "runs with" badge but not visually separated into lanes.

The backend delivers this as the ordered `tasks` vec. No special rendering logic is needed.

Best for: linear pipelines, quick edits, mobile screens, first-time users.

### Level 2: lane view

Parallel tasks occupy side-by-side lanes. Dependency arrows connect tasks across lanes. Drag a card between lanes to change parallelism. Drag a card up or down within a lane to reorder.

The backend delivers this as `tasks` + `parallel_groups` + `dependencies`. The dashboard layout engine computes lane positions from the parallel groups and renders horizontal swim lanes.

Best for: pipelines with 2-4 parallel branches, moderate complexity.

### Level 3: node graph

Full DAG rendered as a node graph (like React Flow). Tasks are nodes. Dependencies are directed edges. Conditional branches appear as diamond decision nodes. Fan-out (one task feeding many) and fan-in (many tasks feeding one) are visible as edge topology. Checkpoints appear as gate nodes between task nodes.

The backend delivers this as the full `PlanSpec`. The dashboard graph engine computes layout using a layered graph algorithm (Sugiyama-style) and renders with a library like xyflow.

Best for: complex multi-branch pipelines, power users, plans with 10+ tasks.

### View-switching API

There is no view-switching API. The backend always returns the full `PlanSpec`. The dashboard chooses how to render it. If a user creates a plan in card stack view and another user opens it in node graph view, both see the same plan. The view is a client-side preference, not a plan property.

---

## Template registry

Every object type has templates. Templates are discoverable, forkable, and user-publishable.

### Template data model

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: TemplateId,
    pub object_type: ObjectType,
    pub name: String,
    pub description: String,
    pub author: AuthorId,
    pub source: TemplateSource,
    pub version: semver::Version,
    /// The full configuration for the object type.
    /// Validated against the object type's schema on publish.
    pub config: serde_json::Value,
    pub forked_from: Option<TemplateId>,
    pub tags: Vec<String>,
    pub downloads: u64,
    pub rating: f32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub type TemplateId = String;
pub type AuthorId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectType {
    Agent,
    Extension,
    Connector,   // NEW (PRD 23)
    Gate,
    Feed,        // NEW (PRD 23)
    Recipe,      // NEW (PRD 23)
    Knowledge,
    Arena,
    Eval,
    Signal,      // Renamed from Pheromone (PRD 23)
    Group,
    Bounty,
    Plan,
    Generator,
    MetaAgent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateSource {
    /// Curated by the platform. Always available.
    System,
    /// Published by a user to the community.
    Community,
    /// Private to the author. Not discoverable by others.
    User,
}
```

### Template API

```
GET /api/templates?type=agent&sort=downloads&limit=20
```

Response:

```json
{
  "templates": [
    {
      "id": "tmpl-coding-rust",
      "object_type": "agent",
      "name": "Rust coding agent",
      "description": "Pre-configured for Rust development with clippy gate, test gate, and cargo-based tool access",
      "author": "system",
      "source": "system",
      "version": "1.2.0",
      "tags": ["rust", "coding", "beginner-friendly"],
      "downloads": 342,
      "rating": 4.7
    }
  ],
  "total": 48,
  "offset": 0,
  "limit": 20
}
```

```
GET /api/templates/{id}
```

Returns the full template including its `config` blob.

```
POST /api/templates
Content-Type: application/json

{
  "object_type": "agent",
  "name": "My custom researcher",
  "description": "Tuned for deep technical research with opus model and extended context",
  "config": { ... },
  "tags": ["research", "technical"],
  "visibility": "community"
}
```

Response:

```json
{
  "id": "tmpl-abc123",
  "version": "1.0.0"
}
```

```
POST /api/templates/{id}/fork
```

Creates a copy under the calling user's ownership. The new template's `forked_from` field points to the original.

```
DELETE /api/templates/{id}
```

Unpublishes a template. Only the author can unpublish. Already-deployed instances that used the template continue to work.

### On-chain registration

Popular templates can optionally register on-chain for permanent discoverability. This uses the same ERC-8004 registry described in [14-registries.md](14-registries.md), extended with a `TemplateRegistered` event. On-chain registration is not required for local or community use -- it provides permanent availability and cross-instance discovery.

---

## Connector Manager (new authoring surface)

> Added 2026-04-24. Per dashboard PRD 23, Connector is a universal primitive with a dedicated 4-stage authoring flow.

| Stage | What | Notes |
|-------|------|-------|
| 1. Type selection | Choose connector type (Chain RPC, Exchange API, MCP Server, Database, Webhook) | Template gallery with icons per type |
| 2. Configuration | Connection string, auth credentials, rate limits, retry policy | Live health check runs during configuration |
| 3. Tool registration | Auto-discover available operations; select which to expose as agent tools | Derived from connector schema (e.g., MCP tool list, exchange order types) |
| 4. Test and deploy | Execute test query; verify health endpoint | Shows latency p50/p99, error rate, connection status |

**API contract:** Follows the standard authoring pattern: `POST /api/connectors` (create), `GET /api/connectors` (list), `POST /api/connectors/{id}/validate`, `POST /api/connectors/{id}/deploy`.

**Relationship to agents:** An agent's `roko.toml` config references connectors by name. The Agent Composer's tool selection stage (Stage 4) pulls available operations from the agent's attached connectors.

---

## Feed Designer (new authoring surface)

> Added 2026-04-24. Per dashboard PRD 23, Feed is a universal primitive with a dedicated 4-stage authoring flow.

| Stage | What | Notes |
|-------|------|-------|
| 1. Source selection | Choose source connector and event type | Connector picker (only deployed connectors appear) |
| 2. Filter and transform | Configure event filters, sampling rate, aggregation window | Visual filter builder with preview of matching events |
| 3. Output configuration | Target: PulseBus topic, recipe input, agent subscription | Wiring diagram showing downstream consumers |
| 4. Monitor | Live event count, latency, error rate, backpressure | Real-time sparkline with 5-minute window |

**API contract:** `POST /api/feeds` (create), `GET /api/feeds` (list with cursor pagination), `POST /api/feeds/{id}/validate`, `POST /api/feeds/{id}/deploy`.

**Relationship to existing feed architecture:** The feed registration, discovery, and subscription mechanisms defined earlier in this spec ([05-feeds.md](05-feeds.md)) provide the backend. The Feed Designer is the dashboard authoring surface that creates and configures those feed registrations.

---

## Recipe Editor (new authoring surface)

> Added 2026-04-24. Per dashboard PRD 23, Recipe is a universal primitive with a dedicated 4-stage authoring flow.

| Stage | What | Notes |
|-------|------|-------|
| 1. Input selection | Choose feed(s) or connector query as input | Drag from feed/connector list; multiple inputs supported |
| 2. Pipeline builder | Chain transform stages: map, filter, window, aggregate, score | Visual DAG editor (similar to node graph view for plans) |
| 3. Output configuration | Emit as: Signal, Knowledge Entry, Feed, or raw value | Type-checked output with schema validation |
| 4. Backtest and validate | Run against historical data; compare output distribution | Chart overlay showing expected vs actual output |

**API contract:** `POST /api/recipes` (create), `GET /api/recipes` (list), `POST /api/recipes/{id}/validate`, `POST /api/recipes/{id}/deploy`, `POST /api/recipes/{id}/backtest`.

**Relationship to existing scoring:** Recipes compose `Scorer` trait instances from `roko-core`. Existing scoring pipelines in `roko-learn` (TradingReflect, FifoMatcher, IndicatorTracker) become built-in recipe templates available in the Recipe Editor's template picker.

---

## Extension compilation service

Users author extensions in the Extension Workshop. The backend compiles them in a sandboxed environment.

### Compile endpoint

```
POST /api/extensions/compile
Content-Type: application/json

{
  "name": "my-custom-gate",
  "source": "use roko_core::extension::*;\n\npub struct MyGate;\n\nimpl Extension for MyGate {\n    // ...\n}",
  "dependencies": ["tokio", "serde"],
  "target_hooks": ["post_action", "pre_gate"]
}
```

Response (success):

```json
{
  "status": "success",
  "artifact_id": "ext-a1b2c3",
  "warnings": [
    { "line": 12, "column": 5, "message": "unused variable `ctx`", "level": "warning" }
  ],
  "compile_time_ms": 2400,
  "artifact_size_bytes": 245760
}
```

Response (failure):

```json
{
  "status": "error",
  "errors": [
    { "line": 42, "column": 18, "message": "expected `;`, found `}`", "level": "error" },
    { "line": 15, "column": 1, "message": "cannot find type `ExtensionContext` in this scope", "level": "error" }
  ],
  "warnings": []
}
```

### Sandbox model

Compilation runs in a container (or Fly Machine for cloud deployments). The sandbox has:

- A pre-cached Rust toolchain and the roko extension SDK.
- Network access restricted to crates.io for dependency fetching.
- A 60-second timeout and 2GB memory limit.
- No access to the host filesystem beyond the compilation workspace.

The compiled artifact (a shared library) is stored in the artifact registry and can be loaded by agents at runtime. Each artifact is content-addressed by its source hash, so recompiling identical source returns the cached artifact.

---

## Cost projection

Before executing a plan, users see an estimated cost and time.

### Estimate endpoint

```
POST /api/plans/{id}/estimate
```

Response:

```json
{
  "total_usd": 1.80,
  "per_task": [
    {
      "task_id": "t1",
      "model": "claude-opus-4-6",
      "estimated_input_tokens": 4000,
      "estimated_output_tokens": 2000,
      "estimated_usd": 0.40,
      "estimated_minutes": 5
    },
    {
      "task_id": "t2a",
      "model": "claude-sonnet-4-6",
      "estimated_input_tokens": 6000,
      "estimated_output_tokens": 3000,
      "estimated_usd": 0.25,
      "estimated_minutes": 12
    },
    {
      "task_id": "t2b",
      "model": "claude-sonnet-4-6",
      "estimated_input_tokens": 5000,
      "estimated_output_tokens": 2500,
      "estimated_usd": 0.20,
      "estimated_minutes": 10
    },
    {
      "task_id": "t3",
      "model": "claude-sonnet-4-6",
      "estimated_input_tokens": 4000,
      "estimated_output_tokens": 4000,
      "estimated_usd": 0.30,
      "estimated_minutes": 8
    },
    {
      "task_id": "t4",
      "model": "claude-sonnet-4-6",
      "estimated_input_tokens": 3000,
      "estimated_output_tokens": 1500,
      "estimated_usd": 0.15,
      "estimated_minutes": 5
    }
  ],
  "time_estimate_mins": 40,
  "critical_path_mins": 27,
  "confidence": 0.7,
  "breakdown": {
    "inference": 1.20,
    "feeds": 0.10,
    "gas": 0.50
  }
}
```

Note the distinction between `time_estimate_mins` (wall clock, accounting for parallelism) and the sum of per-task minutes (total agent-minutes). The `critical_path_mins` field shows the longest sequential chain through the DAG.

### Estimation algorithm

The cost projector uses three inputs:

1. **Task description complexity.** Longer descriptions, more files, broader scope heuristics push token estimates up. This is a simple heuristic, not an LLM call.

2. **Model pricing.** Current per-token rates for each model, stored in `roko.toml` and refreshed from the provider health endpoint.

3. **Historical data from similar tasks.** The learning system (`.roko/learn/efficiency.jsonl`) records actual tokens used and time taken for past tasks. The projector queries this store for tasks with similar agent profiles, models, and description lengths, and uses the p50 from matching historical data when available. When no historical data matches, it falls back to static heuristics.

The `confidence` field reflects how much historical data informed the estimate. A confidence of 0.5 means roughly half the estimate came from heuristics. A confidence of 0.9 means strong historical data supports the numbers.

---

## Gate test runner

Users test gates against fixtures before deploying them to a pipeline.

### Test endpoint

```
POST /api/gates/{id}/test
Content-Type: application/json

{
  "fixture_path": "fixtures/sample-rust-project",
  "expected_result": "pass",
  "timeout_ms": 10000
}
```

Response (pass):

```json
{
  "result": "pass",
  "expected": "pass",
  "match": true,
  "output": "47 tests passed, 0 failed, 0 ignored",
  "duration_ms": 3200,
  "gate_details": {
    "gate_type": "test",
    "command": "cargo test --workspace",
    "exit_code": 0,
    "stdout_lines": 52,
    "stderr_lines": 3
  }
}
```

Response (unexpected failure):

```json
{
  "result": "fail",
  "expected": "pass",
  "match": false,
  "output": "3 tests passed, 2 failed",
  "duration_ms": 4100,
  "gate_details": {
    "gate_type": "test",
    "command": "cargo test --workspace",
    "exit_code": 101,
    "failures": [
      { "test": "test_auth_middleware", "message": "assertion failed: response.status().is_success()" },
      { "test": "test_ws_auth", "message": "timeout after 5000ms" }
    ]
  }
}
```

The test endpoint runs the gate in the same sandbox used for extension compilation. It does not affect any running agents or live plans.

---

## Authoring API contracts

Each of the 13 authoring surfaces follows a consistent REST pattern. The object type name slots into the URL.

### CRUD

```
POST   /api/{object_type}              -- create (from template or blank)
GET    /api/{object_type}              -- list (with pagination, filtering)
GET    /api/{object_type}/{id}         -- read (full detail including composition)
PUT    /api/{object_type}/{id}         -- update (full replacement)
PATCH  /api/{object_type}/{id}         -- partial update
DELETE /api/{object_type}/{id}         -- delete (soft delete for deployed objects)
```

Where `{object_type}` is one of: `agents`, `extensions`, `connectors`, `gates`, `feeds`, `recipes`, `knowledge`, `arenas`, `evals`, `signals`, `groups`, `bounties`, `plans`, `templates`, `generators`.

### Create from template

```
POST /api/agents
Content-Type: application/json

{
  "from_template": "tmpl-coding-rust",
  "overrides": {
    "name": "My Rust agent",
    "model": "claude-opus-4-6",
    "budget_daily_usd": 10.0
  }
}
```

The backend loads the template, applies overrides, validates the result, and creates the object in draft state. If `from_template` is omitted, a blank object is created with required fields empty (validation will flag them).

### Validation

```
POST /api/{object_type}/{id}/validate
```

Response:

```json
{
  "valid": false,
  "errors": [
    {
      "field": "ground_truth",
      "message": "Ground truth source is required for evals",
      "severity": "error",
      "code": "REQUIRED_FIELD"
    }
  ],
  "warnings": [
    {
      "field": "budget_daily_usd",
      "message": "Budget of $0.50/day is low for an opus-tier agent -- typical daily cost is $2-5",
      "severity": "warning",
      "code": "LOW_BUDGET"
    }
  ],
  "suggestions": [
    {
      "field": "model",
      "message": "Consider claude-sonnet for research tasks to reduce cost by ~60%",
      "severity": "suggestion",
      "code": "MODEL_SUGGESTION"
    }
  ]
}
```

Three severity levels:

- **error**: blocks deploy. Missing required fields, invalid values, structural problems.
- **warning**: does not block but flags risk. Suboptimal configuration, potential cost issues.
- **suggestion**: advisory. Recommendations based on domain best practices and historical data.

Validation runs automatically as the user edits (debounced, not on every keystroke). The dashboard calls the validate endpoint after 500ms of inactivity. Errors appear inline next to the relevant field. Warnings and suggestions appear in a sidebar panel.

### Deploy

```
POST /api/{object_type}/{id}/deploy
```

Request:

```json
{
  "target": "local",
  "register_on_chain": false
}
```

Response:

```json
{
  "deployment_id": "dep-xyz",
  "status": "deploying",
  "estimated_cost": {
    "gas_wei": "0",
    "tokens_usd": 0.0,
    "inference_usd": 0.0
  }
}
```

Deploy transitions the object from draft to live. For agents, this means spawning a runtime process. For gates, this means registering in the gate pipeline registry. For templates, this is a no-op (templates are "live" when published to the template registry).

The `register_on_chain` flag triggers ERC-8004 registration for agents, or the appropriate on-chain registry for other object types.

### Publish as template

```
POST /api/{object_type}/{id}/publish
Content-Type: application/json

{
  "template_name": "My optimized researcher",
  "description": "Research agent tuned for Rust codebases with extended context window",
  "tags": ["research", "rust"],
  "visibility": "community"
}
```

This snapshots the current configuration and publishes it to the template registry. The original object continues to exist independently -- future edits to the object do not affect the published template.

---

## Event types for authoring

All authoring events follow the existing `ServerEvent` pattern in roko-serve.

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthoringEvent {
    /// A chat mutation was applied to a plan.
    PlanMutationApplied {
        plan_id: PlanId,
        mutation_count: usize,
        rejected_count: usize,
        new_task_count: usize,
    },
    /// A plan transitioned between states.
    PlanStateChanged {
        plan_id: PlanId,
        from: PlanStatus,
        to: PlanStatus,
    },
    /// A new template was published to the registry.
    TemplatePublished {
        template_id: TemplateId,
        object_type: ObjectType,
        author: AuthorId,
    },
    /// Someone forked a template.
    TemplateForked {
        template_id: TemplateId,
        forked_from: TemplateId,
        author: AuthorId,
    },
    /// Extension compilation completed (success or failure).
    ExtensionCompiled {
        extension_name: String,
        artifact_id: Option<String>,
        success: bool,
        error_count: usize,
    },
    /// An object passed validation.
    ObjectValidated {
        object_type: ObjectType,
        object_id: String,
        error_count: usize,
        warning_count: usize,
    },
    /// An object was deployed (made live).
    ObjectDeployed {
        object_type: ObjectType,
        object_id: String,
        deployment_id: String,
        registered_on_chain: bool,
    },
}
```

Events stream to the dashboard via the existing WebSocket room system. The plan chat session subscribes to `plan:{id}` to receive mutation events. The fleet page subscribes to `system` to receive deployment events. Template pages subscribe to `templates` to receive publish and fork events.

---

## Ecosystem dynamics

User-contributed content creates a flywheel. Each stage feeds the next.

### The template flywheel

1. A user builds an effective agent configuration for Rust development.
2. They click "Publish as template" and share it with the community.
3. Other users discover it in the template picker, fork it, and adapt it.
4. Forks that perform well get higher ratings and more downloads.
5. The original author sees fork activity and can incorporate improvements back.
6. Meta-agents can create and publish templates automatically based on performance data.

### Backend tracking for recommendations

The backend tracks per-template metrics:

```json
{
  "template_id": "tmpl-abc",
  "downloads": 342,
  "forks": 28,
  "active_deployments": 15,
  "avg_rating": 4.7,
  "rating_count": 23,
  "avg_task_success_rate": 0.89,
  "avg_cost_per_task_usd": 0.45,
  "last_used": "2026-04-23T14:30:00Z"
}
```

These metrics feed into template recommendations. When a user creates a new agent and selects the "coding" domain, the template picker ranks templates by a combination of rating, usage, and success rate within that domain.

### Generator-driven template creation

Generators (described in [13-meta.md](13-meta.md)) can produce domain-specific templates at scale. A generator configured for "blockchain security analysis" can produce template variants tuned for different chain types (EVM, Solana, Cosmos), each with appropriate extensions, gates, and model preferences. These generated templates enter the registry with `source: "system"` and go through the same rating/download cycle as user-published templates.

---

## Relationship to existing codebase

### What exists today

The current `roko-serve` plan routes (`/api/plans`, `/api/plans/{id}`, `/api/plans/{id}/execute`, `/api/plans/{id}/status`, `/api/plans/generate`) support basic CRUD and execution. The `Plan` and `PlanTask` types in `roko-serve/src/plan_types.rs` model a flat task list with dependencies and completion status.

### What this spec adds

| Feature | Current state | Spec target |
|---------|---------------|-------------|
| Plan data model | Flat task list, string IDs, `depends_on` vec | Extended with `parallel_groups`, `checkpoints`, `error_handling`, plan-level status |
| Plan editing | Direct JSON/TOML file editing | Chat-driven mutation protocol |
| Plan execution | Single `run_once` call | DAG-aware orchestration with pause/resume |
| Templates | Not present | Full registry with CRUD, forking, versioning, community publishing |
| Validation | Basic field presence checks | Three-tier validation (error/warning/suggestion) for all object types |
| Cost estimation | Not present | Historical-data-informed projection with per-task breakdown |
| Extension compilation | Not present | Sandboxed compilation service |
| Gate testing | Not present | Fixture-based test runner |
| Authoring surfaces | Not present | Consistent CRUD + validate + deploy + publish pattern for 13 object types |

### Implementation path

The plan mutation protocol is the critical path. It depends on:

1. Extending `PlanSpec` with `parallel_groups`, `checkpoints`, and `status` fields.
2. Adding the `/api/plans/{id}/chat` endpoint that dispatches to an LLM and returns structured mutations.
3. Adding `/api/plans/{id}/pause` and `/api/plans/{id}/resume` endpoints backed by the existing `ExecutorSnapshot` mechanism.
4. Building the mutation validation logic (cycle detection, duplicate ID checks).

Everything else -- templates, compilation, gate testing, authoring CRUD -- is independent and can be built in parallel.

---

## Summary of API surface

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/plans/{id}/chat` | POST | Conversation-driven plan editing |
| `/api/plans/{id}/run` | POST | Snapshot plan and begin execution |
| `/api/plans/{id}/pause` | POST | Stop all agents, freeze state |
| `/api/plans/{id}/resume` | POST | Respawn agents for remaining tasks |
| `/api/plans/{id}/estimate` | POST | Cost and time projection |
| `/api/templates` | GET/POST | List and publish templates |
| `/api/templates/{id}` | GET/DELETE | Template detail and unpublish |
| `/api/templates/{id}/fork` | POST | Fork a template |
| `/api/connectors` | GET/POST | List and create connectors (PRD 23) |
| `/api/connectors/{id}` | GET/PUT/PATCH/DELETE | Connector CRUD |
| `/api/feeds` | GET/POST | List and create feeds (PRD 23) |
| `/api/feeds/{id}` | GET/PUT/PATCH/DELETE | Feed CRUD |
| `/api/recipes` | GET/POST | List and create recipes (PRD 23) |
| `/api/recipes/{id}` | GET/PUT/PATCH/DELETE | Recipe CRUD |
| `/api/recipes/{id}/backtest` | POST | Run recipe against historical data |
| `/api/extensions/compile` | POST | Compile extension source in sandbox |
| `/api/gates/{id}/test` | POST | Run gate against test fixture |
| `/api/{object_type}` | GET/POST | List and create objects (15 types) |
| `/api/{object_type}/{id}` | GET/PUT/PATCH/DELETE | Object CRUD |
| `/api/{object_type}/{id}/validate` | POST | Three-tier validation |
| `/api/{object_type}/{id}/deploy` | POST | Deploy object (make live) |
| `/api/{object_type}/{id}/publish` | POST | Publish as template |
