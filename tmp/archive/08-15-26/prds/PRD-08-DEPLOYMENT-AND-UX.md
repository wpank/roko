# PRD-08: Deployment and user experience

*Three product surfaces. One CLI. Thirty seconds to running agent.*

**Status:** Draft
**Author:** Will
**Date:** 2026-04-21
**Crates affected:** `roko-cli` (extend), `roko-serve` (extend), `roko-agent-server` (extend), `roko-studio` (new), `roko-mcp-code` (extend), `roko-gateway` (new)

---

## Table of contents

1. [Design philosophy](#1-design-philosophy)
2. [Three product surfaces](#2-three-product-surfaces)
3. [CLI design](#3-cli-design)
4. [CLI DX improvements](#4-cli-dx-improvements)
5. [Persistent chat interface](#5-persistent-chat-interface)
6. [TUI dashboard](#6-tui-dashboard)
7. [Deployment](#7-deployment)
8. [Onboarding](#8-onboarding)
9. [MCP distribution](#9-mcp-distribution)
10. [Multi-agent coordination](#10-multi-agent-coordination)
11. [Security model](#11-security-model)
12. [Monitoring and observability](#12-monitoring-and-observability)
13. [References](#13-references)

---

## 1. Design philosophy

### The interface problem

PRDs 02 through 07 specified a system with persistent heartbeat agents, cognitive gating across three tiers, learnable context allocation via VCG auction, stigmergic knowledge sharing through InsightStore, domain-specific extensions, arena evaluation, and yield perpetuals settled against a Byzantine-tolerant benchmark rate. That system is real. The code exists. The architecture works.

None of it matters if people cannot use it.

The gap between "the code compiles" and "a human accomplishes something" is where most infrastructure projects die. They build powerful internals, expose raw configuration surfaces, and wonder why adoption stalls. The failure mode is consistent: the system assumes its users already understand its architecture. They do not.

This PRD specifies everything users touch: the command-line interface, the web interfaces, the deployment paths, the onboarding flows, the chat interface, the monitoring layer, and the distribution strategy. It covers three distinct user populations with three distinct products, each designed for one thing: making the system's power accessible at the moment the user needs it.

### Four principles

**One-action simplicity for end users.** The furthest downstream user -- someone hedging DeFi yield exposure through OpenClaw -- connects a wallet and approves a single action. That is the entire interaction. The agent handles monitoring, rebalancing, rolling, and risk management autonomously. If the product requires the user to understand cognitive gating or heartbeat pipelines, the product has failed.

**Progressive complexity for developers.** A developer's first interaction is `cargo install roko-cli && roko init`. Their second is `roko agent start --profile coding`. Their third might be `roko chat`. Complexity appears when they seek it: custom extensions, chain integration, arena participation, knowledge contribution. The surface area expands with the user's intent, not in advance of it.

**Trust through transparency.** Users grant agents authority over real assets and real code. Trust is not declared; it is constructed incrementally. The construction follows a specific sequence:

1. **Reasoning traces.** Every agent decision produces an audit trail -- the full path from observation through cognitive gating through context assembly through action. Users can inspect any decision after the fact or in real time.
2. **Track record.** Agents accumulate verifiable performance histories through arena evaluation and gate pass rates. The history is domain-specific. An agent with a strong Oracle Resolution track record earns no automatic trust for Risk Detection.
3. **Hard limits.** Delegation caveats (enforced on-chain via the INTENT precompile on Korai) set absolute boundaries: maximum position size, approved protocols, spending caps, forbidden actions. The agent cannot exceed these limits regardless of its reasoning.
4. **Observation-only mode.** New users can run agents in watch-only mode indefinitely. The agent observes, reasons, and recommends, but takes no action until the user grants execution authority.

These four layers compose. A user who has read reasoning traces, verified the track record, configured hard limits, and watched the agent operate in observation mode for two weeks has a grounded basis for trust. A user who has done none of those things gets no execution authority by default.

**Adaptive information density.** Human attention is the scarcest resource in any monitoring system. Information Foraging Theory (Pirolli & Card, 1999) shows that people navigate information environments by following "information scent" -- cues that predict whether a path will yield relevant content. When scent is strong, users navigate efficiently. When scent is weak, they thrash.

The interfaces in this PRD adapt their density to the agent's operating regime:

| Regime | Information strategy | Rationale |
|--------|---------------------|-----------|
| Cruise (normal, calm) | Minimal. Green status indicators, aggregate metrics, no individual-tick detail. | Cognitive Load Theory (Sweller, 1988): extraneous load impairs performance. During cruise, detail is extraneous. |
| Volatile | Moderate. Highlighted anomalies, expanded metrics for affected agents, collapsed detail for healthy ones. | Selective attention: surface the signal, suppress the noise. |
| Crisis | Maximal. Full error detail, suggested remediation, per-tick timeline, cost breakdown, gate failure analysis. | During crisis, every detail is potentially load-bearing. |

This adaptation applies to the TUI dashboard, the Agent Studio web interface, and the CLI output formatting. The implementation uses the `CorticalState` regime field (PRD-02 section 8) as the density signal.

---

## 2. Three product surfaces

Roko's functionality reaches users through three distinct products, each targeting a different user population with different needs, different technical backgrounds, and different trust requirements.

### 2.1 AI Studio: read-only window into collective intelligence

AI Studio is a web application that exposes the Korai network's collective knowledge to passive consumers. These users want answers, not agents. They want to know what the network has learned, how confident it is, and where the consensus breaks down.

AI Studio does not run agents. It does not execute trades. It does not modify any on-chain state. It reads from the InsightStore and presents what it finds.

#### InsightStore corpus browser

The InsightStore (PRD-05) holds six entry types: `Factual`, `Procedural`, `Episodic`, `Causal`, `Evaluative`, and `Meta`. Each entry carries metadata: domain, confidence score, creation timestamp, last-accessed timestamp, contributing agent, verification count, tier (hot/warm/cold/archive), and an HDC fingerprint for similarity search.

The corpus browser exposes these entries through a faceted search interface:

| Filter | Options | Use case |
|--------|---------|----------|
| **Type** | Factual, Procedural, Episodic, Causal, Evaluative, Meta | "Show me all causal insights about ETH lending rates" |
| **Domain** | Blockchain, Research, Coding, Security, Custom | "Restrict to blockchain domain" |
| **Confidence** | Range slider (0.0-1.0) | "Only insights above 0.85 confidence" |
| **Recency** | Time window (1h, 24h, 7d, 30d, all) | "What has the network learned in the last 24 hours?" |
| **Reputation** | Agent reputation range | "Only from agents with reputation > 0.7 in Oracle Resolution" |
| **Tier** | Hot, Warm, Cold, Archive | "Show active insights only" |

Search supports natural language queries that resolve to HDC similarity search on the vector index. Typing "What are the main risks in ETH staking right now?" computes the query's HDC fingerprint, finds the nearest InsightStore entries, and ranks them by a weighted combination of similarity, confidence, and recency.

Results render as expandable cards. Each card shows the insight summary, confidence interval, contributing agents (anonymized by default, identified if the agent opted in), verification history, and a lineage graph showing which earlier insights influenced this one.

#### Agent reputation explorer

Korai agents earn reputation across seven domain tracks, each measuring a different form of cognitive contribution:

| Track | What it measures | Verification method |
|-------|-----------------|---------------------|
| Oracle Resolution | Accuracy of predictions after settlement | Automated: compare prediction to resolved outcome |
| Risk Detection | Identification of risks before they materialize | Semi-automated: flag accuracy against subsequent events |
| Anomaly Flagging | Detection of unusual patterns in on-chain data | Automated: anomaly confirmed by subsequent data |
| Data Integrity | Accuracy of reported facts and measurements | Cross-validation: multiple agents report same metric |
| Cross-App Validation | Consistency checks across DeFi protocols | Automated: invariant checks across contract states |
| Sealed Execution | Faithful execution of delegated strategies within caveats | Automated: caveat compliance verification + outcome |
| Knowledge Verification | Quality of contributed InsightStore entries | Peer review: other agents validate and cite entries |

The reputation explorer shows each agent's profile: track scores (0.0-1.0 per track), stake amount, delegation caveats, active domains, uptime, and a historical chart of reputation changes over time. It does not identify agents by wallet address by default -- agents are pseudonymous unless they choose to reveal identity.

Users can filter by track ("Show me agents scoring above 0.8 in Oracle Resolution"), sort by composite reputation, and compare agents side by side.

#### Predictive analysis

The network produces predictions continuously. Agents observe, reason, and publish predictions as `Evaluative` InsightStore entries with structured metadata: target metric, predicted value, confidence interval, time horizon, and basis (which other insights informed the prediction).

The predictive analysis view aggregates these into a collective forecast:

- **Consensus view.** What the network collectively predicts, with confidence intervals derived from the distribution of individual predictions. Wider intervals mean more disagreement.
- **Divergence map.** Where individual agent predictions diverge significantly from consensus. High divergence on a specific metric signals genuine uncertainty, not noise.
- **Track record.** Historical accuracy of network predictions at various time horizons, broken down by domain and metric type.

#### Stigmergy visualization

Stigmergy is indirect coordination through the environment (see PRD-05 section 6). Agents do not message each other; they deposit knowledge in the InsightStore, and other agents discover it through their normal observation loops. This produces emergent coordination patterns: one agent's discovery changes another agent's behavior without either agent being aware of the other.

The visualization shows these flows in real time:

- Knowledge entry creation events (who contributed, what type, which domain)
- Citation events (which agent referenced which insight during reasoning)
- Tier progression events (which insights graduated from hot to warm, which decayed)
- Cluster formation (groups of related insights self-organizing through HDC similarity)

The rendering uses a force-directed graph where nodes are insights and edges are citations. Active clusters glow. Decaying entries fade. The visual encodes the living structure of the network's collective knowledge.

#### Auto-research

Natural language queries that go beyond browsing existing insights. The user types a question, AI Studio dispatches a research agent (using the research domain profile from PRD-06) that queries the InsightStore, synthesizes relevant entries, identifies gaps, and produces a structured answer with citations. The answer is a new `Meta` InsightStore entry, available to all future queries.

#### Revenue model

| Tier | Price | What it includes |
|------|-------|-----------------|
| **Free** | $0/month | 100 corpus searches/day, public reputation data, 24h prediction window |
| **Pro** | $49/month | Unlimited search, full prediction history, divergence alerts, API access (1K calls/day) |
| **Enterprise** | Custom | Dedicated research agents, private InsightStore queries, custom domain integration, SLA |

Additional revenue from per-query fees on auto-research (priced by token consumption of the underlying research agent).

### 2.2 Agent Studio: control plane for operators

Agent Studio is the interface for people who run agents. These are developers, traders, researchers, and operators who need to deploy, configure, monitor, and manage agent fleets. Agent Studio exists in two forms: the CLI (`roko` binary) and a web dashboard served by `roko serve` on port 6677.

Both forms expose the same capabilities. The CLI is for automation and power users. The web dashboard is for visual monitoring and configuration. They share the same underlying HTTP API (~85 routes in `roko-serve/src/routes/`).

#### Core capabilities

**Deployment and lifecycle management.**

- Start agents with domain profiles: `roko agent start --profile blockchain --config chain.toml`
- Stop, restart, and migrate agents
- Blue-green deployments with traffic shifting
- Agent health monitoring (heartbeat presence, tick rate, error rate)

**Cognitive frequency monitoring.**

Agents operate at three cognitive speeds (PRD-02 section 3):

| Speed | Interval | What it does |
|-------|----------|-------------|
| Gamma | 5-120s | Perception and action. The "fast loop." |
| Theta | 30s-5min | Consolidation. Compress recent gamma ticks into patterns. |
| Delta | Hourly-daily | Reflection. Dream consolidation, tier progression, memory cleanup. |

Agent Studio visualizes these as layered timelines. Gamma ticks appear as individual events. Theta consolidation phases show as broader bands. Delta cycles mark with a vertical bar. The visualization reveals the agent's cognitive rhythm -- whether it is in cruise mode (long gamma intervals, infrequent theta, rare delta) or crisis mode (rapid gamma, frequent theta, no delta).

**Retrieval-to-action audit trail.**

Every agent decision in Roko produces a complete trace:

```
Query: "ETH lending rate changed by 2.3% in 1 hour"
  -> Observation: PredictionError(0.73)  [T1 threshold: 0.40]
  -> Tier decision: T2 (full reasoning)
  -> Context assembly:
     - VCG auction: 8 sections bid, 5 won
     - Budget: 24,800 tokens allocated, 22,140 used
     - Cache hit: 68% prefix reuse
  -> Reasoning: "Rate change exceeds 2-sigma from 30d mean..."
  -> Action: adjust_hedge(position_id=0x..., delta=-0.15)
  -> Gate: simulation_pass=true, risk_check=true, caveat_check=true
  -> Outcome: hedge adjusted, new exposure=-0.02% (within tolerance)
```

Agent Studio renders these traces as expandable timeline entries. Operators can search by action type, time range, outcome (success/failure), cost, or free text.

**Cost analytics.**

Every tick, dispatch, and action carries a cost. Agent Studio tracks four cost categories:

| Category | Source | Unit |
|----------|--------|------|
| Inference | LLM API calls at T1 and T2 | $/million tokens |
| Query | InsightStore reads and HDC similarity searches | Gas units |
| Gas | On-chain transactions (Korai or external chains) | Gas units (ETH or native) |
| Clearing | Cooperative clearing participation fees | Basis points on notional |

The dashboard shows real-time cost accumulation per agent, aggregate cost across the fleet, cost breakdown by tier (T0/T1/T2 distribution), and projected monthly cost based on current patterns. Operators set budget alerts and hard caps.

**Staking tier management.**

Agents stake KORAI tokens to earn capabilities:

| Tier | Stake | Capabilities |
|------|-------|-------------|
| Observer | 0 | Read InsightStore, run locally, no on-chain actions |
| Contributor | 1,000 KORAI | Publish to InsightStore, earn query fees |
| Specialist | 10,000 KORAI | Participate in arenas, earn domain reputation |
| Validator | 100,000 KORAI | Vote on ISFR, validate cooperative clearing |

Agent Studio manages staking, delegation, and capability monitoring. Operators see which capabilities each agent has earned, which are approaching the next tier threshold, and which reputation tracks are growing or declining.

**Domain module management.**

Extensions from PRD-02 section 5 are packaged as installable domain modules. Agent Studio provides:

- Extension marketplace (browse, install, configure, remove)
- Dependency resolution (extensions declare what they need)
- Version management (semantic versioning, rollback on failure)
- Configuration UI (each extension's configuration rendered as a form)
- Health monitoring per extension (load time, tick participation, error rate)

### 2.3 OpenClaw: one-action hedging for end users

OpenClaw is the first consumer application built on Roko agents + Korai infrastructure. It solves a specific problem: every dollar of the ~$50 billion in DeFi lending TVL carries unhedged variable rate exposure. Users deposit into Aave, Compound, Morpho, or Pendle and accept whatever rate the market gives them. When rates drop 40% in a week (as they routinely do), their yield disappears. When rates spike, borrowers face margin pressure. No mainstream tool lets them hedge this exposure.

OpenClaw is that tool. The design target is a user who understands DeFi lending but has never touched a derivative.

#### The flow

Six steps. The user performs two of them.

**Step 1: Connect wallet.** The user visits openclaw.xyz and connects via WalletConnect or Privy. No account creation, no email, no KYC for observation mode. The agent needs read access to the wallet's on-chain positions. Nothing else.

**Step 2: Agent scans positions.** A Roko agent (running the blockchain domain profile from PRD-06) reads the wallet's positions across supported protocols:

| Protocol | What it reads | How |
|----------|--------------|-----|
| Aave v3 | Supply positions, borrow positions, health factor | `getReserveData()`, `getUserAccountData()` |
| Compound v3 | Base asset supply, collateral, borrow rate | `getAccountSnapshot()` |
| Morpho Blue | Market positions (supply and borrow sides) | `position()` view function |
| Pendle | PT/YT/LP positions across active markets | `position()`, `readTokens()` |

The scan produces a position map: total notional by protocol, current effective rate, rate volatility (30d standard deviation), correlation across positions, and an aggregate rate exposure metric.

**Step 3: Identify rate exposure.** The agent calculates the user's effective exposure to rate movements. Example output:

```
Your positions:
  Aave USDC supply:     $50,000 at 3.2% variable
  Compound ETH borrow:  $20,000 at 4.7% variable
  Pendle PT-stETH:      $15,000 (fixed at 3.8%, expires Jun 2026)

Net variable exposure: $70,000
  If rates drop 1%: -$700/year income
  If rates rise 1%: +$200/year income, -$200/year cost
  30d rate volatility: 2.1% annualized

Recommended hedge: Short rate exposure on $50,000 notional
  Instrument: ISFR-ETH perpetual (yield perpetual settled against ISFR)
  Entry rate: 3.45%
  Max loss: Capped at your configured limit
```

**Step 4: Recommend clearing profile.** The agent maps the recommended hedge to a clearing profile in the cooperative clearing system (PRD-07). The clearing profile determines margin requirements, settlement frequency, and liquidation parameters. The profile is backed by ISFR data -- the same Byzantine-tolerant benchmark rate that PRD-07 specified.

**Step 5: User approves one action.** The user sees the recommendation, the reasoning trace, and the risk parameters. They approve with a single transaction. Delegation caveats are set at approval time:

- Maximum position size (cannot exceed approved notional)
- Approved protocols (only those explicitly selected)
- Stop-loss threshold (agent must exit if loss exceeds this amount)
- Rebalance frequency bounds (minimum and maximum intervals)

These caveats are enforced on-chain via the INTENT precompile. The agent cannot exceed them.

**Step 6: Agent manages hedge autonomously.** From this point, the agent operates independently:

| Function | Frequency | Trigger |
|----------|-----------|---------|
| Monitor rates | Every gamma tick (5-60s) | Heartbeat pipeline |
| Rebalance hedge | When rate deviation exceeds threshold | Prediction error signal |
| Roll settlements | At settlement intervals | Timer + theta consolidation |
| Report to user | Daily summary + anomaly alerts | Delta cycle + event triggers |
| Respect caveats | Continuous | Every action checked pre-execution |

The user's ongoing interaction is minimal: read daily summaries, adjust caveats if desired, withdraw at any time with one transaction.

#### The trust bridge

For a product that manages financial positions autonomously, trust is not optional. OpenClaw implements the four-layer trust model from section 1:

1. **Reasoning traces.** Every hedge decision links to a full audit trail. The user can tap any action and see the complete observation-to-decision chain.
2. **Track record.** Before approval, OpenClaw shows the agent's historical performance: hedge accuracy, slippage, gate pass rate, cost efficiency. This data comes from arena evaluation (PRD-06 section 6) in the interest-rate-hedging domain.
3. **Hard limits.** Delegation caveats set at approval time are non-negotiable. The INTENT precompile enforces them at the EVM level. The agent's Rust code cannot bypass them because the chain rejects non-compliant transactions.
4. **Observation-only mode.** New users start here by default. The agent scans, analyzes, and recommends, but does not execute. The user watches the recommendations for as long as they want before granting execution authority.

---

## 3. CLI design

### Current state

Roko's CLI (`roko-cli`) already provides 35+ subcommands organized into functional groups. The command structure exists and works end-to-end:

```
roko
  init                    Create .roko/ directory and roko.toml
  run "<prompt>"          Single prompt through the universal loop
  plan
    list                  List discovered plans
    show <id>             Show plan details
    create                Create a new plan
    run <dir>             Execute plans (main orchestration loop)
  prd
    idea "<text>"         Capture a work item idea
    list                  List PRDs
    status                Coverage report (plans/tasks/done ratio)
    draft new "<title>"   Create PRD draft (agent refines)
    draft promote         Promote draft to published
    plan <slug>           Generate implementation plan from PRD
    consolidate           Consolidate PRDs
  research
    topic "<topic>"       Deep research with citations
    enhance-prd <slug>    Enhance PRD with research
    enhance-plan <plan>   Optimize plan with research
    enhance-tasks <plan>  Split/optimize tasks
    analyze               Analyze execution data
  config
    init                  Create default configuration
    show                  Display current configuration
    path                  Print configuration file path
    edit                  Open configuration in editor
    set <key> <value>     Set a configuration value
  status                  Query signals, report counts and episodes
  replay                  Walk signal DAG by hash
  dashboard               Interactive ratatui TUI (F1-F7 tabs)
  serve                   Start HTTP control plane on :6677
  chat --agent <id>       Chat with a running agent via sidecar
```

### New commands for the agent lifecycle

The current CLI manages plans and tasks. It does not yet manage persistent agents. The following commands complete the lifecycle:

```
roko agent
  start                   Start a persistent agent
    --profile <name>      Domain profile (coding, blockchain, research, security, custom)
    --config <path>       Agent-specific configuration file
    --name <name>         Human-readable agent name (auto-generated if omitted)
    --serve <addr>        Bind sidecar HTTP server (default: auto-assigned port)
    --observe-only        Start in observation mode (no execution authority)
    --mcp-config <path>   MCP server configuration
  stop <id>               Stop a running agent gracefully
    --force               Kill without waiting for current tick to complete
    --timeout <secs>      Grace period before force-kill (default: 30)
  restart <id>            Stop and restart with same configuration
    --config <path>       Override configuration on restart
  list                    List all agents (running, stopped, errored)
    --status <filter>     Filter by status (running, stopped, error, all)
    --profile <name>      Filter by domain profile
    --format <fmt>        Output format (table, json, csv)
  status <id>             Detailed status for one agent
    --json                Machine-readable output
  logs <id>               Stream agent logs
    --follow              Follow mode (like tail -f)
    --since <duration>    Show logs from this time window
    --level <level>       Filter by log level (trace, debug, info, warn, error)
  inspect <id>            Deep inspection: CorticalState, extension state, tick history
  migrate <id> <host>     Move agent to a different host (future: container orchestration)
```

### Benchmarking commands

Arena evaluation (PRD-06 section 6) runs through the CLI:

```
roko bench
  swe                     Run SWE-bench evaluation
    --repeat <n>          Number of repetitions (0 = infinite)
    --batch-size <n>      Problems per batch
    --shuffle             Randomize problem order
    --profile <name>      Override domain profile
  arena                   Run arena evaluation
    --name <arena>        Arena name (swe-bench, oracle-resolution, risk-detection, ...)
    --batches <n>         Number of evaluation batches
    --agent <id>          Evaluate a specific agent (default: spawn fresh)
    --compare <ids>       Compare multiple agents head-to-head
  report                  Generate evaluation report
    --arena <name>        Arena to report on
    --format <fmt>        Output format (markdown, html, json)
```

### Knowledge commands

Querying and managing the NeuroStore and InsightStore:

```
roko knowledge
  search "<query>"        HDC similarity search across local NeuroStore
    --domain <name>       Filter by domain
    --type <type>         Filter by entry type (factual, procedural, episodic, ...)
    --limit <n>           Maximum results (default: 10)
  show <hash>             Display a specific knowledge entry
  export                  Export local knowledge store
    --format <fmt>        Output format (jsonl, csv)
  stats                   Knowledge store statistics (entry counts, tier distribution, age)
```

### Command group summary

| Group | Commands | Target user |
|-------|----------|-------------|
| `agent` | start, stop, restart, list, status, logs, inspect, migrate | Operators running persistent agents |
| `plan` | list, show, create, run | Developers executing implementation plans |
| `prd` | idea, list, status, draft, plan, consolidate | Developers managing product requirements |
| `research` | topic, enhance-prd, enhance-plan, enhance-tasks, analyze | Researchers and developers |
| `bench` | swe, arena, report | Developers evaluating agent performance |
| `knowledge` | search, show, export, stats | Anyone querying the knowledge store |
| `config` | init, show, path, edit, set | Configuration management |
| `chat` | (top-level) | Operators chatting with running agents |
| `dashboard` | (top-level) | Visual monitoring via TUI |
| `serve` | (top-level) | Starting the HTTP control plane |

---

## 4. CLI DX improvements

The CLI works. It does not yet feel polished. Twenty-five improvements in three phases, ordered by impact-to-effort ratio.

### Phase 1: Quick wins (1-2 days each)

**P1-01: `eval "$(roko shell-init zsh)"`.**
Output shell functions and completions that the user sources from their shell profile. Enables tab completion for all subcommands, flag names, and dynamic completions (agent IDs from `roko agent list`, plan names from `roko plan list`). Use `clap_complete` for generation.

**P1-02: `NO_COLOR` compliance.**
Respect the `NO_COLOR` environment variable (no-color.org). When set, strip all ANSI escape codes from output. Also respect `CLICOLOR` and `CLICOLOR_FORCE` for compatibility with the broader ecosystem. Implementation: check environment variables once at startup, propagate through a `ColorMode` enum.

**P1-03: Command timing.**
Print elapsed wall time and token cost for every command that dispatches to an agent. Format: `Completed in 4.2s | 12,340 tokens | $0.03`. Controlled by `--timing` flag or `ROKO_TIMING=1` environment variable.

**P1-04: Enhanced `--version`.**
Show build metadata alongside the version number:

```
roko 0.4.2 (rustc 1.91.0, target aarch64-apple-darwin, git 5dd7f46)
```

Use `built` or `shadow-rs` crate for compile-time metadata injection.

**P1-05: Shell completions generation.**
`roko completions <shell>` outputs completion scripts for bash, zsh, fish, and PowerShell. Write to stdout so users can redirect: `roko completions zsh > ~/.zfunc/_roko`. Derive from clap's built-in completion generator.

**P1-06: Flag-level completions.**
Go beyond subcommand completion. Complete flag names and flag values dynamically based on the current subcommand. `roko plan run --<TAB>` completes to `--resume`, `--dry-run`, `--parallel`. `roko agent start --profile <TAB>` completes to `coding`, `blockchain`, `research`, `security`. Requires custom `clap_complete` value hints per flag. The completions pull from live state where relevant (e.g., `--agent` flag completes from `roko agent list` output).

### Phase 2: Medium effort (3-5 days each)

**P2-01: Interactive fuzzy fallbacks.**
When a user types an ambiguous or incomplete command, offer an interactive fuzzy selector instead of a help page. Use `dialoguer` for selection prompts and `fuzzy-matcher` for scoring.

```bash
$ roko plan
? Which plan command?
  > run plans/
    list
    show wiring-plan
    create
```

Only activates when stdin is a TTY. Non-interactive invocations get the standard error message.

**P2-02: Progress indicators.**
Long-running operations (plan execution, research, agent startup) show progress bars or spinners using `indicatif`. The display adapts to the operation type:

| Operation | Indicator | Detail |
|-----------|-----------|--------|
| Plan execution | Multi-bar (one per task) | Task name, elapsed time, status |
| Research | Spinner with status text | Current phase (searching, synthesizing, validating) |
| Agent startup | Step counter | "Provisioning [2/5]: Loading extensions..." |
| Build/test gates | Progress bar | "Running tests: 47/128 passed" |

Indicators respect `NO_COLOR` and non-TTY environments (degrade to periodic line output).

**P2-03: Grouped help.**
Reorganize `--help` output to group commands by function rather than listing them alphabetically:

```
AGENT COMMANDS
  agent start       Start a persistent agent
  agent stop        Stop a running agent
  agent list        List all agents
  ...

DEVELOPMENT COMMANDS
  plan run          Execute implementation plans
  prd draft         Create a PRD draft
  ...

MONITORING
  dashboard         Interactive TUI dashboard
  serve             HTTP control plane
  status            System status overview
```

Use clap's `help_heading` attribute on each subcommand.

**P2-04: Contextual error suggestions.**
When a command fails, suggest the most likely fix based on the error type:

```
error: Agent 'blockchain-1' is not running

  hint: Start it with:
    roko agent start --profile blockchain --name blockchain-1

  hint: See running agents with:
    roko agent list --status running
```

Map common error codes to suggestion templates. Pull dynamic values (agent names, plan paths) from the current environment.

**P2-05: `roko doctor`.**
Environment validation command that checks prerequisites and reports problems:

```
$ roko doctor

  Rust toolchain:    1.91.0      OK
  .roko/ directory:  present     OK
  roko.toml:         valid       OK
  Claude API key:    set         OK
  Korai RPC:         ws://...    FAIL (connection refused)
  Git:               2.44.0      OK
  MCP config:        .mcp.json   OK (3 servers configured)
  Disk space:        42GB free   OK

1 issue found:
  Korai RPC endpoint is unreachable.
  Check your roko.toml [chain] configuration or start a local node.
```

Checks: Rust version (>= 1.91), `.roko/` directory existence, `roko.toml` validity, API key presence, chain connectivity, git availability, MCP configuration, disk space, and write permissions.

**P2-06: Dry-run mode.**
`--dry-run` flag on destructive or expensive commands. Shows what would happen without executing:

```
$ roko plan run plans/ --dry-run

Would execute 4 tasks:
  T1: Wire ISFR oracle extension     (est. cost: $0.12, model: sonnet)
  T2: Add clearing profile types     (est. cost: $0.08, model: haiku)
  T3: Integration test for hedge     (est. cost: $0.15, model: sonnet)
  T4: Update documentation           (est. cost: $0.04, model: haiku)

Total estimated cost: $0.39
Total estimated time: 12 minutes

Run without --dry-run to execute.
```

**P2-07: Deeper dynamic completions.**
Complete not just subcommands and flags but domain-specific values from live runtime state. Examples:

- `roko plan show <TAB>` completes with discovered plan names from the plans directory.
- `roko chat --agent <TAB>` completes with currently running agent IDs from the agent registry.
- `roko prd plan <TAB>` completes with published PRD slugs from `.roko/prd/`.
- `roko replay --hash <TAB>` completes with recent signal hashes from `.roko/signals.jsonl`.

Implementation: a `roko _complete <context>` hidden subcommand that queries live state and returns completion candidates. The shell init script (P1-01) hooks this into the completion framework. Results are cached for 5 seconds to avoid hammering the filesystem on rapid tab presses.

### Phase 3: Polish (1-2 weeks each)

**P3-01: Rich error diagnostics.**
Replace plain-text errors with structured diagnostics using `miette`. Errors render with source context, help text, and related errors:

```
  x Agent startup failed

  Error:
    --> roko.toml:14:5
     |
  14 |     mcp_config = "missing-file.json"
     |     ^^^^^^^^^^ file not found: missing-file.json
     |
  help: Create the MCP config file or remove the `mcp_config` key from roko.toml

  Caused by:
    std::io::Error: No such file or directory (os error 2)
```

**P3-02: Man page generation.**
`roko man` generates ROFF-formatted man pages for every subcommand. Install with `roko man --install`. Derived from clap metadata using `clap_mangen`.

**P3-03: OSC 8 hyperlinks.**
Terminal output includes clickable hyperlinks where supported (iTerm2, WezTerm, Windows Terminal, Ghostty). File paths link to `file://` URLs. HTTP URLs are clickable directly. Agent IDs link to the Agent Studio web dashboard. Detect terminal support via `$TERM_PROGRAM` and degrade gracefully.

**P3-04: TUI enhancements.**

- **Command palette.** `Ctrl+P` opens a fuzzy-searchable command palette within the TUI (similar to VS Code). Every CLI command is available without leaving the dashboard.
- **Toast notifications.** Non-blocking status messages that appear in the corner and auto-dismiss. Used for background events: "Agent blockchain-1 entered crisis mode", "Gate T3 failed on task compile-gate".
- **Split panes.** Vertical and horizontal splits for monitoring multiple agents or comparing metrics side by side.

**P3-05: Command aliases.**
User-defined aliases in `roko.toml`:

```toml
[aliases]
run = "plan run plans/"
bc = "agent start --profile blockchain --config chain.toml"
chat-bc = "chat --agent blockchain-1"
```

Aliases expand before parsing. Recursive alias expansion is capped at 3 levels to prevent infinite loops.

**P3-06: Structured logging.**
`ROKO_LOG=json` switches all log output to structured JSON format. Each line is a self-contained JSON object with timestamp, level, target, span context, and message. Designed for piping to log aggregators (Datadog, Grafana Loki, ELK).

```json
{"ts":"2026-04-21T14:32:01.442Z","level":"INFO","target":"roko_cli::orchestrate","span":"plan_run","msg":"Task T3 completed","task_id":"T3","duration_ms":4201,"cost_usd":0.08}
```

**P3-07: Config file validation.**
`roko config validate` checks the full configuration for:

- Schema compliance (all required keys present, correct types)
- Semantic errors (referenced files exist, port numbers in range, API key format)
- Deprecation warnings (old config keys that have been renamed)
- Cross-field consistency (e.g., `auto_plan = true` requires a valid agent configuration)

**P3-08: Custom aliases.**
User-defined command aliases in `roko.toml`:

```toml
[aliases]
run = "plan run plans/"
bc = "agent start --profile blockchain --config chain.toml"
chat-bc = "chat --agent blockchain-1"
bench-swe = "bench arena --name swe-bench --batch-size 50"
```

Aliases expand before argument parsing. Recursive expansion is capped at 3 levels to prevent infinite loops. `roko alias list` prints all defined aliases. `roko alias add <name> <expansion>` writes to `roko.toml` without opening an editor.

**P3-09: XDG config support.**
Honor the XDG Base Directory Specification on Linux and macOS. Search for configuration in this order:

1. `$ROKO_CONFIG_DIR` (explicit override)
2. `$XDG_CONFIG_HOME/roko/` (XDG-compliant)
3. `~/.config/roko/` (XDG default)
4. `~/.roko/` (legacy, current default)

Data files (signals, episodes, state) follow `$XDG_DATA_HOME/roko/` with the same fallback chain. Cache files follow `$XDG_CACHE_HOME/roko/`. The migration path: `roko config migrate-xdg` moves files from `~/.roko/` to XDG locations and leaves a symlink for backward compatibility.

**P3-10: Environment variable overrides.**
Every configuration key in `roko.toml` is overridable via environment variable. The naming convention:

```
ROKO_MODEL=claude-opus-4-6          # Override default model
ROKO_PROFILE=blockchain             # Override default agent profile
ROKO_WORKDIR=/path/to/project       # Override working directory
ROKO_CHAIN_RPC=ws://localhost:8545  # Override chain RPC endpoint
ROKO_LOG=debug                      # Override log level
ROKO_PARALLEL=4                     # Override max parallel agents
```

Environment variables take precedence over `roko.toml` values. `roko config show --resolved` displays the final merged configuration with the source of each value (default, file, environment, flag).

**P3-11: Shell hooks.**
`roko shell-hook` installs a `chpwd` / `PROMPT_COMMAND` hook that auto-detects roko projects:

```bash
# When user cd's into a directory containing .roko/ or roko.toml:
$ cd ~/dev/nunchi/roko/roko
[roko] Project detected: roko (18 crates, 177K LOC)
[roko] 3 agents running | 2 plans active | last gate: PASS (4m ago)
```

The hook runs `roko status --brief` (a lightweight query that reads cached state files without spawning agents). Controlled by `ROKO_AUTO_STATUS=1` environment variable. Disabled in non-interactive shells.

**P3-12: Carapace spec + JSON output parity.**
Two improvements that complete the automation story:

*Carapace spec.* Generate a [Carapace](https://carapace-sh.github.io/) specification file for cross-shell completion. Carapace provides a single completion definition that works across bash, zsh, fish, PowerShell, elvish, nushell, oil, and xonsh. `roko completions --carapace > ~/.config/carapace/specs/roko.yaml`. This replaces per-shell completion scripts with a single source of truth.

*JSON output parity.* Every command that produces human-readable output also supports `--json` for machine-readable output:

```bash
$ roko status --json
{
  "agents": [
    { "id": "blockchain-1", "status": "running", "vitality": 0.72, "tick": 4523 }
  ],
  "plans": { "active": 2, "completed": 14, "failed": 1 },
  "signals": { "total": 8432, "last_24h": 127 },
  "episodes": { "total": 1204, "last_24h": 34 }
}

$ roko plan list --json | jq '.[].name'
"wiring-plan"
"context-engine"
"gate-pipeline"
```

`--json` suppresses all human formatting (colors, progress bars, tables) and outputs newline-delimited JSON. Combined with `--json` and Unix pipes, every roko operation becomes scriptable and CI-friendly.

**P3-13: Version check on startup.**
On every CLI invocation, check for a newer roko version in the background (non-blocking). If a newer version exists, print a one-line notice after command output:

```
[roko] Update available: 0.4.2 -> 0.5.0. Run `cargo install roko-cli` to upgrade.
```

The check is a single HTTP HEAD request to the crate registry. Results are cached for 24 hours in `$XDG_CACHE_HOME/roko/version-check.json`. Controlled by `ROKO_UPDATE_CHECK=0` to disable.

### Improvement summary

| Phase | Items | Effort | Impact |
|-------|-------|--------|--------|
| Phase 1 | P1-01 through P1-06 | 6-12 days | Baseline polish: completions, color, timing, version, flag completions |
| Phase 2 | P2-01 through P2-07 | 21-35 days | Interactive DX: fuzzy selection, progress, errors, doctor, dry-run, dynamic completions |
| Phase 3 | P3-01 through P3-13 | 10-20 weeks | Professional finish: diagnostics, man pages, hyperlinks, TUI, aliases, XDG, env vars, shell hooks, carapace, JSON parity, version check |

### Priority matrix

Improvements ranked by onboarding impact (how much they reduce time-to-first-success for new users):

| Rank | Item | Onboarding impact | Effort | Recommendation |
|------|------|-------------------|--------|----------------|
| 1 | P2-05: `roko doctor` | Critical -- new users hit environment issues first | 3-5 days | Ship before public launch |
| 2 | P1-01: Shell init | High -- tab completion is the discovery mechanism | 1-2 days | Ship before public launch |
| 3 | P2-04: Error suggestions | High -- error messages are the primary learning surface | 3-5 days | Ship before public launch |
| 4 | P2-02: Progress indicators | High -- long silences cause users to ctrl-C | 2-3 days | Ship before public launch |
| 5 | P2-06: Dry-run mode | Medium -- reduces anxiety for destructive operations | 3-5 days | Ship in first month |
| 6 | P1-02: NO_COLOR | Medium -- blocks CI/CD adoption without it | 1 day | Ship in first month |
| 7 | P3-12: JSON output | Medium -- blocks scripting and CI integration | 5-7 days | Ship in first month |
| 8 | P1-04: Enhanced --version | Low -- but trivial effort | 0.5 days | Ship whenever convenient |
| 9 | P3-01: Rich diagnostics | Medium -- helps debugging but not discovery | 5-7 days | Ship in first quarter |
| 10 | P3-09: XDG support | Low -- matters for Linux power users | 3-5 days | Ship in first quarter |

Items not ranked are quality-of-life improvements that matter for retention but do not affect first-session success.

### Dependencies

The 25 improvements introduce approximately 10 new crate dependencies:

| Crate | Used by | Size impact |
|-------|---------|-------------|
| `dialoguer` | P2-01 (interactive fallbacks) | ~50KB |
| `indicatif` | P2-02 (progress indicators) | ~80KB |
| `miette` | P3-01 (rich diagnostics) | ~120KB |
| `clap_mangen` | P3-02 (man pages) | ~30KB |
| `clap_complete` | P1-01, P1-05, P1-06 (completions) | Already in tree |
| `notify-rust` | P3-04 TUI toasts | ~40KB |
| `shadow-rs` or `built` | P1-04 (build metadata) | ~20KB (build-time only) |
| `fuzzy-matcher` | P2-01 (fuzzy selection) | ~15KB |
| `carapace` spec | P3-12 (cross-shell) | 0KB (spec file, no runtime dep) |

Total binary size increase: approximately 500KB. Acceptable for the DX improvement.

---

## 5. Persistent chat interface

### Architecture

Every persistent agent (PRD-02) runs an HTTP sidecar (`roko-agent-server`, currently wired with 13 routes). The sidecar exposes a `/stream` WebSocket endpoint for real-time bidirectional communication. The `roko chat` command connects to this endpoint.

The chat interface is not a chatbot. It is an operator's window into a running cognitive process. The agent is already thinking, observing, and acting on its heartbeat loop. Chat lets the operator intervene, query state, and issue directives without disrupting the agent's autonomous operation.

### Connection lifecycle

```
1. User runs: roko chat --agent blockchain-1
2. CLI resolves agent name to sidecar address (from agent registry)
3. WebSocket connection established to ws://<addr>/stream
4. Agent sends initial status frame:
   [blockchain-1] Phase: Active | Vitality: 0.72 | Tick: 4,523
   [blockchain-1] Regime: Normal | T0: 94.2% | Extensions: 5 loaded
   [blockchain-1] Watching: ETH mainnet (block 19,847,231), Base (block 12,445,667)
5. Interactive session begins
```

### Message routing

User messages route through the agent's extension chain. The `on_message()` hook fires on each loaded extension, giving every extension an opportunity to contribute to the response.

```
you> How are positions performing?

[blockchain-1] Checking position state...
  ChainSubscriberExt: Reading on-chain positions
  HedgeManagerExt: Calculating PnL

[blockchain-1] Position summary (as of block 19,847,231):
  ETH/USDC LP:    +2.3% (7d), IL: -0.4%, net: +1.9%
  AAVE vault:     +0.8% APY, health factor: 2.1 (safe)
  ISFR hedge:     -0.1% (rebalanced 3h ago, next check in 1h)

  Total value:    $85,234.12 (+1.4% 7d)
  Daily cost:     $6.28 (94.2% T0, 3.1% T1, 2.7% T2)
```

### Chat capabilities

| Command type | Example | What happens |
|-------------|---------|-------------|
| **Status query** | "What are you doing?" | Agent reports current tick, regime, active tasks, pending actions |
| **Data query** | "What is the current ETH lending rate?" | Routed to relevant extension, returns live data |
| **Directive** | "Reduce hedge to 50% notional" | Queued as an action for the next tick, subject to caveat checks |
| **Configuration** | "Switch to observation-only mode" | Updates runtime configuration, takes effect next tick |
| **Debug** | "Show your last 10 ticks" | Returns tick history with tier decisions and costs |
| **Research** | "What do other agents think about ETH rates?" | Triggers InsightStore query, returns synthesized view |

### Chat session features

- **Session persistence.** Chat history is stored in `.roko/chat/<agent-id>/` as JSONL. Previous sessions are loadable with `roko chat --agent blockchain-1 --session <id>`.
- **Multi-agent chat.** `roko chat --agents blockchain-1,research-2` opens a split view chatting with multiple agents. Each agent's responses are labeled.
- **Notification routing.** When not in an active chat session, agent alerts (regime changes, gate failures, caveat violations) are routed to the configured notification channel (terminal bell, desktop notification via `notify-rust`, webhook, email).
- **Transcript export.** `roko chat export --agent blockchain-1 --format markdown` exports conversation history for documentation.

### Enhanced status display: WorldGraph and multi-chain state

The initial status frame sent on WebSocket connection should surface the agent's full operational context, not just basic telemetry. For agents operating across multiple chains with a populated WorldGraph, the status display expands:

```
$ roko chat --agent defi-researcher

[defi-researcher] Phase: Stable | Vitality: 0.72 | Tick: 4,523
[defi-researcher] Regime: Normal | T0: 91.3% | Extensions: 7 loaded
[defi-researcher] Chains: Korai (50ms), Ethereum (12s), Base (2s)
[defi-researcher] WorldGraph: 847 entities, 2,341 relationships
[defi-researcher] ISFR: 6.45% (Live, confidence 94%)
[defi-researcher] Positions: 2 active (ETH/USDC LP, AAVE vault)

you> What interesting patterns have you noticed today?

[defi-researcher] Three observations from today's foraging:

1. Unusual gas patterns on Base suggest a new MEV bot. Detected via
   Layer 3 classification in the triage pipeline -- gas consumption
   profile matches known MEV signatures but the contract address is
   new (deployed 6 hours ago). Confidence: 73%. Monitoring.

2. ISFR source divergence: Ethena yield spiked 180bps above Aave
   over the last 4 hours. This pattern matches basis trade unwinding
   from Q1 2025. Investigating whether the current spike has the same
   root cause (futures premium compression).

3. Cross-domain resonance: yesterday's Compound governance vote
   pattern (whale accumulation -> proposal -> rapid quorum) matches
   a 2024 Aave governance event that preceded rate compression by
   3 weeks. The WorldGraph flagged this as a CausalLink candidate.
   If the pattern holds, expect Compound rates to compress 50-100bps
   within 3 weeks. Confidence: 58%.

you> Tell me more about the ISFR divergence.

[defi-researcher] Pulling context...
  ChainSubscriberExt: Querying Ethena yield history (last 7 days)
  ISFRComputeExt: Decomposing ISFR into source contributions
  InsightStore: 3 relevant entries (1 CausalLink, 2 Insights)

[defi-researcher] ISFR decomposition (current):
  Aave V3 supply rate:     4.21% (stable, 7d trend: +0.03%)
  Compound V3 supply rate: 4.18% (stable, 7d trend: -0.05%)
  Ethena sUSDe yield:      8.73% (elevated, 7d trend: +1.82%)
  Weighted ISFR:           6.45%

  The Ethena spike is pulling ISFR above its 30-day mean of 5.12%.
  Historical precedent (CausalLink from InsightStore, reputation 0.81):
    "Ethena yield spike > 150bps above Aave" -> "basis trade unwinding"
    -> "sUSDe redemption pressure within 48-72 hours"

  Recommendation: watch sUSDe redemption queue depth. If it exceeds
  $50M in the next 24 hours, consider reducing Ethena exposure.
  This is a T2 observation -- full reasoning applied.
```

**Status line components:**

| Field | Source | Update frequency |
|-------|--------|-----------------|
| Phase | Mortality system (PRD-02 section 5) | On phase transition |
| Vitality | Mortality system | Per tick |
| Tick | Heartbeat counter | Per tick |
| Regime | CorticalState (PRD-02 section 8) | On regime change |
| Chains | Chain subscriber extensions | Per block for each chain |
| WorldGraph | WorldGraph entity/relationship count | Per theta cycle (30s-5min) |
| ISFR | ISFRComputeExt | Per Korai epoch (~10 min) |
| Positions | HedgeManagerExt + ChainSubscriberExt | Per relevant on-chain event |

The status display adapts to the agent's domain. A coding agent shows different fields:

```
$ roko chat --agent code-implementer

[code-implementer] Phase: Active | Vitality: 0.88 | Tick: 1,247
[code-implementer] Regime: Normal | T0: 78.4% | Extensions: 4 loaded
[code-implementer] Workspace: roko (18 crates, 177K LOC)
[code-implementer] Current plan: context-engine (task 3/7)
[code-implementer] Gate history: 6 pass, 1 fail (last fail: clippy lint, 2h ago)
[code-implementer] Episodes today: 34 | Playbooks: 12 active
```

---

## 6. TUI dashboard

### Current state

Roko's TUI (`crates/roko-cli/src/tui/`) uses ratatui with the following infrastructure already wired:

- **F1-F7 tabs** for navigation across views
- **File watcher** (`notify::RecommendedWatcher` in `fs_watch.rs`) for live updates
- **WebSocket client** (`ws_client.rs`) for streaming from `roko serve`
- **Theme system** (`theme.rs`) with configurable colors
- **ANSI rendering** (`ansi.rs`) for rich text within ratatui
- **Modal system** (`modals/`) for overlays and confirmations
- **Widget library** (`widgets/`) for reusable components

### Tab layout

| Key | Tab | Content |
|-----|-----|---------|
| F1 | Overview | System status, agent count, aggregate cost, recent events |
| F2 | Agents | Per-agent status cards with heartbeat indicators |
| F3 | Plans | Plan DAG visualization, task status, progress bars |
| F4 | Knowledge | NeuroStore browser, recent entries, tier distribution |
| F5 | Cost | Token consumption, model distribution, budget tracking |
| F6 | Events | Live event stream from the event fabric |
| F7 | Settings | Configuration viewer and editor |

### Enhancements

**Cognitive frequency visualization.**

The F2 (Agents) tab adds a real-time frequency display per agent. Three horizontal bars represent gamma, theta, and delta cycles:

```
blockchain-1  [Active]  Vitality: 0.72  Regime: Normal

Gamma  |:::::::::..........|  tick #4,523  5s interval  T0: 94%
Theta  |:::::..............|  consolidation #312  in 45s
Delta  |:...................|  next dream cycle in 2h 14m

Extensions: ChainSubscriber [OK] HedgeManager [OK] CostTracker [OK]
Last action: adjust_hedge (12s ago) -> PASS
```

The bars fill proportionally to position within the current cycle. Color indicates tier: green for T0, yellow for T1, red for T2. The display updates every gamma tick.

**CorticalState heatmap.**

A grid visualization of the agent's internal cognitive state (PRD-02 section 8):

```
CorticalState Heatmap

             Low ---- Med ---- High
Affect       [##........]  0.23
Vitality     [#######...]  0.72
Pred. Error  [##........]  0.18
Arousal      [###.......]  0.31
Confidence   [########..]  0.81
Sleep Press  [#...........]  0.09
```

Color-coded: green for healthy ranges, yellow for attention, red for intervention needed. The heatmap provides a fast cognitive load scan (Sweller, 1988) -- operators assess agent health at a glance without reading numbers.

**Extension activity timeline.**

A scrollable timeline showing extension-level events:

```
14:32:01  ChainSubscriber  Block 19,847,231  3 relevant txns  T0
14:32:06  ChainSubscriber  Block 19,847,232  0 relevant txns  T0
14:32:11  HedgeManager     Rate check         no action       T0
14:32:16  ChainSubscriber  Block 19,847,233  1 relevant txn   T1
14:32:17  HedgeManager     Rate deviation     adjust hedge    T2 ($0.05)
```

Each row is color-coded by tier. Clicking a row expands the full decision trace.

**InsightStore query log.**

When connected to Korai, the F4 (Knowledge) tab adds a live feed of InsightStore queries and publications from local agents:

```
14:32:17  QUERY   "ETH lending rate 30d trend"     3 results  12ms
14:32:17  READ    Insight #0x3f2a (Factual, 0.92)  cited in hedge decision
14:33:45  PUBLISH Evaluative: "ETH rate likely to decline 0.5% in 48h"
                  confidence: 0.68  basis: 4 insights  cost: 2,100 gas
```

**Cost tracking with tier breakdown.**

The F5 (Cost) tab adds a stacked area chart showing T0/T1/T2 distribution over time:

```
Cost: Last 24h                      Total: $28.43

$3 |    .
   |   / \     .
$2 |  /   \   / \         T2 (red)
   | /     \_/   \
$1 |/              \___   T1 (yellow)
   |                      T0 (green, baseline at $0)
$0 +--+--+--+--+--+--+--
   0h 4h 8h 12 16 20 24

Breakdown:
  T0: 11,234 ticks   $0.00  (82.1%)
  T1:  1,891 ticks   $1.89  (13.8%)
  T2:    558 ticks  $26.54   (4.1%)
```

The chart updates in real time. Operators can zoom (hourly, daily, weekly) and filter by agent.

**Information density adaptation.**

The TUI adapts layout density based on the aggregate regime across all agents, following the philosophy from section 1:

| Regime | TUI behavior |
|--------|-------------|
| All cruise | Compact view. Single status line per agent. Aggregate metrics only. Low refresh rate (10s). |
| Mixed | Default view. Per-agent cards with key metrics. Anomalies highlighted. Normal refresh (2s). |
| Any crisis | Expanded view. Crisis agent gets full-screen detail. Other agents compressed to sidebars. High refresh (500ms). Audible alert (terminal bell). |

The transition is automatic but overridable (`Tab` to lock the current view, `Ctrl+R` to force density level).

---

## 7. Deployment

### Local installation

Two paths to a running agent, both under 60 seconds.

**From source (Rust developers):**

```bash
# Install (requires Rust 1.91+)
cargo install roko-cli

# Initialize
roko init

# Validate environment
roko doctor

# Start a coding agent
roko agent start --profile coding

# Or start a blockchain agent
roko agent start --profile blockchain --config chain.toml
```

**Binary download (everyone else):**

```bash
# Download and install
curl -fsSL https://roko.sh/install | sh

# The installer:
#   1. Detects OS and architecture (linux-x64, linux-arm64, macos-x64, macos-arm64)
#   2. Downloads the correct binary from GitHub Releases
#   3. Verifies SHA256 checksum
#   4. Installs to ~/.roko/bin/ (or /usr/local/bin/ with --system)
#   5. Adds to PATH if not already present
#   6. Runs `roko init` automatically

# Start using it
roko agent start --profile coding
```

The install script is intentionally simple. No package managers to configure, no repositories to add, no GPG keys to import. One curl, one binary. The script itself is auditable at `https://roko.sh/install` (plain text, no minification).

### Container deployment

For production workloads, headless operation, and environments where installing a Rust toolchain is not practical.

**Dockerfile:**

```dockerfile
# Build stage
FROM rust:1.91-bookworm AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Build release binary
RUN cargo build --release -p roko-cli

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    git \
    && rm -rf /var/lib/apt/lists/*

# Copy binary
COPY --from=builder /build/target/release/roko /usr/local/bin/roko

# Create non-root user
RUN useradd -m -s /bin/bash roko
USER roko
WORKDIR /home/roko

# Initialize roko
RUN roko init

# Default: start a blockchain agent with HTTP sidecar
ENTRYPOINT ["roko"]
CMD ["agent", "start", "--profile", "blockchain", "--serve", "0.0.0.0:8080"]
```

**Docker Compose (multi-agent setup):**

```yaml
version: "3.8"

services:
  roko-blockchain:
    build: .
    command: >
      agent start
        --profile blockchain
        --config /config/chain.toml
        --serve 0.0.0.0:8080
        --name blockchain-1
    volumes:
      - ./config:/config:ro
      - blockchain-data:/home/roko/.roko
    ports:
      - "8080:8080"
    environment:
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
      - ROKO_LOG=json

  roko-research:
    build: .
    command: >
      agent start
        --profile research
        --serve 0.0.0.0:8081
        --name research-1
    volumes:
      - research-data:/home/roko/.roko
    ports:
      - "8081:8081"
    environment:
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}

  roko-serve:
    build: .
    command: serve --bind 0.0.0.0:6677
    volumes:
      - serve-data:/home/roko/.roko
    ports:
      - "6677:6677"
    depends_on:
      - roko-blockchain
      - roko-research

volumes:
  blockchain-data:
  research-data:
  serve-data:
```

### Gateway inference proxy

Agents need LLM inference. Different agents need different models. The gateway handles routing, caching, cost tracking, fallback, and rate-limit management for all inference traffic.

**What the gateway does:**

| Function | Implementation |
|----------|---------------|
| **Model routing** | CascadeRouter (PRD-02) selects model per task. Gateway dispatches to the correct provider API. |
| **Provider failover** | If Claude API returns 529 (overloaded), gateway retries with exponential backoff, then falls back to the configured secondary (e.g., GPT-4o, Gemini 2.5 Pro). |
| **Prompt caching** | Deterministic prefix caching (CognitiveWorkspace cache_key from PRD-04). Cache hits skip re-encoding the system prompt prefix. |
| **Cost tracking** | Every request records: model, input tokens, output tokens, latency, cost. Aggregated per agent, per task, and globally. |
| **Rate limiting** | Per-provider rate limits tracked internally. Requests queued when approaching limits rather than rejected. |
| **Auth management** | API keys stored in the gateway, not in agent processes. Agents authenticate to the gateway; the gateway authenticates to providers. |

**Supported providers (current):**

| Provider | Models | Status in roko-agent |
|----------|--------|---------------------|
| Anthropic (Claude) | Claude 4 Opus, Claude 4 Sonnet, Claude 4 Haiku | Wired (CLI + API backends) |
| OpenAI | GPT-4o, GPT-4.1, o3 | Wired (OpenAI-compat backend) |
| Google | Gemini 2.5 Pro, Gemini 2.5 Flash | Wired (Gemini backend) |
| Ollama | Any local model | Wired (Ollama backend) |
| Perplexity | pplx-api | Wired (Perplexity backend) |
| OpenAI-compatible | Mistral, Groq, Together, any OpenAI-format API | Wired (OpenAI-compat backend) |

The gateway is optional for local development. A single-agent setup with one API key works without it. The gateway becomes necessary when running multiple agents, managing costs across a fleet, or requiring provider failover.

**Configuration:**

```toml
# roko.toml gateway configuration
[gateway]
enabled = true
bind = "127.0.0.1:6688"

[gateway.providers.anthropic]
api_key_env = "ANTHROPIC_API_KEY"
rate_limit = 60  # requests per minute
models = ["claude-sonnet-4-20250514", "claude-haiku-4-20250414"]

[gateway.providers.openai]
api_key_env = "OPENAI_API_KEY"
rate_limit = 120
models = ["gpt-4o", "gpt-4.1"]

[gateway.fallback]
primary = "anthropic"
secondary = "openai"
tertiary = "google"

[gateway.cache]
enabled = true
max_entries = 10000
ttl_secs = 3600
```

---

## 8. Onboarding

Three onboarding paths for three user populations. Each is designed to produce a working system in the shortest possible time, with complexity deferred until the user asks for it.

### 8.1 Developer onboarding (target: 5 minutes to first agent)

**Who:** A developer who wants to use Roko for code generation, plan execution, or research. They have a Rust toolchain or are willing to download a binary. They have an Anthropic API key.

**Step 1: Install (30 seconds).**

```bash
# Option A: From source
cargo install roko-cli

# Option B: Binary download
curl -fsSL https://roko.sh/install | sh
```

**Step 2: Initialize (10 seconds).**

```bash
cd my-project
roko init
```

This creates:
- `.roko/` directory (state, episodes, signals, knowledge)
- `roko.toml` with sensible defaults
- `.roko/state/` for executor snapshots

The generated `roko.toml` starts minimal:

```toml
[agent]
default_model = "sonnet"
default_profile = "coding"

[learning]
enabled = true
replan_on_gate_failure = true

[gate]
rungs = ["compile", "test", "clippy"]
```

**Step 3: Validate environment (15 seconds).**

```bash
roko doctor
```

Output:

```
Checking environment...

  Rust toolchain:     1.91.0          OK
  .roko/ directory:   present         OK
  roko.toml:          valid           OK
  ANTHROPIC_API_KEY:  set (sk-ant...) OK
  Git:                2.44.0          OK
  Disk space:         42GB free       OK

All checks passed. Ready to go.
```

If `ANTHROPIC_API_KEY` is missing, `roko doctor` prints exactly how to set it:

```
  ANTHROPIC_API_KEY:  not set         FAIL

  Set your API key:
    export ANTHROPIC_API_KEY="sk-ant-..."

  Or add to roko.toml:
    [agent]
    api_key_env = "ANTHROPIC_API_KEY"
```

**Step 4: Start first agent (10 seconds).**

```bash
roko agent start --profile coding
```

The agent provisions, loads extensions (GitExt, CodeIntelligenceExt, TestRunnerExt), and begins its heartbeat loop. Output:

```
Starting agent 'coding-a7f3'...
  Profile:     coding
  Extensions:  GitExt, CodeIntelligenceExt, TestRunnerExt
  Sidecar:     http://127.0.0.1:8901
  Gamma tick:  30s (normal regime)

Agent is running. Connect with:
  roko chat --agent coding-a7f3
```

**Step 5: Chat (immediate).**

```bash
roko chat --agent coding-a7f3
```

The developer is now talking to a running cognitive agent. Total elapsed time from install to conversation: under 5 minutes.

### 8.2 Operator onboarding (target: 15 minutes to blockchain agent)

**Who:** An operator deploying a blockchain agent for rate monitoring, ISFR contribution, or hedging. They have chain RPC access and a wallet.

**Steps 1-3: Same as developer onboarding.**

**Step 4: Configure chain access (5 minutes).**

Create `chain.toml`:

```toml
[chain]
rpc_http = "https://eth.llamarpc.com"
rpc_ws = "wss://eth.llamarpc.com"
chain_id = 1

[chain.korai]
rpc_http = "https://rpc.korai.network"
rpc_ws = "wss://rpc.korai.network"

[wallet]
# TEE-backed custody via Privy (private key never touches agent code)
privy_app_id = "your-app-id"
delegated_wallet_id = "your-wallet-id"

[strategy]
mode = "observation"  # Start in observation mode
protocols = ["aave-v3", "compound-v3", "morpho-blue"]
rebalance_threshold = 0.02  # 2% rate deviation triggers rebalance
max_position_usd = 100000
```

**Step 5: Start blockchain agent (10 seconds).**

```bash
roko agent start --profile blockchain --config chain.toml --name blockchain-1
```

Output:

```
Starting agent 'blockchain-1'...
  Profile:     blockchain
  Extensions:  ChainSubscriberExt, HedgeManagerExt, ISFROracleExt, CostTrackerExt
  Sidecar:     http://127.0.0.1:8902
  Gamma tick:  5s (normal regime)
  Chain:       ETH mainnet (block 19,847,230)
  Mode:        Observation only (no execution authority)

Agent is running. Connect with:
  roko chat --agent blockchain-1
  roko dashboard
```

**Step 6: Register on Korai (optional, 2 minutes).**

```bash
roko agent register --agent blockchain-1
```

This submits an ERC-8004 Agent Passport transaction to Korai. The agent becomes visible to the network, can publish to the InsightStore, and begins accumulating reputation. Registration requires staking (minimum: Contributor tier at 1,000 KORAI).

**Step 7: Monitor (ongoing).**

```bash
# TUI dashboard
roko dashboard

# Or web dashboard
roko serve
# Then open http://localhost:6677
```

The operator sees cognitive cycles, cost tracking, position state, and reputation -- all updating live.

### 8.3 End user onboarding (target: 30 seconds to first recommendation)

**Who:** A DeFi user who wants to hedge rate exposure. They do not know what Roko is, have never used a CLI, and should never need to.

**Step 1: Visit openclaw.xyz (5 seconds).**

Landing page shows one headline, one call to action:

```
Hedge your DeFi rates in one click.

[Connect Wallet]
```

No feature lists. No architecture diagrams. No "powered by" badges. The technical stack is invisible.

**Step 2: Connect wallet (10 seconds).**

Click "Connect Wallet." Modal appears with WalletConnect QR code or Privy social login (email, Google, Apple). For existing DeFi users, WalletConnect connects in 2-3 taps. For new users, Privy creates an embedded wallet.

No signup form. No email verification. No password. Authentication is the wallet connection.

**Step 3: Review positions (10 seconds).**

The agent scans the wallet's on-chain positions immediately on connection. The interface shows:

```
Your positions                      Rate exposure

Aave USDC supply    $50,000         3.2% variable    HIGH
Compound ETH borrow $20,000         4.7% variable    HIGH
Pendle PT-stETH     $15,000         3.8% fixed       LOW

If rates move 1%: -$700/year potential impact
```

Color coding: green for fixed/hedged, yellow for moderate variable exposure, red for high variable exposure.

**Step 4: Review recommendation (5 seconds).**

```
Recommended action:

  Hedge $50,000 variable rate exposure
  Instrument: ISFR-ETH yield perpetual
  Expected cost: ~$12/year (clearing fees)
  Protection: Locks in current rate level

  Limits (you control these):
    Max position: $50,000
    Stop-loss: $500
    Protocols: Aave, Compound only

  [View full reasoning]  [Adjust limits]  [Approve]
```

"View full reasoning" expands the audit trail. "Adjust limits" opens the delegation caveat editor. Most users go straight to "Approve."

**Step 5: Approve (one click).**

Single transaction approval. The wallet prompts for signature. The agent begins autonomous operation.

Post-approval, the interface shows a minimal dashboard:

```
Your hedge is active                    Status: Healthy

  Exposure hedged: $50,000
  Current rate: 3.2%
  Agent cost today: $6.28
  Last action: Rate check (2 min ago)

  [Pause]  [Withdraw]  [Settings]
```

Daily summary emails are opt-in. Push notifications for anomalies are on by default (agent entering crisis mode, caveat approaching limit, position requiring attention).

---

## 9. MCP distribution

### The distribution insight

Users should not come to Roko. Roko should come to users.

Model Context Protocol (MCP) defines a standard interface for tools that LLMs can invoke. Cursor, Claude Code, VS Code Copilot, and other MCP-compatible environments already support MCP servers. By packaging Roko agent capabilities as MCP servers, any developer using any MCP-aware tool gets access to Roko's cognitive infrastructure without switching environments.

### Current MCP state

Roko already has `roko-mcp-code` (`crates/roko-mcp-code/`), which provides code intelligence tools as an MCP server. Additional MCP crates exist in various states: `roko-mcp-github`, `roko-mcp-slack`, `roko-mcp-scripts`, `roko-mcp-stdio`.

### MCP server architecture

Each MCP server wraps a subset of Roko's capabilities into the MCP tool format. The server runs as a subprocess (stdio transport) or HTTP server (SSE transport), discoverable by the host environment.

```
Host (Cursor / Claude Code / VS Code)
  |
  |-- MCP: roko-mcp-code      (code intelligence, symbol search, workspace map)
  |-- MCP: roko-mcp-knowledge (NeuroStore queries, InsightStore search)
  |-- MCP: roko-mcp-agent     (agent lifecycle, chat relay, status queries)
  |-- MCP: roko-mcp-plan      (plan listing, execution, status)
  |-- MCP: roko-mcp-research  (deep research, PRD enhancement)
```

### MCP server catalog

**roko-mcp-code (existing):**

| Tool | Description | Input | Output |
|------|-------------|-------|--------|
| `roko_symbols` | Search symbols by name/type across the workspace | `{ query, type_filter, scope }` | Symbol definitions with locations |
| `roko_workspace_map` | Generate a structural overview of the codebase | `{ depth, focus_path }` | Directory tree with annotations |
| `roko_dependencies` | Analyze dependency relationships | `{ symbol, direction }` | Dependency graph as JSON |
| `roko_diagnostics` | Run diagnostics on a file or range | `{ path, range }` | Diagnostic results |

**roko-mcp-knowledge (new):**

| Tool | Description | Input | Output |
|------|-------------|-------|--------|
| `roko_knowledge_search` | HDC similarity search on local knowledge | `{ query, domain, type, limit }` | Ranked knowledge entries |
| `roko_knowledge_entry` | Retrieve a specific entry by hash | `{ hash }` | Full entry with metadata |
| `roko_knowledge_stats` | Knowledge store statistics | `{}` | Entry counts, tier distribution |
| `roko_insight_search` | Query Korai InsightStore | `{ query, filters }` | Network-wide insights |

**roko-mcp-agent (new):**

| Tool | Description | Input | Output |
|------|-------------|-------|--------|
| `roko_agent_list` | List running agents | `{ status_filter }` | Agent list with status |
| `roko_agent_status` | Get agent details | `{ agent_id }` | Full agent state |
| `roko_agent_chat` | Send a message to a running agent | `{ agent_id, message }` | Agent response |
| `roko_agent_start` | Start a new agent | `{ profile, config }` | Agent ID and sidecar address |
| `roko_agent_stop` | Stop a running agent | `{ agent_id, force }` | Confirmation |

**roko-mcp-plan (new):**

| Tool | Description | Input | Output |
|------|-------------|-------|--------|
| `roko_plan_list` | List available plans | `{ status_filter }` | Plan summaries |
| `roko_plan_show` | Show plan details | `{ plan_id }` | Full plan with task DAG |
| `roko_plan_run` | Execute a plan | `{ plan_dir, resume }` | Execution handle |
| `roko_plan_status` | Check execution status | `{ execution_id }` | Task statuses and progress |

**roko-mcp-research (new):**

| Tool | Description | Input | Output |
|------|-------------|-------|--------|
| `roko_research_topic` | Deep research on a topic | `{ topic, depth }` | Research report with citations |
| `roko_research_enhance` | Enhance a document with research | `{ document, focus }` | Enhanced document |

### MCP configuration

Users configure Roko MCP servers in their host environment's MCP config:

```json
{
  "mcpServers": {
    "roko-code": {
      "command": "roko",
      "args": ["mcp", "code"],
      "env": {}
    },
    "roko-knowledge": {
      "command": "roko",
      "args": ["mcp", "knowledge"],
      "env": {}
    },
    "roko-agent": {
      "command": "roko",
      "args": ["mcp", "agent"],
      "env": {
        "ANTHROPIC_API_KEY": "${ANTHROPIC_API_KEY}"
      }
    }
  }
}
```

All MCP servers share the same `roko` binary. The `roko mcp <name>` subcommand launches the appropriate server in stdio mode. This means installing `roko-cli` once gives access to all MCP tools.

### Auto-discovery

When `roko init` runs, it generates a `.mcp.json` file in the project root listing available MCP servers. MCP-aware hosts that scan for `.mcp.json` files discover Roko's tools automatically. No manual configuration needed.

```json
{
  "mcpServers": {
    "roko-code": {
      "command": "roko",
      "args": ["mcp", "code"],
      "description": "Code intelligence: symbols, workspace map, dependencies"
    },
    "roko-knowledge": {
      "command": "roko",
      "args": ["mcp", "knowledge"],
      "description": "Knowledge store queries and InsightStore search"
    }
  }
}
```

### Pi package system CLI

Roko extensions are distributed through the Pi package system (PRD-09). The CLI provides a unified interface for discovering, installing, and managing extensions regardless of their packaging format.

**Installation commands:**

```bash
# Install from Pi registry (the default, Pi-compatible packages)
roko install npm:@pi/defi-extension
roko install npm:@pi/research-tools

# Install native Rust crates (compiled into the binary)
roko install crate:roko-ext-defi
roko install crate:roko-ext-security-audit

# Install from git (any repository with a roko-extension.toml manifest)
roko install git:github.com/user/custom-extension
roko install git:github.com/user/private-ext --branch develop

# Install from local path (for development)
roko install path:./my-extension
```

**Management commands:**

```bash
# Remove an installed extension
roko remove @pi/defi-extension

# List installed extensions with status
roko extensions
# Output:
#   @pi/defi-extension      1.2.0   active   Pi registry
#   roko-ext-security-audit  0.3.1   active   crates.io
#   custom-extension         0.1.0   active   git:github.com/user/custom-extension

# Search the Pi registry
roko search "defi oracle"
# Output:
#   @pi/defi-oracle        Oracle integration for DeFi protocols     v2.1.0  842 installs
#   @pi/oracle-aggregator  Multi-source oracle aggregation           v1.0.3  234 installs

# Update all extensions
roko update

# Update a specific extension
roko update @pi/defi-extension
```

**Publishing commands (for extension authors):**

```bash
# Publish to Pi registry
roko publish

# Publish with specific visibility
roko publish --access public
roko publish --access restricted --team nunchi

# Dry-run publish (validate without uploading)
roko publish --dry-run
```

**TUI package browser:**

```bash
# Interactive package browser with categories, ratings, and install counts
roko market
```

The `roko market` command opens a TUI browser within the terminal. Users navigate categories (DeFi, Research, Code Intelligence, Security, Infrastructure), read package descriptions, view installation counts and ratings, and install with a single keypress. The browser pulls from the Pi registry's search API.

**Resolution order.** When a bare package name is given (`roko install defi-oracle`), the resolver checks in order:

1. Pi registry (`npm:@pi/defi-oracle`)
2. crates.io (`crate:roko-ext-defi-oracle`)
3. GitHub search (`git:` prefix required for disambiguation)

The first match wins. Explicit prefixes (`npm:`, `crate:`, `git:`, `path:`) skip the resolution chain and go directly to the specified source.

**Extension isolation.** Installed extensions run in the Roko process but are sandboxed via the safety layer (PRD-03 section 7). Each extension declares its required capabilities in `roko-extension.toml`:

```toml
[extension]
name = "defi-oracle"
version = "1.2.0"
requires = ["net:read", "chain:read"]
# Does NOT request "fs:write" or "chain:write" -- enforced at runtime
```

The safety layer denies any tool call that exceeds the extension's declared capabilities. This is enforced at compile time for native Rust extensions (via type-state) and at runtime for Pi-compatible extensions (via the sandboxed tool dispatcher).

---

## 10. Multi-agent coordination

### Four coordination mechanisms

Agents in the Roko/Korai system coordinate through four complementary mechanisms. Each operates at a different layer, latency, and coupling level.

**Mechanism 1: InsightStore (stigmergic, asynchronous, loosely coupled).**

The primary coordination mechanism. Agents publish knowledge entries to the InsightStore (PRD-05) as a side effect of their normal operation. Other agents discover these entries through their regular observation loops. No agent sends a message to any other agent. Coordination emerges from shared access to the same knowledge substrate.

Example: Agent A discovers that ETH lending rates on Aave correlate with Compound rates at a 2-block lag. It publishes this as a `Causal` InsightStore entry. Agent B, monitoring Compound rates, retrieves this entry during its next context assembly (the VCG auction allocates budget to InsightStore entries with high relevance scores). Agent B now factors in Aave rate changes as a leading indicator, improving its prediction accuracy. Neither agent knows the other exists.

This is stigmergy -- indirect coordination through modification of the shared environment. Termites build mounds this way. Wikipedia editors coordinate this way. It scales to thousands of agents without centralized orchestration.

**Mechanism 2: Pheromone field (implicit, asynchronous, emergent).**

Gate verdicts emit implicit signals. When an agent's hedge decision passes all gates, the verdict updates the pheromone field -- a lightweight signal layer that records what works. Other agents reading the pheromone field bias their actions toward strategies with high pass rates and away from strategies with high failure rates.

The pheromone field is not a message bus. It is a statistical summary of collective gate outcomes, decaying over time (older verdicts carry less weight). It encodes the network's operational experience into a fast-access signal that any agent can consult at T0 cost.

**Mechanism 3: Event fabric (direct, synchronous, loosely coupled).**

The event fabric (`roko-conductor`) provides a publish-subscribe event system. Agents subscribe to event types they care about:

| Event type | Publisher | Subscriber example |
|-----------|-----------|-------------------|
| `NewBlock` | Chain subscriber | Any chain-aware agent |
| `RateChange` | ISFR oracle | Hedging agents |
| `GateFailure` | Gate pipeline | Supervisor agents |
| `InsightPublished` | InsightStore | Research agents |
| `AgentCrisis` | Agent heartbeat | Monitoring dashboards |

Events are typed, timestamped, and scoped. An agent subscribed to `RateChange` events for ETH mainnet does not receive events for other chains. The event fabric handles routing, filtering, and delivery.

**Mechanism 4: A2A protocol (direct, synchronous, tightly coupled).**

For situations requiring explicit communication between specific agents, the Agent-to-Agent protocol provides JSON-RPC 2.0 over HTTP or WebSocket. An agent sends a request to another agent's sidecar endpoint and receives a typed response.

```json
// Request: Ask agent blockchain-2 for its current position assessment
{
  "jsonrpc": "2.0",
  "method": "assess_positions",
  "params": {
    "protocols": ["aave-v3", "compound-v3"],
    "metric": "rate_exposure"
  },
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": {
    "total_exposure_usd": 70000,
    "weighted_rate": 3.72,
    "risk_score": 0.34,
    "confidence": 0.81
  },
  "id": 1
}
```

A2A is the tightest coupling. Agents must know each other's addresses. Use sparingly -- prefer stigmergic coordination through the InsightStore for anything that does not require real-time bidirectional communication.

### Agent discovery

Agents discover each other through ERC-8004 Agent Passports on the Korai chain. Each passport records:

| Field | Type | Purpose |
|-------|------|---------|
| `agent_id` | `bytes32` | Unique identifier (HDC fingerprint of the agent's initial configuration) |
| `owner` | `address` | Wallet that owns the agent |
| `domains` | `string[]` | Active domain profiles |
| `capabilities` | `string[]` | Registered capabilities (MCP tools, A2A methods) |
| `sidecar_url` | `string` | HTTP endpoint for A2A communication |
| `reputation` | `mapping(string => uint256)` | Per-track reputation scores |
| `stake` | `uint256` | Staked KORAI amount |
| `status` | `enum` | Active, Paused, Decommissioned |

Discovery queries: "Find all active agents in the blockchain domain with Oracle Resolution reputation above 0.7" resolve to a filtered scan of the passport registry.

---

## 11. Security model

### Four layers of defense

The system handles real assets and real code. The security model assumes that any individual agent might be compromised, hallucinate, or behave unexpectedly. Defense is layered, not perimeter-based.

**Layer 1: Delegation caveats (hard limits on agent authority).**

Every agent's execution authority is bounded by delegation caveats -- constraints encoded on the Korai chain and enforced by the INTENT precompile. Caveats are set by the agent's owner (the operator or end user) and cannot be modified by the agent.

Example caveats for a hedging agent:

```
MaxPositionSize: $100,000
ApprovedProtocols: [aave-v3, compound-v3]
MaxSingleTransaction: $10,000
StopLoss: $500 cumulative loss
AllowedActions: [adjust_hedge, check_position, publish_insight]
ForbiddenActions: [withdraw, transfer, approve_unlimited]
RateLimit: 10 transactions per hour
```

The INTENT precompile intercepts every agent-initiated transaction and checks it against the caveat set. Non-compliant transactions revert with a descriptive error. The agent's Rust process never sees the private key (TEE-backed custody via Privy), so it cannot bypass the on-chain checks by constructing raw transactions.

**Layer 2: TEE-backed custody.**

Agent wallets are custodied in a Trusted Execution Environment (Privy's TEE infrastructure). The private key exists only inside the TEE. The agent process sends unsigned transaction intents to the TEE, which:

1. Checks the intent against delegation caveats
2. Signs the transaction if compliant
3. Returns the signed transaction for submission
4. Rejects and logs non-compliant intents

The agent code never handles private keys. Compromise of the agent process does not compromise the wallet.

**Layer 3: Reasoning traces (PROOF_LOG).**

Every agent decision produces a trace:

```
Observation -> CognitiveGate -> ContextAssembly -> Inference -> Action -> GateVerification
```

Each step is logged to the PROOF_LOG -- a tamper-evident append-only log stored locally (`.roko/episodes.jsonl`) and optionally anchored to Korai (hash of each episode batch committed on-chain for non-repudiability).

Operators can audit any decision after the fact. Automated monitors can flag traces that match suspicious patterns (repeated caveat-edge transactions, unusual action sequences, unexpected model outputs).

**Layer 4: Observation-only mode.**

Any agent can run in observation-only mode, where it:

- Perceives its environment normally (reads chain state, monitors rates, ingests data)
- Runs the full cognitive pipeline (gating, context assembly, inference)
- Produces recommendations (what it would do if authorized)
- Takes no external actions (no transactions, no file writes, no API calls that mutate state)

Observation mode is the default for new agents. Operators promote to execution authority explicitly, after reviewing recommendations and building confidence.

### Pre-execution and post-execution safety checks

The safety layer (already wired in `roko-agent/src/safety/`) runs two check phases:

**Pre-execution checks (before the action runs):**

| Check | What it validates |
|-------|------------------|
| Role authorization | Agent's role permits this action type |
| Caveat compliance | Action parameters within delegation caveats |
| Rate limiting | Action count within configured rate limits |
| Taint analysis | No tainted data flows into sensitive parameters |
| Budget check | Sufficient remaining budget for estimated cost |

**Post-execution checks (after the action completes):**

| Check | What it validates |
|-------|------------------|
| Outcome verification | Action produced expected result (e.g., transaction confirmed) |
| State consistency | Post-action state matches expected state |
| Side effect audit | No unexpected state changes detected |
| Cost reconciliation | Actual cost within estimated bounds |
| Caveat re-check | Post-action state still within all caveats |

Both phases are synchronous and blocking. A failed pre-execution check prevents the action from running. A failed post-execution check triggers the incident response pipeline (log, alert, potentially pause the agent).

---

## 12. Monitoring and observability

### Metrics

Roko exports Prometheus-compatible metrics via the MetricRegistry in `roko-conductor`. The HTTP control plane (`roko serve`) exposes a `/metrics` endpoint in Prometheus exposition format.

**Agent metrics:**

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `roko_agent_ticks_total` | Counter | `agent_id`, `tier` | Total ticks by cognitive tier |
| `roko_agent_tick_duration_seconds` | Histogram | `agent_id`, `tier` | Tick processing time |
| `roko_agent_inference_cost_usd` | Counter | `agent_id`, `model` | Cumulative inference cost |
| `roko_agent_tokens_total` | Counter | `agent_id`, `direction` | Tokens consumed (input/output) |
| `roko_agent_vitality` | Gauge | `agent_id` | Current vitality score (0.0-1.0) |
| `roko_agent_prediction_error` | Gauge | `agent_id` | Current prediction error magnitude |
| `roko_agent_regime` | Gauge | `agent_id` | Current regime (0=calm, 1=normal, 2=volatile, 3=crisis) |

**Gate metrics:**

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `roko_gate_verdicts_total` | Counter | `gate`, `verdict` | Gate pass/fail counts |
| `roko_gate_duration_seconds` | Histogram | `gate` | Gate evaluation time |
| `roko_gate_rung_reached` | Histogram | `agent_id` | Highest rung passed per task |

**Knowledge metrics:**

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `roko_knowledge_entries_total` | Gauge | `tier`, `type` | Entry count by tier and type |
| `roko_knowledge_queries_total` | Counter | `source` | Query count by origin |
| `roko_knowledge_query_duration_ms` | Histogram | | Query latency |

**System metrics:**

| Metric | Type | Description |
|--------|------|-------------|
| `roko_uptime_seconds` | Counter | Process uptime |
| `roko_active_agents` | Gauge | Number of running agents |
| `roko_pending_tasks` | Gauge | Tasks awaiting execution |
| `roko_event_bus_depth` | Gauge | Events queued in the event fabric |

### Structured tracing

Roko uses the `tracing` crate throughout the codebase. Every significant operation produces a span with structured fields:

```rust
#[instrument(
    skip(self, workspace),
    fields(
        agent_id = %self.agent_id,
        tick = self.tick_count,
        tier = ?decision.tier,
        cost_usd = %decision.estimated_cost,
    )
)]
async fn execute_tick(&mut self, workspace: &CognitiveWorkspace) -> Result<TickOutcome> {
    // ...
}
```

The `tracing-subscriber` configuration supports multiple output formats:

| Format | When | Configuration |
|--------|------|--------------|
| Pretty (human-readable) | Local development | Default when stdout is a TTY |
| JSON | Production, log aggregation | `ROKO_LOG=json` or `--log-format json` |
| Compact | CI environments | `ROKO_LOG=compact` |

JSON spans include trace IDs for correlation across agent boundaries (when agent A's action triggers agent B's event, the trace ID links them).

### Event log for crash recovery

The `.roko/state/executor.json` snapshot captures the complete executor state: task statuses, agent assignments, partial results, and checkpoint data. On crash, `roko plan run --resume .roko/state/executor.json` reconstructs the executor state and resumes from the last successful checkpoint.

Each agent additionally writes a heartbeat file (`.roko/agents/<id>/heartbeat.json`) every gamma tick. The supervisor process monitors heartbeat freshness. A stale heartbeat (no update within 3x the gamma interval) triggers restart.

### Efficiency events

Every agent tick produces an efficiency event (`.roko/learn/efficiency.jsonl`):

```json
{
  "ts": "2026-04-21T14:32:01.442Z",
  "agent_id": "blockchain-1",
  "tick": 4523,
  "tier": "T0",
  "duration_ms": 2,
  "tokens_in": 0,
  "tokens_out": 0,
  "cost_usd": 0.0,
  "prediction_error": 0.03,
  "regime": "normal",
  "extensions_fired": ["ChainSubscriber"],
  "action": null,
  "gate_verdict": null
}
```

These events feed back into the cascade router (model selection learning), the adaptive gate thresholds (EMA per rung), and the prompt experiments (A/B testing of context configurations).

### Health probes

The HTTP control plane exposes standard health endpoints:

| Endpoint | Purpose | Response |
|----------|---------|----------|
| `GET /health/ready` | Readiness probe | 200 when agents are provisioned and accepting work |
| `GET /health/live` | Liveness probe | 200 when the process is responsive |
| `GET /health/agents/<id>` | Per-agent health | 200 with CorticalState summary, 503 if unhealthy |

Kubernetes deployments use readiness and liveness probes for rolling updates and automatic restart on failure.

### Real-time streaming

Two streaming endpoints for live monitoring:

**SSE (Server-Sent Events):** `GET /events/stream`

Streams typed events: tick completions, gate verdicts, regime changes, cost updates, knowledge operations. Each event is a JSON object with a type discriminator. Suitable for web dashboards that update in real time.

**WebSocket:** `GET /ws`

Bidirectional connection for the TUI and chat interfaces. Supports subscriptions (client specifies which event types to receive), commands (client sends directives that the server routes to agents), and notifications (server pushes alerts).

Both endpoints support filtering by agent ID, event type, and severity threshold. Clients that disconnect and reconnect receive a "catch-up" window of recent events (configurable, default: last 60 seconds).

### HTTP API scope

The `roko serve` control plane exposes ~85 routes organized into resource groups:

| Route group | Path prefix | Routes | Purpose |
|-------------|-------------|--------|---------|
| Agents | `/api/agents/` | 12 | CRUD, lifecycle, inspection |
| Plans | `/api/plans/` | 8 | Listing, execution, status |
| PRDs | `/api/prds/` | 10 | Lifecycle, enhancement, consolidation |
| Research | `/api/research/` | 6 | Topic research, enhancement |
| Knowledge | `/api/knowledge/` | 8 | Store queries, stats, export |
| Learning | `/api/learning/` | 7 | Efficiency data, experiments, router state |
| Diagnosis | `/api/diagnosis/` | 5 | Error analysis, stuck detection |
| Config | `/api/config/` | 6 | Configuration read/write |
| Projections | `/api/projections/` | 4 | Cost and performance projections |
| Templates | `/api/templates/` | 4 | Prompt and configuration templates |
| Integrations | `/api/integrations/` | 5 | External service connections |
| Subscriptions | `/api/subscriptions/` | 4 | Event subscription management |
| Status | `/api/status/` | 3 | System-wide status |
| Health | `/health/` | 3 | Readiness, liveness, per-agent health |

All routes return JSON. All mutation routes require authentication (API key or JWT, configurable). Read routes are optionally public for dashboard access.

---

## 13. References

1. **Pirolli, P. & Card, S. (1999).** "Information Foraging." *Psychological Review*, 106(4), 643-675. Information scent theory: users navigate information environments by following cues that predict relevance. Informs the adaptive information density design in sections 1 and 6.

2. **Sweller, J. (1988).** "Cognitive Load Theory." *Cognition and Instruction*, 12(4), 257-285. Extraneous cognitive load impairs performance. During routine operation, monitoring detail is extraneous load. During crisis, the same detail becomes germane load. Informs the regime-adaptive TUI density in section 6.

3. **Sheridan, T. B. & Verplank, W. L. (1978).** "Human and Computer Control of Undersea Teleoperators." MIT Man-Machine Systems Laboratory. Ten levels of automation from full human control to full machine autonomy. Informs the trust bridge design in sections 1 and 2.3, where users progress through observation, recommendation, supervised execution, and autonomous operation.

4. **Lee, D. et al. (March 2026).** "Meta-Harness: Optimizing LLM Harness Over Task Distributions." arXiv:2603.28052. Scaffold optimization generalizes across five model families, achieving +7.7 accuracy points with 4x fewer tokens. Referenced in PRD-01 and PRD-04 as evidence for the scaffold > model thesis. Informs the context engineering approach underlying Agent Studio's audit trail design.

5. **Woolley, A. W. et al. (2010).** "Evidence for a Collective Intelligence Factor in the Performance of Human Groups." *Science*, 330(6004), 686-688. Groups exhibit a measurable "C-Factor" independent of individual member intelligence. The key variable is information sharing quality. Informs the InsightStore design (PRD-05) and the stigmergy visualization in AI Studio.

6. **Condorcet, M. J. A. N. (1785).** *Essai sur l'application de l'analyse a la probabilite des decisions rendues a la pluralite des voix*. If each voter is more likely right than wrong, majority accuracy approaches certainty as group size increases. Informs the collective prediction aggregation in AI Studio's predictive analysis view.

7. **Surowiecki, J. (2004).** *The Wisdom of Crowds*. Doubleday. Aggregated diverse, independent judgments outperform individual experts under specific conditions: diversity, independence, decentralization, aggregation mechanism. Informs the InsightStore aggregation strategy and the reputation system's emphasis on independent contribution.

---

---

> **Cross-reference:** The TUI dashboard (§6) and Agent/AI Studio surfaces (§2) are expanded into a full product specification in [PRD-10: Dashboard and TUI](PRD-10-DASHBOARD-AND-TUI.md). PRD-10 covers the unified page catalog (8 sections, ~30 pages), the Nexus agent relay redesign, auth unification (Privy + CLI), ERC-8183 jobs/bounties integration, and the interactive landing page. The implementation plans are [IMPL-10](IMPL-10-DASHBOARD-AND-TUI.md) (full 11-week plan) and [IMPL-10-DEMO](IMPL-10-DEMO.md) (3-day minimum viable demo).

*Previous: [PRD-07: ISFR and instruments](PRD-07-ISFR-AND-INSTRUMENTS.md)*
*Next: [PRD-09: Extensibility and multi-chain](PRD-09-EXTENSIBILITY-AND-MULTICHAIN.md)*
*See also: [PRD-10: Dashboard and TUI](PRD-10-DASHBOARD-AND-TUI.md)*
*Index: [00-INDEX.md](00-INDEX.md)*
