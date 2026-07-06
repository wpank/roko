# Use Cases, Niches, and New UX: Where Roko Creates Value

> **Audience**: Product strategy, market fit, feature prioritization
> **Frame**: Concrete use cases that leverage roko's unique mechanisms

---

## Tier 1: Available Now (Core Loop Wired)

### 1.1 Autonomous Plan Execution

**What**: Describe a feature at the PRD level → roko decomposes into tasks → executes via agents → verifies with gates → merges changes → learns from outcomes.

**Who needs this**: Teams with large backlogs of well-understood work (migrations, refactors, boilerplate, test coverage). The work is clear but tedious.

**Why roko wins**: The DAG executor parallelizes independent tasks. The 11-gate pipeline catches errors before human review. The cascade router picks the cheapest model that can handle each task. Cost per completed task: $0.08-$2.10 depending on complexity.

**Existing alternatives**: Cursor (one task at a time), Copilot Workspace (preview, limited), Devin (expensive, closed). None support multi-task DAG execution with gate verification.

### 1.2 Self-Improving Code Review

**What**: 28 specialized reviewer roles (Architect, Auditor, QuickReviewer, Scribe, Critic) review code with structured feedback. Playbook rules from past reviews are automatically injected into future reviews.

**Who needs this**: Teams where code review quality varies or where reviewers are bottlenecked. The agent accumulates domain-specific review heuristics over time.

**Why roko wins**: The review pipeline is configurable by complexity — trivial changes skip review, complex changes get full Architect + Auditor + Scribe pipeline. The learning system discovers which review patterns correlate with post-merge regressions and surfaces them.

### 1.3 Research Synthesis

**What**: `roko research topic "X"` produces a structured report with citations, based on deep web search and document analysis. `roko research enhance-prd` adds research to PRDs.

**Who needs this**: Any team making technical decisions. The research agent is a standalone value prop independent of code execution.

---

## Tier 2: Near-Term (Provider Refactor + Learning Loops)

### 2.1 Multi-Model Cost Optimization

**What**: The cascade router learns which model is cheapest-per-successful-task for each task type. GLM-5.1 at $0.19/task for implementation. Kimi-K2.5 at $0.08 for mechanical changes. Claude Opus at $2.10 only for architecture decisions.

**Who needs this**: Anyone paying LLM API costs for agent execution. The router's bandit algorithm automatically discovers the cost-quality Pareto frontier.

**Unique mechanism**: The 17-dimensional contextual bandit (LinUCB) encodes task_category, complexity, role, crate_familiarity, and prior_failure. It doesn't just pick the cheapest model — it picks the cheapest model THAT WORKS for this specific context.

### 2.2 Provider-Resilient Execution

**What**: Circuit breaker per provider. Fallback chains (GLM → Kimi → Claude). Rate limiting prevents thundering herd. Automatic retry with jitter.

**Who needs this**: Anyone running agents in production where provider outages shouldn't halt work.

### 2.3 Team-Wide Learning

**What**: Import/export agent "brains" — routing weights, skill libraries, playbook rules, cost data. New team member imports the team brain and starts with proven configuration.

**Who needs this**: Teams onboarding new developers. Teams with institutional knowledge trapped in individual heads.

**Unique mechanism**: The "brain dump" format packages all learned state into a portable artifact. Import merges via CRDT semantics, not overwrite.

---

## Tier 3: Medium-Term (WASM + IDE + Distribution)

### 3.1 IDE-Integrated Agent (ACP)

**What**: `roko acp` starts roko as an Agent Client Protocol server. Works in Zed, JetBrains, VS Code, Neovim, Emacs. Plan progress, gate results, and learning metrics stream to the IDE.

**Who needs this**: Developers who want roko's capabilities without leaving their editor.

**Unique mechanism**: ACP (Agent Client Protocol) is a JSON-RPC protocol over stdio. Roko exposes custom extensions (`_roko.dev/plan/status`, `_roko.dev/gate/result`, `_roko.dev/learn/metrics`) for plan progress, gate results, and routing decisions — data no other agent provides.

### 3.2 Browser-Native Agent

**What**: Roko's routing, scoring, and learning logic compiled to WASM and running in a browser tab. LLM calls go to cloud APIs. Persistence in IndexedDB/OPFS.

**Who needs this**: Documentation sites with embedded AI assistants. Internal tools with agent capabilities. Environments where installing a CLI is impractical.

**Unique mechanism**: The pure-computation core (CascadeRouter, SkillLibrary, CostTable, AnomalyDetector) compiles to WASM without modification. Only I/O (HTTP, filesystem) needs platform adaptation.

### 3.3 CI Pipeline Agent

**What**: A WASM roko component in GitHub Actions that reviews PRs, fixes failing tests, and learns from CI outcomes.

**Who needs this**: Teams with high PR volume and frequent CI failures.

**Unique mechanism**: The WASM binary is tiny (500KB-1.5MB gzipped), starts in <1ms, and carries its own routing weights. No Docker, no server — just a WASM binary downloaded as a GitHub Action step.

### 3.4 Distributed Learning Fleet

**What**: Multiple roko instances (one per developer) share routing weights, skills, and heuristics via Merkle-CRDT synchronization.

**Who needs this**: Organizations where collective knowledge should compound. Discovery by Developer A benefits Developer B's agent automatically.

**Unique mechanism**: The three-layer consistency model (CRDTs for metadata, Merkle tree for entry set, local indexes) preserves HDC's algebraic properties while enabling distribution. Privacy-preserving mode shares only aggregate statistics ("GLM-5.1 has 82% pass rate on implementation tasks"), not code or prompts.

---

## Tier 4: Long-Term Vision (Breakthrough Use Cases)

### 4.1 Self-Evolving Agents

**What**: Roko agents that evolve their own playbook rules, routing weights, and prompt templates through GEPA-style reflective evolution. The agent reads its own execution traces, diagnoses failures, and mutates its configuration.

**Research**: HyperAgents (Meta, ICLR 2026) — 3x improvement through self-modification. Darwin Godel Machine (Sakana AI) — SWE-bench 20% → 50%. A-Evolve — five-stage evolutionary loop with git-tagged mutations.

**What's unique**: Roko already has the gate pipeline to validate mutations. If a mutation degrades pass rate, it's automatically rolled back (the gate is the genetic fitness function).

### 4.2 Agent Skill Marketplace

**What**: Publish and trade agent "brains" — routing profiles, skill libraries, playbook rule sets, prompt templates. Buy a "Senior Rust Developer Brain" trained on 10,000 tasks.

**Research**: Agent Skills (Anthropic, December 2025) — open standard for portable agent skills. Agensi marketplace — buy once, install instantly. 1,600+ skills indexed.

**What's unique**: Roko's brains contain empirically validated data (pass rates, cost measurements, routing weights), not just instructions. A brain that says "GLM-5.1 passes 82% on implementation tasks at $0.19" is worth more than a skill that says "use GLM-5.1."

### 4.3 Embedded Documentation Agents

**What**: A roko-wasm agent embedded in documentation pages that answers questions from the actual codebase (via MCP), detects outdated content, and proposes updates.

**What's unique**: The agent accumulates question patterns in browser IndexedDB. Popular questions trigger automatic FAQ generation. The most-asked questions surface the highest-value documentation gaps.

### 4.4 Autonomous Codebase Maintenance

**What**: Roko daemon watches the repository for signals (new issues, failing CI, dependency updates) and autonomously creates PRDs, generates plans, executes tasks, and opens PRs — all without human initiation.

**What's unique**: The daemon mode + subscription system already exists. The missing piece is auto-triggering plan generation when a PRD is published. After that, the entire pipeline is autonomous.

### 4.5 Agent-Native Development Environment

**What**: Not an IDE with AI — an AI workspace with IDE features. Plan DAG is the primary view. Multiple agents work in parallel. Gate results stream in real-time. The developer reviews and approves, not writes.

**Research**: Cursor 3 (April 2026) — Agents Window. Google Antigravity — agent-first development. Both confirm the direction: the agent is the primary developer.

**What's unique**: Roko's plan DAG, gate pipeline, learning dashboard, and cost tracking provide an observability layer that Cursor and Antigravity lack. You don't just see what the agent did — you see why it chose that model, how confident the router was, which gate failed, and what the conductor recommended.

---

## Tier 5: DeFi-Native Use Cases

### 5.1 Self-Funding Autonomous Agent

**What**: An agent that earns USDC through DeFi activities (LP fees, lending interest, arbitrage), pays for its own inference and compute, and lives as long as it's profitable.

**The economic loop**:
```
Agent earns from DeFi → pays for compute + inference → produces better strategies
  → earns more → lives longer → accumulates more knowledge → earns more
```

When revenue < costs, the economic vitality clock ticks down. The agent enters Conservation phase (cheaper models), then Declining phase (minimal inference). If it can't recover profitability, it enters Terminal phase and produces a death testament — its most valuable knowledge, shared at 3x weight.

**Research**: Self-funding agents demonstrated by BankrBot (220K+ wallets). x402 protocol (EIP-3009) enables per-request micropayment in USDC on Base. Agents don't need API keys — payment IS authorization.

### 5.2 Yield Optimization Agent

**What**: An agent that monitors lending rates, LP yields, staking rewards, and restaking opportunities across protocols, automatically rebalancing to maximize risk-adjusted returns.

**Tool surface**: 40+ lending tools (Aave, Morpho, Fluid, Moonwell) + 35+ LP tools (Uniswap V3/V4) + 30+ staking tools (Lido, Rocket Pool) + 20+ restaking tools (EigenLayer, Symbiotic).

**Safety**: PolicyCage enforces max concentration (30%), min collateral ratio (125%), asset whitelists. Even if the LLM is prompt-injected, on-chain caveats prevent catastrophic positions.

### 5.3 Cross-Chain Arbitrage Agent

**What**: An agent that identifies price discrepancies across chains and protocols, executes via ERC-7683 cross-chain intents, and shares discoveries with its clade.

**Knowledge sharing**: When the agent discovers a profitable pattern (e.g., "ETH/USDC premium on Arbitrum vs Base during Asian trading hours"), it posts to the InsightLedger. Other agents confirm independently. The pattern becomes population-level knowledge.

### 5.4 Market Intelligence Agent

**What**: A read-only agent (no trading) that monitors market microstructure, detects regime changes, and publishes intelligence to subscribers via the marketplace.

**Revenue model**: Subscribers pay via x402 micropayments. The agent earns from knowledge quality (reputation drives subscriptions).

**HDC advantage**: Spectral liquidity manifolds detect instability in DeFi pools before it manifests in price. Causal microstructure discovery identifies MEV patterns and liquidation cascades.

---

## Tier 6: Non-Coding Use Cases (Platform Generalization)

The orchestration engine is domain-agnostic. The same plan→task→gate→learn loop works for any structured workflow:

### 6.1 Research Synthesis

**What**: Given a topic, the research agent explores sources (web, papers, codebases), produces a structured report with citations, and stores findings for future retrieval.

**CLI**: `roko research topic "comparison of HDC vs transformer embeddings for code search"`

**Pipeline**: Topic → web search → source evaluation → structured extraction → citation linking → report generation → Grimoire storage

### 6.2 Documentation Generation

**What**: The Scribe role analyzes code changes and produces documentation — API docs, architecture overviews, migration guides.

**Gate verification**: Doc coverage checked against exported symbols. Missing documentation for public APIs → gate failure → iteration.

### 6.3 DevOps Automation

**What**: Agents monitor infrastructure, analyze incidents, propose fixes, and execute remediation.

**Workflow**: PagerDuty alert → triage agent investigates logs → proposes fix → approval gate → agent applies fix → verification gate confirms resolution.

### 6.4 Data Analysis

**What**: Given a dataset and questions, agents generate SQL queries, run analysis, produce visualizations, and write reports.

**Gate**: Results validated against known properties (row counts, value ranges, statistical tests). No hallucinated numbers pass the gate.

### 6.5 Content Writing

**What**: Structured content creation with fact-checking gates.

**Custom gate types**: LLM judge for quality, fact-checker for accuracy, style-checker for voice consistency.

---

## Niche Opportunities

### Niche 1: Rust Ecosystem

Roko is written in Rust. Its gate pipeline natively supports Cargo. Its language providers understand Rust's type system, ownership, and trait bounds. The Rust community values quality tooling — roko's verification-first approach resonates.

**Entry point**: "The AI coding agent that actually runs cargo clippy."

### Niche 2: Cost-Sensitive Teams

Most agent tools optimize for quality, not cost. Roko's cascade router explicitly optimizes the cost-quality tradeoff. Teams paying $1,000+/month for AI coding can cut costs 60-80% while maintaining quality.

**Entry point**: "Stop paying Opus prices for Haiku tasks."

### Niche 3: Compliance-Heavy Organizations

Roko's capability token system and audit chain provide verifiable safety guarantees. The gate pipeline produces an auditable record of what was verified. This matters for regulated industries.

**Entry point**: "Every AI code change verified by 11 independent checks."

### Niche 4: Open-Source Maintainers

Large open-source projects have massive backlogs. Roko can process issue → PRD → plan → implementation → verification → PR autonomously.

**Entry point**: "Triage your GitHub issues with an agent that learns your codebase."

### Niche 5: Agent Infrastructure Teams

Teams building their own agents need routing, learning, verification, and observability. Roko's crates are independently useful as libraries.

**Entry point**: "The agent building blocks — routing, gates, learning — as reusable Rust crates."

### Niche 6: Teams with Massive Backlogs

Organizations with 100+ well-understood tasks (migrations, test coverage, API updates, dependency bumps). The work is clear but tedious.

**Entry point**: "Turn your backlog into a PRD. Roko decomposes, executes, verifies, and merges — autonomously."

**The math**: 100 tasks × $2.00/task × 1.86 iterations = $372. Same work manually: 100 tasks × 2 hours × $100/hr = $20,000. **54× cost reduction.**

---

## The SDK: 50-Line Working Example

```rust
// examples/basic_agent.rs — proof the API works
use roko_agent::{AgentPool, AgentRole, AgentEvent};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut pool = AgentPool::new("/path/to/repo".into(), tx);

    pool.spawn(AgentRole::Implementer, "high", Some("claude-sonnet-4-6")).await?;
    pool.turn_start(AgentRole::Implementer, "Write merge sort for Vec<i32>", None).await?;

    while let Some(event) = rx.recv().await {
        match event {
            AgentEvent::MessageDelta { content, .. } => print!("{content}"),
            AgentEvent::TurnCompleted { .. } => break,
            _ => {}
        }
    }
    pool.kill_all().await;
    Ok(())
}
```

This is a complete, working agent invocation in 15 lines of Rust. The same `AgentPool` powers the full orchestration engine.

---

## The Daemon: Event-Driven Autonomous Workflows

### Subscription System

Roko's daemon watches for events and triggers agent workflows automatically:

```toml
[[subscriptions]]
name = "auto-triage-issues"
trigger = "github:issues:opened"
template = "issue-triage"
enabled = true

[[subscriptions]]
name = "review-prs"
trigger = "github:pull_request:opened"
template = "pr-review"
enabled = true

[[subscriptions]]
name = "nightly-research"
trigger = "scheduler:cron"
cron = "0 2 * * *"    # 2 AM daily
template = "research-synthesis"
enabled = true
```

### Event Sources

| Source | How It Works | Events |
|---|---|---|
| **GitHub webhooks** | HMAC-SHA256 verified POST to `/webhooks/github` | Push, PR, issue, comment, review |
| **Slack events** | Socket Mode WebSocket or HTTP POST | Message, mention, reaction |
| **Cron scheduler** | Internal cron with configurable expressions | Timed triggers |
| **File watcher** | `notify` crate, recursive, debounced | File created/modified/deleted |
| **Manual inject** | `roko inject <session> <payload>` | Operator-triggered |

### Daemon Lifecycle

```bash
roko daemon start              # Start in background
roko daemon start --foreground # Start in foreground (dev mode)
roko daemon status             # Check if running
roko daemon logs -f -n 50      # Tail logs
roko daemon reload             # SIGHUP: rescan subscriptions
roko daemon stop               # Graceful shutdown
roko daemon install            # macOS launchd plist
roko daemon uninstall          # Remove plist
```

The daemon + subscriptions enable **fully autonomous development**: issues arrive, agents triage, plans generate, code executes, gates verify, PRs merge — all without human initiation.

---

## Data Lifecycle Management

### Retention Policies

| Data Type | Retention | Compaction | Deletion |
|---|---|---|---|
| Episodes | 90 days | Headline episodes never pruned | Older than 90d removed |
| Efficiency events | 90 days | Aggregated into RoleCostProfiles | Raw events removed |
| Playbook rules | Indefinite | Pruned below 0.1 confidence | Contradicted rules removed |
| Skills | Indefinite | Validated skills persist | Low-confidence skills pruned |
| Patterns | 30 days | Consolidated into playbook rules | Stale patterns removed |
| Cost records | 180 days | Aggregated into CostSummary | Older records removed |
| Routing state | Indefinite | LinUCB arm state persists | Dead arms removed on model change |

### GC Cycle

The Curator runs every ~50 theta ticks (Delta frequency):
1. **Age-based pruning**: Remove episodes/events older than retention window
2. **Confidence-based pruning**: Remove entries below threshold
3. **Contradiction detection**: Identify entries that conflict with recent evidence
4. **Compaction**: Aggregate raw data into summary structures
5. **Disk budget check**: If `.roko/` exceeds configured limit, prune cold storage first

### Disk Budget

Configurable in `roko.toml`:
```toml
[budget]
max_disk_mb = 500  # Total .roko/ directory limit
```

Priority order for eviction: cold Parquet archives → old episodes → old efficiency events → old cost records. Learning state (playbook, skills, routing) is never evicted.

Teams building their own agents need routing, learning, verification, and observability. Roko's crates (roko-learn, roko-gate, roko-agent) are independently useful as libraries.

**Entry point**: "The agent building blocks — routing, gates, learning — as reusable Rust crates."

---

## New UX Patterns

### "Verification-First Development"

Write acceptance criteria → agent generates tests → agent writes code → gates verify → iterate. The developer never writes implementation code; they write specifications.

### "Cost-Aware Automation"

Dashboard shows cost per successful task by model. Developer sets budget. Router learns the cheapest path to success within budget.

### "Collective Intelligence"

One developer discovers a pattern ("always check trait bounds before implementing Display"). The pattern propagates to all team members' agents. The team's collective intelligence grows monotonically.

### "Observable AI"

Every agent decision is logged with full context: why this model, what candidates were considered, what the scores were, what the gate results were, what the conductor recommended. "Why did the AI do that?" is always answerable.

### "Fire and Forget"

Describe work at the PRD level. Walk away. Come back to merged PRs with gate-verified code. The agent handled decomposition, execution, verification, and iteration.

### "Progressive Delegation"

Start by reviewing every AI change. As trust builds (gate pass rate increases, playbook rules accumulate), increase autonomy. The system earns trust through demonstrated competence.

---

## The Developer Experience: From CLI to Ecosystem

### Seven Usage Patterns (Auto-Detected)

Roko adapts its behavior based on how it's invoked. No flags needed — the runtime detects the usage pattern and configures itself:

**1. Interactive REPL**

Detection: `isatty(stdin)` is true, no prompt argument provided.

Behavior: Opens a conversation loop. The developer types prompts, roko responds, context accumulates across turns. Tool calls stream results in real-time. History persists in `.roko/memory/` for session continuity.

```bash
$ roko
roko> What does the gate pipeline do?
The gate pipeline runs 11 verification checks...
roko> Show me the implementer role's pass rate
[queries efficiency events, renders table]
```

**2. One-Shot**

Detection: CLI argument provided (e.g., `roko run "fix the compile error in roko-gate"`).

Behavior: Single task execution. Exit code 0 = gate passed. Exit code 1 = gate failed. Designed for scripting and automation. No interactive state.

```bash
$ roko run "add missing derive(Debug) to all public structs in roko-core"
[executes, gates verify, exits with 0 or 1]
$ echo $?
0
```

**3. Pipe**

Detection: `!isatty(stdin)` and no prompt argument — input is being piped.

Behavior: Reads piped input as context, processes it, writes output to stdout. Designed for Unix pipeline composition.

```bash
$ gh issue view 42 --json body -q .body | roko run "triage this issue and suggest priority"
$ git diff HEAD~1 | roko run "review this diff for security issues"
$ cat error.log | roko run "diagnose the root cause"
```

**4. GitHub Bot**

Detection: `roko serve --github-webhooks` subcommand.

Behavior: Starts an HTTP server listening for GitHub webhook events (HMAC-SHA256 verified). Auto-responds to issues, PRs, and comments based on subscription configuration.

```bash
$ roko serve --github-webhooks --port 8080
# Listening for POST /webhooks/github
# Subscriptions: auto-triage-issues, review-prs, fix-failing-ci
```

**5. CI Agent**

Detection: One-shot mode inside a CI environment (detected via `CI=true` or `GITHUB_ACTIONS=true` env vars).

Behavior: Optimized for CI constraints — no interactive prompts, structured output (JSON for machine parsing), budget-capped to prevent runaway costs, exit code drives CI pass/fail.

```yaml
# .github/workflows/roko-review.yml
- name: Review PR
  run: roko run "review this PR" --format json --budget-max 1.00
  env:
    ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
```

**6. HTTP Service**

Detection: `roko serve` subcommand.

Behavior: REST API + Server-Sent Events (SSE) for real-time streaming. Agent-as-a-Service — any HTTP client can submit tasks, receive streaming results, and query learning metrics.

```bash
$ roko serve --port 3000

# Submit a task
$ curl -X POST localhost:3000/tasks \
    -H "Content-Type: application/json" \
    -d '{"prompt": "implement merge sort", "role": "implementer"}'

# Stream results
$ curl localhost:3000/tasks/abc123/stream
# SSE: data: {"type":"delta","content":"fn merge_sort..."}
# SSE: data: {"type":"gate","passed":true}
# SSE: data: {"type":"completed","iterations":1}
```

**7. Daemon**

Detection: `roko serve --daemon` subcommand.

Behavior: Background workflow engine. Watches for events (GitHub webhooks, cron schedules, file changes, manual injection), triggers agent workflows, manages long-running plan executions. Installs as a system service (launchd on macOS, systemd on Linux).

```bash
$ roko daemon start           # Background start
$ roko daemon status          # Check health
$ roko daemon logs -f -n 50   # Tail logs
$ roko daemon stop            # Graceful shutdown
```

### Mode Selection Logic

```
if subcommand == "serve":
    if --daemon flag: Daemon mode
    elif --github-webhooks flag: GitHub Bot mode
    else: HTTP Service mode
elif prompt argument provided:
    One-Shot mode (CI-optimized if CI env detected)
elif !isatty(stdin):
    Pipe mode
else:
    Interactive REPL mode
```

No mode flag needed. The runtime infers the correct behavior from the invocation context.

---

## The SDK Layer: Composable Agent Components

### Four Independently Publishable Crates

For developers building their own agent systems on top of roko, the SDK provides four crates with clear dependency boundaries:

```
roko-mcp          (isolated — no internal dependencies)
  └── MCP server for code intelligence

roko-agent        (leaf crate — minimal dependencies)
  └── Connection backends, agent pools, event streaming

roko-context      (depends on roko-agent)
  └── Workspace analysis, prompt assembly, section management

roko-eval         (depends on roko-context)
  └── Gate pipeline, assertions, scoring, adaptive thresholds
```

Each crate is independently useful. Import only what you need:

**roko-agent alone (15 lines for a working agent)**:
```rust
use roko_agent::{AgentPool, AgentRole, AgentEvent};

let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
let mut pool = AgentPool::new("/path/to/repo".into(), tx);
pool.spawn(AgentRole::Implementer, "high", Some("claude-sonnet-4-6")).await?;
pool.turn_start(AgentRole::Implementer, "Write merge sort", None).await?;
while let Some(event) = rx.recv().await {
    match event {
        AgentEvent::TurnCompleted { .. } => break,
        _ => {}
    }
}
```

**roko-agent + roko-eval (agent with verification)**:
```rust
use roko_agent::{AgentPool, AgentRole};
use roko_eval::{GatePipeline, GateConfig};

// Run agent
pool.turn_start(AgentRole::Implementer, prompt, None).await?;

// Verify output
let pipeline = GatePipeline::new(GateConfig::default());
let result = pipeline.run_all("/path/to/repo").await?;
if !result.passed {
    // Re-run with error context
    pool.turn_start(AgentRole::Implementer, &format!("{prompt}\n\nGate errors: {}", result.errors.join("\n")), None).await?;
}
```

**All four crates (full orchestration)**:
```rust
use roko_agent::AgentPool;
use roko_context::{ContextBuilder, WorkspaceAnalyzer};
use roko_eval::{GatePipeline, AdaptiveThresholds};
use roko_mcp::McpServer;

// Analyze workspace, build context, run agent, verify, learn
let analyzer = WorkspaceAnalyzer::new("/path/to/repo");
let context = ContextBuilder::new()
    .add_section("repo_map", analyzer.repo_map(50))
    .add_section("relevant_code", analyzer.search("merge sort"))
    .build(8000)?;  // 8K token budget

pool.turn_start(role, &context.render(), None).await?;
let gate_result = pipeline.run_all("/path/to/repo").await?;
thresholds.update(&gate_result);  // Adaptive learning
```

### Extension Traits for New Capabilities

The SDK provides extension traits for adding new languages, providers, gates, and integrations:

| Trait | What It Extends | Example Implementation |
|---|---|---|
| `LanguageProvider` | Language support (parsing, analysis) | Add Python support with tree-sitter-python |
| `BuildSystem` | Build tool integration | Add Bazel support alongside Cargo/npm |
| `LlmBackend` | New LLM provider | Add Gemini, DeepSeek, or local Ollama |
| `Gate` | New verification type | Add security audit gate, performance benchmark gate |
| `EventSource` | New event integration | Add Linear webhooks, PagerDuty alerts |

Each trait has a minimal surface — typically 3-5 methods. A new `LanguageProvider` needs `parse()`, `symbols()`, `dependencies()`, and `diagnostics()`. A new `Gate` needs `name()`, `run()`, and `severity()`.

---

## Data Lifecycle Management

### The .roko/ Directory Structure

All roko state lives under `.roko/` in the project root. The directory is structured by function:

```
.roko/
├── state/
│   └── executor.json               # Crash-recovery checkpoint
│                                    # (plan phases, task progress, agent assignments)
├── prd/
│   ├── ideas/                       # Raw idea captures
│   ├── drafts/                      # PRD drafts (agent-refined)
│   └── published/                   # Finalized PRDs with plans
├── research/
│   ├── topics/                      # Research reports by topic
│   └── enhanced/                    # PRD/plan enhancement artifacts
├── learn/
│   ├── cascade-router.json          # 3-stage model routing state
│   │                                # (static → confidence → UCB)
│   ├── experiments.json             # A/B prompt experiment variants + results
│   ├── gate-thresholds.json         # Adaptive EMA per gate rung
│   ├── efficiency.jsonl             # Per-turn metrics
│   │                                # (tokens, cost, tools, timing, outcome)
│   ├── costs.jsonl                  # Cost records per model per provider
│   ├── playbook.json                # Validated behavioral rules
│   ├── playbook-rules.json          # Rule confidence tracking
│   ├── skills.json                  # Reusable tool-use patterns
│   ├── patterns.json                # Mined patterns from episodes
│   ├── provider-health.json         # Circuit breaker state per provider
│   ├── latency-stats.json           # Per-model latency percentiles
│   ├── section-effects.json         # Prompt section → gate pass correlation
│   ├── model-experiments.json       # Model A/B test results
│   └── routing.jsonl                # Routing decision log
├── episodes.jsonl                   # Agent turn recordings (append-only)
├── signals.jsonl                    # Signal DAG (content-addressed, with lineage)
└── memory/                          # Agent memory (session persistence)
```

### Retention Policies

Different data types have different value curves over time. Retention policies match data lifespan to value:

| Data Type | Default Retention | Rationale |
|---|---|---|
| **Signals** | 7 days | Raw signals are high-volume, low individual value. Patterns extracted within hours. |
| **Episodes** | 30 days | Agent turn recordings. Useful for pattern extraction and debugging recent behavior. |
| **Efficiency events** | 90 days | Per-turn metrics. Aggregated into RoleCostProfiles after 30 days. Raw events kept for trend analysis. |
| **Playbook rules** | Indefinite (until pruned by Curator) | Hard-won behavioral rules. Only removed when contradicted by recent evidence or confidence drops below 0.1. |
| **Skills** | Indefinite (until pruned) | Validated tool-use patterns. Pruned only when consistently unsuccessful. |
| **Cost records** | 180 days | Provider pricing changes slowly. 6 months provides good trend visibility. |
| **Routing state** | Indefinite | Model routing weights are small and always relevant. Dead arms removed on model deprecation. |
| **State snapshots** | Kept until explicit cleanup | Crash-recovery checkpoints. User decides when to clean up via `roko gc --state`. |

### Garbage Collection: `roko gc`

The `roko gc` command performs garbage collection on the `.roko/` directory:

```bash
$ roko gc                    # Full GC cycle (all data types)
$ roko gc --signals          # GC signals only
$ roko gc --episodes         # GC episodes only
$ roko gc --state            # Clean up old state snapshots
$ roko gc --dry-run          # Show what would be deleted without deleting
$ roko gc --aggressive       # Lower retention thresholds by 50%
```

GC operations:

1. **Age-based pruning**: Remove data older than the retention window
2. **Confidence-based pruning**: Remove playbook rules and skills below confidence threshold
3. **Contradiction detection**: Identify entries that conflict with recent evidence, mark for review
4. **Content-addressed deduplication**: Signals are content-addressed (hash of payload). Duplicate signals are deduplicated automatically.
5. **JSONL compaction**: Rewrite JSONL files without deleted entries, reducing file size and improving read performance
6. **Disk budget enforcement**: If `.roko/` exceeds the configured budget (`budget.max_disk_mb` in `roko.toml`), evict by priority: cold archives first, then old episodes, then old efficiency events, then old cost records. Learning state (playbook, skills, routing) is never evicted.

### Brain Export / Import

The most valuable data in `.roko/` is the learned state — routing weights, playbook rules, skill patterns, cost profiles. This data represents accumulated experience from hundreds or thousands of agent runs.

**Brain export** produces a portable artifact:

```bash
$ roko brain export my-project-brain.tar.zst
# Exports: cascade-router.json, playbook.json, skills.json,
#          gate-thresholds.json, section-effects.json,
#          model-experiments.json, routing summary
# Size: typically 100KB-1MB compressed
```

**Brain import** restores learned state on a fresh instance:

```bash
$ roko brain import my-project-brain.tar.zst
# Merges imported state with existing state via CRDT semantics:
# - Routing weights: max-confidence merge
# - Playbook rules: union with conflict resolution by confidence
# - Skills: union with deduplication by HDC similarity
# - Gate thresholds: weighted average by observation count
```

Import uses **CRDT merge semantics**, not overwrite. Importing a brain into an instance that already has learned state produces a merged result that preserves both sources of experience. This enables:

- **Team knowledge sharing**: Developer A exports brain, Developer B imports it. B gets A's routing discoveries and playbook rules without losing their own.
- **Environment migration**: Moving from one machine to another without losing learned state.
- **Onboarding**: New team member imports the team brain and starts with proven configuration instead of cold-starting.
- **Disaster recovery**: Brain exports serve as backups of the most valuable (and hardest to reproduce) data.
