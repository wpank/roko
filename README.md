# Roko Workspace

**Roko is a modular agent orchestration stack built on one noun + six verbs + one async extension.** Ground-up redesign of Mori; every capability — LLM coding agents, chain-native agents, orchestration, learning, demo environments — folds into the same 7 primitives.

Agents aren't a magic class. They're compositions: a `Substrate` stores signals, a `Scorer` rates them, a `Gate` verifies them, a `Router` picks, a `Composer` assembles, a `Policy` emits, and an `Agent` executes. Swap any impl, keep the rest. The "universal loop" is what every agent runs.

```text
┌──────────────────────────────────────────────────────────────────────┐
│  query Substrate → Scorer → Router/Composer → Gate → write → Policy  │
│       ↑                                                        │     │
│       └────────────────── Agent emits new Signals ─────────────┘     │
└──────────────────────────────────────────────────────────────────────┘
```

A **coding agent** and a **chain-native agent** are the same `RokoAgent` struct with different trait impls registered. Adding chain support is additive config, not a rewrite.

---

## Table of contents

1. [Architecture](#architecture)
2. [Crates by layer](#crates-by-layer)
   - [Kernel](#kernel) · [Standard impls](#standard-impls) · [Gates](#gates) · [Persistence](#persistence) · [Composition](#composition) · [Agent execution](#agent-execution) · [Orchestration](#orchestration) · [Learning](#learning) · [Chain](#chain) · [CLI](#cli) · [Demo](#demo) · [Support primitives](#support-primitives)
3. [Apps](#apps)
4. [Running things](#running-things)
5. [Tests](#tests)
6. [Extension recipes](#extension-recipes)
7. [What to build with this](#what-to-build-with-this)
8. [File map](#file-map)
9. [Design principles](#design-principles)

---

## Architecture

### One noun: `Signal`

Content-addressed (BLAKE3), decaying, scored, traced, composable unit of work. Every output from every primitive is a `Signal`. Signals form a DAG through parent pointers, letting any agent replay or explain itself.

```rust
struct Signal {
    hash: ContentHash,        // BLAKE3 of canonical bytes
    kind: SignalKind,         // Prompt | AgentOutput | GateVerdict | Episode | ...
    parents: Vec<ContentHash>,// lineage
    score: Option<Score>,     // 0..1, from Scorers
    decay: DecayState,        // half-life
    ts: Timestamp,
    payload: Bytes,           // opaque per kind
}
```

### Six verbs (traits in `roko-core`)

| Trait | Purpose | Example impls |
|---|---|---|
| `Substrate` | Store + query signals (async I/O) | `MemorySubstrate`, `FileSubstrate`, `ChainSubstrate` |
| `Scorer` | Rate a signal (sync, pure) | `NoveltyScorer`, `RecencyScorer`, `PriorityScorer` |
| `Gate` | Verify against ground truth (async I/O) | `CompileGate`, `TestGate`, `ShellGate`, `WalletGate` |
| `Router` | Pick one from many (sync) | `TopKRouter`, `ThompsonBandit`, `FirstRouter` |
| `Composer` | Combine under a budget (sync) | `PromptComposer` (U-shape, priority drop) |
| `Policy` | Emit new signal from stream state (sync) | `EpisodePolicy`, `RetryPolicy` |

### One async extension: `Agent`

```rust
#[async_trait]
pub trait Agent: Send + Sync {
    async fn run(&self, prompt: &Prompt) -> AgentOutput;
    fn name(&self) -> &str;
}
```

Agents have side effects (LLM calls, subprocess spawning, chain txs). Everything else is pure or at most file I/O.

### The universal loop

Every Roko-shaped system runs this:

```text
1. Substrate.query() → relevant signals
2. Scorers rate them
3. Router picks top-K (or Composer packs them under a budget)
4. Agent consumes the prompt → AgentOutput
5. Gates verify the output → Verdicts
6. Substrate writes back
7. Policies fire on accumulated signals → new Signals (Episodes, retries, …)
```

Every rung is independently testable; stopping at any rung gives a working system.

---

## Crates by layer

**16 workspace members**. Status indicators: `[shipping]` production-ready, `[scaffold]` trait complete, minimal impls, `[extension]` opt-in, layered on top.

### Kernel

#### `crates/roko-core` · **376 tests** · `[shipping]`
The contract. Defines `Signal`, the six traits, `ContentHash`, `Score`, `Decay`, `Verdict`, `Prompt`, `PromptSection`, error types, and the replay-able signal lineage model. Zero I/O — pure types + traits.

**Key exports:**
- `Signal`, `SignalKind`, `ContentHash`, `Score`, `DecayState`, `Verdict`
- `Substrate`, `Scorer`, `Gate`, `Router`, `Composer`, `Policy` traits
- `Prompt`, `PromptSection`, `Budget`, `PlacementPolicy`
- `RokoError`, `RokoResult`

When to reach for it: writing a new impl of any trait, or any library that shouldn't depend on concrete storage/compute.

#### `crates/bardo-primitives` · **16 tests** · `[support]`
Zero-dep compute primitives: 10,240-bit HDC (hyperdimensional) vectors, Hamming-similarity search, inference-tier routing signatures. Used by `roko-hdc` (pending) and `apps/mirage-rs`'s HDC precompile.

#### `crates/bardo-runtime` · **17 tests** · `[support]`
Typed event bus, process supervision hooks, cancellation primitives. Reused from bardo/.

### Standard impls

#### `crates/roko-std` · **96 tests** · `[shipping]`
In-memory defaults for every trait. Good for unit tests + small agent embeddings.

**Key exports:** `MemorySubstrate`, `NoOpScorer`, `NoOpRouter`, `NoOpComposer`, `NoOpPolicy`, `TopKRouter`, `PriorityRouter`, `RecencyScorer`, `PriorityScorer`, `CombinedScorer`.

### Gates

#### `crates/roko-gate` · **200 tests** · `[shipping]`
Concrete `Gate` implementations that verify agent output against ground truth.

**Key exports:**
- `ShellGate` — run an arbitrary shell command, success = exit 0
- `CompileGate` — `cargo check` / `npm build` / forge build; detects build system
- `TestGate` — `cargo test` / `npm test` with pattern selection
- `LintGate` — `cargo clippy` / `eslint`
- `SymbolGate` — grep for required symbols in diffs
- `VerifyChainGate` — composable chain of sub-gates with short-circuit
- `LLMJudge` — delegate to an Agent for subjective checks

Each wraps real subprocess I/O + structured error output; all return `Verdict`.

### Persistence

#### `crates/roko-fs` · **37 tests** · `[shipping]`
`FileSubstrate`: append-only JSONL persistence with in-memory index + compaction. Signals survive restart; lineage DAG is fully replayable.

Storage layout: `.roko/signals.jsonl` (append log), `.roko/index/` (optional secondary indexes).

### Composition

#### `crates/roko-compose` · **23 tests** · `[shipping]`
`PromptComposer`: assembles `PromptSection`s into a token-budgeted prompt using U-shape placement (intro + conclusion prioritized) + priority-ordered dropping when over budget.

**Key exports:** `PromptComposer`, `SectionScorer`, `Budget`, `TokenEstimator`.

### Agent execution

#### `crates/roko-agent` · **346 tests** · `[shipping]`
Concrete `Agent` backends.

**Key exports:**
- `MockAgent` — deterministic scripted replies for tests
- `ExecAgent` — spawn any CLI (`claude`, `ollama run <model>`, `mods`, `llm`, `gpt`) that reads prompt on stdin, writes response on stdout
- `ClaudeAgent` — claude-specific wrapper (file injection, JSON modes)
- Output cleaning: ANSI escape stripping, chain-of-thought stripping, markdown code-fence extraction

Supports LLM spawning with timeouts, retry budgets, env-var propagation, working-directory setup, and file-in-prompt injection.

### Orchestration

#### `crates/roko-orchestrator` · **158 tests** · `[shipping]`
Plan discovery, task DAG, worktree manager, parallel executor, capability tokens, taint tracking.

**Key exports:** `PlanDiscovery`, `TaskGraph`, `WorktreeManager`, `Executor`, `SafetyPolicy`, `CapabilityToken`.

Build larger systems (multi-agent workflows, gated merges) on top of this crate.

### Learning

#### `crates/roko-learn` · **101 tests** · `[shipping]`
Episode logs, playbook library, skill library, context cache, pattern discovery.

**Key exports:** `EpisodeLog`, `Playbook`, `SkillLibrary`, `ContextCache`, `PatternMiner`.

Plug into `Policy` impls to feed episode-informed decisions back into agents.

### Chain

#### `crates/roko-chain` · **52 tests** · `[shipping]`
`ChainClient` + `ChainWallet` traits with mock and real implementations.

**Key exports:**
- `ChainClient`, `ChainWallet` traits
- `MockChainClient`, `MockChainWallet`, `paired_mocks()`
- `WalletGate`, `TxSimGate` — Gate implementations that check balance/nonce + simulate txs pre-sign
- `TxRequest`, `Receipt`, `LogEntry`, `ChainError`, `TxHash`

**With `--features alloy-backend`** (3 live tests):
- `AlloyChainClient::http(rpc_url)` — real JSON-RPC provider
- `AlloyChainWallet::from_hex_key(rpc_url, key, chain_id)` — signer + provider wrapper

Use this crate to write chain-native agents whose reads/writes obey the same `ChainClient`/`ChainWallet` abstraction regardless of backend (mirage-rs, anvil, live L1/L2).

### CLI

#### `crates/roko-cli` · **38 tests** · `[shipping]`
The `roko` binary: wires the universal loop against a working directory. Reads config from `./roko.toml` + `~/.config/roko/config.toml`.

**Subcommands:**
- `init [path]` — bootstrap `.roko/` + default `roko.toml`
- `run "<prompt>"` — one pass through the universal loop
- `status` — signal counts, recent episode, gate pass/fail
- `replay <hash>` — walk the lineage DAG rooted at a signal
- `config init|show|set|path|edit` — manage layered config
- `serve --listen <addr>` — long-running HTTP mode

Out of the box it works with `cat` as the agent (echoes prompt back). Point `[agent]` at any LLM CLI for real behaviour.

### Demo

#### `crates/roko-demo` · **4 unit tests, 4 scenarios E2E** · `[shipping]`
Manifest-driven deploy + fixture + multi-agent orchestrator for the Roko chain stack. Ships with 4 pre-canned scenarios (job-board, consortium, defi-routing, flywheel) that deploy ERC contracts, register agents, and run scripted spines end-to-end against mirage-rs.

**Binary:** `roko-demo` (`list|deploy|seed|up|verify`).

See [`demo/README.md`](demo/README.md) for the full guide.

---

## Apps

### `apps/mirage-rs` · **141 tests** · `[shipping]`
In-process Ethereum fork simulator with lazy upstream reads, copy-on-write scenario branching, and a JSON-RPC server. Fully `alloy`-compatible.

**Features:**
- Pure EVM surface on `:8545` (default build)
- Opt-in `chain` feature: HDC precompile (0xA00), InsightEntry storage precompile (0xA01), pheromone-weighted stigmergy
- Opt-in `roko` feature: bridges `ChainSubstrate` + `SimulationGate` to roko-core
- 10 pre-funded anvil test accounts, `evm_mine`, `hardhat_impersonateAccount`
- Metrics on `:9091` (prometheus)

Use cases: local EVM for tests, fork replays against mainnet via `--rpc-url`, roko-demo's execution substrate.

### `apps/roko-chain-watcher` · `[shipping]`
Long-running roko agent that subscribes to a mirage chain and posts insights over HTTP JSON-RPC. Demo of a chain-native agent built on the kernel.

### `contracts/` (Foundry project) · **36 forge tests** · `[shipping]`
6 Solidity contracts used by the demo environment:
- `MockERC20.sol` — "DAEJI" test token with open faucet
- `AgentRegistry.sol` — ERC-8004 compat identity + heartbeat liveness
- `WorkerRegistry.sol` — stake bonds + EMA reputation + 30-day halving decay
- `BountyMarket.sol` — ERC-8183 4-state escrow with slash on reject
- `ConsortiumValidator.sol` — 3-agent 2-of-3 voting committees
- `InsightBoard.sol` — post/confirm insights with pheromone weights

---

## Running things

### Run the roko CLI

```bash
# Build + init a workspace
cargo run -p roko-cli -- init /tmp/demo

# Run the universal loop once (default: `cat` echoes prompt)
cd /tmp/demo && cargo run -p roko-cli -- run "write a hello function"

# Configure an LLM backend
roko config init                           # interactive wizard, detects installed CLIs
roko config set agent.command ollama --global
roko config set agent.args '["run", "llama3.2"]' --global

# Inspect the signal store
roko status                                # counts + recent episode + pass/fail
roko replay <content-hash>                 # walk lineage DAG
```

Tested with **claude**, **ollama** (llama3.2, gemma4:26b, glm-4.7-flash), **mods**, **llm**. For reasoning models, `clean_output = true` (default) strips chain-of-thought + ANSI progress escapes.

Full CLI reference: [`crates/roko-cli/README.md`](crates/roko-cli/README.md).

### Run the chain demo

```bash
# 1. Build mirage-rs
cargo build -p mirage-rs --bin mirage-rs --release

# 2. Start mirage in the background
$CARGO_TARGET_DIR/release/mirage-rs --host 127.0.0.1 --port 18545 --chain-id 31337 &

# 3. Run any of the 4 scenarios
export ROKO_MIRAGE_URL=http://127.0.0.1:18545
cargo run -p roko-demo -- --demo-dir demo --runtime-dir demo/.runtime up job-board
cargo run -p roko-demo -- --demo-dir demo --runtime-dir demo/.runtime verify job-board
```

Scenarios: `job-board`, `consortium`, `defi-routing`, `flywheel`. Full guide: [`demo/README.md`](demo/README.md).

### Run via docker

```bash
cd docker
SCENARIO=job-board docker compose --profile demo up --build --exit-code-from roko-demo
```

Stack: mirage (:8545), roko-demo (runs to completion), prometheus (:9090), grafana (:3000).

---

## Tests

| Crate | Tests | Notes |
|---|---:|---|
| `roko-core` | **376** | kernel types + traits |
| `roko-agent` | **346** | LLM spawning + output cleaning |
| `roko-gate` | **200** | real subprocess Gates |
| `roko-orchestrator` | **158** | plan discovery, DAG, worktrees |
| `mirage-rs` (lib) | **141** | EVM fork sim regression |
| `roko-learn` | **101** | episodes, playbooks, patterns |
| `roko-std` | **96** | in-memory defaults |
| `roko-chain` | **52** | traits + mocks |
| `roko-chain` (alloy live) | **3** | vs running mirage |
| `roko-fs` | **37** | JSONL persistence |
| `roko-cli` | **38** | CLI + config |
| `roko-compose` | **23** | prompt assembly |
| `bardo-runtime` | **17** | async runtime |
| `bardo-primitives` | **16** | HDC + compute |
| `roko-tests` (integration) | **5** | end-to-end universal loop |
| `roko-demo` | **4** | orchestrator unit tests |
| `contracts/` (forge) | **36** | Solidity tests |
| **Total (Rust)** | **1,613** | |
| **Total incl. Solidity** | **1,649** | |

### Running tests

```bash
# All Rust, no network — ~10s first build, <2s incremental
cargo test

# Per-crate
cargo test -p roko-core
cargo test -p roko-gate
cargo test -p roko-agent

# Alloy live integration (needs mirage running on $ROKO_TEST_RPC_URL)
cargo test -p roko-chain --features alloy-backend --test alloy_live

# Solidity
cd contracts && forge test
```

---

## Extension recipes

Every primitive is a trait in `roko-core`. Add new capability = write a new impl.

### A new Substrate (persistence backend)

```rust
use roko_core::{Signal, Substrate, ContentHash, RokoResult};
use async_trait::async_trait;

pub struct MyDbSubstrate { pool: sqlx::PgPool }

#[async_trait]
impl Substrate for MyDbSubstrate {
    async fn write(&self, sig: Signal) -> RokoResult<()> { /* insert */ Ok(()) }
    async fn read(&self, hash: &ContentHash) -> RokoResult<Option<Signal>> { /* … */ Ok(None) }
    async fn query(&self, q: &SubstrateQuery) -> RokoResult<Vec<Signal>> { /* … */ Ok(vec![]) }
    fn name(&self) -> &str { "postgres" }
}
```

Use in any loop: `MemorySubstrate` → `FileSubstrate` → `MyDbSubstrate` → `ChainSubstrate` are drop-in compatible.

### A new Gate (verification backend)

```rust
use roko_core::{AgentOutput, Gate, Verdict, RokoResult};
use async_trait::async_trait;

pub struct SolcGate;

#[async_trait]
impl Gate for SolcGate {
    async fn verify(&self, output: &AgentOutput) -> RokoResult<Verdict> {
        let status = tokio::process::Command::new("solc")
            .arg("--bin").arg(&output.path)
            .status().await?;
        Ok(if status.success() { Verdict::pass() } else { Verdict::fail("solc") })
    }
    fn name(&self) -> &str { "solc" }
}
```

Chain multiple Gates via `VerifyChainGate::new([g1, g2, g3]).short_circuit(true)`.

### A new Scorer, Router, Composer, Policy

Same pattern — implement the trait from `roko-core`. See `roko-std` for minimal examples, `roko-gate` for complex ones.

### A new Agent backend

```rust
use roko_core::{Prompt, AgentOutput};
use roko_agent::Agent;
use async_trait::async_trait;

pub struct AnthropicApiAgent { api_key: String, model: String }

#[async_trait]
impl Agent for AnthropicApiAgent {
    async fn run(&self, prompt: &Prompt) -> AgentOutput {
        // POST to api.anthropic.com/v1/messages
        // return AgentOutput { content, tool_calls, tokens_used, .. }
    }
    fn name(&self) -> &str { "claude-api" }
}
```

### A new demo scenario

Declarative (`demo/scenarios/my.toml`) + implement `Scenario` trait (`crates/roko-demo/src/scenarios/my.rs`) + register in `scenarios::all()`. See [`demo/README.md#adding-a-new-scenario`](demo/README.md#adding-a-new-scenario).

### A new Solidity contract

Drop into `contracts/src/Foo.sol`, write `forge test`, reference in a scenario's `[[deploy.contracts]]`, add `sol!` binding in `crates/roko-demo/src/bindings.rs`. See [`demo/README.md#adding-a-new-contract`](demo/README.md#adding-a-new-contract).

---

## What to build with this

The whole stack is designed to compose. Concrete things you can build:

### Coding agents (already works today)
- SWE-bench-style agents: read issue, inject files, call LLM, compile-gate the diff, write result.
- Multi-step refactors: plan DAG in `roko-orchestrator`, each task runs its own universal loop.
- Code-review bots: `GitSubstrate` (write it) + `LLMJudge` Gate + `EpisodePolicy` feedback.

### Chain-native agents
- Market-making bot: `ChainSubstrate` stores prices as signals, `Scorer` ranks opportunities, `TxSimGate` verifies pre-sign, `WalletGate` checks balance, `Agent` emits signed txs.
- MEV searcher: `Router` picks best bundle, `Policy` triggers retries on outbid.
- Reputation aggregator: reads consortium votes across time, emits aggregate `Episode`s.

### Multi-agent systems
- Job marketplaces (→ see `demo/scenarios/job-board.toml`)
- 2-of-3 validation committees (→ see `demo/scenarios/consortium.toml`)
- Stigmergic curation with pheromone decay (→ see `demo/scenarios/flywheel.toml`)
- Anything with "N agents see the same chain state, each decides independently"

### Benchmarks
- DeFi routing evaluations against mirage forks (→ see `demo/scenarios/defi-routing.toml`)
- Agent-vs-agent tournaments (wire multiple `Scenario` impls, compare success rates)
- LLM regression suites: fixed prompts + `MockAgent` baseline + real-agent comparison

### Research platforms
- Context engineering: swap `PromptComposer` strategies, measure via `EpisodeLog`
- Reward shaping: plug different `Scorer`s into the router loop, observe divergence
- HDC-indexed knowledge retrieval: `HdcSubstrate` over `bardo-primitives` vectors

### Infrastructure
- Ephemeral chain simulators: wrap mirage-rs in a test harness that forks mainnet per-test
- Signal-based event routers: `Substrate` + `Policy` is a pub/sub system with replay
- Audit trails: `FileSubstrate` + signed signals = tamper-evident log

---

## File map

```
roko/
├── Cargo.toml                          # 16-member workspace
├── README.md                           # this file
│
├── crates/
│   ├── roko-core/           376 tests  # Signal + 6 traits + types
│   ├── roko-std/             96 tests  # in-memory impls
│   ├── roko-gate/           200 tests  # CompileGate, TestGate, ShellGate, VerifyChain, LLMJudge
│   ├── roko-fs/              37 tests  # FileSubstrate (JSONL)
│   ├── roko-compose/         23 tests  # PromptComposer + U-shape + priority drop
│   ├── roko-agent/          346 tests  # MockAgent, ExecAgent, ClaudeAgent
│   ├── roko-orchestrator/   158 tests  # Plan DAG, worktrees, safety
│   ├── roko-chain/           52 tests  # ChainClient, ChainWallet (+ alloy-backend)
│   ├── roko-cli/             38 tests  # `roko` binary
│   ├── roko-learn/          101 tests  # episodes, playbooks, patterns
│   ├── roko-demo/             4 tests  # `roko-demo` orchestrator
│   ├── bardo-primitives/     16 tests  # HDC + compute
│   └── bardo-runtime/        17 tests  # event bus + supervision
│
├── apps/
│   ├── mirage-rs/           141 tests  # EVM fork simulator on :8545
│   └── roko-chain-watcher/             # long-running chain-native agent
│
├── contracts/                36 tests  # Foundry project
│   ├── foundry.toml
│   ├── src/                            # MockERC20, AgentRegistry, WorkerRegistry,
│   │                                   # BountyMarket, ConsortiumValidator, InsightBoard
│   └── test/                           # *.t.sol
│
├── demo/                               # declarative demo config
│   ├── README.md                       # 450-line demo guide
│   ├── manifest.toml                   # scenario registry
│   ├── wallets.toml                    # 10 anvil dev wallets
│   ├── scenarios/                      # 4 scenario TOMLs
│   └── prompts/                        # 4 LLM role templates
│
├── docker/
│   ├── docker-compose.yml              # mirage + roko + demo profile + prometheus + grafana
│   ├── mirage.Dockerfile
│   ├── roko.Dockerfile
│   └── demo.Dockerfile
│
└── tests/                     5 tests  # end-to-end universal loop integration
```

---

## Design principles

1. **One noun, six verbs, one async extension.** Every capability in 110+ source docs maps to this.
2. **Every rung is testable.** Stop at any rung → working system. Start from any rung → additive.
3. **Coding ≡ chain-native.** Same `RokoAgent` struct, different trait impls registered. No rewrites.
4. **Content-addressed everything.** BLAKE3 hashes give deduplication, replay, caching, and signed provenance for free.
5. **Local-first.** `FileSubstrate` is the default; ChainSubstrate, HdcSubstrate, PostgresSubstrate are opt-in.
6. **No magic.** Every impl is a public struct with a short `name()`. `roko replay <hash>` walks the DAG for any run.
7. **Unsafe denied, undocumented public items warned.** Enforced at workspace level.

---

## Related docs

- [`demo/README.md`](demo/README.md) — running + extending the chain demo
- [`crates/roko-cli/README.md`](crates/roko-cli/README.md) — CLI reference + LLM backend worked examples
- [`docker/README.md`](docker/README.md) — docker-compose reference
- `../tmp/roko-progress/` — design docs (if present in your checkout)
  - `12-unified-primitives.md` — 1 Signal + 6 verbs architecture
  - `13-dual-nature-agents.md` — coding + chain-native dual-nature model
