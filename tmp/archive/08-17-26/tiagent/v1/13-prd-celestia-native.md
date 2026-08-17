# PRD: Celestia Integration Layer (Optional Enhancement)

## Document Information

| Field | Value |
|-------|-------|
| **Product** | tiagent Celestia integration layer (optional) |
| **One-line** | Optional enhancement that adds deep Celestia ecosystem tooling, shared cross-agent learning via DA, and native rollup/IBC/governance capabilities on top of the standalone core harness |
| **Status** | Design phase |
| **Document** | 13 of 15 in the tiagent design suite |
| **Prerequisites** | 01-vision-and-overview.md, 02-architecture.md, 04-celestia-integration.md, 12-prd-core-harness.md |

> **This PRD describes the optional Celestia integration layer.** tiagent's core agent
> harness (see 12-prd-core-harness.md) works as a fully functional, self-improving coding
> agent without any of the features described here. These features add shared cross-agent
> learning, verifiable traces, and native Celestia development tooling for developers who
> want them. Nothing in this document is required to use tiagent as a standalone coding
> agent.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Who Needs This PRD](#2-who-needs-this-prd)
3. [Problem Statement](#3-problem-statement)
4. [Goals and Non-Goals](#4-goals-and-non-goals)
5. [Features](#5-features)
6. [Requirements Table](#6-requirements-table)
7. [Technical Design](#7-technical-design)
8. [Success Metrics](#8-success-metrics)
9. [Milestones](#9-milestones)
10. [Dependencies](#10-dependencies)
11. [Open Questions](#11-open-questions)
12. [Related Documents](#12-related-documents)

---

## 1. Overview

The core harness MVP (12-prd-core-harness.md) establishes tiagent as a fully functional,
self-improving coding agent: an LLM dispatches tool calls, gates validate output, episodes
are logged, and state survives crashes. That standalone harness works for any developer,
on any codebase, without any blockchain dependency. It includes basic Celestia integration
--- blob submit and blob get via `celestia-rpc` --- as optional P1 tools, sufficient to
prove that an agent can write to and read from Celestia's data availability layer.

This document scopes the optional enhancement layer beyond those two tools.

For developers who work in the Celestia ecosystem, or who want shared cross-agent learning
via DA, tiagent offers a deep integration layer. This PRD covers those optional
capabilities. "Celestia-native" means that when this layer is enabled, tiagent understands
Celestia's primitives, protocols, and developer workflows as first-class concepts, not as
thin RPC wrappers behind generic tool definitions. A generic agent framework can submit a
blob if you hand it the right curl command. With the Celestia layer enabled, tiagent knows
what a namespace is, why you would pick one naming scheme over another, what a rollup's DA
backend configuration looks like, how IBC channels connect Celestia to other chains, what
gas costs look like at different block heights, and how to scaffold a project that posts
data to Celestia from scratch.

When this layer is enabled, the target end state: a developer says "deploy a rollup on
Celestia" and tiagent handles the entire workflow --- scaffolding the project, configuring
the DA backend, deploying to a testnet, setting up monitoring, and reporting status. A
second developer says "why are my blob costs spiking?" and tiagent analyzes submission
history, identifies the pattern, and recommends a batching strategy. A third developer
says "set up IBC between my rollup and Osmosis" and tiagent configures the relayer,
creates the channel, and verifies the first transfer.

None of this is possible with blob submit and blob get alone. This PRD defines the seven
feature areas that the optional Celestia layer provides.

---

## 2. Who Needs This PRD

Not every tiagent user needs this document. The core harness (12-prd-core-harness.md)
is the complete standalone coding agent. This PRD layers on top of it. Here is who
benefits from which parts:

| Audience | What to enable | Relevant features |
|----------|---------------|-------------------|
| **Developers building rollups, dApps, or infrastructure on Celestia** | All features in this PRD | F1 (rollup tools), F2 (namespace management), F3 (IBC), F4 (cost optimization), F5 (validator/governance), F6 (light node), F7 (dev assistant) |
| **Developers who want shared cross-agent learning but don't care about Celestia development** | F6 (light node) + shared learning features only | F6 provides the embedded light node that enables verifiable trace publishing and cross-agent learning via DA. Skip F1--F5 and F7 |
| **Developers using tiagent as a standalone coding agent** | Nothing from this PRD | Skip this document entirely. The core harness (12-prd-core-harness.md) is your product. tiagent works fully without any Celestia integration |

If you fall into the third category, stop reading here.

---

## 3. Problem Statement

### 3.1 Fragmented CLI tooling

Celestia developers currently interact with the ecosystem through a collection of
disconnected command-line tools, each with its own configuration format, authentication
mechanism, and mental model:

- `celestia` CLI for node operations and blob submission
- `rollkit` CLI for Rollkit-based rollup scaffolding and management
- `hermes` or `rly` for IBC relayer configuration and operation
- `celestia-appd` for validator and governance operations
- Various SDK-specific CLIs (Sovereign SDK, OP Stack) for rollup development

There is no unified interface that spans these concerns. A developer building a rollup
on Celestia must learn and switch between multiple tools, each operating on different
abstractions. There is no tool that can answer "show me my rollup's DA submission
health over the last 24 hours" without the developer manually querying multiple data
sources and correlating the results.

### 3.2 No agent-assisted Celestia development

The existing agent frameworks (LangChain, CrewAI, Rig, etc.) have no Celestia-specific
knowledge. They can be given tool definitions that wrap RPC calls, but they do not
understand:

- Celestia's namespace architecture and how to design namespace schemas
- The relationship between rollups and the DA layer
- IBC channel lifecycle (creation, maintenance, troubleshooting)
- Gas economics and how blob size, block occupancy, and fee markets interact
- Light node operation and Data Availability Sampling
- Governance proposal mechanics and their impact on the ecosystem

Without this domain knowledge embedded in the agent's tool set and prompt context, the
agent is just a passthrough to RPC endpoints. It cannot reason about trade-offs, suggest
optimizations, or diagnose problems.

### 3.3 Manual workflows for common operations

Several high-frequency Celestia developer workflows are entirely manual today:

| Workflow | Current process |
|----------|----------------|
| Deploy a rollup with Celestia DA | Read docs for your chosen rollup framework, manually configure DA endpoints, set up key management, deploy, configure monitoring separately |
| Monitor blob submission costs | Query node for gas prices, manually track blob sizes, build your own cost model |
| Set up IBC between chains | Install and configure a relayer binary, create clients and connections, open channels, monitor packet flow --- each step is a separate manual operation |
| Migrate namespace schema | No tooling exists; developers manually re-deploy with new namespace IDs |
| Analyze DA layer usage patterns | Query blobs manually, export to a spreadsheet, analyze offline |

Each of these workflows involves 10--30 manual steps, most of which are mechanical and
error-prone. An agent with the right tools and domain knowledge can automate them
end-to-end.

### 3.4 No proactive monitoring or optimization

Existing tools are reactive. A developer discovers that their blob costs have doubled
only when they check their wallet balance. They learn that their IBC channel is stuck
only when a user reports a failed transfer. There is no agent that watches for anomalies,
estimates future costs, or suggests optimizations based on historical patterns.

---

## 4. Goals and Non-Goals

### 4.1 Goals

| # | Goal | User-visible behavior |
|---|------|----------------------|
| G1 | End-to-end rollup lifecycle management | Developer says "scaffold a Sovereign SDK rollup with Celestia DA" and gets a working project with DA configuration, deployment scripts, and monitoring |
| G2 | Intelligent namespace management | Agent designs namespace schemas based on data access patterns, monitors namespace activity, and plans migrations |
| G3 | Native IBC operations | Agent sets up, monitors, and troubleshoots IBC channels and transfers without the developer touching relayer configuration files |
| G4 | Cost-aware DA operations | Agent estimates costs before submission, analyzes historical costs, and applies optimization strategies (batching, compression) automatically |
| G5 | Validator and governance awareness | Agent provides status on validators and governance, analyzes proposal impact, and helps with staking decisions |
| G6 | Embedded light node for direct DA access | tiagent runs a Celestia light node in-process, eliminating the dependency on an external node for DA verification and blob retrieval |
| G7 | Context-aware Celestia development assistant | Agent understands Celestia SDK patterns, generates idiomatic code, and looks up documentation on demand |

### 4.2 Non-Goals

| Item | Why excluded | Where scoped instead |
|------|-------------|---------------------|
| Full node operation | tiagent is a development tool, not node infrastructure | Out of scope |
| Validator key management | Security-critical; should use dedicated key management tools | Out of scope |
| Cross-chain arbitrage | Financial tooling is out of scope for a development harness | Out of scope |
| Block production | tiagent consumes DA, it does not produce blocks | Out of scope |
| Custom rollup execution logic | tiagent scaffolds rollups; the developer writes execution logic | Out of scope |

---

## 5. Features

### F1: Rollup Development Tools

Celestia's primary value proposition is as a DA layer for rollups. The rollup developer
experience --- from "I want to build a rollup" to "my rollup is live and posting data to
Celestia" --- is the highest-impact workflow for tiagent to automate.

**Tools:**

| Tool | Description | Input | Output |
|------|-------------|-------|--------|
| `celestia_rollup_scaffold` | Generate a complete rollup project from a template | Framework (Sovereign SDK, Rollkit, OP Stack + Celestia), project name, configuration options | Project directory with source code, Cargo.toml / go.mod, DA configuration, deployment scripts, README |
| `celestia_rollup_deploy` | Deploy a rollup to a target environment | Project directory, target (local devnet, Mocha testnet, mainnet), DA endpoint configuration | Deployment status, endpoints, DA submission confirmation |
| `celestia_rollup_status` | Check rollup health and DA submission status | Rollup identifier or project directory | Sync status, latest DA height, blob submission rate, error count, gas usage |
| `celestia_rollup_upgrade` | Execute a rollup upgrade workflow | Project directory, upgrade type (binary, config, DA migration) | Step-by-step upgrade execution with rollback capability |

The scaffold tool is the entry point. It generates a working project --- not just a
skeleton, but a project that compiles, has a basic test suite, and includes DA
configuration pointing at Mocha testnet. The agent understands the differences between
rollup frameworks: Sovereign SDK uses Rust and a specific module system, Rollkit uses Go
and its own SDK patterns, OP Stack + Celestia DA uses a modified op-node with Celestia as
the DA backend. The scaffold tool generates framework-appropriate code.

The deploy tool handles the multi-step deployment process: building the binary,
configuring keys, setting up the DA endpoint, submitting the genesis blob, and verifying
that the rollup is posting data. It reports each step's status and can resume from
failure.

The status tool consolidates information from multiple sources --- the rollup node, the
DA layer, and the local project state --- into a single view. It answers "is my rollup
healthy?" without the developer querying three different APIs.

The upgrade tool is particularly valuable because rollup upgrades are error-prone. The
tool generates a plan, executes it step by step, and can roll back if any step fails.

### F2: Namespace Management

Namespaces are Celestia's partitioning mechanism for blob data. Every blob belongs to a
namespace (a 29-byte identifier), and clients can query blobs by namespace rather than
downloading entire blocks. For applications that submit multiple types of data ---
traces, state snapshots, coordination messages, learning artifacts --- namespace design
is a critical architectural decision.

**Tools:**

| Tool | Description | Input | Output |
|------|-------------|-------|--------|
| `celestia_namespace_design` | AI-assisted namespace schema design | Description of data types, access patterns, expected volume | Recommended namespace hierarchy with rationale, naming convention, versioning strategy |
| `celestia_namespace_monitor` | Watch a namespace for new blobs | Namespace ID, optional filter criteria, alert conditions | Real-time stream of new blobs with metadata; alerts when conditions are met |
| `celestia_namespace_analyze` | Analyze namespace usage patterns | Namespace ID, time range | Blob count, size distribution, submission frequency, cost breakdown, anomaly detection |
| `celestia_namespace_migrate` | Plan and execute namespace schema migrations | Current schema, target schema, migration constraints | Migration plan with data re-routing steps, estimated cost, rollback procedure |

The design tool is where domain knowledge matters most. The agent knows that Celestia
namespaces are fixed-size (29 bytes), that namespace ordering affects Merkle proof
efficiency, that version 0 namespaces use a specific byte layout, and that namespace
design is effectively permanent (you cannot rename a namespace after data is written to
it). The tool asks the right questions --- "how many distinct data types do you have?",
"do different consumers need to read different subsets?", "what is your expected blob
submission rate?" --- and produces a schema that balances isolation, discoverability, and
proof efficiency.

The monitor tool enables reactive workflows: an agent watches a namespace and takes
action when specific patterns appear (new blobs above a size threshold, submission gaps
indicating downtime, unexpected data formats suggesting misconfiguration).

The analyze tool provides the historical view. It queries blobs across a time range,
computes statistics, and identifies trends. This feeds directly into cost optimization
(see F4).

The migrate tool handles the hard problem: when a namespace schema needs to change (new
data types, restructured hierarchy, version bump), the tool plans the migration, sets
up dual-write during the transition period, and verifies that consumers have switched to
the new namespace before decommissioning the old one.

### F3: IBC (Inter-Blockchain Communication)

IBC is the protocol that connects Celestia to other Cosmos-ecosystem chains (Osmosis,
Cosmos Hub, Stride, and others). It enables token transfers, cross-chain queries, and
interchain accounts. For rollup developers, IBC is how users bridge tokens to and from
their rollup. For agent developers, IBC is how agents on different chains coordinate.

**Tools:**

| Tool | Description | Input | Output |
|------|-------------|-------|--------|
| `celestia_ibc_channels` | List and inspect IBC channels | Chain ID (optional), channel state filter (optional) | Channel list with: channel ID, counterparty chain/channel, connection ID, state (open/closed/init), packet counts |
| `celestia_ibc_transfer` | Execute an IBC token transfer | Source chain, destination chain, amount, denom, recipient address | Transaction hash, transfer status, estimated delivery time |
| `celestia_ibc_relay` | Set up and monitor an IBC relayer | Chain pair, relayer configuration, key paths | Relayer status, packet relay metrics, error log |
| `celestia_ibc_diagnose` | Troubleshoot failed or stuck transfers | Transaction hash or channel ID | Root cause analysis: timeout, sequence mismatch, client expiry, relayer down, with suggested fix |

The diagnose tool is the highest-value tool in this set. IBC failures are notoriously
difficult to debug. A transfer can fail because the receiving chain's light client has
expired, because the relayer missed a packet, because the channel was closed during the
transfer, or because of a sequence number mismatch. The diagnose tool queries both chains,
inspects the channel state, checks the light client status, and identifies the specific
failure mode. It then suggests the fix: "the light client on Osmosis has expired; run
`hermes update client --host-chain osmosis-1 --client 07-tendermint-42` to update it."

The relay tool automates relayer setup, which is one of the most tedious manual processes
in the Cosmos ecosystem. Configuring a relayer requires: generating keys on both chains,
funding the relayer accounts, creating a configuration file with chain RPC endpoints and
gas prices, creating clients and connections, opening channels, and starting the relay
process. The tool handles all of this and monitors the relayer once it is running.

### F4: Cost Optimization

Blob submission costs on Celestia depend on three factors: blob size, the current gas
price (which fluctuates with block occupancy), and the number of blobs in the transaction.
For applications that submit data frequently --- rollups posting state roots, agents
publishing traces, monitoring systems recording metrics --- cost optimization directly
impacts operational viability.

**Tools:**

| Tool | Description | Input | Output |
|------|-------------|-------|--------|
| `celestia_cost_estimate` | Estimate blob submission cost before sending | Blob data (or size), target namespace | Estimated cost in TIA, current gas price, cost breakdown (base fee, blob gas, priority fee) |
| `celestia_cost_analyze` | Historical cost analysis | Time range, namespace (optional) | Total cost, cost per blob, cost trend, gas price history, cost comparison against network average |
| `celestia_cost_optimize` | Suggest and apply optimization strategies | Current submission pattern (frequency, sizes), cost target | Recommended strategy with projected savings: batching parameters, compression ratio, submission timing |

**Auto-batching:**

Beyond individual tools, tiagent implements an automatic batching layer for blob
submissions. When multiple tools or agent turns produce blobs destined for the same
namespace within a configurable time window, the batcher aggregates them into a single
transaction. This reduces per-transaction overhead (each transaction has a fixed base
cost regardless of how many blobs it contains) and takes advantage of Celestia's support
for multi-blob transactions.

The batching configuration is exposed in `tiagent.toml`:

```toml
[celestia.batching]
enabled = true
window_ms = 5000          # Aggregate blobs within this time window
max_batch_size_kb = 512   # Submit when batch reaches this size
max_batch_count = 16      # Submit when batch reaches this many blobs
compression = "zstd"      # Compress blobs before submission (none, zstd, lz4)
```

The cost optimizer uses historical data (from `celestia_cost_analyze`) to recommend
batching parameters. If blobs are small and frequent, it recommends a longer window and
higher batch count. If blobs are large and infrequent, it recommends disabling batching
to avoid latency. It also considers gas price patterns: if gas prices are consistently
lower during certain hours (e.g., off-peak), it can recommend deferred submission for
non-urgent data.

### F5: Validator and Governance

Celestia is a proof-of-stake network with an active governance process. Validators secure
the network by proposing and voting on blocks. Governance proposals can change network
parameters (minimum gas price, maximum blob size, staking rewards), upgrade the protocol,
or allocate community pool funds. Developers and operators who build on Celestia need
visibility into both.

**Tools:**

| Tool | Description | Input | Output |
|------|-------------|-------|--------|
| `celestia_validator_status` | Check validator health and performance | Validator address (optional; defaults to configured validator) | Uptime, missed blocks, commission rate, delegator count, voting power, jailing risk |
| `celestia_governance_proposals` | List governance proposals | Status filter (voting, passed, rejected, deposit), pagination | Proposal list with: ID, title, type, status, vote tally, deadline |
| `celestia_governance_analyze` | AI analysis of a governance proposal | Proposal ID | Plain-language summary, impact assessment (who benefits, who is affected, parameter changes), risk analysis, historical comparison to similar proposals |
| `celestia_staking_optimize` | Staking strategy recommendations | Available TIA balance, risk tolerance, time horizon | Recommended allocation across validators, expected yield, diversification analysis |

The governance analysis tool is where tiagent's AI capabilities provide unique value. A
governance proposal on Celestia might propose changing the minimum gas price from 0.002
utia to 0.01 utia. The tool does not just summarize the proposal text --- it calculates
the impact: "This would increase blob submission costs by approximately 5x at current
gas prices. Based on your submission history (average 50 blobs/day at 2 KB each), your
daily DA costs would increase from approximately 0.5 TIA to 2.5 TIA." This kind of
personalized impact analysis is impossible with a generic tool.

### F6: Light Node Management

Celestia's architecture is designed around light nodes. Unlike most blockchains, where
light clients are second-class citizens with limited functionality, Celestia light nodes
can verify data availability through Data Availability Sampling (DAS) without downloading
full blocks. This makes them lightweight enough to embed directly in an application.

tiagent embeds a Celestia light node via the `lumina-node` crate. This provides several
advantages over connecting to an external RPC endpoint:

- **No external dependency**: tiagent does not require a separately running Celestia node.
- **DA verification**: the embedded node performs DAS, so tiagent can verify that blobs
  it submits are actually available on the network.
- **Reduced latency**: blob reads hit a local node instead of traversing the network to
  a remote RPC.
- **Offline capability**: once synced, the node can serve cached data without network
  access.

**Tools:**

| Tool | Description | Input | Output |
|------|-------------|-------|--------|
| `celestia_node_status` | Light node sync and health status | None (operates on the embedded node) | Sync height, network head, sync percentage, peer count, DAS sampling stats, storage usage |
| `celestia_node_config` | Configure light node parameters | Configuration key-value pairs | Updated configuration, restart status if required |

**Auto-start behavior:**

The light node does not start when tiagent starts. It starts on demand --- when the first
Celestia tool is invoked. This avoids consuming resources (network bandwidth, disk space,
CPU for DAS) when the user is not using Celestia features. The startup sequence:

1. First Celestia tool call is intercepted by the tool dispatcher.
2. If the light node is not running, the dispatcher starts it with configuration from
   `tiagent.toml`.
3. The node connects to the Celestia network and begins header sync.
4. Once the node has synced enough headers to service the request (typically seconds for
   recent data), the original tool call is executed.
5. The node continues running in the background for subsequent tool calls.
6. On tiagent shutdown, the node is gracefully stopped and its state is persisted for
   faster startup next time.

Light node configuration in `tiagent.toml`:

```toml
[celestia.light_node]
enabled = true                          # Set to false to use external RPC only
network = "mocha"                       # mocha (testnet) or celestia (mainnet)
store_path = ".tiagent/celestia/store"  # Light node data directory
bootnodes = []                          # Additional bootstrap peers (optional)
```

### F7: Celestia Development Assistant

The development assistant is not a single tool but a capability that spans the entire
tool set. It provides context-aware coding assistance for developers building on
Celestia. Where a generic coding assistant generates boilerplate from documentation it
has seen during training, tiagent's Celestia assistant understands the SDK types, the
namespace primitives, the blob lifecycle, and the common patterns.

**Capabilities:**

| Capability | Description | Example |
|------------|-------------|---------|
| Code generation | Generate idiomatic Celestia SDK code for common patterns | "Generate a function that submits a batch of blobs to namespace X with retry logic" |
| Type assistance | Auto-import and use correct Celestia types | Agent knows that `Namespace::new_v0(bytes)` is the constructor, that `Blob::new(ns, data, ...)` takes a commitment type argument |
| Pattern library | Common integration patterns as reusable templates | DA client initialization, blob submission with confirmation, namespace query with proof verification |
| Documentation lookup | Query Celestia documentation on demand | "What is the maximum blob size on Celestia?" returns the current limit with a link to the spec |
| Error explanation | Explain Celestia-specific error messages | "ErrBlobSizeTooLarge" gets a plain-language explanation with the current limit and suggestions for splitting the data |

The development assistant is implemented through a combination of:

1. **Enriched system prompts**: when a task involves Celestia development, the prompt
   composer injects relevant SDK documentation, type definitions, and pattern examples
   into the system prompt.
2. **MCP tool for documentation**: a `celestia_docs_search` tool that queries a local
   index of Celestia documentation, returning relevant sections for the agent to
   reference.
3. **Code analysis tools**: the existing `search` and `file_read` tools, combined with
   Celestia-specific heuristics that identify when code is interacting with the DA layer
   and inject relevant context.

---

## 6. Requirements Table

**Audience key**: Requirements are tagged by who benefits.
- **Celestia** = only relevant to developers building on Celestia
- **General + Celestia** = benefits any developer who enables the Celestia layer (e.g., shared learning via DA)

### 6.1 P0 --- Must Have

These features are required for tiagent to claim Celestia integration layer status.
They are irrelevant to developers using tiagent as a standalone coding agent.

| ID | Feature | Audience | Requirement | Acceptance Criteria |
|----|---------|----------|------------|---------------------|
| CN-01 | F1 | Celestia | `celestia_rollup_scaffold` generates a compilable rollup project | Running the tool with framework="rollkit" produces a Go project that compiles and includes DA configuration for Mocha testnet |
| CN-02 | F1 | Celestia | `celestia_rollup_status` reports DA submission health | Running the tool against a live rollup returns sync status, latest DA height, submission rate, and error count |
| CN-03 | F2 | Celestia | `celestia_namespace_design` produces a namespace schema | Given a description of data types and access patterns, the tool outputs a namespace hierarchy with byte-level IDs and a rationale for the design |
| CN-04 | F2 | Celestia | `celestia_namespace_analyze` reports usage statistics | Given a namespace ID and time range, the tool returns blob count, size distribution histogram, and total cost |
| CN-05 | F6 | General + Celestia | Embedded light node starts on demand | The first Celestia tool call starts the light node automatically. The node syncs headers and the tool call completes within 30 seconds on Mocha testnet |
| CN-06 | F6 | General + Celestia | Light node state persists across restarts | Stopping and restarting tiagent resumes the light node from its last synced height, not from genesis |
| CN-07 | F6 | General + Celestia | `celestia_node_status` reports sync state | The tool returns sync height, network head, peer count, and DAS sampling statistics |

### 6.2 P1 --- Should Have

These features add significant value and are expected in the first post-MVP release.

| ID | Feature | Audience | Requirement | Acceptance Criteria |
|----|---------|----------|------------|---------------------|
| CN-08 | F1 | Celestia | `celestia_rollup_deploy` deploys to Mocha testnet | Running the tool with a scaffolded project deploys the rollup and verifies DA submissions are appearing on Mocha |
| CN-09 | F2 | Celestia | `celestia_namespace_monitor` watches for new blobs | Running the tool with a namespace ID produces a stream of new blob events. Alert conditions trigger callbacks |
| CN-10 | F2 | Celestia | `celestia_namespace_migrate` produces a migration plan | Given current and target namespace schemas, the tool outputs a step-by-step migration plan with estimated cost |
| CN-11 | F3 | Celestia | `celestia_ibc_channels` lists IBC channel state | Running the tool returns all IBC channels with state, counterparty info, and packet counts |
| CN-12 | F3 | Celestia | `celestia_ibc_transfer` executes a token transfer | Running the tool with source/destination/amount completes an IBC transfer on Mocha testnet |
| CN-13 | F3 | Celestia | `celestia_ibc_diagnose` identifies transfer failures | Given a failed transfer's tx hash, the tool identifies the root cause (timeout, client expiry, etc.) and suggests a fix |
| CN-14 | F4 | General + Celestia | `celestia_cost_estimate` estimates submission cost | Given blob data, the tool returns estimated cost in TIA with a breakdown of base fee, blob gas, and priority fee |
| CN-15 | F4 | General + Celestia | `celestia_cost_analyze` reports historical costs | Given a time range, the tool returns total cost, cost per blob, and a trend line |
| CN-16 | F4 | General + Celestia | Auto-batching aggregates blobs | When multiple blob submissions occur within the configured time window, they are batched into a single transaction |

### 6.3 P2 --- Nice to Have

These features are desirable but not required for the Celestia integration layer launch.

| ID | Feature | Audience | Requirement | Acceptance Criteria |
|----|---------|----------|------------|---------------------|
| CN-17 | F1 | Celestia | `celestia_rollup_upgrade` executes upgrade workflows | The tool generates an upgrade plan, executes it step by step, and supports rollback on failure |
| CN-18 | F3 | Celestia | `celestia_ibc_relay` sets up a relayer | The tool configures and starts an IBC relayer between two chains, handling key generation, client creation, and channel opening |
| CN-19 | F4 | General + Celestia | `celestia_cost_optimize` applies optimization strategies | Given historical data, the tool recommends batching parameters and projects cost savings |
| CN-20 | F5 | Celestia | `celestia_validator_status` reports validator health | The tool returns uptime, missed blocks, commission rate, and jailing risk for a given validator |
| CN-21 | F5 | Celestia | `celestia_governance_proposals` lists active proposals | The tool returns proposals filtered by status with vote tallies and deadlines |
| CN-22 | F5 | Celestia | `celestia_governance_analyze` provides impact analysis | The tool summarizes a proposal in plain language and calculates its impact on the user's operations |
| CN-23 | F5 | Celestia | `celestia_staking_optimize` recommends staking strategy | The tool analyzes validator performance and suggests allocation across validators |
| CN-24 | F7 | Celestia | Celestia development assistant enriches prompts | When tasks involve Celestia SDK code, the prompt composer automatically injects relevant type definitions and patterns |
| CN-25 | F7 | Celestia | `celestia_docs_search` queries documentation | The tool returns relevant documentation sections for a given query from a local index of Celestia docs |

---

## 7. Technical Design

### 7.1 Crate Structure

All Celestia-native features live in a single crate with feature-gated subsystems:

```
tiagent-celestia/
  src/
    lib.rs                  # Crate root, feature gate re-exports
    rpc.rs                  # Celestia RPC client (shared by all tools)
    light_node.rs           # Embedded lumina-node lifecycle management
    tools/
      mod.rs                # Tool registration and dispatch
      rollup.rs             # F1: celestia_rollup_* tools
      namespace.rs          # F2: celestia_namespace_* tools
      ibc.rs                # F3: celestia_ibc_* tools
      cost.rs               # F4: celestia_cost_* tools
      validator.rs          # F5: celestia_validator_*, celestia_governance_*, celestia_staking_*
      node.rs               # F6: celestia_node_* tools
      docs.rs               # F7: celestia_docs_search tool
    batching.rs             # Auto-batching layer for blob submissions
    templates/
      rollkit/              # Rollkit rollup project templates
      sovereign/            # Sovereign SDK rollup project templates
      opstack/              # OP Stack + Celestia DA project templates
```

The crate uses Cargo feature flags to control which subsystems are compiled:

```toml
[features]
default = ["rpc", "light-node", "rollup-tools", "namespace-tools"]
rpc = ["celestia-rpc", "celestia-types"]
light-node = ["lumina-node"]
rollup-tools = ["rpc"]
namespace-tools = ["rpc"]
ibc = ["rpc", "ibc-rs"]
cost = ["rpc"]
validator = ["rpc"]
full = ["rpc", "light-node", "rollup-tools", "namespace-tools", "ibc", "cost", "validator"]
```

### 7.2 MCP Tool Server

All Celestia-native tools are exposed as an MCP server: `tiagent-mcp-celestia`. This is
a separate binary that communicates with the tiagent harness over stdio. The server
registers all enabled tools (based on feature flags) and handles tool call dispatch.

The MCP server architecture means that Celestia tools are usable from any MCP-compatible
client, not just tiagent. A developer using Claude Desktop or Cursor can connect to
`tiagent-mcp-celestia` and get access to the same rollup scaffolding, namespace analysis,
and cost estimation tools.

MCP server configuration in `tiagent.toml`:

```toml
[mcp.servers.celestia]
command = "tiagent-mcp-celestia"
args = ["--config", "tiagent.toml"]
env = { CELESTIA_NODE_AUTH_TOKEN = "${CELESTIA_AUTH_TOKEN}" }
```

### 7.3 Light Node Integration

The embedded light node uses the `lumina-node` crate, which provides a Rust
implementation of the Celestia light node protocol. Integration points:

- **Startup**: the `LightNodeManager` struct owns the node lifecycle. It starts the node
  lazily (on first Celestia tool call) and stops it on tiagent shutdown.
- **Header sync**: the node syncs block headers from the network. This is required before
  any blob queries can be served.
- **DAS**: the node performs Data Availability Sampling on new blocks, verifying that
  blob data is actually available.
- **Blob retrieval**: blob read tools query the local node first, falling back to RPC
  only if the node does not have the requested data.
- **State persistence**: the node's header store and DAS state are persisted to
  `.tiagent/celestia/store/` for fast restart.

### 7.4 IBC Integration

IBC operations use the `ibc-rs` crate for protocol types and the `hermes` relayer for
actual packet relay. tiagent does not implement its own relayer --- it configures,
starts, and monitors an instance of Hermes.

The IBC tools interact with two chains simultaneously. Each tool call specifies the
source and destination chain, and tiagent maintains RPC connections to both. Chain
configuration is in `tiagent.toml`:

```toml
[celestia.ibc.chains.osmosis]
rpc_url = "https://rpc.osmosis.zone:443"
grpc_url = "https://grpc.osmosis.zone:443"
chain_id = "osmosis-1"
gas_price = "0.025uosmo"
```

### 7.5 Cost Estimation Model

Cost estimation uses on-chain data to compute blob submission costs:

```
total_cost = base_gas_cost + (blob_size * gas_per_byte * gas_price) + priority_fee
```

Where:
- `base_gas_cost` is the fixed cost per transaction.
- `gas_per_byte` is Celestia's blob gas rate (queried from the node).
- `gas_price` is the current market gas price (queried from recent blocks).
- `priority_fee` is an optional tip for faster inclusion.

The cost analyzer maintains a local cache of historical gas prices and blob sizes,
enabling trend analysis and cost projections without repeated RPC queries.

---

## 8. Success Metrics

### 8.1 Rollup Lifecycle

| # | Metric | How to verify |
|---|--------|--------------|
| SM-C1 | Developer can scaffold a rollup in one command | Run `celestia_rollup_scaffold` with framework="rollkit". The output project compiles and includes correct DA configuration |
| SM-C2 | Rollup deployment takes fewer than 5 tool calls | Deploy a scaffolded rollup to Mocha testnet. Count the number of agent turns required. Must be 5 or fewer |
| SM-C3 | Rollup status provides actionable health data | Run `celestia_rollup_status` against a live rollup. The output includes sync status, DA submission rate, and identifies any errors |

### 8.2 Namespace Management

| # | Metric | How to verify |
|---|--------|--------------|
| SM-C4 | Namespace design covers common use cases | Give the design tool 5 different data schemas (single-type, multi-type, versioned, hierarchical, multi-tenant). All 5 produce valid namespace schemas with correct byte-level IDs |
| SM-C5 | Namespace analysis covers 90% of common queries | Run the analyze tool against a namespace with known data. It correctly reports blob count, size distribution, and total cost |

### 8.3 Cost Optimization

| # | Metric | How to verify |
|---|--------|--------------|
| SM-C6 | Cost estimates are within 20% of actual cost | Estimate cost for 10 different blob sizes, then submit them. Compare estimated vs actual cost. At least 8 of 10 must be within 20% |
| SM-C7 | Auto-batching reduces cost by at least 20% | Submit 100 small blobs (1 KB each) with batching enabled vs disabled. Batched total cost must be at least 20% lower |

### 8.4 Light Node

| # | Metric | How to verify |
|---|--------|--------------|
| SM-C8 | Light node starts and syncs within 30 seconds | Time the interval between the first Celestia tool call and the tool call completing. Must be under 30 seconds on Mocha testnet |
| SM-C9 | Light node resumes from persisted state | Start tiagent, invoke a Celestia tool (triggering node start), stop tiagent, restart tiagent, invoke another Celestia tool. The second startup must be faster than the first |

### 8.5 IBC

| # | Metric | How to verify |
|---|--------|--------------|
| SM-C10 | IBC transfer completes end-to-end | Execute `celestia_ibc_transfer` between Mocha testnet and a connected chain. The transfer completes and the token balance on the destination chain increases |
| SM-C11 | IBC diagnosis identifies known failure modes | Simulate 3 failure scenarios (timeout, client expiry, sequence mismatch). The diagnose tool correctly identifies each one |

---

## 9. Milestones

The Celestia-native features build on the core harness MVP (12-prd-core-harness.md). Each
milestone assumes that the MVP milestones M1--M6 are complete.

### MC1: Light Node and Enhanced Namespace Tools

**Delivers**: Embedded light node, namespace design and analysis tools.

| Task | Description | Crate |
|------|-------------|-------|
| Embed lumina-node | Integrate lumina-node with lazy startup, state persistence, and graceful shutdown | `tiagent-celestia` |
| Node status tool | `celestia_node_status` reporting sync height, peers, DAS stats | `tiagent-celestia` |
| Node configuration tool | `celestia_node_config` for runtime parameter changes | `tiagent-celestia` |
| Namespace design tool | AI-assisted namespace schema design with domain knowledge | `tiagent-celestia` |
| Namespace analyze tool | Historical namespace usage analysis with statistics | `tiagent-celestia` |

**Exit criterion**: Invoking a Celestia tool starts the embedded light node. The node
syncs and serves blob queries. The namespace design tool produces valid schemas for at
least 3 different data models.

### MC2: Rollup Development Tools

**Delivers**: Rollup scaffolding, deployment, and status monitoring.

| Task | Description | Crate |
|------|-------------|-------|
| Project templates | Rollkit, Sovereign SDK, and OP Stack + Celestia DA templates | `tiagent-celestia` |
| Scaffold tool | `celestia_rollup_scaffold` generating compilable projects | `tiagent-celestia` |
| Deploy tool | `celestia_rollup_deploy` deploying to Mocha testnet | `tiagent-celestia` |
| Status tool | `celestia_rollup_status` consolidating health data | `tiagent-celestia` |

**Exit criterion**: A developer can scaffold a Rollkit project and deploy it to Mocha
testnet using only tiagent commands. `celestia_rollup_status` reports accurate DA
submission health.

### MC3: Cost Optimization and Batching

**Delivers**: Cost estimation, analysis, auto-batching.

| Task | Description | Crate |
|------|-------------|-------|
| Gas price tracking | Local cache of historical gas prices from on-chain data | `tiagent-celestia` |
| Cost estimate tool | `celestia_cost_estimate` with gas price lookup | `tiagent-celestia` |
| Cost analyze tool | `celestia_cost_analyze` with trend computation | `tiagent-celestia` |
| Auto-batching layer | Time-window and size-based blob aggregation | `tiagent-celestia` |
| Compression support | zstd and lz4 compression for blobs | `tiagent-celestia` |

**Exit criterion**: Cost estimates are within 20% of actual costs. Auto-batching reduces
the cost of 100 small blob submissions by at least 20%.

### MC4: IBC, Governance, and Development Assistant

**Delivers**: IBC tools, governance tools, documentation-enriched coding assistance.

| Task | Description | Crate |
|------|-------------|-------|
| IBC channel inspector | `celestia_ibc_channels` querying both chains | `tiagent-celestia` |
| IBC transfer tool | `celestia_ibc_transfer` executing cross-chain transfers | `tiagent-celestia` |
| IBC diagnostics | `celestia_ibc_diagnose` with root cause analysis | `tiagent-celestia` |
| Governance tools | Proposal listing, analysis, staking recommendations | `tiagent-celestia` |
| Documentation index | Local index of Celestia docs for `celestia_docs_search` | `tiagent-celestia` |
| Prompt enrichment | Celestia SDK context injection into system prompts | `tiagent-compose` |

**Exit criterion**: An IBC transfer between Mocha testnet and a connected chain
completes end-to-end through tiagent. Governance analysis produces personalized impact
assessments. The development assistant generates correct Celestia SDK code patterns.

---

## 10. Dependencies

### 10.1 Rust Crate Dependencies

| Crate | Used for | Feature gated? |
|-------|----------|----------------|
| `celestia-types` | Blob, namespace, and commitment types | No (always required) |
| `celestia-rpc` | RPC client for Celestia node communication | No (always required) |
| `lumina-node` | Embedded Celestia light node | Yes (`light-node` feature) |
| `ibc-rs` (`ibc` crate) | IBC protocol types (client state, channel, packet) | Yes (`ibc` feature) |
| `zstd` | Blob compression | Yes (`cost` feature) |
| `lz4_flex` | Alternative blob compression | Yes (`cost` feature) |
| `tantivy` | Full-text search for documentation index | Yes (dev assistant only) |

### 10.2 External Dependencies

| Dependency | Required for | Notes |
|------------|-------------|-------|
| Celestia Mocha testnet | Testing all Celestia-native features | Free to use; requires a funded testnet account for blob submission |
| Hermes relayer binary | IBC relay tool | Must be installed separately; tiagent configures and manages it |
| Rollkit CLI | Rollkit rollup scaffolding | Required only for Rollkit framework support |
| Sovereign SDK tooling | Sovereign SDK rollup scaffolding | Required only for Sovereign SDK framework support |

### 10.3 Relationship to MVP Dependencies

This PRD does not introduce any new P0 dependencies beyond what the core harness MVP
(12-prd-core-harness.md) already requires. The `celestia-types` and `celestia-rpc`
crates are already MVP dependencies for blob submit/get. The additional dependencies
(`lumina-node`, `ibc-rs`, compression crates) are all feature-gated and only compiled
when the corresponding feature is enabled.

---

## 11. Open Questions

### Q1: lumina-node maturity

**Question**: Is `lumina-node` stable enough for production embedding?

**Current assessment**: lumina-node is actively developed and used in Celestia's WASM
light node. However, embedding it as a library (rather than running it as a standalone
process) may surface edge cases that are not well-tested. The fallback is to run the
light node as a subprocess and communicate via IPC.

**Decision needed before**: MC1 implementation begins.

### Q2: Rollup template maintenance

**Question**: How do we keep rollup project templates current as Rollkit, Sovereign SDK,
and OP Stack evolve?

**Current leaning**: Templates are versioned and stored in the `tiagent-celestia` crate.
Each template specifies the SDK version it targets. When a new SDK version is released,
the template is updated in a new tiagent release. For rapid updates between tiagent
releases, templates can be fetched from a remote repository.

### Q3: IBC relayer lifecycle

**Question**: Should tiagent manage the Hermes relayer as a long-running subprocess, or
should it configure Hermes and let the user run it separately?

**Current leaning**: Manage it as a subprocess for the "set up IBC" workflow (start
relayer, open channel, verify first packet, report success). For production relaying,
tiagent generates the Hermes configuration and the user runs Hermes independently.
tiagent's role shifts to monitoring and diagnostics.

### Q4: Documentation freshness

**Question**: How does the documentation index for the development assistant stay current?

**Current leaning**: Ship a snapshot of Celestia documentation with each tiagent release.
Provide a `tiagent update-docs` command that fetches the latest documentation and
rebuilds the local search index. This avoids runtime network dependencies while keeping
documentation reasonably current.

### Q5: Cost model accuracy

**Question**: How accurate can cost estimates be given gas price volatility?

**Current leaning**: Use a 10-block rolling average of gas prices for estimates, with a
confidence interval rather than a point estimate. For example: "estimated cost: 0.05 TIA
(range: 0.03--0.08 TIA based on recent gas price volatility)." The analyze tool can
report how volatile gas prices have been over the requested time range, helping users
understand the uncertainty.

---

## 12. Related Documents

| Document | Relationship |
|----------|-------------|
| 01-vision-and-overview.md | Explains why Celestia-native integration is a core differentiator for tiagent |
| 02-architecture.md | Defines the universal loop, Signal type, and trait system that Celestia-native tools plug into |
| 03-crate-structure.md | Defines the `tiagent-celestia` crate where all code in this PRD lives |
| 04-celestia-integration.md | Deep design for Celestia DA integration: namespace schemas, blob formats, light node embedding, tiered storage --- the technical foundation this PRD builds on |
| 05-da-storage-patterns.md | Defines how agent data (traces, embeddings, fingerprints) maps to blobs and namespaces --- the data model that namespace management tools operate on |
| 06-tool-system.md | Defines the MCP tool architecture that Celestia-native tools use for registration and dispatch |
| 07-tracecommons-integration.md | Defines trace quality scoring; Celestia-native tools enable trace publication to the DA layer |
| 12-prd-core-harness.md | Companion PRD; defines the MVP that must ship before this PRD's features are relevant |
