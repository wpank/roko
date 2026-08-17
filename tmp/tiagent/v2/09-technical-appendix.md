# Technical Appendix

This appendix provides the Rust-level detail behind the tiagent grant proposal.
It covers workspace layout, core type definitions, the universal execution loop,
configuration schema, design patterns, CLI surface, Celestia API usage, and
external dependencies. It is intended for technical reviewers performing due
diligence on the implementation plan.

---

## 1. Crate Structure

tiagent is a Cargo workspace with 14 crates: 12 library crates and 2 standalone
MCP tool server binaries.

```
tiagent/
  Cargo.toml                         # workspace root
  crates/
    tiagent-core/                    # kernel: Signal, 6 verb traits, errors, config
    tiagent-agent/                   # LLM dispatch, tool loop, backends, safety
    tiagent-celestia/                # [optional] Celestia DA integration
    tiagent-gate/                    # gate pipeline, 7 rungs, adaptive thresholds
    tiagent-compose/                 # prompt assembly, templates, context bidding
    tiagent-orchestrator/            # plan DAG, task execution, parallel dispatch
    tiagent-learn/                   # episodes, cascade router, efficiency, playbooks
    tiagent-store/                   # local substrate (JSONL/SQLite), GC
    tiagent-tools/                   # built-in tools, MCP client, tool registry
    tiagent-serve/                   # HTTP API (axum), SSE, WebSocket
    tiagent-runtime/                 # process supervision, event bus, cancellation
    tiagent-cli/                     # CLI binary (clap), all subcommands
  tools/
    tiagent-mcp-celestia/            # MCP server: Celestia developer tools
    tiagent-mcp-code/                # MCP server: code intelligence tools
```

| Crate | Purpose | Depends on | Optional |
|---|---|---|---|
| `tiagent-core` | Signal type, 6 verb traits, errors, config | -- | No |
| `tiagent-agent` | LLM backends (Claude, OpenAI, Ollama, CLI), tool loop, safety | core | No |
| `tiagent-celestia` | Celestia DA substrate, namespace manager, fee estimator, light node | core | **Yes** |
| `tiagent-gate` | 7-rung gate pipeline, adaptive EMA thresholds | core | No |
| `tiagent-compose` | Prompt assembly, role templates, context bidding | core | No |
| `tiagent-orchestrator` | Plan DAG, parallel executor, merge queue | core | No |
| `tiagent-learn` | Episode logger, cascade router, playbooks, efficiency tracking | core | No |
| `tiagent-store` | FileSubstrate (JSONL), DbSubstrate (SQLite), garbage collection | core | No |
| `tiagent-tools` | Built-in tools (file, shell, search, git), MCP client, tool registry | core | No |
| `tiagent-serve` | HTTP control plane (axum), SSE streaming, WebSocket | core, agent, gate, learn | No |
| `tiagent-runtime` | ProcessSupervisor, tokio event bus, cancellation tokens | core | No |
| `tiagent-cli` | CLI binary: all subcommands, ratatui TUI dashboard | all domain crates | No |
| `tiagent-mcp-celestia` | MCP server exposing Celestia tools over stdio | core, celestia | **Yes** |
| `tiagent-mcp-code` | MCP server exposing code intelligence tools over stdio | core | No |

Dependencies flow strictly downward: **kernel -> domain -> application -> binary**.
Circular dependencies are forbidden. `tiagent-celestia` and `tiagent-mcp-celestia`
are the only optional crates, enabled via `--features celestia`.

---

## 2. Key Type Definitions

### Signal

The universal data atom. Every piece of data in tiagent -- prompts, responses,
tool outputs, gate verdicts, episodes -- is a Signal.

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Signal {
    pub hash: Blake3Hash,
    pub kind: SignalKind,
    pub parents: Vec<Blake3Hash>,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
```

### SignalKind

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalKind {
    Prompt,
    Response,
    ToolCall,
    ToolResult,
    GateVerdict,
    EpisodeRecord,
    PlanTask,
    CodePatch,
    Effect,
    Commitment,
}
```

### The 6 Verb Traits

```rust
#[async_trait]
pub trait Substrate: Send + Sync {
    async fn read(&self, hash: &Blake3Hash) -> Result<Signal>;
    async fn write(&self, signal: Signal) -> Result<Blake3Hash>;
    async fn query(&self, filter: &SignalFilter) -> Result<Vec<Signal>>;
}

#[async_trait]
pub trait Scorer: Send + Sync {
    async fn score(&self, signals: &[Signal], ctx: &TaskContext) -> Result<Vec<ScoredSignal>>;
}

#[async_trait]
pub trait Gate: Send + Sync {
    async fn check(&self, signal: &Signal, context: &Signal) -> Result<GateResult>;
}

#[async_trait]
pub trait Router: Send + Sync {
    async fn route(&self, signal: &Signal) -> Result<Route>;
}

#[async_trait]
pub trait Composer: Send + Sync {
    async fn compose(&self, scored: &[ScoredSignal], route: &Route) -> Result<AssembledPrompt>;
}

#[async_trait]
pub trait Policy: Send + Sync {
    async fn authorize(
        &self,
        tool: &ToolManifest,
        params: &serde_json::Value,
        ctx: &AgentContext,
    ) -> Result<PolicyDecision>;
}
```

### Result and Decision Types

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GateResult {
    pub passed: bool,
    pub rung_verdicts: Vec<RungVerdict>,
    pub summary: String,
    pub duration_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Route {
    pub model_id: String,
    pub backend: BackendKind,
    pub tools: Vec<String>,
    pub token_budget: usize,
    pub tier: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PolicyDecision {
    Approve(Vec<Condition>),
    Deny(String),
    RequireHuman,
}
```

---

## 3. Universal Loop Pseudocode

Every agent operation -- chat messages, plan tasks, research queries -- follows
the same eight-stage loop. Each stage maps to a verb trait method call.

```rust
async fn execute(input: Signal, ctx: &Context) -> Result<Signal> {
    // 1. Query: fetch relevant signals from storage
    let related = ctx.substrate.query(&input.as_filter()).await?;

    // 2. Score: rank by relevance to current task
    let scored = ctx.scorer.score(&related, &ctx.task).await?;

    // 3. Route: pick model, backend, and token budget
    let route = ctx.router.route(&input).await?;

    // 4. Compose: assemble prompt from scored context
    let prompt = ctx.composer.compose(&scored, &route).await?;

    // 5. Act: send to LLM, execute tool calls in a loop
    let response = ctx.backend.complete(&prompt, &route).await?;

    // 6. Verify: run gate pipeline on the output
    let verified = ctx.gate.check(&response, &input).await?;

    // 7. Persist: write result signal to storage
    ctx.substrate.write(&verified).await?;

    // 8. React: decide next action (continue, escalate, stop)
    ctx.policy.react(&verified).await?;

    Ok(verified)
}
```

Stage mapping:

| Stage | Trait | What happens |
|---|---|---|
| query | Substrate | Fetch relevant Signals from storage |
| score | Scorer | Rank by relevance to the current task |
| route | Router | Pick model/backend/budget for this task |
| compose | Composer | Assemble prompt from scored context |
| act | (agent) | Send prompt to LLM, collect response + tool calls |
| verify | Gate | Run 7-rung gate pipeline on the output |
| persist | Substrate | Write result Signals to storage |
| react | Policy | Decide next action (continue, escalate, stop) |

---

## 4. Configuration Schema

Full `tiagent.toml` example with all sections:

```toml
# ── Agent identity ──────────────────────────────────────────────
[agent]
name = "my-coding-agent"
default_model = "claude-sonnet-4-20250514"
workspace_root = "."
data_dir = ".tiagent"

# ── Model providers ─────────────────────────────────────────────
[[models.providers]]
name = "anthropic"
kind = "claude-api"
api_key_env = "ANTHROPIC_API_KEY"
models = ["claude-sonnet-4-20250514", "claude-opus-4-20250514"]

[[models.providers]]
name = "openai"
kind = "openai-compat"
api_key_env = "OPENAI_API_KEY"
base_url = "https://api.openai.com/v1"
models = ["gpt-4o", "gpt-4o-mini"]

[[models.providers]]
name = "local"
kind = "ollama"
base_url = "http://localhost:11434"
models = ["llama3.1:70b", "codestral"]

# ── MCP tool servers ────────────────────────────────────────────
[tools.mcp.code-intel]
command = "tiagent-mcp-code"
args = ["--workspace", "."]
transport = "stdio"

[tools.mcp.github]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]
transport = "stdio"
env = { GITHUB_TOKEN = "env:GITHUB_TOKEN" }

# ── Gate pipeline ────────────────────────────────────────────────
[gates]
rungs = ["parse", "compile", "test", "lint", "diff", "semantic"]
# Adaptive EMA thresholds per rung
[gates.thresholds]
compile = 0.95
test = 0.90
lint = 0.85
semantic = 0.70
[gates.adaptive]
alpha = 0.1         # EMA smoothing factor
floor = 0.5         # minimum threshold (prevents drift to zero)

# ── Learning subsystem ───────────────────────────────────────────
[learning]
episode_log = ".tiagent/episodes.jsonl"
efficiency_log = ".tiagent/learn/efficiency.jsonl"

[learning.cascade_router]
persistence_path = ".tiagent/learn/cascade-router.json"
tiers = ["gpt-4o-mini", "claude-sonnet-4-20250514", "claude-opus-4-20250514"]
confidence_threshold = 0.7

[learning.playbooks]
store_path = ".tiagent/learn/playbooks.json"
auto_extract = true    # extract reusable patterns from successful episodes

# ── Celestia integration (optional) ──────────────────────────────
[celestia]
enabled = true
node_url = "http://localhost:26658"
namespace_prefix = "tiagent/v1"
auth_token_env = "CELESTIA_AUTH_TOKEN"
network = "mocha"               # mocha | arabica | mainnet
batch_size = 10                 # signals per commitment blob
submit_interval_secs = 60      # max wait before submitting a batch
```

---

## 5. Design Patterns Catalog

tiagent uses 12 recurring design patterns. 11 work in standalone mode;
1 requires Celestia.

| # | Pattern | Summary | Celestia? |
|---|---|---|---|
| 1 | **Signal DAG** | Every artifact is a content-addressed, immutable Signal linked to its parents by hash. The full history of any output forms a DAG that can be walked backwards for provenance. | Standalone |
| 2 | **Verb Traits** | Six async traits (Substrate, Scorer, Gate, Router, Composer, Policy) define all operations on data. Implementations are swappable: tests use in-memory mocks, production uses filesystem or database backends. | Standalone |
| 3 | **Universal Loop** | Every agent operation follows 8 stages: query, score, route, compose, act, verify, persist, react. Each stage maps to a verb trait. Consistent error handling and logging across all task types. | Standalone |
| 4 | **Cascade Router** | Starts tasks at the cheapest model tier and escalates on failure. Learns per-category success rates via EMA and routes directly to the correct tier after enough data accumulates. Weights persist across sessions. | Standalone |
| 5 | **Gate Pipeline** | A sequence of 7 validation rungs (parse, compile, test, lint, diff, semantic, human) with adaptive EMA thresholds. Short-circuits on blocking failures. Gate rejections trigger replanning with error context. | Standalone |
| 6 | **Snapshot-Resume** | The executor serializes state to JSON after each task. On crash, `--resume <path>` loads the snapshot and skips completed tasks. Schema-versioned for forward compatibility. | Standalone |
| 7 | **Episode Logging** | Every model interaction is recorded as a structured JSONL entry: prompts, responses, tool calls, token counts, cost, latency, gate verdicts. Feeds the cascade router, dashboard, and TraceCommons integration. | Standalone |
| 8 | **Context Bidding** | Multiple context sources (task, code, research, knowledge, playbooks) submit priority-weighted bids for limited token budget. The Composer allocates greedily by priority and assembles the prompt from winning bids. | Standalone |
| 9 | **Dual-Layer Storage** | Hot layer (local filesystem) for fast reads/writes. Warm layer (Celestia DA) stores commitment hashes for verifiability. Cold layer (Arweave, optional) for permanent archival. Full data stays local; only commitments go on-chain. | **Requires Celestia** |
| 10 | **Effect Pipeline** | Side effects pass through 4 stages: Intent, Claim (policy approval), Attempt (execution), Outcome (result capture). Each stage is a Signal in the DAG for full auditability. | Standalone |
| 11 | **Push-Based Dashboard** | Runtime state changes emit events through `tokio::sync::watch` channels. TUI, SSE, and WebSocket consumers subscribe for instant updates without polling. | Standalone |
| 12 | **Tool Safety Tiers** | Tools are classified as Safe (auto-approve), Moderate (log + auto-approve), or Dangerous (require human confirmation). Unknown tools default to Dangerous (fail-closed). The Policy trait enforces tiers before every invocation. | Standalone |

---

## 6. CLI Commands Reference

### Core workflow

| Command | Description |
|---|---|
| `tiagent init` | Create `.tiagent/` directory and `tiagent.toml` config |
| `tiagent run "<prompt>"` | Single prompt through the universal loop |
| `tiagent status` | Report signal counts, episode stats, learning state |
| `tiagent doctor` | Diagnose workspace: config, providers, tools, gates |

### Planning and execution

| Command | Description |
|---|---|
| `tiagent plan list` | List all plans in the workspace |
| `tiagent plan show <id>` | Display plan details and task DAG |
| `tiagent plan run <dir>` | Execute a plan (the main orchestration loop) |
| `tiagent plan run <dir> --resume <path>` | Resume from a snapshot after interruption |
| `tiagent plan validate <dir>` | Lint tasks.toml without executing |
| `tiagent plan generate "<prompt>"` | Generate a plan from a natural language prompt |

### PRD lifecycle

| Command | Description |
|---|---|
| `tiagent prd draft new "<title>"` | Create a new PRD draft |
| `tiagent prd draft edit <slug>` | Edit an existing draft |
| `tiagent prd draft promote <slug>` | Promote draft to published |
| `tiagent prd draft list` | List all drafts |
| `tiagent prd plan <slug>` | Generate implementation plan from a published PRD |
| `tiagent prd list` | List all PRDs with status |

### Configuration

| Command | Description |
|---|---|
| `tiagent config show` | Display current configuration |
| `tiagent config edit` | Open config in `$EDITOR` |
| `tiagent config set <key> <value>` | Set a configuration value |
| `tiagent config validate` | Validate config against schema |
| `tiagent config providers list` | List configured LLM providers |
| `tiagent config providers health` | Check provider connectivity |

### Learning and feedback

| Command | Description |
|---|---|
| `tiagent learn show` | Display learning state (router weights, efficiency) |
| `tiagent learn show episodes` | List recent episodes with cost and latency |
| `tiagent learn show router` | Display cascade router weights per category |
| `tiagent learn tune gates` | Manually adjust gate thresholds |
| `tiagent learn tune routing` | Reset or adjust cascade router weights |

### Dashboard

| Command | Description |
|---|---|
| `tiagent dashboard` | Interactive ratatui TUI with real-time plan progress |

---

## 7. Celestia API Surface

tiagent uses three categories of Celestia RPC calls. All calls go through the
`celestia-rpc` client crate and are isolated in `tiagent-celestia`.

### Blob operations

| RPC Method | Usage |
|---|---|
| `blob.Submit` | Submit commitment blobs (batched signal hashes + metadata) to a namespace |
| `blob.Get` | Retrieve a specific blob by height and namespace |
| `blob.GetAll` | Retrieve all blobs in a namespace at a given height |
| `blob.GetProof` | Obtain an NMT inclusion proof for a blob (used for verification) |

### State operations

| RPC Method | Usage |
|---|---|
| `state.SubmitPayForBlobs` | Submit a `MsgPayForBlobs` transaction for blob inclusion |
| `state.Balance` | Query account balance for fee estimation |

### Header operations

| RPC Method | Usage |
|---|---|
| `header.GetByHeight` | Fetch block header by height (used for timestamp correlation) |
| `header.NetworkHead` | Get the latest network head (used for sync status checks) |

### Namespace schema

tiagent organizes Celestia blobs under structured namespaces:

```
tiagent/v1/commitments/{agent-name}     # signal commitment batches
tiagent/v1/episodes/{agent-name}        # episode commitment hashes
tiagent/v1/gates/{agent-name}           # gate verdict commitments
tiagent/v1/plans/{plan-id}              # plan execution commitments
```

Each namespace uses Celestia's v0 namespace format (29 bytes). The
`NamespaceManager` in `tiagent-celestia` handles creation and lookup.

### Blob encoding

Commitment blobs are encoded as:

```rust
#[derive(Serialize, Deserialize)]
pub struct CommitmentBlob {
    pub version: u8,                        // schema version (1)
    pub agent_id: String,
    pub batch_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub signal_hashes: Vec<Blake3Hash>,      // the committed hashes
    pub merkle_root: Blake3Hash,             // root of hash tree
    pub metadata: BTreeMap<String, String>,
}
```

The blob payload is CBOR-encoded for compactness (30-40% smaller than JSON for
typical batches). The `BlobBuilder` handles serialization, compression, and
commitment generation.

---

## 8. Dependencies

### Core Rust crates

| Crate | Version | Purpose |
|---|---|---|
| `tokio` | 1.x | Async runtime (multi-threaded scheduler) |
| `axum` | 0.8.x | HTTP framework for `tiagent-serve` |
| `clap` | 4.x | CLI argument parsing with derive macros |
| `serde` | 1.x | Serialization/deserialization framework |
| `serde_json` | 1.x | JSON encoding/decoding |
| `tracing` | 0.1.x | Structured, async-aware logging |
| `tracing-subscriber` | 0.3.x | Log output formatters and filters |
| `reqwest` | 0.12.x | HTTP client for LLM API calls |
| `blake3` | 1.x | Content-addressing hash function |
| `thiserror` | 2.x | Derive macro for error enums |
| `async-trait` | 0.1.x | Async methods in traits (until Rust stabilizes AFIT) |
| `chrono` | 0.4.x | Date/time types with UTC support |
| `uuid` | 1.x | UUID generation for episode and batch IDs |
| `toml` | 0.8.x | Config file parsing |
| `ratatui` | 0.29.x | Terminal UI framework for dashboard |

### Celestia crates (optional, behind `--features celestia`)

| Crate | Version | Purpose |
|---|---|---|
| `celestia-types` | 0.10.x | Blob, Namespace, Commitment types |
| `celestia-rpc` | 0.10.x | JSON-RPC client for Celestia node |
| `lumina-node` | 0.8.x | Embeddable Celestia light node (behind `light-node` feature) |
| `nmt-rs` | 0.2.x | Namespaced Merkle Tree proof construction and verification |
| `ciborium` | 0.2.x | CBOR encoding for blob payloads |

### Feature flag matrix

| Feature flag | Crates enabled | Default |
|---|---|---|
| `claude-api` | `reqwest` in `tiagent-agent` | Yes |
| `openai` | `reqwest` in `tiagent-agent` | Yes |
| `ollama` | `reqwest` in `tiagent-agent` | No |
| `cli-backends` | `tokio::process` in `tiagent-agent` | No |
| `celestia` | `tiagent-celestia`, `celestia-rpc`, `celestia-types` | No |
| `light-node` | `lumina-node` in `tiagent-celestia` | No |
| `serve` | `axum`, `tower` in `tiagent-serve` | No |
| `tui` | `ratatui`, `crossterm` in `tiagent-cli` | No |

A minimal build (`cargo build -p tiagent-cli`) compiles the core harness with
Claude and OpenAI backends. No Celestia code, no HTTP server, no TUI. Total
dependency count for the minimal build is approximately 180 crates; a full build
with all features is approximately 350.

---

## Build and Test

```bash
# Minimal build (no Celestia, no TUI, no HTTP server)
cargo build -p tiagent-cli

# Full build with all features
cargo build --workspace --all-features

# Run tests (no external services required)
cargo test --workspace

# Run tests including Celestia integration (requires local node)
cargo test --workspace --features celestia,integration

# Lint
cargo clippy --workspace --no-deps -- -D warnings

# Format
cargo +nightly fmt --all
```

CI runs all three checks (test, clippy, fmt) on every pull request.
Integration tests that require a Celestia node are gated behind
`#[cfg(feature = "integration")]` and run in a separate CI job against
a Mocha testnet light node.
