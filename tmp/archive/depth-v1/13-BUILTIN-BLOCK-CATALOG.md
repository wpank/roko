# 13 — Builtin Block Catalog

> Every Block that ships with Roko, organized by protocol conformance.

**Source**: wf-06 (Builtin Workflow Catalog), expanded and reorganized with unified vocabulary.

---

## 1. Overview

Roko ships with a catalog of built-in Blocks covering all nine protocols. Each Block declares typed I/O, capabilities, cost estimates, and protocol conformance. Blocks compose into Graphs — the catalog is deliberately large because more composable pieces yield more emergent value.

Naming convention: kebab-case noun-or-verb-phrase. Blocks describe operations; Graphs describe outcomes.

### Catalog summary

| Protocol | Built-in Blocks | Primary domain |
|---|---|---|
| Store | 3 | Signal persistence |
| Score | 3 | Signal quality rating |
| Verify | 6 | Truth checking, gates |
| Route | 3 | Candidate selection |
| Compose | 3 | Signal combination |
| React | 3 | Policy enforcement |
| Observe | 10 | Telemetry (Lenses) |
| Connect | 5 | External I/O |
| Trigger | 7 | Event-driven Graph firing |

Total: **43 built-in Blocks** at launch.

---

## 2. Store Blocks

Blocks implementing the Store protocol: `put / get / query / prune` Signals.

### `file-store`

Persists Signals as JSONL on the local filesystem.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Store |
| Input | `Signal` |
| Output | `SignalRef` |
| Capabilities | `FsRead`, `FsWrite` |
| Description | Default Store for local development. Append-only JSONL with content-addressed IDs. Supports query by Kind, time range, and HDC similarity. Prune removes entries below decay threshold. |

```toml
[[nodes]]
id = "persist"
block = "file-store@^1"
[nodes.params]
path = ".roko/signals.jsonl"
```

### `memory-store`

In-memory Store for ephemeral Flows and testing.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Store |
| Input | `Signal` |
| Output | `SignalRef` |
| Capabilities | (none) |
| Description | Fast, volatile Store for unit tests and ephemeral Flows. All data lost on process exit. Supports the same query interface as FileStore. |

```toml
[[nodes]]
id = "test-store"
block = "memory-store@^1"
```

### `chain-store`

Persists Signal commitments on-chain for tamper-evident audit.

| Field | Value |
|---|---|
| Version | 0.1.0 |
| Protocols | Store |
| Input | `Signal` |
| Output | `SignalRef` (with tx hash) |
| Capabilities | `Chain { read: true, write: true }` |
| Description | Writes content hashes (not full content) to an on-chain registry. Used for custody proofs and cross-agent attestation. Phase 2+. |

```toml
[[nodes]]
id = "anchor"
block = "chain-store@^0.1"
[nodes.params]
network = "base-sepolia"
```

---

## 3. Score Blocks

Blocks implementing the Score protocol: rate Signals along dimensions (relevance, quality, confidence, novelty, utility).

### `llm-scorer`

Model-based Signal scoring.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Score |
| Input | `Signal` |
| Output | `ScoreResult { relevance, quality, confidence, novelty, utility }` |
| Capabilities | `Llm` |
| Description | Sends the Signal to an LLM with a scoring rubric. Returns five-dimensional score. Model selected via CascadeRouter unless overridden. |

```toml
[[nodes]]
id = "score"
block = "llm-scorer@^1"
[nodes.params]
rubric = "code-quality"
model = "claude-haiku-4-5"
```

### `rule-scorer`

Rule-based Signal scoring.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Score |
| Input | `Signal` |
| Output | `ScoreResult` |
| Capabilities | (none) |
| Description | Evaluates Signals against declarative rules (regex matches, field presence, length thresholds, keyword density). Zero LLM cost. |

```toml
[[nodes]]
id = "filter-score"
block = "rule-scorer@^1"
[nodes.params]
rules = [
  { field = "content.length", op = "gte", value = 100, dimension = "quality", weight = 0.3 },
  { field = "kind", op = "eq", value = "code", dimension = "relevance", weight = 0.5 },
]
```

### `hdc-scorer`

HDC vector similarity scoring.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Score |
| Input | `Signal` (with HDC fingerprint) |
| Output | `ScoreResult` (similarity-weighted) |
| Capabilities | (none) |
| Description | Computes HDC cosine similarity between the input Signal and a reference set. Returns similarity as the relevance dimension. Used for knowledge retrieval ranking. |

```toml
[[nodes]]
id = "similarity"
block = "hdc-scorer@^1"
[nodes.params]
reference_set = "knowledge"
top_k = 20
```

---

## 4. Verify Blocks

Blocks implementing the Verify protocol: check Signals against truth criteria, produce Verdicts.

### `compile-gate`

Checks that code compiles.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Verify |
| Input | `Signal { kind: Code }` |
| Output | `Verdict { passed, confidence, findings, evidence }` |
| Capabilities | `Shell { commands: ["cargo", "rustc", "tsc", "go"] }`, `FsRead` |
| Description | Runs the language-appropriate compiler. Captures stderr as Finding Signals. Confidence is binary (1.0 on pass, 0.0 on fail). Gate rung: 1. |

```toml
[[nodes]]
id = "compile"
block = "compile-gate@^1"
[nodes.params]
command = "cargo check --workspace"
```

### `test-gate`

Runs test suites and checks for regressions.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Verify |
| Input | `Signal { kind: Code }` |
| Output | `Verdict` |
| Capabilities | `Shell { commands: ["cargo", "npm", "pytest", "jest"] }`, `FsRead` |
| Description | Runs the appropriate test runner, parses results, emits per-test Findings. Confidence = pass_rate. Gate rung: 2. |

```toml
[[nodes]]
id = "test"
block = "test-gate@^1"
[nodes.params]
command = "cargo test --workspace"
target = "all"
```

### `clippy-gate`

Static analysis and lint checks.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Verify |
| Input | `Signal { kind: Code }` |
| Output | `Verdict` |
| Capabilities | `Shell { commands: ["cargo", "clippy", "eslint", "ruff"] }`, `FsRead` |
| Description | Runs the linter with `-D warnings` (deny all warnings). Findings are lint violations with location and severity. Gate rung: 3. |

```toml
[[nodes]]
id = "lint"
block = "clippy-gate@^1"
[nodes.params]
command = "cargo clippy --workspace --no-deps -- -D warnings"
```

### `diff-gate`

Validates that a diff matches expected patterns or constraints.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Verify |
| Input | `Signal { kind: Diff }` |
| Output | `Verdict` |
| Capabilities | `FsRead`, `Shell { commands: ["git"] }` |
| Description | Checks diff against constraints: max lines changed, no secret patterns, no binary files, restricted paths respected. Gate rung: 4. |

```toml
[[nodes]]
id = "diff-check"
block = "diff-gate@^1"
[nodes.params]
max_lines = 500
forbidden_patterns = ["API_KEY", "SECRET", "password"]
```

### `llm-judge-gate`

LLM-based quality evaluation producing a Verdict.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Score, Verify |
| Input | `Signal` |
| Output | `Verdict` (with ScoreResult embedded) |
| Capabilities | `Llm` |
| Description | Sends the Signal to an LLM with evaluation criteria. Scores quality, then verifies against a threshold. Multi-protocol: Score + Verify. Gate rung: 5. |

```toml
[[nodes]]
id = "judge"
block = "llm-judge-gate@^1"
[nodes.params]
criteria = ["correctness", "completeness", "clarity"]
threshold = 0.7
model = "claude-sonnet-4-6"
```

### `consensus-gate`

Multi-evaluator consensus verification.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Verify |
| Input | `Vec<Verdict>` (from multiple upstream Verify Blocks) |
| Output | `Verdict` (aggregate) |
| Capabilities | (none) |
| Description | Takes N Verdicts, applies a consensus strategy (majority, unanimous, weighted, quorum). Used as the final gate in multi-judge pipelines. Gate rung: 6. |

```toml
[[nodes]]
id = "consensus"
block = "consensus-gate@^1"
[nodes.params]
strategy = "majority"
min_voters = 3
```

---

## 5. Route Blocks

Blocks implementing the Route protocol: select among candidates, learn from outcomes.

### `cascade-router`

LinUCB bandit-based model routing with learning.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Route, Observe |
| Input | `Vec<Signal>` (candidates) |
| Output | `RouteResult { selected, confidence, reason }` |
| Capabilities | (none) |
| Description | Selects the best candidate (typically a model) using a contextual bandit. Observes outcomes for feedback. Persists state to `.roko/learn/cascade-router.json`. Multi-protocol: Route + Observe. |

```toml
[[nodes]]
id = "model-select"
block = "cascade-router@^1"
[nodes.params]
candidates = ["claude-opus-4-6", "claude-sonnet-4-6", "claude-haiku-4-5"]
context_features = ["task_complexity", "domain", "budget_remaining"]
```

### `rule-router`

Deterministic rule-based routing.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Route |
| Input | `Vec<Signal>` (candidates) |
| Output | `RouteResult` |
| Capabilities | (none) |
| Description | Evaluates candidates against declarative rules. No learning. Used for fixed routing policies (e.g., "always use Opus for security reviews"). |

```toml
[[nodes]]
id = "fixed-route"
block = "rule-router@^1"
[nodes.params]
rules = [
  { condition = "task.domain == 'security'", select = "claude-opus-4-6" },
  { condition = "task.budget_usd < 0.10", select = "claude-haiku-4-5" },
  { condition = "true", select = "claude-sonnet-4-6" },
]
```

### `cost-router`

Cheapest-viable candidate selection.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Route |
| Input | `Vec<Signal>` (candidates with cost estimates) |
| Output | `RouteResult` |
| Capabilities | (none) |
| Description | Selects the cheapest candidate that meets a minimum quality threshold. Useful for cost-sensitive Graphs. |

```toml
[[nodes]]
id = "cheap-route"
block = "cost-router@^1"
[nodes.params]
min_quality = 0.6
prefer = "cheapest"
```

---

## 6. Compose Blocks

Blocks implementing the Compose protocol: combine Signals under budget into one Signal.

### `prompt-composer`

9-layer system prompt assembly.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Compose |
| Input | `Vec<Signal>` (context Signals: role, task, knowledge, history, constraints) |
| Output | `Signal { kind: Prompt }` |
| Capabilities | (none) |
| Description | Assembles a system prompt from up to 9 layers (role, domain, task, context, knowledge, history, constraints, tools, format). Budget-aware: truncates lower-priority layers to fit token limit. Maps to existing `RoleSystemPromptSpec`. |

```toml
[[nodes]]
id = "build-prompt"
block = "prompt-composer@^1"
[nodes.params]
role = "strategist"
max_tokens = 8000
priority_order = ["role", "task", "context", "knowledge", "constraints"]
```

### `vcg-composer`

VCG auction-based Signal combination.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Compose |
| Input | `Vec<Signal>` (with bids) |
| Output | `Signal` (combined) |
| Capabilities | (none) |
| Description | Runs a Vickrey-Clarke-Groves auction among context bidders (Neuro, Task, Research). Each bidder declares value for token budget. VCG allocates efficiently. Built and exported but greedy path currently dominates at runtime. |

```toml
[[nodes]]
id = "auction-compose"
block = "vcg-composer@^1"
[nodes.params]
max_tokens = 16000
bidders = ["neuro", "task", "research"]
```

### `greedy-composer`

Top-K by score Signal combination.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Compose |
| Input | `Vec<Signal>` (scored) |
| Output | `Signal` (combined) |
| Capabilities | (none) |
| Description | Sorts input Signals by score, takes top K that fit within budget. Simple, fast, predictable. The default composition strategy. |

```toml
[[nodes]]
id = "top-k"
block = "greedy-composer@^1"
[nodes.params]
max_signals = 10
max_tokens = 4000
sort_by = "relevance"
```

---

## 7. React Blocks

Blocks implementing the React protocol: watch Signal streams, emit new Signals as interventions.

### `safety-reactor`

Halt on danger signals.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | React |
| Input | `Signal` (any) |
| Output | `Vec<Signal>` (intervention Signals) |
| Capabilities | (none) |
| Description | Monitors Signal stream for safety violations: cost anomalies, permission escalation attempts, infinite loops, prompt injection indicators. Emits halt Signals that the execution engine respects. |

```toml
[[nodes]]
id = "safety"
block = "safety-reactor@^1"
[nodes.params]
sensitivity = "high"
actions = ["halt", "alert"]
```

### `budget-reactor`

Alert and throttle on budget thresholds.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | React |
| Input | `Signal { kind: CostReport }` |
| Output | `Vec<Signal>` (alert or throttle Signals) |
| Capabilities | (none) |
| Description | Watches cost Signals. At 75% budget: emits warning. At 90%: emits throttle (engine switches to cheaper models). At 100%: emits halt. Thresholds configurable. |

```toml
[[nodes]]
id = "budget-watch"
block = "budget-reactor@^1"
[nodes.params]
warn_pct = 75
throttle_pct = 90
halt_pct = 100
```

### `escalation-reactor`

Notify humans on escalation conditions.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | React |
| Input | `Signal` (any) |
| Output | `Vec<Signal>` (notification Signals) |
| Capabilities | `Net` (for notification delivery) |
| Description | When conditions are met (repeated failures, confidence below threshold, structural changes proposed), emits notification Signals routed to configured channels (Slack, email, dashboard). |

```toml
[[nodes]]
id = "escalate"
block = "escalation-reactor@^1"
[nodes.params]
conditions = ["gate_fail_count > 3", "confidence < 0.3"]
channels = ["slack", "dashboard"]
```

---

## 8. Observe Blocks (Lenses)

Blocks implementing the Observe protocol. Lenses are read-only observers that emit observation Signals onto the Bus. They never modify what they observe. See [doc-09 (Telemetry)](09-TELEMETRY.md) for the full Lens system.

### `cost-lens`

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Observe |
| Scope | Block, Graph, Agent |
| Emits | `CostReport` Signals per interval |
| Description | Aggregates USD cost across observed scope. Emits periodic CostReport Signals with total, rate, breakdown by model/provider. |

### `latency-lens`

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Observe |
| Scope | Block, Graph |
| Emits | Latency distribution Signals (p50, p95, p99) |
| Description | Tracks execution duration across observed scope. Emits percentile distributions at configurable intervals. |

### `quality-lens`

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Observe |
| Scope | Graph |
| Emits | Pass-rate Signals from Verify Blocks |
| Description | Observes Verify Block Verdicts within the Graph. Emits rolling pass rate, trend direction, and per-gate breakdown. |

### `efficiency-lens`

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Observe |
| Scope | Agent |
| Emits | Tokens-per-task ratio Signals |
| Description | Tracks token usage relative to task completion. Lower ratio = more efficient agent. Feeds the CascadeRouter's learning loop. |

### `error-lens`

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Observe |
| Scope | Block, Graph, Agent |
| Emits | Classified error report Signals |
| Description | Categorizes errors by type (timeout, capability, external, logic, cancelled). Emits error frequency and trend data. |

### `drift-lens`

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Observe |
| Scope | Memory |
| Emits | Knowledge quality degradation Signals |
| Description | Monitors a Memory Block for staleness: entries not re-validated, citations gone dead, scores declining. Emits drift alerts. |

### `budget-lens`

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Observe |
| Scope | Agent, Space |
| Emits | Threshold alert Signals |
| Description | Watches budget consumption rate. Emits alerts at configurable thresholds (warn, throttle, halt). Consumed by BudgetReactor. |

### `trend-lens`

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Observe |
| Scope | Any other Lens |
| Emits | Slope, EMA, derivative Signals |
| Description | Meta-Lens: observes another Lens's output stream and computes statistical trends. Chains with any other Lens (e.g., TrendLens watching CostLens computes cost trajectory). |

### `anomaly-lens`

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Observe |
| Scope | Any other Lens |
| Emits | Statistical outlier alert Signals |
| Description | Meta-Lens: detects anomalies in another Lens's output using Z-score and IQR methods. Configurable sensitivity. Feeds SafetyReactor. |

### `usage-lens`

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Observe |
| Scope | Space, Marketplace |
| Emits | Install/run/fork count Signals |
| Description | Tracks usage metrics for published artifacts: installs, active runs, forks, error rates, cost averages. Powers marketplace creator analytics and trending algorithms. |

---

## 9. Connect Blocks (Connectors)

Blocks implementing the Connect protocol: `connect / query / execute / health / disconnect`. See [doc-12 (Connectivity)](12-CONNECTIVITY.md) for the full Connector model.

### `chain-rpc-connector`

Ethereum / EVM / Solana RPC connection.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Connect |
| Input | `QueryRequest` or `ExecuteRequest` |
| Output | `QueryResponse` or `ExecuteResponse` |
| Capabilities | `Chain { read: true, write: configurable }`, `Net` |
| Description | Connects to blockchain RPC endpoints. Supports read queries (balances, events, contract state) and write operations (transactions). Health check via `eth_blockNumber`. |

```toml
[[nodes]]
id = "chain"
block = "chain-rpc-connector@^1"
[nodes.params]
rpc_url = "https://mainnet.base.org"
network = "base"
read_only = true
```

### `mcp-connector`

Model Context Protocol server connection.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Connect |
| Input | MCP tool call requests |
| Output | MCP tool call responses |
| Capabilities | (depends on the MCP server) |
| Description | Wraps an MCP server as a Connector. Auto-discovered from `agent.mcp_config` in `roko.toml`. Exposes the server's tools as queryable operations. |

```toml
[[nodes]]
id = "code-intel"
block = "mcp-connector@^1"
[nodes.params]
server = "roko-mcp-code"
config_path = ".roko/mcp-config.json"
```

### `database-connector`

SQL database connection (Postgres, SQLite).

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Connect |
| Input | SQL query or command |
| Output | Result rows or affected count |
| Capabilities | `Net` (for remote), `FsRead` (for SQLite) |
| Description | Connection-pooled database access. Read queries via `query()`, mutations via `execute()`. Health check via `SELECT 1`. |

```toml
[[nodes]]
id = "db"
block = "database-connector@^1"
[nodes.params]
driver = "postgres"
connection_string_secret = "database_url"
pool_size = 5
```

### `webhook-connector`

Outbound HTTP webhook delivery.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Connect |
| Input | Webhook payload |
| Output | HTTP response |
| Capabilities | `Net { domains: configurable }` |
| Description | Delivers Signals to external HTTP endpoints. Supports retry with exponential backoff, HMAC signature, configurable headers. |

```toml
[[nodes]]
id = "notify"
block = "webhook-connector@^1"
[nodes.params]
url = "https://hooks.slack.com/services/..."
method = "POST"
retry_count = 3
```

### `api-connector`

Generic REST / gRPC API client.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Connect |
| Input | Request (method, path, headers, body) |
| Output | Response (status, headers, body) |
| Capabilities | `Net { domains: configurable }` |
| Description | General-purpose API client for external services. Supports authentication strategies (Bearer, API key, OAuth2). Rate limiting and circuit breaker built in. |

```toml
[[nodes]]
id = "external-api"
block = "api-connector@^1"
[nodes.params]
base_url = "https://api.example.com/v1"
auth = { type = "bearer", secret = "api_token" }
rate_limit_rps = 10
```

---

## 10. Trigger Blocks

Blocks implementing the Trigger protocol: `arm / disarm / poll` for events that fire Graphs. See [doc-06 (Trigger System)](06-TRIGGER-SYSTEM.md) for the full Trigger model.

### `cron-trigger`

Schedule-based Graph firing.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Trigger |
| Capabilities | (none) |
| Description | Fires when a cron expression matches. Standard 5-field syntax plus `@hourly`, `@daily`, `@weekly` shortcuts. |

```toml
[[triggers]]
block = "cron-trigger@^1"
[triggers.params]
schedule = "0 */6 * * *"
graph = "knowledge-consolidation"
```

### `webhook-trigger`

Inbound HTTP webhook listener.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Trigger |
| Capabilities | `Net` |
| Description | Registers an HTTP endpoint. Fires when a request arrives. Supports payload filtering, HMAC verification, and path-based routing. Used for GitHub webhooks, Slack events, etc. |

```toml
[[triggers]]
block = "webhook-trigger@^1"
[triggers.params]
path = "/hooks/github"
secret_key = "github_webhook_secret"
filter = "event == 'pull_request' AND action == 'opened'"
graph = "code-review"
```

### `file-watch-trigger`

Filesystem change detection.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Trigger |
| Capabilities | `FsRead` |
| Description | Watches files and directories for changes. Debounced with configurable delay. Supports glob patterns for inclusion/exclusion. Uses `notify::RecommendedWatcher`. |

```toml
[[triggers]]
block = "file-watch-trigger@^1"
[triggers.params]
path = "src/"
patterns = ["**/*.rs"]
debounce_ms = 500
graph = "compile-and-test"
```

### `bus-trigger`

Signal Bus topic listener.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Trigger |
| Capabilities | (none) |
| Description | Fires when a Signal of a specific Kind appears on a Bus topic. The primary internal event mechanism for chaining Graphs. |

```toml
[[triggers]]
block = "bus-trigger@^1"
[triggers.params]
topic = "prd.published"
kind = "Prd"
graph = "plan-generate"
```

### `chain-event-trigger`

Smart contract event listener.

| Field | Value |
|---|---|
| Version | 0.1.0 |
| Protocols | Trigger |
| Capabilities | `Chain { read: true }`, `Net` |
| Description | Listens for on-chain events (EVM log topics). Fires when matching events appear. Used for chain-reactive agent workflows. Phase 2+. |

```toml
[[triggers]]
block = "chain-event-trigger@^0.1"
[triggers.params]
rpc_url = "https://mainnet.base.org"
contract = "0x..."
event_signature = "Transfer(address,address,uint256)"
graph = "chain-event-handler"
```

### `manual-trigger`

User-initiated Graph firing.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Trigger |
| Capabilities | (none) |
| Description | Fires when a user explicitly invokes via CLI (`roko run`), TUI, or dashboard. The most common Trigger for interactive use. Every `roko run <graph>` creates an implicit ManualTrigger. |

### `signal-pattern-trigger`

HDC-similarity pattern matching on Signal stream.

| Field | Value |
|---|---|
| Version | 1.0.0 |
| Protocols | Trigger |
| Capabilities | (none) |
| Description | Fires when a Signal with HDC fingerprint similar to a reference pattern appears on the Bus above a configurable threshold. Enables content-addressable event matching. |

```toml
[[triggers]]
block = "signal-pattern-trigger@^1"
[triggers.params]
reference_fingerprint = "hdc:abc123..."
threshold = 0.85
graph = "anomaly-investigate"
```

---

## 11. Domain-Specific Blocks

Beyond protocol Blocks, Roko ships domain Blocks that implement the base `Block` trait and compose into common Graphs.

### 11.1 Authoring

| Block | Description | Capabilities |
|---|---|---|
| `fs-walk` | Walk directory, emit file list | `FsRead` |
| `markdown-segment` | Split markdown by headings | (none) |
| `markdown-classify` | Classify segments by intent (context, task, spec, reference) | `Llm` |
| `doc-cluster` | Cluster classified segments into logical groups | `Llm` |
| `prd-synthesize` | Generate a PRD from clustered segments | `Llm` |
| `prd-audit` | Audit a PRD for contradictions, vague language, missing criteria | `Llm` |
| `prd-plan` | Generate a tasks.toml plan from a published PRD | `Llm` |
| `plan-validate` | Static analysis on tasks.toml: cycles, missing deps, orphans | (none) |
| `artifact-persist` | Persist produced artifacts to Store | `FsWrite` |

### 11.2 Research

| Block | Description | Capabilities |
|---|---|---|
| `web-search` | Web search via Perplexity or configured provider | `Net`, `Llm` |
| `academic-search` | arXiv + Semantic Scholar paper search | `Net` |
| `citation-check` | Verify cited claims against sources | `Net`, `Llm` |
| `fact-check` | Check factual claims against a corpus | `Llm` |
| `knowledge-ingest` | Import Signals into a Memory Block | `FsWrite` |
| `knowledge-link` | Discover HDC-similar cross-domain bridges | (none) |

### 11.3 Execution

| Block | Description | Capabilities |
|---|---|---|
| `agent-dispatch` | Dispatch a task to an Agent (Claude CLI, API, Codex, etc.) | `Llm`, `Shell` |
| `test-run` | Run a test suite (`cargo test`, `npm test`, `pytest`) | `Shell`, `FsRead` |
| `build` | Compile/bundle (`cargo build`, `vite build`) | `Shell`, `FsRead` |
| `script-run` | Execute a script with capability gating | `Shell` |
| `refactor-apply` | Apply a refactor pattern across files | `FsRead`, `FsWrite`, `Llm` |

### 11.4 Deploy

| Block | Description | Capabilities |
|---|---|---|
| `deploy-railway` | Deploy to Railway | `Net`, `Shell`, `Secrets` |
| `deploy-fly` | Deploy to Fly.io | `Net`, `Shell`, `Secrets` |
| `deploy-vercel` | Deploy to Vercel | `Net`, `Shell`, `Secrets` |
| `deploy-shell` | Custom shell-script deploy | `Shell`, `Secrets` |
| `smoke-test` | Post-deploy endpoint + page verification | `Net` |
| `rollback` | Revert a failed deployment | `Net`, `Shell`, `Secrets` |

### 11.5 Operations

| Block | Description | Capabilities |
|---|---|---|
| `backup` | Snapshot `.roko/` to configured remote | `FsRead`, `Net`, `Secrets` |
| `restore` | Restore from backup snapshot | `FsWrite`, `Net`, `Secrets` |
| `gc` | Garbage collect old runs/artifacts/episodes | `FsWrite` |
| `cost-report` | Generate per-Graph/Agent/model cost summary | (none) |
| `dependency-update` | Bump deps and run gates | `Shell`, `FsWrite` |
| `dependency-audit` | Check deps for CVEs and abandonment | `Shell` |

### 11.6 Communication

| Block | Description | Capabilities |
|---|---|---|
| `slack-notify` | Post message to Slack channel | `Net`, `Secrets` |
| `github-comment` | Post comment on PR or issue | `Net`, `Secrets` |
| `email-send` | Send email via configured provider | `Net`, `Secrets` |
| `discord-notify` | Post message to Discord channel | `Net`, `Secrets` |

### 11.7 Code Intelligence

| Block | Description | Capabilities |
|---|---|---|
| `index-build` | Build the code-intel index | `FsRead`, `Shell` |
| `code-search` | Semantic + symbolic code search | `FsRead` |
| `type-check` | Language-specific type checker | `Shell`, `FsRead` |
| `symbol-graph` | Build symbol-relationship graph | `FsRead` |
| `impact-analysis` | Given a diff, report downstream impacts | `FsRead` |

---

## 12. Synergy Patterns

The catalog is designed for emergent composition. Blocks combine via Graphs to create synergistic pipelines:

| Pattern | Blocks | Trigger | Effect |
|---|---|---|---|
| **Doc-to-Plan** | `fs-walk` + `markdown-classify` + `doc-cluster` + `prd-synthesize` + `prd-plan` | `file-watch-trigger` on docs/ | New docs auto-produce plans |
| **PR Review** | `webhook-trigger` (GitHub) + `agent-dispatch` (code-review role) + `github-comment` | `webhook-trigger` | Every PR gets automated review |
| **Code-to-Docs** | `file-watch-trigger` on src/ + `impact-analysis` + `agent-dispatch` (doc-writer) | `file-watch-trigger` | Docs stay in sync with code |
| **Local CI** | `file-watch-trigger` on src/ + `compile-gate` + `test-gate` + `clippy-gate` | `file-watch-trigger` | Continuous local verification |
| **Ship Pipeline** | `build` + `deploy-railway` + `smoke-test` + `slack-notify` | `manual-trigger` | One-command ship-with-confidence |
| **Idea Pipeline** | `web-search` + `prd-synthesize` + `prd-plan` + `agent-dispatch` | `manual-trigger` | Idea to shipped code |
| **Knowledge GC** | `gc` + `knowledge-link` | `cron-trigger` weekly | Pruning + new connections weekly |
| **Cost Alert** | `cost-lens` + `trend-lens` + `budget-reactor` + `escalation-reactor` | `bus-trigger` on CostReport | Auto-triage on cost spikes |
| **Visual Quality** | `file-watch-trigger` on dist/ + `llm-judge-gate` (visual criteria) + `slack-notify` | `file-watch-trigger` | Continuous UI quality monitoring |
| **Learning Loop** | `cascade-router` + `efficiency-lens` + `trend-lens` | Implicit (every run) | System improves model selection per Block |

These patterns are not hardcoded pipelines. They emerge from composing individual Blocks via Graphs and Triggers. Users discover useful patterns and publish them as Graphs in the marketplace ([doc-15](15-MARKETPLACE-AND-SHARING.md)).

---

## 13. Implementation Tiers

| Tier | When | Blocks |
|---|---|---|
| **Tier 0** (kernel) | First | All Verify Blocks (gates), `prompt-composer`, `cascade-router`, `agent-dispatch`, `file-store`, `prd-synthesize`, `prd-plan` |
| **Tier 1** (authoring) | First | `fs-walk`, `markdown-classify`, `doc-cluster`, `prd-audit`, `citation-check`, `artifact-persist`, `knowledge-ingest` |
| **Tier 2** (deploy + verify) | Soon | All Deploy Blocks, `smoke-test`, `llm-judge-gate`, `consensus-gate`, `webhook-trigger`, `file-watch-trigger` |
| **Tier 3** (operations) | Soon | `backup`, `gc`, `cost-report`, `dependency-update`, all Communication Blocks, `cron-trigger` |
| **Tier 4** (knowledge + observe) | Mid | All Observe Blocks (Lenses), all React Blocks, `knowledge-link`, `hdc-scorer`, `signal-pattern-trigger` |
| **Tier 5** (chain + advanced) | Late | `chain-store`, `chain-rpc-connector`, `chain-event-trigger`, `vcg-composer` |

---

## 14. Acceptance Criteria

| Criterion | Verification |
|---|---|
| Every Tier 0 Block ships with typed I/O, capabilities, and a TOML usage example | `roko block list` returns the full Tier 0 set |
| Each Block validates clean when composed in a Graph | `roko graph validate` passes for all builtin Graphs |
| Capability intersection enforced at runtime per Block | Block requesting ungranteed capability fails closed |
| Synergy patterns from section 12 work via Graph + Trigger composition | Multi-step integration test |
| Every Block declares cost estimation | `roko block show <name>` displays estimates |
| Every catalog entry's TOML lives in `<roko-install>/builtin/blocks/` | Filesystem invariant check |
