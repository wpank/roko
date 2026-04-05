# Roko

**Roko is a Rust toolkit for building agents.** It gives you a small set of composable pieces — storage, scoring, verification, routing, prompt assembly, policy, and LLM execution — that snap together into coding agents, chain-native agents, multi-agent workflows, or anything else that needs to observe, decide, and act in a loop.

If you've ever built an agent and found yourself reinventing a signal store, a retry policy, a prompt assembler, or a verification step, Roko is the layer you keep rewriting. This repo is that layer — plus a ready-to-run demo environment that deploys Solidity contracts to an in-process Ethereum simulator and runs 4 pre-canned multi-agent scenarios against them.

---

## Why it exists

Every agent system ends up needing the same machinery:

- **Somewhere to put things it's learned** (a substrate)
- **A way to decide what's relevant** (scoring)
- **A way to check if its output is any good** (verification)
- **A way to pick among options** (routing)
- **A way to assemble prompts under a token budget** (composition)
- **A way to react to patterns across time** (policy)
- **A way to actually call an LLM** (agent execution)

Most agent frameworks either bake these in rigidly or make you glue them together yourself. Roko makes each one a tiny Rust trait, ships default implementations, and lets you swap any piece without touching the others. The "universal loop" — `query → score → route → compose → act → verify → write back → react` — is what every agent in the system runs.

A side-effect of this architecture: a coding agent and a chain-native agent (one that signs Ethereum transactions) are **the same struct** with different pieces plugged in. You write the loop once; the difference between them is which `Substrate` they talk to and which `Gate`s they pass through.

---

## Table of contents

1. [5-minute tour](#5-minute-tour)
2. [How the architecture works](#how-the-architecture-works)
3. [What's in the workspace](#whats-in-the-workspace)
4. [Try it yourself](#try-it-yourself)
5. [Running the tests](#running-the-tests)
6. [Building on top of it](#building-on-top-of-it)
7. [What you can build](#what-you-can-build)
8. [File map](#file-map)
9. [Design principles](#design-principles)

---

## 5-minute tour

**If you just want to see something run**, here's a 3-command path to a working multi-agent chain demo:

```bash
# 1. Build the included Ethereum simulator
cargo build -p mirage-rs --bin mirage-rs --release

# 2. Start it in the background (port 18545, chain id 31337)
$CARGO_TARGET_DIR/release/mirage-rs --host 127.0.0.1 --port 18545 --chain-id 31337 &

# 3. Deploy ERC contracts + run a 5-worker job-board scenario against them
export ROKO_MIRAGE_URL=http://127.0.0.1:18545
cargo run -p roko-demo -- --demo-dir demo --runtime-dir demo/.runtime up job-board
```

In ~700ms you'll see: 4 Solidity contracts deployed deterministically, 5 worker agents registered on-chain with 1000-token stakes, 3 jobs posted with bounties 10/40/70 DAEJI, each assigned to a worker, submitted, and resolved with payouts. Then run `verify job-board` and it'll confirm 3 `JobResolved` events fired.

If you prefer an LLM coding-agent demo without any chain stuff:

```bash
cargo run -p roko-cli -- init /tmp/agent-demo
cd /tmp/agent-demo
cargo run -p roko-cli -- run "write a hello function in rust"
```

By default that uses `cat` as the "agent" (it echoes the prompt back), which is enough to see the full pipeline — prompt assembly, agent call, gate check, signal persistence — without needing an API key. Point the config at `claude`, `ollama`, `mods`, `llm` or any CLI that reads a prompt on stdin and you have a real coding agent.

---

## How the architecture works

### One noun: `Signal`

Everything in Roko is a **Signal** — a content-addressed (BLAKE3-hashed), timestamped, scored record of something that happened. A prompt is a signal. An LLM response is a signal. A compile check's verdict is a signal. An "episode" (a summary of N signals) is itself a signal.

Signals have parent pointers, so they form a DAG. That means you can always answer: "why did the agent decide this?" — by walking backwards through its lineage.

```rust
struct Signal {
    hash: ContentHash,        // BLAKE3 of canonical bytes
    kind: SignalKind,         // Prompt | AgentOutput | GateVerdict | Episode | ...
    parents: Vec<ContentHash>,// lineage → replayable
    score: Option<Score>,     // 0..1, from Scorers
    decay: DecayState,        // half-life for relevance
    ts: Timestamp,
    payload: Bytes,           // kind-specific content
}
```

### Six verbs: the core traits

These are defined in `crates/roko-core` and implemented in every other crate. Think of them as interchangeable parts:

| Verb | Job | Example concrete impls |
|---|---|---|
| **`Substrate`** | Store + query signals | `MemorySubstrate` (HashMap), `FileSubstrate` (JSONL log), `ChainSubstrate` (on-chain storage) |
| **`Scorer`** | Rate a signal | `RecencyScorer`, `NoveltyScorer`, `PriorityScorer` (combine via weighted sum) |
| **`Gate`** | Check if output is any good | `CompileGate` (runs `cargo check`), `TestGate`, `ShellGate`, `WalletGate` |
| **`Router`** | Pick one option from many | `TopKRouter`, `ThompsonBandit` (learns over time), `FirstRouter` |
| **`Composer`** | Pack things under a budget | `PromptComposer` (U-shape placement + priority-order drop) |
| **`Policy`** | Emit new signals from patterns | `EpisodePolicy` (summarizes runs), `RetryPolicy` (kicks off retries on failure) |

### One async extension: `Agent`

Everything above is either synchronous or I/O-only. The thing that actually spends money / spawns subprocesses / calls LLMs is the **Agent**:

```rust
#[async_trait]
pub trait Agent: Send + Sync {
    async fn run(&self, prompt: &Prompt) -> AgentOutput;
    fn name(&self) -> &str;
}
```

`MockAgent` (scripted replies for tests), `ExecAgent` (spawns any CLI tool), and `ClaudeAgent` (file injection, JSON modes) all live in `crates/roko-agent`.

### The universal loop

Every Roko agent runs this:

```text
1. Substrate.query() → fetch relevant signals from storage
2. Scorers rate them
3. Router picks the top-K (or Composer packs them under a budget)
4. Agent consumes the prompt → produces AgentOutput
5. Gates verify the output → Verdicts
6. Substrate.write() → persist everything with parent links
7. Policies fire on accumulated signals → emit new signals (Episodes, retries, …)
```

Every step is a trait — you can stop at any step and still have something useful (e.g., a "fetch + compose" pipeline without an agent), and you can add capabilities (HDC indexing, on-chain persistence, bandit routing) without rewriting anything else.

---

## What's in the workspace

16 Rust crates + 1 Foundry project + 2 binaries. Organized into layers. Tests verified by running them.

### Kernel — the contracts that everyone else implements

| Crate | Tests | What it is |
|---|---:|---|
| [`crates/roko-core`](crates/roko-core) | **376** | The `Signal` type, the six trait definitions, `ContentHash`, `Score`, `Decay`, `Verdict`, `Prompt`, `PromptSection`, error types. Pure — no I/O. This is the contract that every other crate implements or consumes. |

### Standard implementations — sensible defaults

| Crate | Tests | What it is |
|---|---:|---|
| [`crates/roko-std`](crates/roko-std) | **96** | `MemorySubstrate`, `TopKRouter`, `PriorityRouter`, `RecencyScorer`, `CombinedScorer`, NoOp defaults for everything. Good for unit tests and small embeddings. |

### Verification — ground-truth checks

| Crate | Tests | What it is |
|---|---:|---|
| [`crates/roko-gate`](crates/roko-gate) | **200** | Real-subprocess `Gate` implementations: `CompileGate` (detects `cargo`/`npm`/`forge`), `TestGate`, `LintGate`, `SymbolGate`, `ShellGate`, `VerifyChainGate` (short-circuit chain), `LLMJudge` (delegates to another agent for subjective checks). |

### Persistence — where signals live

| Crate | Tests | What it is |
|---|---:|---|
| [`crates/roko-fs`](crates/roko-fs) | **37** | `FileSubstrate`: append-only JSONL persistence with in-memory index + compaction. Signals survive restart; lineage DAG is fully replayable via `roko replay <hash>`. |

### Composition — prompt assembly

| Crate | Tests | What it is |
|---|---:|---|
| [`crates/roko-compose`](crates/roko-compose) | **23** | `PromptComposer`: assembles `PromptSection`s into a token-budgeted prompt using U-shape placement (intro + conclusion prioritized) + priority-ordered dropping when over budget. `TokenEstimator` for cheap counts. |

### Agent execution — the async extension

| Crate | Tests | What it is |
|---|---:|---|
| [`crates/roko-agent`](crates/roko-agent) | **346** | `MockAgent` (tests), `ExecAgent` (spawns any CLI — `claude`, `ollama`, `mods`, `llm`, `gpt`), `ClaudeAgent` (file injection + JSON modes). Handles timeouts, retries, env propagation, CoT + ANSI stripping, code-fence extraction. |

### Orchestration — building larger workflows

| Crate | Tests | What it is |
|---|---:|---|
| [`crates/roko-orchestrator`](crates/roko-orchestrator) | **158** | `PlanDiscovery`, `TaskGraph` (task DAG), `WorktreeManager` (git worktrees for parallel agents), `Executor`, `SafetyPolicy`, `CapabilityToken`. Build multi-agent workflows on top of this. |

### Learning — feedback loops

| Crate | Tests | What it is |
|---|---:|---|
| [`crates/roko-learn`](crates/roko-learn) | **101** | `EpisodeLog`, `Playbook` library, `SkillLibrary`, `ContextCache`, `PatternMiner`. Plug into `Policy` impls to feed past experience back into future decisions. |

### Chain — on-chain agents

| Crate | Tests | What it is |
|---|---:|---|
| [`crates/roko-chain`](crates/roko-chain) | **52 + 3** | `ChainClient` + `ChainWallet` traits, mock impls, `WalletGate` (balance/nonce check), `TxSimGate` (simulate pre-sign). With `--features alloy-backend`: real JSON-RPC `AlloyChainClient` + `AlloyChainWallet` against any Ethereum endpoint. |

### User-facing binaries

| Crate | Tests | What it is |
|---|---:|---|
| [`crates/roko-cli`](crates/roko-cli) | **38** | The `roko` binary. Subcommands: `init`, `run`, `status`, `replay`, `config`, `serve`. Reads layered config from `~/.config/roko/config.toml` + `./roko.toml`. |
| [`crates/roko-demo`](crates/roko-demo) | **4** | The `roko-demo` binary. Manifest-driven orchestrator that deploys Solidity contracts, seeds fixtures, runs multi-agent scripted scenarios against mirage-rs. See [`demo/README.md`](demo/README.md). |

### Support primitives

| Crate | Tests | What it is |
|---|---:|---|
| `crates/bardo-primitives` | **16** | Zero-dep compute: 10,240-bit HDC (hyperdimensional) vectors with Hamming similarity search. |
| `crates/bardo-runtime` | **17** | Typed event bus, process supervision hooks, cancellation primitives. |

### Apps (binaries, not libraries)

| App | Tests | What it is |
|---|---:|---|
| [`apps/mirage-rs`](apps/mirage-rs) | **141** | **In-process Ethereum fork simulator**. Full EVM via `revm`, `alloy`-compatible JSON-RPC on `:8545`, copy-on-write scenario branching, optional HDC/InsightEntry precompiles, lazy reads from upstream mainnet. Runs on `localhost`, boots in <100ms, mines ~4000 blocks/sec. |
| [`apps/roko-chain-watcher`](apps/roko-chain-watcher) | — | Long-running agent that subscribes to a mirage chain and posts insights via HTTP JSON-RPC. Example of a chain-native agent. |

### Solidity contracts

| Project | Tests | What it is |
|---|---:|---|
| [`contracts/`](contracts/) | **36** | Foundry project with 6 contracts for the demo: `MockERC20` (DAEJI test token), `AgentRegistry` (ERC-8004 identity), `WorkerRegistry` (stake + EMA reputation + 30-day halving decay), `BountyMarket` (ERC-8183 4-state escrow), `ConsortiumValidator` (2-of-3 voting), `InsightBoard` (pheromone-weighted knowledge). |

**Test totals:** 1,613 Rust tests + 36 Solidity tests = **1,649 tests, all passing**.

---

## Try it yourself

### Option A — the coding-agent loop (no chain, no external deps)

```bash
cargo run -p roko-cli -- init /tmp/agent-demo
cd /tmp/agent-demo

# Runs with `cat` as the default agent — echoes your prompt back.
# Useful for smoke-testing the pipeline.
cargo run -p roko-cli -- run "write a hello function"

# Inspect what happened
cargo run -p roko-cli -- status        # counts + recent episode + gate pass/fail
cargo run -p roko-cli -- replay <hash> # walk the full signal lineage
```

To use a real LLM:

```bash
roko config init                              # interactive wizard, detects installed CLIs
roko config set agent.command ollama --global
roko config set agent.args '["run", "llama3.2"]' --global
```

Tested with `claude`, `ollama` (llama3.2, gemma4:26b), `mods`, `llm`. Any CLI that reads a prompt on stdin and writes the response to stdout will work.

### Option B — the multi-agent chain demo

```bash
# 1. Build mirage-rs (the in-process Ethereum simulator)
cargo build -p mirage-rs --bin mirage-rs --release

# 2. Start it
$CARGO_TARGET_DIR/release/mirage-rs --host 127.0.0.1 --port 18545 --chain-id 31337 &

# 3. Run any of 4 scenarios
export ROKO_MIRAGE_URL=http://127.0.0.1:18545
cd roko
cargo run -p roko-demo -- --demo-dir demo --runtime-dir demo/.runtime up job-board
cargo run -p roko-demo -- --demo-dir demo --runtime-dir demo/.runtime verify job-board
```

The 4 scenarios, all runnable with the same commands (substitute scenario name):

| Scenario | What it demos |
|---|---|
| `job-board` | 1 poster posts 3 jobs, 5 workers take turns fulfilling them, resolver pays out |
| `consortium` | 3 validators form a committee, 2-of-3 vote to resolve a submitted job |
| `defi-routing` | 1 poster posts a routing benchmark, 5 workers race, first wins the bounty |
| `flywheel` | 3 posters submit 9 knowledge insights, 3 confirmers generate 18 confirmations |

**Full demo guide:** [`demo/README.md`](demo/README.md) — covers state, CLI reference, example log outputs, extension recipes, and how to add more scenarios.

### Option C — docker

```bash
cd docker
SCENARIO=job-board docker compose --profile demo up --build --exit-code-from roko-demo
```

Brings up: mirage (`:8545`), roko-demo (runs scenario to completion), prometheus (`:9090`), grafana (`:3000`).

---

## Running the tests

```bash
# Everything, no network required — ~10s first build, <2s incremental
cargo test

# A single crate
cargo test -p roko-core
cargo test -p roko-agent

# Solidity tests (needs Foundry installed: foundryup)
cd contracts && forge test

# Live integration tests (needs mirage running on $ROKO_TEST_RPC_URL)
ROKO_TEST_RPC_URL=http://127.0.0.1:18545 \
  cargo test -p roko-chain --features alloy-backend --test alloy_live
```

| Crate | Tests | Needs |
|---|---:|---|
| `roko-core` | 376 | nothing |
| `roko-agent` | 346 | nothing |
| `roko-gate` | 200 | nothing |
| `roko-orchestrator` | 158 | nothing |
| `mirage-rs` | 141 | nothing |
| `roko-learn` | 101 | nothing |
| `roko-std` | 96 | nothing |
| `roko-chain` | 52 | nothing |
| `roko-cli` | 38 | nothing |
| `roko-fs` | 37 | nothing |
| `roko-compose` | 23 | nothing |
| `bardo-runtime` | 17 | nothing |
| `bardo-primitives` | 16 | nothing |
| `roko-tests` (integration) | 5 | nothing |
| `roko-demo` | 4 | nothing |
| `roko-chain` (alloy live) | 3 | running mirage |
| `contracts/` (forge) | 36 | foundry installed |

---

## Building on top of it

Every capability is a trait from `roko-core`. Adding something new = writing a new impl.

### Add a storage backend

```rust
use roko_core::{Signal, Substrate, ContentHash, RokoResult};
use async_trait::async_trait;

pub struct PostgresSubstrate { pool: sqlx::PgPool }

#[async_trait]
impl Substrate for PostgresSubstrate {
    async fn write(&self, sig: Signal) -> RokoResult<()> { /* INSERT */ Ok(()) }
    async fn read(&self, hash: &ContentHash) -> RokoResult<Option<Signal>> { /* SELECT */ Ok(None) }
    async fn query(&self, q: &SubstrateQuery) -> RokoResult<Vec<Signal>> { /* … */ Ok(vec![]) }
    fn name(&self) -> &str { "postgres" }
}
```

Now plug it into any loop that uses `dyn Substrate` — it's drop-in compatible with `MemorySubstrate` and `FileSubstrate`.

### Add a verification step

```rust
use roko_core::{AgentOutput, Gate, Verdict, RokoResult};

pub struct SolcGate;

#[async_trait::async_trait]
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

Chain multiple: `VerifyChainGate::new([g1, g2, g3]).short_circuit(true)`.

### Add a new LLM backend

```rust
use roko_core::{Prompt, AgentOutput};
use roko_agent::Agent;

pub struct OpenAIApiAgent { api_key: String, model: String }

#[async_trait::async_trait]
impl Agent for OpenAIApiAgent {
    async fn run(&self, prompt: &Prompt) -> AgentOutput { /* POST /v1/chat/completions */ }
    fn name(&self) -> &str { "openai-api" }
}
```

### Add a demo scenario

Declarative TOML + implement `Scenario` trait + one-line registration. Full recipe: [`demo/README.md#adding-a-new-scenario`](demo/README.md#adding-a-new-scenario).

### Add a Solidity contract to the demo

Drop `Foo.sol` into `contracts/src/`, write `forge test`, reference it in a scenario's `[[deploy.contracts]]`, add an alloy `sol!` binding. Full recipe: [`demo/README.md#adding-a-new-contract`](demo/README.md#adding-a-new-contract).

---

## What you can build

Because every piece is a trait, the same crates power very different applications:

### Coding agents
- **SWE-bench runners**: read issue → inject files → call LLM → `CompileGate` the diff → `TestGate` → write result. Loop until green or budget exhausted.
- **Multi-step refactors**: `roko-orchestrator` plans a task DAG; each task runs its own universal loop in a git worktree.
- **Code-review bots**: write a `GitSubstrate`, plug in `LLMJudge` as a Gate, drive from `EpisodePolicy`.

### Chain-native agents
- **Market-making bots**: `ChainSubstrate` stores prices as signals, `Scorer` ranks opportunities, `TxSimGate` dry-runs every tx, `WalletGate` checks balance, `Agent` signs and broadcasts.
- **Liquidation searchers**: `Router` picks the most profitable target, `Policy` fires retries on outbid.
- **Reputation aggregators**: read consortium votes across time, emit aggregate `Episode` signals.

### Multi-agent systems
- **Job marketplaces** (see `demo/scenarios/job-board.toml`)
- **Validation committees** (see `demo/scenarios/consortium.toml`)
- **Knowledge curation with pheromone decay** (see `demo/scenarios/flywheel.toml`)
- Anything where "N independent agents see the same state, each decides what to do"

### Benchmarks + evals
- **DeFi routing benchmarks** on forked mainnet state via mirage-rs
- **Agent-vs-agent tournaments**: wire multiple `Scenario` impls, compare success rates
- **LLM regression suites**: fixed prompts + `MockAgent` baseline + real-agent comparison

### Research platforms
- **Context engineering experiments**: swap `PromptComposer` strategies, measure episode outcomes
- **Reward shaping**: plug different `Scorer`s into the router loop, observe divergence
- **Semantic retrieval**: `HdcSubstrate` over `bardo-primitives` binary vectors for sub-ms knowledge lookups

### Infrastructure pieces
- **Ephemeral chain simulators**: wrap mirage-rs in a test harness that forks mainnet per-test
- **Pub/sub with replay**: `Substrate` + `Policy` is an event-sourced message bus
- **Tamper-evident audit logs**: `FileSubstrate` + signed signals = content-addressed provenance

---

## File map

```
roko/
├── README.md                           ← you are here
├── Cargo.toml                          ← 16-member workspace
│
├── crates/                             ← libraries
│   ├── roko-core/           376 tests  ← Signal + 6 traits (the contract)
│   ├── roko-std/             96 tests  ← in-memory defaults
│   ├── roko-gate/           200 tests  ← CompileGate, TestGate, ShellGate, …
│   ├── roko-fs/              37 tests  ← FileSubstrate (JSONL)
│   ├── roko-compose/         23 tests  ← PromptComposer
│   ├── roko-agent/          346 tests  ← MockAgent, ExecAgent, ClaudeAgent
│   ├── roko-orchestrator/   158 tests  ← Plan DAG, worktrees, safety
│   ├── roko-chain/           52 tests  ← ChainClient, ChainWallet (+ alloy-backend)
│   ├── roko-cli/             38 tests  ← `roko` binary
│   ├── roko-learn/          101 tests  ← episodes, playbooks, patterns
│   ├── roko-demo/             4 tests  ← `roko-demo` binary (chain scenarios)
│   ├── bardo-primitives/     16 tests  ← HDC + compute
│   └── bardo-runtime/        17 tests  ← event bus + supervision
│
├── apps/                               ← binaries (not libraries)
│   ├── mirage-rs/           141 tests  ← Ethereum fork simulator on :8545
│   └── roko-chain-watcher/             ← long-running chain agent
│
├── contracts/                36 tests  ← Foundry project: 6 .sol contracts
├── demo/                               ← declarative demo config
│   └── README.md                       ← full demo guide (450 lines)
├── docker/                             ← docker-compose + dockerfiles
└── tests/                     5 tests  ← end-to-end integration tests
```

---

## Design principles

1. **One noun, six verbs, one async extension.** Every capability in the stack folds into this.
2. **Every rung is testable.** Stop at any rung → working system. Start from any rung → additive.
3. **Coding ≡ chain-native.** Same struct, different trait impls registered. No rewrites.
4. **Content-addressed everything.** BLAKE3 hashes give deduplication, replay, caching, and signed provenance for free.
5. **Local-first.** `FileSubstrate` is the default; chain/HDC/Postgres substrates are opt-in.
6. **No magic.** Every impl is a public struct with a `name()` method. `roko replay <hash>` walks the DAG for any run.
7. **Unsafe denied, undocumented public items warned.** Enforced at workspace level.

---

## Related docs

- [`demo/README.md`](demo/README.md) — running + extending the chain demo, example log outputs, known limitations
- [`crates/roko-cli/README.md`](crates/roko-cli/README.md) — CLI reference + LLM backend worked examples
- [`docker/README.md`](docker/README.md) — docker-compose reference

---

## License

MIT OR Apache-2.0 (dual-licensed).
