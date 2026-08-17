# PRD-09: Extensibility, multi-chain agents, and predictive foraging

*Package ecosystem. Chain-agnostic ingestion. Attention as a budget.*

**Status:** Draft
**Author:** Will
**Date:** 2026-04-21
**Crates affected:** `roko-ext-registry` (new), `roko-quickjs` (new), `roko-chain-ingest` (new), `roko-foraging` (new), `roko-worldgraph` (new), `roko-core` (extend `DomainProfile`, add `ChainConnector` trait), `roko-runtime` (multi-actor chain subscription), `roko-cli` (new subcommands), `roko-compose` (WorldGraph context injection), `roko-learn` (foraging bandit integration)

---

## Table of contents

1. [Why this PRD exists](#1-why-this-prd-exists)
2. [The package ecosystem vision](#2-the-package-ecosystem-vision)
3. [Three tiers of packages](#3-three-tiers-of-packages)
4. [The `roko install` command](#4-the-roko-install-command)
5. [Manifest format](#5-manifest-format)
6. [Package loading architecture](#6-package-loading-architecture)
7. [QuickJS bridge for Pi-compatible extensions](#7-quickjs-bridge-for-pi-compatible-extensions)
8. [Multi-domain agents](#8-multi-domain-agents)
9. [Multi-chain blockchain architecture](#9-multi-chain-blockchain-architecture)
10. [Hierarchical temporal resolution](#10-hierarchical-temporal-resolution)
11. [Reorg handling and finality](#11-reorg-handling-and-finality)
12. [The ChainConnector trait](#12-the-chainconnector-trait)
13. [Predictive foraging](#13-predictive-foraging)
14. [Dynamic contract discovery](#14-dynamic-contract-discovery)
15. [Dynamic worldview building](#15-dynamic-worldview-building)
16. [Active inference for attention allocation](#16-active-inference-for-attention-allocation)
17. [The `roko publish` ecosystem](#17-the-roko-publish-ecosystem)
18. [HuggingFace integration (Stream C)](#18-huggingface-integration-stream-c)
19. [SWE-bench and arena integration](#19-swe-bench-and-arena-integration)
20. [Integration with prior PRDs](#20-integration-with-prior-prds)
21. [Synergistic scaling properties](#21-synergistic-scaling-properties)
22. [Implementation phasing](#22-implementation-phasing)
23. [References](#23-references)

---

## 1. Why this PRD exists

### The extensibility gap

Roko's architecture separates concerns cleanly. The runtime runs the heartbeat. Extensions inject domain logic. Profiles configure timing. Gates verify output. The composition layer assembles context. These pieces connect through well-defined trait boundaries.

What Roko lacks is a way for anyone outside the core team to contribute new pieces.

Today, adding a new extension means writing Rust code inside the roko monorepo, adding a crate to the workspace, and rebuilding the binary. Adding a new chain means modifying `roko-chain` internals. Adding a new domain profile means editing `roko-core`. The system is modular internally but closed externally.

Pi (pi.dev) solved this for a simpler problem. Pi is a TypeScript agent framework with a package ecosystem: extensions, skills, prompts, and themes. Anyone can publish a Pi package to npm. Anyone can install one with `pi install`. The ecosystem grows independently of the core team.

Pi's model works because the units of composition are small and the interface contract is narrow. An extension registers tools and hooks. A skill is a Markdown file with frontmatter. A prompt is a template. None of these require understanding the framework internals.

Roko needs the same -- but Roko is not a thin tool wrapper. Roko has a cognitive architecture. Extensions in Roko participate in a heartbeat pipeline with 22 hooks, access CorticalState, interact with somatic markers, and influence context assembly. The package system must support this depth while remaining accessible to someone who wants to write a three-function tool extension.

### The multi-chain gap

PRD-06 defined the blockchain domain profile. It specified tick timing, event subscriptions, and gate configuration for a chain agent. What it did not specify is how an agent subscribes to multiple chains simultaneously.

This matters because no serious blockchain agent operates on a single chain. A DeFi agent monitoring yield across Aave (Ethereum), Morpho (Base), and Hyperliquid needs data from three chains with three different block times. An arbitrage agent needs sub-second visibility across Korai (50ms blocks), Arbitrum (~250ms), and Base (2s). A bridge monitoring agent, by definition, watches at least two chains.

The current architecture has no mechanism for this. One agent, one chain subscription, one event stream. Multi-chain requires an actor-per-chain model where each chain runs its own async task, normalizes events into a chain-agnostic format, and feeds a unified event bus that the heartbeat pipeline consumes.

### The attention gap

Giving an agent access to five chains and 10,000 contracts creates a new problem: what should it pay attention to? Monitoring everything at full resolution is computationally infeasible and economically wasteful. Most contracts on most chains produce nothing relevant to the agent's strategy on most blocks.

The agent needs a mechanism to allocate attention dynamically -- spending more monitoring budget on high-value sources and less on low-value ones, with exploration of uncertain sources built in. This is the predictive foraging model: an application of optimal foraging theory (Charnov, 1976) and multi-armed bandit theory (Gittins & Jones, 1974) to the problem of which data sources an agent should monitor.

### What this PRD covers

Six connected capabilities:

1. **Package ecosystem** -- a way to install, publish, and manage extensions, skills, profiles, chain connectors, and arena definitions. Compatible with Pi's ecosystem. Goes beyond it.
2. **`roko install` command** -- the CLI surface for package management.
3. **Multi-domain agents** -- agents that span multiple domains simultaneously (e.g., blockchain + research).
4. **Multi-chain architecture** -- actor-per-chain ingestion with a unified canonical event bus.
5. **Predictive foraging** -- dynamic attention allocation using Gittins indices and active inference.
6. **Dynamic worldview building** -- a living graph of entities and relationships discovered through foraging, with no hardcoded knowledge.

These six form a coherent system. The package ecosystem lets anyone add chain connectors and domain profiles. Multi-domain composition lets an agent use several profiles at once. Multi-chain ingestion feeds the agent data from many sources. Foraging decides which sources deserve attention. The worldview accumulates what the agent discovers.

---

## 2. The package ecosystem vision

### Philosophy

Roko should be a strict superset of Pi's capabilities. Any Pi package -- extension, skill, prompt, theme -- should work with `roko install`. The compatibility is not approximate. A Pi extension that calls `pi.registerTool()` and `pi.on('tool_call', ...)` runs inside Roko's QuickJS sandbox and its tools appear in Roko's DynamicToolRegistry.

But compatibility is table stakes. Roko adds package types that Pi cannot support because Pi does not have a cognitive architecture:

- **Cognitive extensions** participate in the 22-hook heartbeat pipeline. They can influence context assembly, modify somatic markers, inject context sections, and shape tier routing decisions. A Pi extension registers tools and handles events. A Roko cognitive extension perceives, reasons, and adapts within the heartbeat loop.
- **Domain profiles** configure an agent's entire cognitive posture: tick timing per regime, extension sets, event subscriptions, context weights, gate pipelines, and infrastructure requirements. Pi has no equivalent because Pi does not have a heartbeat.
- **Arena definitions** implement benchmarking protocols for evaluating agent capabilities. Roko agents earn reputation through arena scores (PRD-06, section 6). Pi has no reputation or arena system.
- **Chain connectors** integrate new blockchains into the multi-chain actor model. Pi has no chain awareness.
- **InsightStore modules** define query strategies for the on-chain knowledge substrate (PRD-05, section 3). Pi has no knowledge network.

The result is a three-tier package taxonomy. Tier 1 works in both Pi and Roko. Tier 2 works in Roko with graceful degradation in Pi. Tier 3 is Roko-only.

### Design principles

**Convention over configuration.** A minimal package needs one file: a `SKILL.md` for skills, a `Cargo.toml` with `[package.metadata.roko]` for Rust extensions, or a `package.json` with a `pi` key for JavaScript extensions. Everything else has sensible defaults.

**Type safety at the boundary.** Rust packages implement concrete traits (`Extension`, `ChainConnector`, `Arena`). The compiler catches incompatibilities at install time, not at runtime. JavaScript packages run in a sandboxed QuickJS environment with a typed API surface.

**No ambient authority.** Installed packages declare their capabilities in the manifest. An extension that needs network access declares it. An extension that needs filesystem access declares it. The runtime checks declarations against the agent's security policy (PRD-02, section 10) before granting access. A package cannot silently escalate its privileges.

**Reproducible environments.** `roko install` records exact versions in a lockfile (`.roko/packages.lock`). A fresh clone with `roko install` produces the same extension set on every machine.

---

## 3. Three tiers of packages

### Tier 1: Pi-compatible

These packages work identically in Pi and Roko. They use only the shared API surface.

| Type | Format | Description | Example |
|------|--------|-------------|---------|
| Extension | TypeScript/JavaScript | Registers tools and event handlers via the `pi` API object | A weather tool, a Jira integration, a file search utility |
| Skill | Markdown with SKILL.md frontmatter | Structured knowledge that agents load on demand | A code review skill, a writing style guide, a debugging checklist |
| Prompt | Markdown with `{{variable}}` placeholders | Reusable prompt templates | A commit message template, a PR description template |
| Theme | JSON | Visual configuration for the TUI/CLI | A dark theme, a high-contrast accessibility theme |

**Extension API surface (Pi-compatible subset):**

```typescript
// This is the API that Pi extensions use.
// Roko's QuickJS bridge implements this identically.

interface PiExtension {
  // Tool registration
  registerTool(tool: ToolDefinition): void;

  // Event hooks (subset that maps cleanly to Roko extension hooks)
  on(event: "tool_call", handler: (call: ToolCall) => MaybePromise<void>): void;
  on(event: "tool_result", handler: (result: ToolResult) => MaybePromise<void>): void;
  on(event: "turn_start", handler: (turn: TurnInfo) => MaybePromise<void>): void;
  on(event: "turn_end", handler: (turn: TurnInfo) => MaybePromise<void>): void;
  on(event: "error", handler: (error: ErrorInfo) => MaybePromise<void>): void;

  // Configuration
  getConfig(): Record<string, unknown>;
  setConfig(key: string, value: unknown): void;

  // Logging
  log(level: "debug" | "info" | "warn" | "error", message: string): void;
}

interface ToolDefinition {
  name: string;
  description: string;
  parameters: JsonSchema;
  handler: (params: Record<string, unknown>) => MaybePromise<ToolOutput>;
}
```

**Skill format:**

```markdown
---
name: code-review
version: 1.0.0
description: Structured code review methodology
triggers:
  - review
  - pr
  - pull request
---

# Code review skill

## When reviewing code

1. Read the diff completely before commenting.
2. Separate style issues from logic issues.
3. ...
```

Skills are Markdown files with YAML frontmatter. The `triggers` field tells the agent when to load the skill into context. Triggers match against task descriptions, tool names, and explicit user requests.

### Tier 2: Roko-enhanced

These packages use Roko-specific APIs that go beyond Pi's surface. When loaded in Pi, the Roko-specific hooks are no-ops and the extension functions with reduced capability.

```typescript
// Tier 2 extension: uses both pi and roko APIs.
// In Pi: roko object is undefined, extension works with pi API only.
// In Roko: both APIs are available.

export function activate(pi: PiExtension, roko?: RokoExtension) {
  // Pi-compatible tool registration
  pi.registerTool({
    name: "check_position",
    description: "Check DeFi position health",
    parameters: { /* ... */ },
    handler: async (params) => {
      // ... tool implementation
    },
  });

  // Roko-specific: heartbeat hook
  if (roko) {
    roko.on("tick_start", async (tick) => {
      // Access CorticalState for prediction error
      const pe = tick.corticalState.predictionError;
      if (pe > 0.7) {
        roko.injectContext("position_alert", buildAlertContext());
      }
    });

    roko.on("assemble_context", async (ctx) => {
      // Bid for context space in the VCG auction
      ctx.bid("defi_positions", {
        content: await getCurrentPositions(),
        value: 0.8,  // bid weight
        category: "strategy",
      });
    });
  }
}
```

**Roko-specific API surface (available to Tier 2 and Tier 3 JS extensions):**

```typescript
interface RokoExtension {
  // Heartbeat hooks (subset of the 22 Rust hooks, exposed to JS)
  on(event: "tick_start", handler: (tick: TickInfo) => MaybePromise<void>): void;
  on(event: "tick_end", handler: (tick: TickInfo) => MaybePromise<void>): void;
  on(event: "assemble_context", handler: (ctx: ContextAssembly) => MaybePromise<void>): void;
  on(event: "before_inference", handler: (req: InferenceRequest) => MaybePromise<void>): void;
  on(event: "after_inference", handler: (res: InferenceResponse) => MaybePromise<void>): void;
  on(event: "observe", handler: (obs: Observation) => MaybePromise<void>): void;

  // Context injection
  injectContext(key: string, content: string): void;

  // CorticalState read access (read-only)
  getCorticalState(): CorticalStateSnapshot;

  // Somatic marker access
  getSomaticMarkers(): SomaticMarkerSet;

  // Knowledge store queries
  queryKnowledge(query: string, limit?: number): Promise<KnowledgeEntry[]>;
}
```

The Roko API is deliberately narrower than the full Rust `Extension` trait. JavaScript extensions cannot modify somatic markers, alter tier routing directly, or write to the knowledge store. Those operations require the safety guarantees that Rust's type system provides.

### Tier 3: Roko-native

These packages exist only in Roko. They implement Rust traits and participate fully in the cognitive architecture. No Pi equivalent exists.

| Type | Trait | Description |
|------|-------|-------------|
| Cognitive extension | `Extension` (22 hooks) | Full heartbeat participant with CorticalState access, somatic marker mutation, tier routing influence |
| Domain profile | `DomainProfileProvider` | Complete agent configuration: timing, extensions, events, context weights, gates, infrastructure |
| Arena definition | `Arena` | Benchmark protocol for agent evaluation with scoring functions and leaderboards |
| Chain connector | `ChainConnector` | Blockchain integration: block subscription, event normalization, contract calls |
| InsightStore module | `InsightQueryStrategy` | On-chain knowledge query strategy: how to search, filter, and rank shared knowledge |

**Cognitive extension example (Rust):**

```rust
use roko_core::extension::{Extension, ExtensionContext, HookResult};
use roko_core::cortical::CorticalState;
use roko_core::somatic::SomaticMarker;

/// Monitors DeFi position health across all tracked vaults and pools.
/// Injects alerts into context when positions approach liquidation thresholds.
pub struct PositionMonitorExt {
    positions: Vec<TrackedPosition>,
    alert_threshold: f64,
    last_check: Instant,
}

#[async_trait]
impl Extension for PositionMonitorExt {
    fn name(&self) -> &str { "position-monitor" }
    fn layer(&self) -> u8 { 4 } // Application layer

    async fn on_tick_start(
        &mut self,
        ctx: &mut ExtensionContext,
    ) -> HookResult {
        let cortical = ctx.cortical_state();
        let regime = cortical.regime();

        // In crisis regime, check positions every tick.
        // In calm regime, check every 10th tick.
        let check_interval = match regime {
            Regime::Crisis => Duration::from_secs(0),
            Regime::Volatile => Duration::from_secs(15),
            Regime::Normal => Duration::from_secs(60),
            Regime::Calm => Duration::from_secs(120),
        };

        if self.last_check.elapsed() < check_interval {
            return HookResult::Continue;
        }
        self.last_check = Instant::now();

        for position in &self.positions {
            let health = position.health_factor().await?;
            if health < self.alert_threshold {
                // Inject alert into context for this tick
                ctx.inject_context(
                    "position_alert",
                    format!(
                        "ALERT: Position {} on {} health factor {:.2} (threshold: {:.2}). \
                         Collateral: {}, Debt: {}, Liquidation price: {}",
                        position.id, position.chain, health, self.alert_threshold,
                        position.collateral, position.debt, position.liquidation_price,
                    ),
                );

                // Set somatic marker so the agent "feels" urgency
                ctx.set_somatic_marker(
                    SomaticMarker::urgency(health.recip()),
                );
            }
        }

        HookResult::Continue
    }

    async fn on_assemble_context(
        &mut self,
        ctx: &mut ExtensionContext,
        assembly: &mut ContextAssembly,
    ) -> HookResult {
        // Bid for context space proportional to position risk
        let max_risk = self.positions.iter()
            .filter_map(|p| p.cached_health())
            .map(|h| 1.0 / h)
            .fold(0.0_f64, f64::max);

        if max_risk > 0.1 {
            assembly.bid(ContextBid {
                key: "defi_positions".into(),
                content: self.format_position_summary(),
                value: (max_risk * 0.9).min(1.0), // Scale bid with risk
                category: ContextCategory::Strategy,
                ttl: Duration::from_secs(30),
            });
        }

        HookResult::Continue
    }

    async fn on_outcome(
        &mut self,
        ctx: &mut ExtensionContext,
        outcome: &TaskOutcome,
    ) -> HookResult {
        // Learn from position management outcomes.
        // If the agent took an action on a position and it succeeded,
        // record the health-factor-at-action as a somatic reference point.
        if let Some(position_action) = outcome.extract_position_action() {
            let health_at_action = position_action.health_factor;
            ctx.record_episode_annotation(
                "position_action",
                serde_json::json!({
                    "position": position_action.position_id,
                    "action": position_action.action_type,
                    "health_at_action": health_at_action,
                    "outcome": outcome.success,
                }),
            );
        }

        HookResult::Continue
    }
}
```

This extension participates in the heartbeat pipeline, injects context through the VCG auction, sets somatic markers, and records structured episode annotations. None of this is possible in Pi's model.

---

## 4. The `roko install` command

### Source types

Four source types, matching Pi's three plus local paths:

```bash
# Cargo crates (Rust native, compiled, most performant)
roko install crate:roko-ext-defi
roko install crate:roko-ext-defi@0.3.0

# npm packages (Pi-compatible, JS/TS via embedded QuickJS)
roko install npm:@pi/my-extension
roko install npm:@foo/pi-tools@1.2.3

# Git repositories (either Cargo or npm, auto-detected)
roko install git:github.com/user/roko-extension
roko install git:github.com/user/roko-extension@v1.0.0

# Local paths (for development)
roko install ./my-local-extension
```

### Resolution algorithm

```
1. Parse source specifier (crate:, npm:, git:, path)
2. Fetch metadata:
   - crate: query crates.io (or configured registry) for [package.metadata.roko]
   - npm: query npm registry for package.json with "pi" or "roko" keys
   - git: clone repo, detect Cargo.toml or package.json
   - path: read local manifest
3. Determine package type (extension, skill, prompt, theme, profile, arena, chain-connector)
4. Resolve dependencies:
   - Cargo dependencies via cargo's resolver
   - npm dependencies via a minimal npm-compatible resolver
   - Cross-ecosystem dependencies not supported (a Cargo package cannot depend on an npm package)
5. Check compatibility:
   - Verify roko version compatibility (roko_version field in manifest)
   - Verify required capabilities against agent security policy
6. Install:
   - Cargo: download, compile, place binary artifact in .roko/packages/bin/
   - npm: download, extract to .roko/packages/npm/<name>/
   - Skills/prompts/themes: copy to .roko/packages/<type>/<name>/
7. Update lockfile (.roko/packages.lock)
8. Register in package database (.roko/packages/registry.json)
```

### Full command surface

```bash
# Install
roko install <source>              # Install a package
roko install <source> --profile    # Install and activate for a specific profile

# Remove
roko remove <package-name>         # Remove an installed package

# List
roko list                          # List all installed packages
roko list --type extension         # Filter by type
roko list --type skill
roko list --type profile
roko list --type arena
roko list --type chain-connector
roko list --type prompt
roko list --type theme

# Update
roko update <package-name>         # Update a specific package
roko update --all                  # Update all packages

# Search
roko search <query>                # Search the registry
roko search defi --type extension  # Search with type filter
roko search solana --type chain    # Find chain connectors

# Publish
roko publish                       # Publish current directory to roko registry
roko publish --dry-run             # Validate without publishing

# Marketplace
roko market                        # Open TUI package browser
roko market --web                  # Open web-based package browser
roko market --category blockchain  # Filter by category

# Package info
roko info <package-name>           # Detailed info about a package
roko info <package-name> --deps    # Show dependency tree
```

### Compilation model for Rust packages

Rust extensions compile to dynamic libraries (`.dylib` / `.so` / `.dll`). Roko loads them at runtime via `libloading`. The extension exposes a C-compatible entry point:

```rust
/// Every Roko Rust extension exports this function.
/// The runtime calls it once during loading to obtain the Extension trait object.
#[no_mangle]
pub extern "C" fn roko_extension_create() -> *mut dyn Extension {
    let ext = PositionMonitorExt::new(Default::default());
    Box::into_raw(Box::new(ext))
}

/// Version check. The runtime verifies ABI compatibility before loading.
#[no_mangle]
pub extern "C" fn roko_extension_abi_version() -> u32 {
    roko_core::ABI_VERSION // Currently 1
}
```

ABI stability is enforced through a version check. If the extension was compiled against a different `ABI_VERSION` than the running roko binary, loading fails with a clear error message telling the user to recompile.

For extensions that need tighter integration (access to internal types without FFI marshaling), `roko install crate:` can alternatively add the crate as a workspace dependency and trigger a workspace rebuild. This path is slower but avoids all ABI concerns. The manifest's `loading_mode` field controls which strategy to use:

```toml
[package.metadata.roko]
loading_mode = "dynamic"   # Default. Compile to .dylib, load at runtime.
# loading_mode = "static"  # Add to workspace, rebuild binary. Slower install, no ABI boundary.
```

### Storage layout

```
.roko/
  packages/
    registry.json              # Index of all installed packages
    packages.lock              # Exact versions for reproducibility
    bin/                       # Compiled Rust extension .dylib files
      roko-ext-defi.dylib
      roko-chain-solana.dylib
    npm/                       # Extracted npm packages
      @pi/
        my-extension/
          package.json
          dist/
    skills/                    # Installed skill files
      code-review/
        SKILL.md
    prompts/                   # Installed prompt templates
      commit-message/
        prompt.md
    themes/                    # Installed themes
      dark-pro/
        theme.json
    profiles/                  # Installed domain profiles
      quant/
        profile.toml
    arenas/                    # Installed arena definitions
      humaneval/
        arena.toml
```

---

## 5. Manifest format

### Rust packages (Cargo.toml)

Every Rust package uses the standard `[package.metadata.roko]` section in `Cargo.toml`:

```toml
[package]
name = "roko-ext-defi"
version = "0.1.0"
edition = "2024"
description = "DeFi position monitoring and yield optimization"
license = "MIT OR Apache-2.0"

[package.metadata.roko]
# Package classification
type = "cognitive-extension"
# Allowed values:
#   extension             - Pi-compatible tool extension (Tier 1)
#   cognitive-extension   - Full heartbeat participant (Tier 3)
#   skill                 - Markdown skill file
#   prompt                - Prompt template
#   theme                 - TUI theme
#   domain-profile        - Agent configuration bundle
#   arena                 - Benchmark definition
#   chain-connector       - Blockchain integration
#   insight-module        - InsightStore query strategy

# Extension layer (cognitive extensions only).
# Layers 0-3: core (reserved for roko-ext-core).
# Layer 4: application (most extensions).
# Layer 5+: monitoring/diagnostic.
layer = 4

# Domains this extension is designed for.
# Extensions load automatically when their domain profile is active.
domains = ["blockchain"]

# Roko version compatibility.
# Semver range. The runtime checks this before loading.
roko_version = ">=0.5.0, <1.0.0"

# ABI version. Must match roko_core::ABI_VERSION.
abi_version = 1

# Loading mode (cognitive extensions only).
loading_mode = "dynamic"

# Whether this also works as a Pi extension.
# If true, the package must include a package.json with pi-compatible entry.
pi_compatible = false

# Dependencies on other roko packages.
# These are roko-ecosystem dependencies, not Cargo dependencies.
# The installer resolves them before installing this package.
depends_on = ["roko-ext-chain-subscriber"]

# Required capabilities. The runtime checks these against the agent's
# security policy before granting access.
capabilities = [
    "network:rpc",          # Make RPC calls to blockchain nodes
    "network:http",         # Make HTTP requests
    "state:cortical:read",  # Read CorticalState
    "state:somatic:write",  # Modify somatic markers
    "context:inject",       # Inject context sections
    "context:bid",          # Participate in VCG auction
]

# Bundled resources.
[package.metadata.roko.resources]
skills = ["skills/"]
prompts = ["prompts/"]

# Gallery metadata for marketplace display.
[package.metadata.roko.gallery]
description = "DeFi position monitoring and yield optimization for Roko agents"
tags = ["defi", "yield", "blockchain", "lending", "positions"]
image = "./assets/preview.webp"
video = "https://example.com/demo.mp4"
author_url = "https://github.com/example"
```

### npm packages (package.json)

Pi-compatible packages use the `pi` key. Roko-enhanced packages add a `roko` key:

```json
{
  "name": "@example/defi-tools",
  "version": "1.0.0",
  "description": "DeFi monitoring tools for Pi and Roko agents",
  "pi": {
    "extensions": ["./dist/extensions"],
    "skills": ["./skills"],
    "prompts": ["./prompts"]
  },
  "roko": {
    "tier": 2,
    "cognitive_hooks": [
      "tick_start",
      "assemble_context",
      "observe"
    ],
    "domains": ["blockchain"],
    "capabilities": [
      "network:rpc",
      "state:cortical:read",
      "context:bid"
    ],
    "roko_version": ">=0.5.0"
  }
}
```

When `roko install npm:@example/defi-tools` runs:

1. The installer reads `package.json`.
2. If a `pi` key exists, the package is at least Tier 1.
3. If a `roko` key also exists, the package is Tier 2.
4. The `cognitive_hooks` array tells the QuickJS bridge which Roko-specific hooks to wire up.
5. If a hook is listed in `cognitive_hooks` but the extension does not call `roko.on()` for that hook, the bridge silently ignores it.

### Skill manifest

Skills use YAML frontmatter in a Markdown file:

```yaml
---
name: delta-neutral-strategy
version: 1.2.0
description: Delta-neutral market making strategy for DeFi agents
author: example
domains:
  - blockchain
triggers:
  - delta neutral
  - market making
  - hedging
  - basis trade
context_category: strategy
priority: 0.7
roko_version: ">=0.5.0"
---
```

### Domain profile manifest

Domain profiles use TOML:

```toml
[profile]
name = "quant"
version = "0.1.0"
description = "Quantitative trading agent profile"
extends = "blockchain"  # Inherits from built-in blockchain profile

[clock.gamma]
calm = 60
normal = 15
volatile = 5
crisis = 2

[clock.theta]
calm = 300
normal = 60
volatile = 20
crisis = 10

[clock.delta]
episode_threshold = 200
idle_timeout_secs = 600
sleep_pressure_threshold = 40.0

[extensions]
required = [
    "heartbeat",
    "context",
    "daimon",
    "learning",
    "dreams",
    "chain-subscriber",
    "position-monitor",
    "foraging",
    "worldgraph",
]
optional = [
    "mev-detection",
    "bridge-monitor",
]

[[wakeup_events]]
event_type = "NewBlock"
severity_threshold = 0.0

[[wakeup_events]]
event_type = "PriceFeed"
severity_threshold = 0.3

[[wakeup_events]]
event_type = "LiquidationRisk"
severity_threshold = 0.0

[context_weights]
chain_events = 0.25
positions = 0.20
strategy = 0.20
knowledge = 0.15
market_state = 0.10
affect = 0.05
playbook = 0.05

[[gates]]
name = "simulation"
required = true
timeout_secs = 30

[[gates]]
name = "invariant_check"
required = true
timeout_secs = 10

[[gates]]
name = "risk_limit"
required = true
timeout_secs = 5

[[gates]]
name = "slippage_check"
required = false
timeout_secs = 5

[infrastructure]
rpc_endpoints = ["ethereum", "base", "arbitrum", "korai"]
websocket_subscriptions = ["ethereum:newHeads", "base:newHeads", "korai:newHeads"]
http_apis = ["coingecko", "defillama"]
```

### Chain connector manifest

```toml
[package.metadata.roko]
type = "chain-connector"
chain_type = "evm"         # evm, svm, movevm, custom
chain_id = "solana"
block_time_ms = 400
finality_mode = "probabilistic"
finality_confirmations = 32
roko_version = ">=0.5.0"
abi_version = 1
```

---

## 6. Package loading architecture

### Loading order

The runtime loads packages in a strict order during agent provisioning. Later stages can depend on earlier stages but not the reverse.

```
Stage 0: Core extensions
         HeartbeatExt, ContextExt, DaimonExt, LearningExt, DreamsExt,
         GateExt, ConductorExt
         Always loaded. Cannot be overridden.

Stage 1: Domain extensions (from active DomainProfile)
         ChainSubscriberExt, GitExt, ResearchExt, etc.
         Loaded based on the profile's `extensions.required` list.

Stage 2: Installed cognitive extensions (Rust, .dylib)
         Loaded from .roko/packages/bin/
         Filtered by the extension's `domains` field:
           - If the extension declares domains, load only when those domains are active.
           - If the extension declares no domains, load always.

Stage 3: Installed Pi-compatible extensions (JS/TS, QuickJS)
         Loaded from .roko/packages/npm/
         Each runs in its own QuickJS isolate.

Stage 4: Project-local extensions
         From .roko/extensions/ in the workspace root.
         Can be Rust (.rs compiled to .dylib) or JavaScript (.js/.ts).

Stage 5: Skills, prompts, themes
         Discovered and indexed from .roko/packages/skills/, prompts/, themes/
         and project-local equivalents.
         Not loaded into memory -- retrieved on demand when triggers match.
```

### Extension chain assembly

After loading, extensions are ordered into a chain. The chain determines hook execution order:

```rust
/// Assemble the extension chain for an agent.
///
/// Extensions are sorted by (layer, load_order) where:
/// - layer: the extension's declared layer (0-5)
/// - load_order: the order within the layer (core first, then domain,
///   then installed, then project-local)
///
/// Hook execution follows this chain order. Each hook receives
/// an ExtensionContext that accumulates mutations across the chain.
pub fn assemble_extension_chain(
    core: Vec<Box<dyn Extension>>,
    domain: Vec<Box<dyn Extension>>,
    installed_rust: Vec<Box<dyn Extension>>,
    installed_js: Vec<Box<dyn Extension>>,
    local: Vec<Box<dyn Extension>>,
) -> ExtensionChain {
    let mut all: Vec<(u8, usize, Box<dyn Extension>)> = Vec::new();

    for (load_order, ext) in core.into_iter().enumerate() {
        all.push((ext.layer(), load_order, ext));
    }
    let base = all.len();
    for (i, ext) in domain.into_iter().enumerate() {
        all.push((ext.layer(), base + i, ext));
    }
    let base = all.len();
    for (i, ext) in installed_rust.into_iter().enumerate() {
        all.push((ext.layer(), base + i, ext));
    }
    let base = all.len();
    for (i, ext) in installed_js.into_iter().enumerate() {
        all.push((ext.layer(), base + i, ext));
    }
    let base = all.len();
    for (i, ext) in local.into_iter().enumerate() {
        all.push((ext.layer(), base + i, ext));
    }

    all.sort_by_key(|(layer, order, _)| (*layer, *order));

    ExtensionChain {
        extensions: all.into_iter().map(|(_, _, ext)| ext).collect(),
    }
}
```

### Dynamic loading for Rust extensions

```rust
use libloading::{Library, Symbol};

/// Load a compiled Rust extension from a .dylib file.
///
/// Verifies ABI compatibility before returning the Extension trait object.
/// Returns an error if:
/// - The library cannot be loaded (missing file, wrong architecture)
/// - The ABI version does not match
/// - The entry point function is missing
pub unsafe fn load_rust_extension(
    path: &Path,
) -> Result<(Library, Box<dyn Extension>), ExtensionLoadError> {
    let lib = Library::new(path)
        .map_err(|e| ExtensionLoadError::LoadFailed {
            path: path.to_path_buf(),
            source: e,
        })?;

    // Check ABI version first.
    let abi_version: Symbol<unsafe extern "C" fn() -> u32> =
        lib.get(b"roko_extension_abi_version")
            .map_err(|_| ExtensionLoadError::MissingAbiVersion {
                path: path.to_path_buf(),
            })?;

    let version = abi_version();
    if version != roko_core::ABI_VERSION {
        return Err(ExtensionLoadError::AbiMismatch {
            path: path.to_path_buf(),
            expected: roko_core::ABI_VERSION,
            found: version,
        });
    }

    // Load the extension.
    let create: Symbol<unsafe extern "C" fn() -> *mut dyn Extension> =
        lib.get(b"roko_extension_create")
            .map_err(|_| ExtensionLoadError::MissingEntryPoint {
                path: path.to_path_buf(),
            })?;

    let raw = create();
    let ext = Box::from_raw(raw);

    // Keep the Library alive -- dropping it would unload the dylib
    // and invalidate the Extension vtable.
    Ok((lib, ext))
}
```

---

## 7. QuickJS bridge for Pi-compatible extensions

### Why QuickJS

Pi extensions are TypeScript/JavaScript. Running them in Roko requires an embedded JS runtime. The options:

| Runtime | Binary size | Startup time | ES2023 support | Sandboxing | Maintenance |
|---------|-------------|--------------|----------------|------------|-------------|
| V8 (via rusty_v8) | ~30MB | ~50ms | Full | Process isolation | Google-maintained |
| QuickJS (via rquickjs) | ~600KB | <1ms | Full (ES2023) | Single-thread isolate | Bellard, active |
| Boa | ~2MB | ~5ms | Partial | Single-thread | Community, partial spec |
| Deno core | ~40MB | ~100ms | Full | Multi-layer | Deno team |

QuickJS wins on binary size and startup time. A roko agent that loads 10 Pi extensions should not add 300MB to the binary. QuickJS adds 600KB total, starts in under a millisecond per isolate, and supports every ES2023 feature that Pi extensions use. The `rquickjs` crate provides safe Rust bindings with proper lifetime management.

The tradeoff is performance. V8 JIT-compiles JavaScript; QuickJS interprets it. For agent extensions, this does not matter. Extension hooks run once per tick (every 5-120 seconds). The hook body makes RPC calls, formats strings, and returns JSON. None of this is compute-bound. A QuickJS-interpreted hook that takes 2ms vs a V8-JIT-compiled hook that takes 0.1ms is invisible when the tick interval is 5,000ms.

### Bridge architecture

Each JavaScript extension runs in its own QuickJS `Runtime` (isolate). Isolates share nothing: no globals, no prototype chain, no mutable state. An extension cannot interfere with another extension or with the Roko runtime.

```rust
use rquickjs::{Context, Runtime, Function, Object, Value};

/// Wraps a Pi-compatible JavaScript extension in a QuickJS isolate
/// and implements the Roko Extension trait.
///
/// The bridge translates between Roko's 22-hook Extension trait
/// and Pi's event-based API. Hooks that the JS extension did not
/// register handlers for are no-ops.
pub struct JsExtensionBridge {
    /// Name from package.json.
    name: String,
    /// QuickJS runtime (one per extension, isolated).
    runtime: Runtime,
    /// QuickJS context within the runtime.
    context: Context,
    /// Registered tool definitions (from pi.registerTool calls).
    tools: Vec<JsToolDefinition>,
    /// Hook handlers registered via pi.on() and roko.on().
    hooks: HashMap<String, JsHookHandler>,
    /// Declared capabilities from manifest.
    capabilities: Vec<Capability>,
}

impl JsExtensionBridge {
    /// Load and initialize a JavaScript extension.
    ///
    /// 1. Creates a QuickJS runtime with memory limits.
    /// 2. Injects the `pi` global object (Pi-compatible API).
    /// 3. Optionally injects the `roko` global object (Roko-specific API).
    /// 4. Evaluates the extension's entry point script.
    /// 5. Calls the `activate(pi, roko?)` export.
    pub fn load(
        package_dir: &Path,
        manifest: &PackageManifest,
        capabilities: &[Capability],
    ) -> Result<Self, ExtensionLoadError> {
        let runtime = Runtime::new()?;

        // Memory limit: 64MB per extension isolate.
        // Extensions that exceed this are killed with an error.
        runtime.set_memory_limit(64 * 1024 * 1024);

        // Max stack size: 1MB.
        runtime.set_max_stack_size(1024 * 1024);

        let context = Context::full(&runtime)?;
        let mut tools = Vec::new();
        let mut hooks = HashMap::new();

        context.with(|ctx| {
            // Inject the `pi` global object.
            let pi_obj = create_pi_api(&ctx, &mut tools, &mut hooks)?;
            ctx.globals().set("pi", pi_obj)?;

            // If the package declares roko-specific hooks, inject the `roko` object.
            if manifest.has_roko_hooks() {
                let roko_obj = create_roko_api(&ctx, &mut hooks, capabilities)?;
                ctx.globals().set("roko", roko_obj)?;
            }

            // Load and evaluate the entry point.
            let entry = manifest.entry_point();
            let source = std::fs::read_to_string(package_dir.join(entry))?;
            ctx.eval::<(), _>(source)?;

            // Call activate(pi, roko?).
            let activate: Function = ctx.globals().get("activate")?;
            let pi_ref: Value = ctx.globals().get("pi")?;
            let roko_ref: Value = ctx.globals().get("roko").unwrap_or(Value::new_undefined(&ctx));
            activate.call::<_, ()>((pi_ref, roko_ref))?;

            Ok::<(), ExtensionLoadError>(())
        })?;

        Ok(Self {
            name: manifest.name.clone(),
            runtime,
            context,
            tools,
            hooks,
            capabilities: capabilities.to_vec(),
        })
    }
}
```

### Hook mapping

The bridge translates between Pi's event model and Roko's hook model:

| Pi event | Roko Extension hook | Notes |
|----------|-------------------|-------|
| `turn_start` | `on_tick_start` | Pi "turns" map to Roko "ticks" |
| `turn_end` | `on_tick_end` | |
| `tool_call` | `before_tool_call` | |
| `tool_result` | `after_tool_call` | |
| `error` | `on_error` | |
| (no Pi equivalent) | `on_observe` | Roko-only, available via `roko.on("observe", ...)` |
| (no Pi equivalent) | `on_assemble_context` | Roko-only, available via `roko.on("assemble_context", ...)` |
| (no Pi equivalent) | `before_inference` | Roko-only |
| (no Pi equivalent) | `after_inference` | Roko-only |
| (no Pi equivalent) | `on_outcome` | Roko-only |
| (no Pi equivalent) | `on_regime_change` | Roko-only |

When the runtime fires `on_tick_start`, the bridge:
1. Serializes the `TickInfo` to JSON.
2. Calls `hooks["tick_start"]` in the QuickJS context.
3. Deserializes the result back to Rust types.
4. Applies any context injections or tool registrations.

The serialization round-trip adds ~100 microseconds per hook invocation. At tick rates of 5-120 seconds, this overhead is unmeasurable.

### Tool registration bridge

When a JavaScript extension calls `pi.registerTool()`, the bridge:

```rust
/// Handle pi.registerTool() call from JavaScript.
/// Translates the JS tool definition into Roko's DynamicToolRegistry format.
fn handle_register_tool(
    ctx: &rquickjs::Ctx<'_>,
    tools: &mut Vec<JsToolDefinition>,
    args: rquickjs::function::Args<'_>,
) -> rquickjs::Result<()> {
    let tool_def: Object = args.get(0)?;

    let name: String = tool_def.get("name")?;
    let description: String = tool_def.get("description")?;
    let parameters: Value = tool_def.get("parameters")?;
    let handler: Function = tool_def.get("handler")?;

    // Convert JSON Schema from JS object to serde_json::Value.
    let schema: serde_json::Value = ctx.json_stringify_replacer_space(
        parameters, Value::new_undefined(ctx), Value::new_undefined(ctx),
    )?.and_then(|s| serde_json::from_str(&s.to_string()?).ok())
     .unwrap_or_default();

    tools.push(JsToolDefinition {
        name: name.clone(),
        description,
        parameters: schema,
        handler_ref: handler.into_persistent(),
    });

    // Register in the global DynamicToolRegistry so the agent can call it.
    // The handler will be invoked through the QuickJS bridge when the agent
    // uses the tool.
    tracing::info!(tool = %name, "registered JS tool from Pi extension");

    Ok(())
}
```

When the agent invokes a JS-registered tool, the runtime:
1. Serializes tool call parameters to JSON.
2. Enters the extension's QuickJS context.
3. Calls the stored handler function with the parameters.
4. Awaits the result (QuickJS supports async via microtask queue).
5. Deserializes the result back to Rust.
6. Returns the result through the normal tool pipeline.

### Security sandbox

JavaScript extensions run in a restricted environment:

- **No filesystem access** unless the extension declares `capability: "fs:read"` or `"fs:write"` and the agent's security policy allows it.
- **No network access** unless declared and allowed. Network calls go through a Rust-side proxy that enforces URL allowlists.
- **No process spawning.** The QuickJS environment has no `child_process` or `Deno.run` equivalent.
- **No shared state.** Each extension isolate is independent. Extensions communicate only through the tool registry and context injection mechanisms.
- **Memory-limited.** 64MB per isolate. Extensions that exceed this are killed and their tools deregistered.
- **Time-limited.** Each hook invocation has a 5-second timeout. Extensions that exceed this are warned on first offense and killed on second.

---

## 8. Multi-domain agents

### The problem with single-domain assumption

PRD-06 defined one `DomainProfile` per agent. This works for pure cases -- a coding agent uses the coding profile, a blockchain agent uses the blockchain profile. It breaks for hybrid agents.

Consider a "DeFi research agent." This agent needs:
- Blockchain capabilities: chain subscriptions, position monitoring, contract discovery, event decoding.
- Research capabilities: document retrieval, citation verification, synthesis, multi-source analysis.

Under the single-profile model, you pick one:
- Blockchain profile: the agent gets 5-second gamma ticks and chain event subscriptions but no research tooling. It cannot retrieve or synthesize documents.
- Research profile: the agent gets 60-second gamma ticks and research tools but no chain subscriptions. It cannot monitor positions or decode events.

Neither profile alone produces a competent DeFi researcher.

### Profile composition

The solution is composing multiple profiles into a single configuration:

```rust
/// A composed profile that merges multiple DomainProfiles into
/// a single agent configuration.
///
/// The merge strategy determines how conflicts between profiles are resolved.
/// This is how multi-domain agents (e.g., blockchain + research) get
/// capabilities from both domains.
#[derive(Debug, Clone)]
pub struct ComposedProfile {
    /// Human-readable name for the composition.
    pub name: String,

    /// The profiles being composed, in priority order.
    /// The first profile is the primary. Conflict resolution
    /// favors the primary in PrimarySecondary mode.
    pub profiles: Vec<FullDomainProfile>,

    /// How to resolve conflicts between profiles.
    pub merge_strategy: MergeStrategy,
}

/// Strategy for resolving conflicts when composing domain profiles.
#[derive(Debug, Clone)]
pub enum MergeStrategy {
    /// Use the most aggressive value for each parameter.
    /// Fastest gamma (minimum), all extensions (union), all events (union),
    /// all gates (union), all infrastructure (union).
    Union,

    /// Primary profile controls timing and gates.
    /// Secondary profiles add extensions, events, and context categories.
    PrimarySecondary {
        /// Index into the `profiles` vec. Usually 0.
        primary: usize,
    },

    /// Custom merge function for cases that need specific logic.
    /// Example: a security agent that uses coding timing but chain events.
    Custom(Arc<dyn Fn(&[FullDomainProfile]) -> FullDomainProfile + Send + Sync>),
}
```

### Merge rules

When composing profiles, each axis has a defined merge behavior:

| Axis | Union behavior | PrimarySecondary behavior |
|------|----------------|--------------------------|
| **Tick frequency** | Use the minimum (fastest) interval per regime | Primary controls all intervals |
| **Extensions** | Union of all `required` and `optional` sets, deduplicated | Primary's required + union of all optional |
| **Event subscriptions** | Union of all filters, lowest severity threshold wins for duplicates | Primary's filters + secondary filters added with severity floor 0.3 |
| **Context weights** | Average weights across profiles, then normalize to sum 1.0 | Primary's weights; new categories from secondary get weight 0.05 |
| **Gates** | Union of all gates; if same name appears twice, use the stricter config | Primary's gates |
| **Infrastructure** | Union of all requirements | Union of all requirements |

```rust
impl ComposedProfile {
    /// Flatten the composed profile into a single FullDomainProfile.
    ///
    /// This is called once during agent provisioning. The resulting profile
    /// is indistinguishable from a standard FullDomainProfile to the runtime.
    pub fn flatten(&self) -> FullDomainProfile {
        match &self.merge_strategy {
            MergeStrategy::Union => self.flatten_union(),
            MergeStrategy::PrimarySecondary { primary } => {
                self.flatten_primary_secondary(*primary)
            }
            MergeStrategy::Custom(f) => f(&self.profiles),
        }
    }

    fn flatten_union(&self) -> FullDomainProfile {
        let mut result = self.profiles[0].clone();
        result.label = self.name.clone();

        for profile in &self.profiles[1..] {
            // Timing: use the fastest (minimum) interval per regime.
            result.clock.gamma.calm =
                result.clock.gamma.calm.min(profile.clock.gamma.calm);
            result.clock.gamma.normal =
                result.clock.gamma.normal.min(profile.clock.gamma.normal);
            result.clock.gamma.volatile =
                result.clock.gamma.volatile.min(profile.clock.gamma.volatile);
            result.clock.gamma.crisis =
                result.clock.gamma.crisis.min(profile.clock.gamma.crisis);

            result.clock.theta.calm =
                result.clock.theta.calm.min(profile.clock.theta.calm);
            result.clock.theta.normal =
                result.clock.theta.normal.min(profile.clock.theta.normal);
            result.clock.theta.volatile =
                result.clock.theta.volatile.min(profile.clock.theta.volatile);
            result.clock.theta.crisis =
                result.clock.theta.crisis.min(profile.clock.theta.crisis);

            // Extensions: union, deduplicated.
            for ext in &profile.extensions.required {
                if !result.extensions.required.contains(ext) {
                    result.extensions.required.push(ext.clone());
                }
            }
            for ext in &profile.extensions.optional {
                if !result.extensions.optional.contains(ext)
                    && !result.extensions.required.contains(ext)
                {
                    result.extensions.optional.push(ext.clone());
                }
            }

            // Wakeup events: union, lowest severity wins.
            for filter in &profile.wakeup_events {
                if let Some(existing) = result.wakeup_events.iter_mut()
                    .find(|f| f.event_type == filter.event_type)
                {
                    existing.severity_threshold = match (
                        existing.severity_threshold,
                        filter.severity_threshold,
                    ) {
                        (Some(a), Some(b)) => Some(a.min(b)),
                        (None, _) | (_, None) => None,
                    };
                } else {
                    result.wakeup_events.push(filter.clone());
                }
            }

            // Gates: union, stricter config wins.
            for gate in &profile.gates {
                if let Some(existing) = result.gates.iter_mut()
                    .find(|g| g.name == gate.name)
                {
                    existing.required = existing.required || gate.required;
                    existing.timeout_secs =
                        existing.timeout_secs.min(gate.timeout_secs);
                } else {
                    result.gates.push(gate.clone());
                }
            }

            // Infrastructure: union.
            for ep in &profile.infrastructure.rpc_endpoints {
                if !result.infrastructure.rpc_endpoints.contains(ep) {
                    result.infrastructure.rpc_endpoints.push(ep.clone());
                }
            }
            for ws in &profile.infrastructure.websocket_subscriptions {
                if !result.infrastructure.websocket_subscriptions.contains(ws) {
                    result.infrastructure.websocket_subscriptions.push(ws.clone());
                }
            }
            for api in &profile.infrastructure.http_apis {
                if !result.infrastructure.http_apis.contains(api) {
                    result.infrastructure.http_apis.push(api.clone());
                }
            }
            result.infrastructure.git_worktree =
                result.infrastructure.git_worktree
                || profile.infrastructure.git_worktree;
            result.infrastructure.file_watcher =
                result.infrastructure.file_watcher
                || profile.infrastructure.file_watcher;
        }

        // Context weights: average then normalize.
        let mut weight_map: HashMap<ContextCategory, Vec<f32>> = HashMap::new();
        for profile in &self.profiles {
            for (cat, w) in &profile.context_weights {
                weight_map.entry(*cat).or_default().push(*w);
            }
        }
        let total_profiles = self.profiles.len() as f32;
        let mut weights: Vec<(ContextCategory, f32)> = weight_map
            .into_iter()
            .map(|(cat, ws)| (cat, ws.iter().sum::<f32>() / total_profiles))
            .collect();
        let sum: f32 = weights.iter().map(|(_, w)| w).sum();
        if sum > 0.0 {
            for (_, w) in &mut weights {
                *w /= sum;
            }
        }
        result.context_weights = weights;

        result
    }

    fn flatten_primary_secondary(&self, primary_idx: usize) -> FullDomainProfile {
        let mut result = self.profiles[primary_idx].clone();
        result.label = self.name.clone();

        for (i, profile) in self.profiles.iter().enumerate() {
            if i == primary_idx { continue; }

            // Extensions: add secondary's as optional (they enhance, don't override).
            for ext in &profile.extensions.required {
                if !result.extensions.required.contains(ext)
                    && !result.extensions.optional.contains(ext)
                {
                    result.extensions.optional.push(ext.clone());
                }
            }

            // Wakeup events: add secondary's with a severity floor.
            for filter in &profile.wakeup_events {
                if !result.wakeup_events.iter()
                    .any(|f| f.event_type == filter.event_type)
                {
                    result.wakeup_events.push(WakeupEventFilter {
                        event_type: filter.event_type.clone(),
                        severity_threshold: Some(
                            filter.severity_threshold.unwrap_or(0.0).max(0.3),
                        ),
                    });
                }
            }

            // Context weights: add missing categories at minimum weight.
            for (cat, _) in &profile.context_weights {
                if !result.context_weights.iter().any(|(c, _)| c == cat) {
                    result.context_weights.push((*cat, 0.05));
                }
            }

            // Infrastructure: always union.
            for ep in &profile.infrastructure.rpc_endpoints {
                if !result.infrastructure.rpc_endpoints.contains(ep) {
                    result.infrastructure.rpc_endpoints.push(ep.clone());
                }
            }
            for ws in &profile.infrastructure.websocket_subscriptions {
                if !result.infrastructure.websocket_subscriptions.contains(ws) {
                    result.infrastructure.websocket_subscriptions.push(ws.clone());
                }
            }
        }

        result
    }
}
```

### CLI interface

```bash
# Start a multi-domain agent with composed profiles.
# Comma-separated profiles. First is primary (for PrimarySecondary strategy).
roko agent start --profile blockchain,research --name defi-researcher

# Explicit merge strategy.
roko agent start --profile blockchain,research --merge union --name defi-researcher
roko agent start --profile blockchain,research --merge primary-secondary --name defi-researcher

# Profile composition is also valid in roko.toml:
# [agent.defi-researcher]
# profiles = ["blockchain", "research"]
# merge_strategy = "union"
```

---

## 9. Multi-chain blockchain architecture

### The core problem

Different blockchains produce blocks at radically different rates:

| Chain | Block time | Blocks/minute | Finality | Reorg depth |
|-------|-----------|---------------|----------|-------------|
| Korai | 50ms | 1,200 | Deterministic (1 block) | 0 |
| Arbitrum | ~250ms | ~240 | Probabilistic (confirmations on L1) | ~10 |
| Hyperliquid | ~1s | ~60 | Proprietary consensus | ~5 |
| Base | 2s | 30 | Probabilistic (L1 confirmations) | ~20 |
| Ethereum | 12s | 5 | Probabilistic (2 epochs / ~13 min) | ~64 |
| Solana | ~400ms | ~150 | Probabilistic (32 confirmations) | ~32 |

An agent that monitors yield across Aave on Ethereum, Morpho on Base, and a native lending protocol on Korai must ingest events from all three chains simultaneously, at their native speeds, and reason about cross-chain state using a unified model.

The naive approach -- one event loop processing all chains sequentially -- fails because:
1. Ethereum's 12-second blocks would stall Korai event processing (50ms blocks pile up).
2. A slow RPC response from one chain blocks event processing for all chains.
3. Reorg handling for one chain requires rewinding state that other chains have already processed past.

Each chain needs its own independent processing pipeline that feeds into a shared event bus.

### Actor-per-chain architecture

Each chain runs as an independent async actor. An actor is a long-lived async task with its own state, its own mailbox (channel), and its own error handling. Actors do not share mutable state. They communicate through message passing.

```rust
use tokio::sync::mpsc;
use futures::stream::{Stream, StreamExt};

/// One actor per chain. Runs as an independent tokio task.
///
/// The actor:
/// 1. Connects to the chain via its ChainConnector.
/// 2. Subscribes to new blocks (and optionally pending txs).
/// 3. Decodes and normalizes events into CanonicalEvent format.
/// 4. Manages a reorg buffer for the chain's expected reorg depth.
/// 5. Sends finalized canonical events to the shared CanonicalEventBus.
/// 6. Reports health metrics to the supervisor.
pub struct ChainActor {
    /// Unique identifier for this chain.
    chain_id: ChainId,

    /// Chain-specific configuration (RPC URL, WebSocket URL, etc.).
    chain_config: ChainConfig,

    /// The connector that abstracts chain-specific RPC calls.
    /// EVM chains use EvmConnector (via Alloy).
    /// Non-EVM chains implement ChainConnector directly.
    connector: Box<dyn ChainConnector>,

    /// How finality works on this chain.
    finality_mode: FinalityMode,

    /// Native block time. Used for health monitoring
    /// (if no block arrives within 3x block_time, report stale).
    block_time: Duration,

    /// Ring buffer of recent blocks for reorg detection.
    /// Depth = expected_reorg_depth for this chain.
    reorg_buffer: VecDeque<BlockEnvelope>,

    /// Expected reorg depth. Derived from chain characteristics.
    reorg_depth: usize,

    /// Channel to send canonical events to the shared bus.
    canonical_tx: mpsc::Sender<CanonicalEvent>,

    /// Current attention budget from the foraging model.
    /// High budget = poll every block, full decode.
    /// Low budget = poll every Nth block, selective decode.
    attention_budget: f64,

    /// Registry of known contracts on this chain.
    /// Populated by the dynamic contract discovery pipeline.
    contract_registry: ContractRegistry,

    /// Last block number successfully processed.
    last_block: u64,

    /// Last block timestamp (for health monitoring).
    last_block_time: Instant,

    /// Cumulative metrics for observability.
    metrics: ChainActorMetrics,
}

/// Chain-agnostic block wrapper that the actor uses internally.
struct BlockEnvelope {
    number: u64,
    hash: [u8; 32],
    parent_hash: [u8; 32],
    timestamp: DateTime<Utc>,
    transactions: Vec<CanonicalTransaction>,
    logs: Vec<DecodedLog>,
}

/// Metrics tracked per chain actor for observability and foraging decisions.
#[derive(Debug, Default)]
struct ChainActorMetrics {
    blocks_processed: u64,
    events_emitted: u64,
    reorgs_detected: u64,
    rpc_errors: u64,
    decode_errors: u64,
    avg_block_processing_time: Duration,
    events_per_block: f64,
}
```

### The canonical event schema

Every chain actor normalizes its native events into a single schema. This is the interface between chain-specific code and the rest of the agent.

```rust
/// A chain-agnostic event produced by a ChainActor and consumed by
/// the heartbeat pipeline.
///
/// CanonicalEvents are the universal currency of multi-chain agents.
/// The heartbeat pipeline, foraging model, worldgraph, and context
/// assembly all operate on CanonicalEvents without knowing which
/// chain produced them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalEvent {
    /// Deterministic ID derived from (chain, block_number, tx_index, log_index).
    /// Two events with the same id are guaranteed to represent the same
    /// on-chain occurrence. Used for deduplication after reorgs.
    pub id: DeterministicId,

    /// Which chain produced this event.
    pub chain: ChainId,

    /// Block number on the source chain.
    pub block_number: u64,

    /// Block timestamp (from the chain's block header).
    pub block_time: DateTime<Utc>,

    /// When the actor received and processed this event.
    /// Used for latency monitoring: received_at - block_time = ingestion lag.
    pub received_at: Instant,

    /// Current finality state. Events progress through:
    /// Pending -> Observed -> Confirmed -> (possibly Reorged)
    pub finality: FinalityState,

    /// The event payload, normalized to chain-agnostic types.
    pub payload: EventPayload,

    /// Optional classification from the contract discovery pipeline.
    /// None for newly discovered contracts. Populated as classification
    /// confidence grows.
    pub classification: Option<EventClass>,

    /// Relevance score from the foraging model (0.0 to 1.0).
    /// Events below the agent's attention threshold are dropped
    /// before reaching the heartbeat pipeline.
    pub relevance: f64,
}

/// Deterministic event ID. Computed from chain-specific identifiers
/// so that the same on-chain event always produces the same ID,
/// even if processed by different agents on different machines.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct DeterministicId {
    pub chain: ChainId,
    pub block_number: u64,
    pub tx_index: u32,
    pub log_index: u32,
}

impl DeterministicId {
    pub fn new(chain: ChainId, block: u64, tx: u32, log: u32) -> Self {
        Self { chain, block_number: block, tx_index: tx, log_index: log }
    }

    /// Compute a 32-byte hash for compact storage and comparison.
    pub fn hash(&self) -> [u8; 32] {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(self.chain.as_bytes());
        hasher.update(self.block_number.to_le_bytes());
        hasher.update(self.tx_index.to_le_bytes());
        hasher.update(self.log_index.to_le_bytes());
        hasher.finalize().into()
    }
}

/// Finality states for a canonical event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinalityState {
    /// Transaction is in the mempool, not yet included in a block.
    /// Only available for chains where ChainConnector::subscribe_pending_txs
    /// returns Some.
    Pending,

    /// Included in a block but not yet confirmed.
    /// The block may still be reorged.
    Observed,

    /// Confirmed with sufficient confidence for the chain's finality model.
    /// - Deterministic chains (Korai): confirmed = observed (1 block).
    /// - Probabilistic chains: confirmed after N blocks/epochs.
    Confirmed,

    /// The block containing this event was reorged away.
    /// Downstream processors treat this as an undo operation:
    /// reverse any state changes caused by this event.
    Reorged,
}

/// Normalized event payload types.
///
/// These cover the event types that agent strategies commonly need.
/// The enum is non_exhaustive -- chain connectors can extend it
/// with custom variants via EventPayload::Custom.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EventPayload {
    /// A new block summary (header + aggregate stats).
    Block(BlockSummary),

    /// A single transaction, normalized to chain-agnostic fields.
    Transaction(CanonicalTransaction),

    /// A decoded log event from a smart contract.
    Log(DecodedLog),

    /// A state change detected by comparing storage slots
    /// between consecutive blocks.
    StateChange(StateChange),

    /// A price feed update (from DEX trades, oracle updates, etc.).
    PriceFeed(PriceFeedUpdate),

    /// An ISFR update (from Korai's ISFR precompile).
    ISFRUpdate(ISFRValue),

    /// A liquidity event (add/remove liquidity in a pool).
    Liquidity(LiquidityEvent),

    /// A lending event (supply, borrow, repay, liquidation).
    Lending(LendingEvent),

    /// A bridge event (lock, mint, burn, release).
    Bridge(BridgeEvent),

    /// Custom payload for chain-specific events that do not fit
    /// the standard categories. Serialized as JSON.
    Custom {
        event_type: String,
        data: serde_json::Value,
    },
}

/// Normalized block summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockSummary {
    pub number: u64,
    pub hash: String,
    pub parent_hash: String,
    pub timestamp: DateTime<Utc>,
    pub transaction_count: u32,
    pub gas_used: u64,
    pub gas_limit: u64,
    pub base_fee: Option<u128>,
}

/// Normalized transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalTransaction {
    pub hash: String,
    pub from: String,
    pub to: Option<String>,
    pub value: String,       // Decimal string to avoid precision loss
    pub input: Vec<u8>,      // Calldata
    pub gas_used: u64,
    pub status: TxStatus,
}

/// Decoded log event with contract classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedLog {
    pub address: String,
    pub topics: Vec<String>,
    pub data: Vec<u8>,
    /// If the contract is classified, the decoded event name and parameters.
    pub decoded: Option<DecodedEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedEvent {
    pub event_name: String,
    pub params: Vec<(String, serde_json::Value)>,
}
```

### The canonical event bus

All chain actors feed into a single bus that the heartbeat pipeline consumes:

```rust
/// The canonical event bus aggregates events from all chain actors
/// and feeds them to the heartbeat pipeline.
///
/// The bus provides:
/// 1. Ordering: events are ordered by (block_time, chain, block_number)
///    across chains. This gives a globally consistent view.
/// 2. Deduplication: if a reorg causes an event to be re-emitted with
///    the same DeterministicId, the bus drops the duplicate.
/// 3. Windowing: events are grouped into temporal windows for hierarchical
///    processing (see section 10).
/// 4. Backpressure: if the heartbeat pipeline falls behind, the bus
///    buffers events up to a configurable limit before dropping lowest-
///    relevance events.
pub struct CanonicalEventBus {
    /// Receiver for events from all chain actors.
    rx: mpsc::Receiver<CanonicalEvent>,

    /// Deduplication set (LRU cache of recent DeterministicIds).
    seen: LruCache<DeterministicId, ()>,

    /// Current temporal window being assembled.
    current_window: TemporalWindow,

    /// Window duration for the current resolution level.
    window_duration: Duration,

    /// Backpressure buffer.
    buffer: BinaryHeap<Reverse<(f64, CanonicalEvent)>>,

    /// Maximum buffer size before low-relevance events are dropped.
    max_buffer_size: usize,
}

impl CanonicalEventBus {
    /// Create a new bus that receives from the given channel.
    pub fn new(
        rx: mpsc::Receiver<CanonicalEvent>,
        window_duration: Duration,
        max_buffer_size: usize,
    ) -> Self {
        Self {
            rx,
            seen: LruCache::new(NonZeroUsize::new(100_000).unwrap()),
            current_window: TemporalWindow::new(Utc::now(), window_duration),
            window_duration,
            buffer: BinaryHeap::new(),
            max_buffer_size,
        }
    }

    /// Poll for the next completed temporal window.
    ///
    /// Returns None if no window is ready. The heartbeat pipeline calls
    /// this on every gamma tick.
    pub async fn next_window(&mut self) -> Option<TemporalWindow> {
        // Drain all available events from the channel.
        while let Ok(event) = self.rx.try_recv() {
            // Deduplicate.
            if self.seen.contains(&event.id) {
                continue;
            }
            self.seen.put(event.id.clone(), ());

            // Check if this event belongs to the current window.
            if event.block_time >= self.current_window.end {
                // Current window is complete. Start a new one.
                let completed = std::mem::replace(
                    &mut self.current_window,
                    TemporalWindow::new(
                        self.current_window.end,
                        self.window_duration,
                    ),
                );

                // Buffer the event for the next window.
                self.current_window.add(event);

                return Some(completed);
            }

            self.current_window.add(event);
        }

        None
    }
}

/// A temporal window of canonical events.
///
/// Groups events within a time range for batch processing.
/// The hierarchical temporal resolution (section 10) creates
/// windows at different granularities.
#[derive(Debug)]
pub struct TemporalWindow {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub events: Vec<CanonicalEvent>,
    pub chains_represented: HashSet<ChainId>,
    pub event_count_by_chain: HashMap<ChainId, usize>,
}

impl TemporalWindow {
    pub fn new(start: DateTime<Utc>, duration: Duration) -> Self {
        Self {
            start,
            end: start + chrono::Duration::from_std(duration).unwrap(),
            events: Vec::new(),
            chains_represented: HashSet::new(),
            event_count_by_chain: HashMap::new(),
        }
    }

    pub fn add(&mut self, event: CanonicalEvent) {
        *self.event_count_by_chain
            .entry(event.chain.clone())
            .or_insert(0) += 1;
        self.chains_represented.insert(event.chain.clone());
        self.events.push(event);
    }
}
```

### Supervisor and lifecycle

The `ProcessSupervisor` (PRD-02, section 10) manages chain actors as child tasks:

```rust
/// Spawn chain actors for all configured chains.
///
/// Called during agent provisioning when the domain profile includes
/// blockchain capabilities.
pub async fn spawn_chain_actors(
    supervisor: &ProcessSupervisor,
    chains: &[ChainConfig],
    canonical_tx: mpsc::Sender<CanonicalEvent>,
) -> Result<Vec<ChainActorHandle>, ChainError> {
    let mut handles = Vec::with_capacity(chains.len());

    for config in chains {
        let connector = create_connector(config)?;
        let actor = ChainActor::new(
            config.chain_id.clone(),
            config.clone(),
            connector,
            canonical_tx.clone(),
        );

        let handle = supervisor.spawn_task(
            format!("chain-actor-{}", config.chain_id),
            actor.run(),
        );

        handles.push(ChainActorHandle {
            chain_id: config.chain_id.clone(),
            task_handle: handle,
        });
    }

    Ok(handles)
}

/// Create the appropriate connector for a chain configuration.
fn create_connector(config: &ChainConfig) -> Result<Box<dyn ChainConnector>, ChainError> {
    match config.chain_type {
        ChainType::Evm => {
            let connector = EvmConnector::new(
                config.rpc_url.clone(),
                config.ws_url.clone(),
                config.chain_id.clone(),
            );
            Ok(Box::new(connector))
        }
        ChainType::Custom { ref connector_crate } => {
            // Load from installed chain connector package.
            let connector = load_chain_connector(connector_crate)?;
            Ok(connector)
        }
    }
}
```

### Chain actor main loop

```rust
impl ChainActor {
    /// The actor's main loop. Runs as a tokio task until cancelled.
    pub async fn run(mut self) -> Result<(), ChainError> {
        // Connect to the chain.
        self.connector.connect(&self.chain_config).await?;

        // Subscribe to new blocks.
        let mut block_stream = self.connector.subscribe_blocks().await;

        // Optionally subscribe to pending transactions.
        let mut pending_stream = self.connector.subscribe_pending_txs().await;

        let stale_timeout = self.block_time * 3;

        loop {
            tokio::select! {
                // New block received.
                Some(raw_block) = block_stream.next() => {
                    self.handle_block(raw_block).await?;
                }

                // Pending transaction (if available).
                Some(raw_tx) = async {
                    if let Some(ref mut stream) = pending_stream {
                        stream.next().await
                    } else {
                        // No pending tx stream. Park this branch forever.
                        std::future::pending::<Option<RawTx>>().await
                    }
                } => {
                    self.handle_pending_tx(raw_tx).await?;
                }

                // Stale check: if no block arrives within 3x block_time,
                // report unhealthy.
                _ = tokio::time::sleep(stale_timeout) => {
                    tracing::warn!(
                        chain = %self.chain_id,
                        last_block = self.last_block,
                        elapsed = ?self.last_block_time.elapsed(),
                        "chain actor stale: no new blocks",
                    );
                    self.metrics.rpc_errors += 1;

                    // Attempt to reconnect.
                    if let Err(e) = self.connector.connect(&self.chain_config).await {
                        tracing::error!(chain = %self.chain_id, error = %e, "reconnect failed");
                    } else {
                        block_stream = self.connector.subscribe_blocks().await;
                    }
                }
            }
        }
    }

    async fn handle_block(&mut self, raw: RawBlock) -> Result<(), ChainError> {
        let start = Instant::now();

        // Normalize the block to chain-agnostic format.
        let block = self.connector.normalize_block(raw);

        // Reorg detection: check if parent_hash matches our last block.
        if let Some(last) = self.reorg_buffer.back() {
            if block.parent_hash != last.hash && block.number == last.number + 1 {
                // Reorg detected.
                self.handle_reorg(&block).await?;
            }
        }

        // Add to reorg buffer.
        self.reorg_buffer.push_back(block.clone());
        if self.reorg_buffer.len() > self.reorg_depth {
            // Oldest block is now past the reorg window.
            // Emit Confirmed events for it.
            if let Some(confirmed) = self.reorg_buffer.pop_front() {
                self.emit_confirmed_events(&confirmed).await?;
            }
        }

        // Process and emit Observed events for the new block.
        self.emit_observed_events(&block).await?;

        // Update state.
        self.last_block = block.number;
        self.last_block_time = Instant::now();
        self.metrics.blocks_processed += 1;
        self.metrics.avg_block_processing_time =
            (self.metrics.avg_block_processing_time * 7 + start.elapsed()) / 8;

        Ok(())
    }

    async fn handle_reorg(&mut self, new_block: &BlockEnvelope) -> Result<(), ChainError> {
        tracing::warn!(
            chain = %self.chain_id,
            new_block = new_block.number,
            "reorg detected",
        );
        self.metrics.reorgs_detected += 1;

        // Walk backwards through the buffer, emitting Reorged events
        // for blocks that are no longer canonical.
        while let Some(orphaned) = self.reorg_buffer.pop_back() {
            if orphaned.hash == new_block.parent_hash {
                // Found the fork point. This block is still canonical.
                self.reorg_buffer.push_back(orphaned);
                break;
            }

            // Emit Reorged events for all events in this orphaned block.
            for event in self.block_to_events(&orphaned, FinalityState::Reorged) {
                let _ = self.canonical_tx.send(event).await;
            }
        }

        Ok(())
    }

    async fn emit_observed_events(&self, block: &BlockEnvelope) -> Result<(), ChainError> {
        // Block summary event.
        let block_event = CanonicalEvent {
            id: DeterministicId::new(self.chain_id.clone(), block.number, 0, 0),
            chain: self.chain_id.clone(),
            block_number: block.number,
            block_time: block.timestamp,
            received_at: Instant::now(),
            finality: FinalityState::Observed,
            payload: EventPayload::Block(BlockSummary {
                number: block.number,
                hash: hex::encode(block.hash),
                parent_hash: hex::encode(block.parent_hash),
                timestamp: block.timestamp,
                transaction_count: block.transactions.len() as u32,
                gas_used: block.transactions.iter().map(|tx| tx.gas_used).sum(),
                gas_limit: 0, // Filled by connector
                base_fee: None, // Filled by connector if EIP-1559
            }),
            classification: None,
            relevance: self.compute_block_relevance(block),
        };

        let _ = self.canonical_tx.send(block_event).await;

        // Individual log events, filtered by attention budget.
        for (tx_idx, tx) in block.transactions.iter().enumerate() {
            for (log_idx, log) in block.logs.iter()
                .filter(|l| l.address == tx.to.as_deref().unwrap_or_default())
                .enumerate()
            {
                let relevance = self.compute_log_relevance(log);

                // Skip low-relevance events when attention budget is constrained.
                if relevance < self.attention_threshold() {
                    continue;
                }

                let event = CanonicalEvent {
                    id: DeterministicId::new(
                        self.chain_id.clone(),
                        block.number,
                        tx_idx as u32,
                        log_idx as u32,
                    ),
                    chain: self.chain_id.clone(),
                    block_number: block.number,
                    block_time: block.timestamp,
                    received_at: Instant::now(),
                    finality: FinalityState::Observed,
                    payload: EventPayload::Log(log.clone()),
                    classification: self.contract_registry
                        .classify(&log.address),
                    relevance,
                };

                let _ = self.canonical_tx.send(event).await;
                self.metrics.events_emitted += 1;
            }
        }

        Ok(())
    }

    /// Attention threshold derived from the foraging model's budget.
    /// Higher budget = lower threshold = more events pass through.
    fn attention_threshold(&self) -> f64 {
        // Budget of 1.0 = pass everything (threshold 0.0).
        // Budget of 0.1 = only pass top 10% relevance (threshold 0.9).
        (1.0 - self.attention_budget).max(0.0)
    }
}
```

---

## 10. Hierarchical temporal resolution

### The temporal mismatch problem

Korai produces 1,200 blocks per minute. Ethereum produces 5. If the agent processes every event at the same granularity, it drowns in Korai data while starving for Ethereum data. The inverse problem also holds: if the agent aggregates everything into 30-second windows, it misses sub-second MEV opportunities on Korai.

The solution is hierarchical temporal resolution. The agent processes events at multiple time scales simultaneously, matching the resolution to the decision type.

### Four resolution levels

```rust
/// Four resolution levels for multi-chain temporal processing.
///
/// Each level serves a different class of decisions. The agent runs
/// all four levels concurrently, with each level feeding the next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalResolution {
    /// Per-block resolution. Only for sub-second chains (Korai, Arbitrum).
    /// Decisions: MEV detection, arbitrage execution, frontrun protection.
    /// Processed by: ChainSubscriberExt fast-path (bypasses normal heartbeat).
    /// Latency budget: <10ms.
    R0,

    /// 1-second windows. Cross-chain snapshot.
    /// Aggregates: Korai ~20 blocks, Hyperliquid ~1, Base ~0.5, ETH ~0.08.
    /// Decisions: cross-chain arbitrage detection, bridge monitoring, price divergence.
    /// Processed by: gamma tick in the heartbeat pipeline.
    /// Latency budget: <100ms.
    R1,

    /// 30-second windows. Position assessment.
    /// All chains aggregated. TWAPs, volume, TVL changes.
    /// Decisions: position rebalancing, risk assessment, yield comparison.
    /// Processed by: theta tick in the heartbeat pipeline.
    /// Latency budget: <1s.
    R2,

    /// 5-minute windows. Strategic planning.
    /// Full cross-chain state summary. Trend analysis, regime detection.
    /// Decisions: strategy updates, allocation changes, domain switching.
    /// Processed by: theta/delta tick boundary.
    /// Latency budget: <5s (can involve T2 inference).
    R3,
}
```

### How resolution maps to heartbeat timescales

| Resolution | Window | Heartbeat timescale | Tier routing |
|-----------|--------|-------------------|-------------|
| R0 | Per-block | Fast-path (bypasses heartbeat) | Always T0 |
| R1 | 1 second | Gamma tick | T0 or T1 |
| R2 | 30 seconds | Theta tick | T0, T1, or T2 |
| R3 | 5 minutes | Theta/delta boundary | T1 or T2 |

R0 is special. It does not go through the normal heartbeat pipeline because the heartbeat's minimum gamma interval (2 seconds in crisis mode) is too slow for per-block processing on 50ms chains. Instead, the `ChainSubscriberExt` extension maintains a dedicated fast-path:

```rust
/// Fast-path processor for R0 (per-block) resolution.
///
/// Runs as a separate async task within the ChainSubscriberExt.
/// Only activated for chains with block_time < 1s.
/// All processing is T0 (deterministic, no LLM).
struct R0FastPath {
    /// Per-chain state machines for fast-path detection.
    detectors: HashMap<ChainId, Vec<Box<dyn R0Detector>>>,

    /// Channel to report R0 findings to the main heartbeat pipeline.
    findings_tx: mpsc::Sender<R0Finding>,
}

/// Trait for R0-resolution detectors.
///
/// Detectors run at per-block speed. They must be deterministic
/// and complete within 10ms. If a detector needs LLM reasoning,
/// it reports a Finding and the heartbeat pipeline handles it
/// at the appropriate tier.
pub trait R0Detector: Send + Sync {
    fn name(&self) -> &str;

    /// Process a single canonical event. Return findings if any.
    fn process(&mut self, event: &CanonicalEvent) -> Vec<R0Finding>;

    /// Reset state (called after reorg).
    fn reset(&mut self);
}

/// A finding from R0 processing that needs further analysis.
#[derive(Debug)]
pub struct R0Finding {
    pub detector: String,
    pub severity: f64,
    pub chain: ChainId,
    pub block: u64,
    pub description: String,
    pub data: serde_json::Value,
}
```

Built-in R0 detectors:
- **SandwichDetector**: detects sandwich attack patterns (large trade before and after a smaller trade in the same block).
- **ArbitrageDetector**: detects cyclic arbitrage (trade A->B->C->A where output > input).
- **LiquidationDetector**: detects liquidation calls on known lending protocols.
- **LargeTransferDetector**: detects transfers above a configurable threshold.

### Aggregation pipeline

R1 through R3 windows aggregate R0 events into progressively coarser summaries:

```rust
/// Aggregates R0 events into higher-resolution windows.
///
/// Each resolution level maintains running statistics that
/// the heartbeat pipeline queries at the appropriate tick speed.
pub struct TemporalAggregator {
    /// R1 (1-second) window state.
    r1: WindowState,
    /// R2 (30-second) window state.
    r2: WindowState,
    /// R3 (5-minute) window state.
    r3: WindowState,
}

#[derive(Debug)]
struct WindowState {
    duration: Duration,
    current_start: DateTime<Utc>,

    /// Per-chain event counts in the current window.
    event_counts: HashMap<ChainId, u64>,

    /// Per-token TWAP accumulators.
    twap_accumulators: HashMap<TokenPair, TwapAccumulator>,

    /// Aggregate volume per DEX pool.
    volume: HashMap<Address, f64>,

    /// TVL snapshots per protocol.
    tvl: HashMap<String, f64>,

    /// Lending rate snapshots.
    lending_rates: HashMap<(String, String), f64>, // (protocol, asset) -> rate

    /// R0 findings that occurred in this window.
    findings: Vec<R0Finding>,
}

/// TWAP accumulator using time-weighted observation.
#[derive(Debug)]
struct TwapAccumulator {
    cumulative_price_time: f64,
    cumulative_time: f64,
    last_price: f64,
    last_update: DateTime<Utc>,

    pub fn observe(&mut self, price: f64, time: DateTime<Utc>) {
        let dt = (time - self.last_update)
            .num_milliseconds() as f64 / 1000.0;
        self.cumulative_price_time += self.last_price * dt;
        self.cumulative_time += dt;
        self.last_price = price;
        self.last_update = time;
    }

    pub fn twap(&self) -> f64 {
        if self.cumulative_time == 0.0 {
            self.last_price
        } else {
            self.cumulative_price_time / self.cumulative_time
        }
    }
}
```

---

## 11. Reorg handling and finality

### The reorg model

A blockchain reorg occurs when the network replaces a previously accepted block with a different block at the same height. Reorgs happen because of network latency, competing miners/validators, or deliberate attacks. Each chain has a characteristic reorg depth -- the maximum number of blocks that might be replaced.

The agent must handle reorgs correctly or it will act on events that never happened.

### State machine per event

Each canonical event has a finality state that progresses through a state machine:

```
Pending ──────> Observed ──────> Confirmed
                    │
                    └──────> Reorged
```

- **Pending**: transaction seen in mempool, not yet in a block. Optional -- only available for chains where `subscribe_pending_txs` returns data.
- **Observed**: included in a block, but the block is within the reorg buffer. May be rolled back.
- **Confirmed**: the block is past the reorg depth. Safe to act on.
- **Reorged**: the block was replaced by a competing block. All state changes from this event should be reversed.

### Downstream reorg handling

The heartbeat pipeline tracks which events have been processed and at which finality:

```rust
/// Tracks events that have been processed at Observed finality
/// but not yet Confirmed. If a Reorged event arrives, the tracker
/// identifies which agent actions need to be unwound.
pub struct FinalityTracker {
    /// Events processed at Observed but not yet Confirmed.
    /// Keyed by DeterministicId for O(1) lookup.
    pending_confirmation: HashMap<DeterministicId, ProcessedEvent>,

    /// Events that were reorged after processing.
    /// These require undo actions.
    reorged: Vec<ReorgedEvent>,
}

#[derive(Debug)]
struct ProcessedEvent {
    event: CanonicalEvent,
    /// What the agent did in response to this event.
    actions_taken: Vec<AgentAction>,
    /// When the event was processed.
    processed_at: Instant,
}

#[derive(Debug)]
struct ReorgedEvent {
    original: ProcessedEvent,
    reorg_detected_at: Instant,
}

impl FinalityTracker {
    /// Called when an event arrives with FinalityState::Reorged.
    pub fn handle_reorg(&mut self, event_id: &DeterministicId) -> Option<Vec<AgentAction>> {
        if let Some(processed) = self.pending_confirmation.remove(event_id) {
            let actions = processed.actions_taken.clone();
            self.reorged.push(ReorgedEvent {
                original: processed,
                reorg_detected_at: Instant::now(),
            });
            Some(actions) // Return actions that need to be undone
        } else {
            None // Event was not processed or was already confirmed
        }
    }

    /// Called when an event arrives with FinalityState::Confirmed.
    pub fn confirm(&mut self, event_id: &DeterministicId) {
        self.pending_confirmation.remove(event_id);
    }
}
```

### Strategy-dependent finality requirements

Not all agent actions require confirmed finality. The domain profile specifies which action types can proceed at Observed finality and which must wait for Confirmed:

```toml
# In a domain profile's finality configuration.
[finality_requirements]
# Actions that can proceed immediately on Observed events.
observed_ok = [
    "update_worldgraph",
    "adjust_attention_budget",
    "log_observation",
]
# Actions that require Confirmed finality.
confirmed_required = [
    "submit_transaction",
    "update_position_state",
    "report_to_insight_store",
]
```

This lets the agent update its worldview (fast, reversible) immediately on Observed events while waiting for Confirmed before committing transactions or reporting to the InsightStore (slow, irreversible).

---

## 12. The ChainConnector trait

### The abstraction

Every blockchain integration in Roko implements the `ChainConnector` trait. The trait normalizes chain-specific operations into a common interface that the `ChainActor` consumes.

```rust
use async_trait::async_trait;
use futures::stream::BoxStream;

/// Trait for integrating a blockchain into Roko's multi-chain actor model.
///
/// Each chain type implements this trait. EVM chains share a single
/// implementation (EvmConnector via Alloy). Non-EVM chains implement
/// the trait directly.
///
/// # Implementing a new chain connector
///
/// 1. Implement this trait for your chain.
/// 2. Add [package.metadata.roko] to your Cargo.toml with type = "chain-connector".
/// 3. Publish with `roko publish`.
/// 4. Users install with `roko install crate:roko-chain-yourchain`.
///
/// # Contract
///
/// - `connect()` must establish a persistent connection (WebSocket preferred, HTTP polling fallback).
/// - `subscribe_blocks()` must return a stream that yields blocks as they are produced.
/// - Blocks must include all transactions and logs.
/// - `normalize_block()` must produce a deterministic `BlockEnvelope` for any given raw block.
/// - `decode_logs()` must attempt ABI decoding against known signatures.
#[async_trait]
pub trait ChainConnector: Send + Sync + 'static {
    /// Unique chain identifier (e.g., "ethereum", "base", "korai", "solana").
    fn chain_id(&self) -> ChainId;

    /// VM type. Determines which ABI decoding strategy to use.
    fn chain_type(&self) -> ChainType;

    /// Expected block time. Used for stale detection and attention budgeting.
    fn block_time(&self) -> Duration;

    /// How finality works on this chain.
    fn finality_mode(&self) -> FinalityMode;

    /// Expected maximum reorg depth in blocks.
    fn reorg_depth(&self) -> usize;

    /// Establish a connection to the chain.
    ///
    /// This is called once during chain actor initialization.
    /// It should establish WebSocket connections (preferred) or
    /// set up HTTP polling intervals.
    async fn connect(&mut self, config: &ChainConfig) -> Result<(), ChainError>;

    /// Subscribe to new blocks as they are produced.
    ///
    /// The returned stream must yield blocks in order. If the connection
    /// drops, the stream should attempt to reconnect and resume from
    /// the last known block.
    async fn subscribe_blocks(&self) -> BoxStream<'static, RawBlock>;

    /// Subscribe to pending transactions (mempool).
    ///
    /// Returns None if the chain does not support mempool visibility
    /// or if the RPC endpoint does not expose it.
    async fn subscribe_pending_txs(
        &self,
    ) -> Option<BoxStream<'static, RawTx>>;

    /// Fetch a specific block by number.
    ///
    /// Used for backfilling after reconnection or reorg recovery.
    async fn get_block(&self, number: u64) -> Result<RawBlock, ChainError>;

    /// Execute a read-only contract call (eth_call equivalent).
    ///
    /// Used by the contract discovery pipeline for ERC-165 interface detection
    /// and storage slot reads.
    async fn call(
        &self,
        call: ContractCall,
        block: Option<u64>,
    ) -> Result<Vec<u8>, ChainError>;

    /// Decode raw log entries using known ABI signatures.
    ///
    /// Logs that match a known signature are returned with decoded
    /// event names and parameters. Logs with unknown signatures are
    /// returned with decoded = None.
    fn decode_logs(&self, logs: &[RawLog]) -> Vec<DecodedLog>;

    /// Normalize a raw block into the chain-agnostic BlockEnvelope format.
    fn normalize_block(&self, raw: RawBlock) -> BlockEnvelope;

    /// Normalize a raw transaction into the chain-agnostic format.
    fn normalize_tx(&self, raw: RawTx) -> CanonicalTransaction;
}

/// VM types supported by the chain connector system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainType {
    /// Ethereum Virtual Machine (Ethereum, Base, Arbitrum, Korai, etc.).
    Evm,
    /// Solana Virtual Machine.
    Svm,
    /// Move VM (Sui, Aptos).
    MoveVm,
    /// Custom VM with chain-specific handling.
    Custom,
}

/// Finality model for a chain.
#[derive(Debug, Clone)]
pub enum FinalityMode {
    /// Finality after exactly N blocks (e.g., Korai: N=1, Tendermint: N=1).
    Deterministic { blocks: u64 },
    /// Finality after N confirmations with probabilistic guarantee.
    Probabilistic { confirmations: u64 },
    /// Finality depends on L1 inclusion (L2 rollups).
    L1Dependent {
        l1_chain: ChainId,
        l1_confirmations: u64,
    },
}
```

### Built-in connectors

**EvmConnector** handles all EVM-compatible chains through Alloy:

```rust
use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use alloy::rpc::types::{Block, Filter, Log};

/// EVM chain connector using Alloy.
///
/// A single implementation covers Ethereum, Base, Arbitrum, Korai,
/// Optimism, Polygon, and any other EVM chain. Chain-specific behavior
/// (block time, finality, reorg depth) comes from ChainConfig.
pub struct EvmConnector {
    chain_id: ChainId,
    rpc_url: String,
    ws_url: Option<String>,
    provider: Option<Box<dyn Provider>>,
    /// ABI signature database for log decoding.
    /// Loaded from 4byte.directory cache + local registry.
    sig_db: SignatureDatabase,
}

impl EvmConnector {
    pub fn new(
        rpc_url: String,
        ws_url: Option<String>,
        chain_id: ChainId,
    ) -> Self {
        Self {
            chain_id,
            rpc_url,
            ws_url,
            provider: None,
            sig_db: SignatureDatabase::load_default(),
        }
    }
}

#[async_trait]
impl ChainConnector for EvmConnector {
    fn chain_id(&self) -> ChainId { self.chain_id.clone() }
    fn chain_type(&self) -> ChainType { ChainType::Evm }

    fn block_time(&self) -> Duration {
        // Configured per-chain in ChainConfig, but we provide sensible defaults.
        match self.chain_id.as_str() {
            "korai" => Duration::from_millis(50),
            "arbitrum" => Duration::from_millis(250),
            "hyperliquid" => Duration::from_secs(1),
            "base" | "optimism" => Duration::from_secs(2),
            "ethereum" => Duration::from_secs(12),
            "polygon" => Duration::from_secs(2),
            _ => Duration::from_secs(12), // Conservative default
        }
    }

    fn finality_mode(&self) -> FinalityMode {
        match self.chain_id.as_str() {
            "korai" => FinalityMode::Deterministic { blocks: 1 },
            "ethereum" => FinalityMode::Probabilistic { confirmations: 64 },
            "base" => FinalityMode::L1Dependent {
                l1_chain: ChainId::from("ethereum"),
                l1_confirmations: 64,
            },
            "arbitrum" => FinalityMode::L1Dependent {
                l1_chain: ChainId::from("ethereum"),
                l1_confirmations: 64,
            },
            _ => FinalityMode::Probabilistic { confirmations: 32 },
        }
    }

    fn reorg_depth(&self) -> usize {
        match self.chain_id.as_str() {
            "korai" => 0,       // Deterministic finality
            "arbitrum" => 10,
            "base" => 20,
            "ethereum" => 64,
            _ => 32,
        }
    }

    async fn connect(&mut self, config: &ChainConfig) -> Result<(), ChainError> {
        let provider = if let Some(ref ws_url) = self.ws_url {
            // Prefer WebSocket for real-time block subscription.
            let ws = WsConnect::new(ws_url);
            ProviderBuilder::new().on_ws(ws).await
                .map_err(|e| ChainError::ConnectionFailed {
                    chain: self.chain_id.clone(),
                    source: e.into(),
                })?
        } else {
            // Fall back to HTTP polling.
            ProviderBuilder::new().on_http(self.rpc_url.parse()?)
        };

        self.provider = Some(Box::new(provider));
        Ok(())
    }

    async fn subscribe_blocks(&self) -> BoxStream<'static, RawBlock> {
        let provider = self.provider.as_ref().expect("must call connect() first");
        let sub = provider.subscribe_blocks().await
            .expect("block subscription failed");
        Box::pin(sub.into_stream().map(|block| RawBlock::Evm(block)))
    }

    async fn subscribe_pending_txs(
        &self,
    ) -> Option<BoxStream<'static, RawTx>> {
        let provider = self.provider.as_ref()?;
        let sub = provider.subscribe_pending_transactions().await.ok()?;
        Some(Box::pin(sub.into_stream().map(|tx| RawTx::Evm(tx))))
    }

    async fn get_block(&self, number: u64) -> Result<RawBlock, ChainError> {
        let provider = self.provider.as_ref()
            .ok_or(ChainError::NotConnected { chain: self.chain_id.clone() })?;
        let block = provider
            .get_block_by_number(number.into(), true)
            .await
            .map_err(|e| ChainError::RpcError {
                chain: self.chain_id.clone(),
                source: e.into(),
            })?
            .ok_or(ChainError::BlockNotFound {
                chain: self.chain_id.clone(),
                number,
            })?;
        Ok(RawBlock::Evm(block))
    }

    async fn call(
        &self,
        call: ContractCall,
        block: Option<u64>,
    ) -> Result<Vec<u8>, ChainError> {
        let provider = self.provider.as_ref()
            .ok_or(ChainError::NotConnected { chain: self.chain_id.clone() })?;

        let tx = alloy::rpc::types::TransactionRequest {
            to: Some(call.to.parse()?),
            input: Some(call.data.into()),
            ..Default::default()
        };

        let result = provider
            .call(&tx)
            .block(block.map(|n| n.into()).unwrap_or_default())
            .await
            .map_err(|e| ChainError::CallFailed {
                chain: self.chain_id.clone(),
                address: call.to,
                source: e.into(),
            })?;

        Ok(result.to_vec())
    }

    fn decode_logs(&self, logs: &[RawLog]) -> Vec<DecodedLog> {
        logs.iter().map(|raw| {
            let topics: Vec<String> = raw.topics().iter()
                .map(|t| hex::encode(t))
                .collect();

            // Try to decode using the signature database.
            let decoded = if let Some(first_topic) = topics.first() {
                self.sig_db.decode_event(first_topic, raw.data())
            } else {
                None
            };

            DecodedLog {
                address: format!("{:?}", raw.address()),
                topics,
                data: raw.data().to_vec(),
                decoded,
            }
        }).collect()
    }

    fn normalize_block(&self, raw: RawBlock) -> BlockEnvelope {
        match raw {
            RawBlock::Evm(block) => BlockEnvelope {
                number: block.header.number,
                hash: block.header.hash.0,
                parent_hash: block.header.parent_hash.0,
                timestamp: DateTime::from_timestamp(
                    block.header.timestamp as i64, 0,
                ).unwrap_or_default(),
                transactions: block.transactions.iter()
                    .map(|tx| self.normalize_tx(RawTx::Evm(tx.clone())))
                    .collect(),
                logs: self.decode_logs(&block.logs()),
            },
        }
    }

    fn normalize_tx(&self, raw: RawTx) -> CanonicalTransaction {
        match raw {
            RawTx::Evm(tx) => CanonicalTransaction {
                hash: format!("{:?}", tx.hash),
                from: format!("{:?}", tx.from),
                to: tx.to.map(|a| format!("{:?}", a)),
                value: tx.value.to_string(),
                input: tx.input.to_vec(),
                gas_used: tx.gas_used.unwrap_or(0),
                status: if tx.status == Some(1) {
                    TxStatus::Success
                } else {
                    TxStatus::Reverted
                },
            },
        }
    }
}
```

### Adding a new chain

Anyone can add a chain by implementing `ChainConnector`:

```rust
// In crate: roko-chain-hyperliquid

use roko_core::chain::{ChainConnector, ChainId, ChainType, FinalityMode, ChainConfig};

pub struct HyperliquidConnector {
    ws_url: String,
    client: Option<HyperliquidClient>,
}

#[async_trait]
impl ChainConnector for HyperliquidConnector {
    fn chain_id(&self) -> ChainId { ChainId::from("hyperliquid") }
    fn chain_type(&self) -> ChainType { ChainType::Custom }
    fn block_time(&self) -> Duration { Duration::from_secs(1) }
    fn finality_mode(&self) -> FinalityMode {
        FinalityMode::Deterministic { blocks: 1 }
    }
    fn reorg_depth(&self) -> usize { 0 }

    async fn connect(&mut self, config: &ChainConfig) -> Result<(), ChainError> {
        self.client = Some(HyperliquidClient::connect(&self.ws_url).await?);
        Ok(())
    }

    // ... remaining trait methods
}
```

Publish and install:

```bash
cd roko-chain-hyperliquid/
roko publish --type chain-connector

# Any user can now:
roko install crate:roko-chain-hyperliquid
```

---

## 13. Predictive foraging

### The foraging-bandit equivalence

Optimal foraging theory (Charnov, 1976) studies how animals allocate time across food patches. The Marginal Value Theorem states: leave a patch when its marginal return drops below the average return across all patches. Spend more time in rich patches. Spend less in depleted ones.

Frazier and Yu (2013) proved that optimal foraging is mathematically equivalent to the multi-armed bandit problem. Each patch is a bandit arm. The optimal policy is to pull the arm with the highest Gittins index -- an index that balances exploitation (expected reward) against exploration (uncertainty about the reward).

For blockchain agents, each monitored entity (chain, contract, address) is a patch. The "reward" is information relevant to the agent's strategy. The "cost" is compute, RPC calls, and context window budget. The Gittins index determines how much attention each entity deserves.

This is not a metaphor. The mathematical structure is identical. An agent deciding which contracts to monitor faces the same optimization problem as a bird deciding which bushes to forage.

### The attention budget

Every monitored entity has a Gittins index G_i that determines its attention allocation:

```rust
/// Computes the Gittins index for a monitored entity.
///
/// The index balances three factors:
/// - Expected information rate: how often this entity produces
///   events relevant to the agent's strategy.
/// - Uncertainty bonus: UCB-style exploration term that increases
///   attention to entities with uncertain value.
/// - Monitoring cost: RPC calls, decode compute, context budget
///   consumed per observation.
///
/// Higher G_i = more attention. The foraging model allocates
/// observation resources proportional to G_i.
#[derive(Debug, Clone)]
pub struct GittinsIndex {
    /// Entity being monitored.
    pub entity_id: EntityId,

    /// Expected relevant events per second.
    /// Computed from exponentially weighted moving average of
    /// historical event rates.
    pub expected_info_rate: f64,

    /// Uncertainty in the info rate estimate.
    /// Decreases as observations accumulate.
    /// Uses UCB formula: sqrt(2 * ln(total_observations) / entity_observations).
    pub uncertainty_bonus: f64,

    /// Cost per observation in abstract "attention units."
    /// Includes RPC call cost, decode compute, and context budget.
    pub monitoring_cost: f64,

    /// The computed Gittins index.
    pub index: f64,

    /// Number of observations of this entity.
    pub observation_count: u64,

    /// Timestamp of last observation.
    pub last_observed: DateTime<Utc>,
}

impl GittinsIndex {
    /// Recompute the index from current statistics.
    pub fn update(
        &mut self,
        total_observations: u64,
        discount_factor: f64,
    ) {
        // UCB1-style uncertainty bonus.
        self.uncertainty_bonus = if self.observation_count == 0 {
            f64::INFINITY // Unexplored entities have infinite index (explore first)
        } else {
            (2.0 * (total_observations as f64).ln()
                / self.observation_count as f64)
                .sqrt()
        };

        // Gittins index approximation.
        // The exact Gittins index requires solving a stopping problem,
        // which is computationally expensive. We use Whittle's approximation
        // (Whittle, 1988) which is asymptotically optimal.
        self.index = if self.monitoring_cost > 0.0 {
            (self.expected_info_rate + discount_factor * self.uncertainty_bonus)
                / self.monitoring_cost
        } else {
            self.expected_info_rate + discount_factor * self.uncertainty_bonus
        };
    }

    /// Update the expected info rate with a new observation.
    /// Uses exponentially weighted moving average with alpha = 0.05.
    pub fn observe_event(&mut self, relevant: bool) {
        let alpha = 0.05;
        let value = if relevant { 1.0 } else { 0.0 };
        self.expected_info_rate =
            alpha * value + (1.0 - alpha) * self.expected_info_rate;
        self.observation_count += 1;
        self.last_observed = Utc::now();
    }
}
```

### Attention allocation

The foraging model runs at R3 resolution (every 5 minutes) and produces an attention budget for each entity:

```rust
/// The foraging model that allocates attention across monitored entities.
///
/// Runs every R3 window (5 minutes). Updates Gittins indices for all
/// entities and computes attention budgets that the chain actors use
/// to decide which events to process at full resolution.
pub struct ForagingModel {
    /// All monitored entities and their Gittins indices.
    indices: HashMap<EntityId, GittinsIndex>,

    /// Total observation count across all entities.
    total_observations: u64,

    /// Discount factor for the uncertainty bonus.
    /// Higher values favor exploration. Lower values favor exploitation.
    /// Starts at 1.0 and decays toward 0.3 as the agent's worldview stabilizes.
    discount_factor: f64,

    /// Minimum attention budget. Entities below this threshold
    /// are monitored passively (only via block-level log scanning).
    min_budget: f64,

    /// Maximum fraction of total budget any single entity can receive.
    max_entity_fraction: f64,

    /// Strategy relevance function. Evaluates how relevant an entity's
    /// events are to the agent's current strategy.
    strategy_relevance: Box<dyn StrategyRelevance>,
}

/// How the agent evaluates event relevance against its strategy.
pub trait StrategyRelevance: Send + Sync {
    /// Score the relevance of an event type from an entity type
    /// to the current strategy. Returns 0.0 (irrelevant) to 1.0 (critical).
    fn score(
        &self,
        entity_type: &EntityType,
        event_type: &EventPayload,
    ) -> f64;

    /// Return the current strategy description for logging.
    fn description(&self) -> &str;
}

impl ForagingModel {
    /// Recompute attention budgets for all entities.
    ///
    /// Called at every R3 tick. The output is a map from EntityId
    /// to attention budget (0.0 to 1.0), where:
    ///   1.0 = monitor every block at full decode resolution
    ///   0.5 = monitor every other block, selective decode
    ///   0.1 = monitor every 10th block, minimal decode
    ///   0.0 = passive monitoring only (block-level log scan)
    pub fn allocate(&mut self) -> HashMap<EntityId, f64> {
        // Update all Gittins indices.
        for idx in self.indices.values_mut() {
            idx.update(self.total_observations, self.discount_factor);
        }

        // Compute raw budgets proportional to Gittins index.
        let total_index: f64 = self.indices.values()
            .map(|idx| idx.index)
            .sum();

        let mut budgets = HashMap::new();

        if total_index == 0.0 {
            // No data yet. Equal budget for all.
            let equal = 1.0 / self.indices.len() as f64;
            for id in self.indices.keys() {
                budgets.insert(id.clone(), equal);
            }
            return budgets;
        }

        for (id, idx) in &self.indices {
            let raw_budget = idx.index / total_index;
            let clamped = raw_budget
                .max(self.min_budget)
                .min(self.max_entity_fraction);
            budgets.insert(id.clone(), clamped);
        }

        // Decay the discount factor (reduce exploration over time).
        self.discount_factor = (self.discount_factor * 0.999).max(0.3);

        budgets
    }

    /// Register a new entity for monitoring.
    pub fn register(&mut self, entity_id: EntityId, initial_cost: f64) {
        self.indices.insert(entity_id.clone(), GittinsIndex {
            entity_id,
            expected_info_rate: 0.0,
            uncertainty_bonus: f64::INFINITY,
            monitoring_cost: initial_cost,
            index: f64::INFINITY, // Explore immediately
            observation_count: 0,
            last_observed: Utc::now(),
        });
    }

    /// Remove an entity from monitoring.
    pub fn deregister(&mut self, entity_id: &EntityId) {
        self.indices.remove(entity_id);
    }

    /// Record an observation for an entity.
    pub fn observe(
        &mut self,
        entity_id: &EntityId,
        event: &CanonicalEvent,
    ) {
        if let Some(idx) = self.indices.get_mut(entity_id) {
            let relevant = self.strategy_relevance
                .score(
                    &EntityType::from_classification(event.classification.as_ref()),
                    &event.payload,
                ) > 0.3;
            idx.observe_event(relevant);
            self.total_observations += 1;
        }
    }
}
```

### Patch switching

Following the Marginal Value Theorem, the agent "leaves a patch" (stops monitoring an entity) when its marginal information rate drops below the average:

```rust
impl ForagingModel {
    /// Identify entities that should be dropped from active monitoring.
    ///
    /// An entity is dropped when:
    /// 1. Its Gittins index falls below the average index across all entities.
    /// 2. It has been observed at least `min_observations` times
    ///    (to avoid dropping entities too early).
    /// 3. It has not produced a relevant event in the last `stale_window`.
    pub fn identify_drop_candidates(
        &self,
        min_observations: u64,
        stale_window: Duration,
    ) -> Vec<EntityId> {
        let avg_index: f64 = self.indices.values()
            .map(|idx| idx.index)
            .sum::<f64>()
            / self.indices.len() as f64;

        let now = Utc::now();
        let stale_cutoff = now - chrono::Duration::from_std(stale_window)
            .unwrap_or(chrono::Duration::hours(1));

        self.indices.iter()
            .filter(|(_, idx)| {
                idx.observation_count >= min_observations
                    && idx.index < avg_index * 0.5
                    && idx.last_observed < stale_cutoff
            })
            .map(|(id, _)| id.clone())
            .collect()
    }
}
```

---

## 14. Dynamic contract discovery

### The five-layer pipeline

When the agent encounters a new contract address (from a log, a transaction target, or a factory event), it classifies the contract through a five-layer pipeline. Each layer adds confidence without requiring an LLM call. The entire pipeline runs at T0.

```rust
/// Five-layer contract discovery pipeline.
///
/// Classifies smart contracts by type (DEX pool, lending vault,
/// bridge, token, factory, etc.) using progressively more expensive
/// techniques. All layers are T0 (deterministic, no LLM).
pub struct ContractDiscovery {
    /// Layer 0: ERC-165 interface detection.
    erc165: Erc165Detector,
    /// Layer 1: Function selector fingerprinting.
    selector_db: SelectorDatabase,
    /// Layer 2: Bytecode similarity matching.
    bytecode_db: BytecodeDatabase,
    /// Layer 3: Transaction pattern analysis.
    pattern_analyzer: PatternAnalyzer,
    /// Layer 4: Factory contract tracking.
    factory_tracker: FactoryTracker,
}

/// Classification result from the discovery pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Classification {
    pub entity_type: EntityType,
    pub confidence: f64,
    pub source: ClassificationSource,
    pub details: ClassificationDetails,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClassificationSource {
    Erc165,          // Layer 0: authoritative for compliant contracts
    Selector,        // Layer 1: probable from function signatures
    Bytecode,        // Layer 2: high confidence from code similarity
    TransactionFlow, // Layer 3: behavioral, grows with observation time
    Factory,         // Layer 4: inherited from known factory
    Stigmergy,       // Layer 5: from other agents via InsightStore
}

/// Known entity types in the DeFi universe.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EntityType {
    // Tokens
    Erc20Token,
    Erc721Collection,
    Erc1155MultiToken,
    WrappedNative,  // WETH, WMATIC, etc.

    // DEX
    UniswapV2Pair,
    UniswapV3Pool,
    CurvePool,
    BalancerPool,
    GenericAmm,

    // Lending
    AaveLendingPool,
    CompoundCToken,
    MorphoVault,
    Erc4626Vault,
    GenericLending,

    // Derivatives
    PerpetualPool,
    OptionsVault,

    // Infrastructure
    Bridge,
    Router,
    Factory,
    Oracle,
    Proxy,
    Multisig,

    // Unknown (not yet classified)
    Unknown,
}
```

### Layer 0: ERC-165 interface detection

ERC-165 defines a standard method for contracts to declare which interfaces they support. A single `staticcall` to `supportsInterface(bytes4)` returns true or false.

```rust
/// Layer 0: ERC-165 interface detection.
///
/// Costs: one staticcall per interface checked (~30K gas equivalent,
/// ~0.1ms per RPC round-trip).
///
/// Coverage: authoritative for ERC-165-compliant contracts.
/// Many DeFi contracts (especially tokens) comply. Most DEX pools
/// and lending protocols do not (they predate ERC-165 adoption).
pub struct Erc165Detector {
    /// Known interface IDs mapped to entity types.
    interfaces: Vec<(InterfaceId, EntityType)>,
}

/// ERC-165 interface identifiers.
struct InterfaceId([u8; 4]);

impl Erc165Detector {
    pub fn new() -> Self {
        Self {
            interfaces: vec![
                (InterfaceId::from_hex("80ac58cd"), EntityType::Erc721Collection),   // ERC-721
                (InterfaceId::from_hex("d9b67a26"), EntityType::Erc1155MultiToken),  // ERC-1155
                (InterfaceId::from_hex("36372b07"), EntityType::Erc20Token),         // ERC-20 (unofficial)
                (InterfaceId::from_hex("a219a025"), EntityType::Erc4626Vault),       // ERC-4626
            ],
        }
    }

    /// Check if a contract supports known interfaces.
    /// Returns the first matching classification, or None.
    pub async fn detect(
        &self,
        connector: &dyn ChainConnector,
        address: &str,
    ) -> Option<Classification> {
        for (iface, entity_type) in &self.interfaces {
            let calldata = build_supports_interface_call(iface);
            match connector.call(
                ContractCall { to: address.into(), data: calldata },
                None,
            ).await {
                Ok(result) if result.len() >= 32 && result[31] == 1 => {
                    return Some(Classification {
                        entity_type: entity_type.clone(),
                        confidence: 0.95, // ERC-165 is authoritative but not infallible
                        source: ClassificationSource::Erc165,
                        details: ClassificationDetails::Erc165 {
                            interface_id: hex::encode(iface.0),
                        },
                    });
                }
                _ => continue,
            }
        }
        None
    }
}
```

### Layer 1: function selector fingerprinting

Extract 4-byte function selectors from contract bytecode and match against a database of known signatures.

```rust
/// Layer 1: Function selector fingerprinting.
///
/// Extracts 4-byte function selectors from deployed bytecode by scanning
/// for PUSH4 opcodes before JUMPI instructions. Matches selectors against
/// a database of known signatures (from 4byte.directory + local additions).
///
/// Cost: one eth_getCode call + local bytecode scan. ~5ms total.
///
/// Classification logic: certain selector combinations are diagnostic.
/// A contract with swap(address,address,uint256,uint256,address) and
/// getReserves() is almost certainly a Uniswap V2-style pair.
pub struct SelectorDatabase {
    /// Map from 4-byte selector to (function signature, classification hint).
    signatures: HashMap<[u8; 4], Vec<SelectorEntry>>,
    /// Classification rules: "if contract has selectors A AND B, classify as X."
    rules: Vec<SelectorRule>,
}

struct SelectorEntry {
    signature: String,
    hint: Option<EntityType>,
}

struct SelectorRule {
    required_selectors: Vec<[u8; 4]>,
    entity_type: EntityType,
    confidence: f64,
}

impl SelectorDatabase {
    /// Classify a contract by its function selectors.
    pub fn classify(&self, bytecode: &[u8]) -> Option<Classification> {
        let selectors = extract_selectors(bytecode);

        // Check rules in order (most specific first).
        for rule in &self.rules {
            if rule.required_selectors.iter()
                .all(|s| selectors.contains(s))
            {
                return Some(Classification {
                    entity_type: rule.entity_type.clone(),
                    confidence: rule.confidence,
                    source: ClassificationSource::Selector,
                    details: ClassificationDetails::Selector {
                        matched_selectors: rule.required_selectors.iter()
                            .filter_map(|s| self.signatures.get(s))
                            .flatten()
                            .map(|e| e.signature.clone())
                            .collect(),
                    },
                });
            }
        }

        None
    }
}

/// Extract 4-byte function selectors from EVM bytecode.
///
/// Scans for PUSH4 (0x63) opcodes followed by JUMPI patterns.
/// This is a heuristic -- not all PUSH4 values are selectors --
/// but it achieves >95% recall on production contracts.
fn extract_selectors(bytecode: &[u8]) -> HashSet<[u8; 4]> {
    let mut selectors = HashSet::new();
    let mut i = 0;
    while i + 4 < bytecode.len() {
        if bytecode[i] == 0x63 { // PUSH4
            let selector: [u8; 4] = bytecode[i + 1..i + 5].try_into().unwrap();
            selectors.insert(selector);
            i += 5;
        } else {
            i += 1;
        }
    }
    selectors
}
```

Built-in selector rules:

| Selectors present | Classification | Confidence |
|-------------------|---------------|------------|
| `swap()` + `getReserves()` + `token0()` + `token1()` | UniswapV2Pair | 0.90 |
| `swap()` + `liquidity()` + `fee()` + `tick()` | UniswapV3Pool | 0.90 |
| `exchange()` + `get_dy()` + `coins()` | CurvePool | 0.85 |
| `supply()` + `borrow()` + `repay()` + `liquidate()` | GenericLending | 0.80 |
| `deposit()` + `withdraw()` + `asset()` + `totalAssets()` | Erc4626Vault | 0.85 |
| `deposit()` + `sendMessage()` | Bridge | 0.75 |
| `createPair()` or `createPool()` | Factory | 0.90 |

### Layer 2: bytecode similarity

Compare deployed bytecode against known contract families using code hash matching and SSG-based similarity.

```rust
/// Layer 2: Bytecode similarity matching.
///
/// Two strategies:
/// 1. Exact code hash: deployed bytecode hash matches a known contract.
///    Fast (O(1) lookup), authoritative, but misses modified clones.
/// 2. SSG similarity: Semantic Similarity Graph comparison.
///    Based on Esim (arXiv:2511.12971) which achieves 96.3% AUC
///    on smart contract clone detection. Handles contracts with
///    modified parameters, different Solidity versions, or minor
///    code changes.
pub struct BytecodeDatabase {
    /// Exact code hashes of known contract families.
    exact_hashes: HashMap<[u8; 32], (String, EntityType)>,

    /// SSG representations of known contract families for similarity matching.
    ssg_templates: Vec<SsgTemplate>,

    /// Similarity threshold for SSG matching.
    similarity_threshold: f64,
}

struct SsgTemplate {
    name: String,
    entity_type: EntityType,
    /// Precomputed SSG representation of the reference contract.
    ssg: Vec<f32>,
}

impl BytecodeDatabase {
    /// Classify a contract by bytecode similarity.
    pub fn classify(&self, bytecode: &[u8]) -> Option<Classification> {
        // Strategy 1: exact hash match.
        let hash = sha256(bytecode);
        if let Some((name, entity_type)) = self.exact_hashes.get(&hash) {
            return Some(Classification {
                entity_type: entity_type.clone(),
                confidence: 0.99,
                source: ClassificationSource::Bytecode,
                details: ClassificationDetails::Bytecode {
                    match_type: "exact_hash".into(),
                    reference: name.clone(),
                    similarity: 1.0,
                },
            });
        }

        // Strategy 2: SSG similarity.
        let candidate_ssg = compute_ssg(bytecode);
        let mut best_match: Option<(f64, &SsgTemplate)> = None;

        for template in &self.ssg_templates {
            let sim = cosine_similarity(&candidate_ssg, &template.ssg);
            if sim > self.similarity_threshold {
                if best_match.map_or(true, |(best, _)| sim > best) {
                    best_match = Some((sim, template));
                }
            }
        }

        best_match.map(|(sim, template)| Classification {
            entity_type: template.entity_type.clone(),
            confidence: sim * 0.9, // Scale down slightly from raw similarity
            source: ClassificationSource::Bytecode,
            details: ClassificationDetails::Bytecode {
                match_type: "ssg_similarity".into(),
                reference: template.name.clone(),
                similarity: sim,
            },
        })
    }
}
```

### Layer 3: transaction pattern analysis

Observe actual transaction flow over time and classify based on behavioral patterns.

```rust
/// Layer 3: Transaction pattern analysis.
///
/// Observes actual transaction patterns over time and classifies
/// contracts based on behavior rather than code.
///
/// This is the dynamic layer. Classification confidence grows
/// with observation time. A contract that has been observed for
/// 1 hour has lower confidence than one observed for 1 week.
pub struct PatternAnalyzer {
    /// Per-contract observation state.
    observations: HashMap<String, ContractObservation>,

    /// Pattern rules: "if the contract exhibits pattern X, classify as Y."
    rules: Vec<PatternRule>,
}

#[derive(Debug)]
struct ContractObservation {
    address: String,
    first_seen: DateTime<Utc>,
    last_seen: DateTime<Utc>,
    total_txs: u64,
    /// Distribution of function selectors called.
    selector_distribution: HashMap<[u8; 4], u64>,
    /// Unique callers.
    unique_callers: HashSet<String>,
    /// Value flow statistics.
    total_value_in: f64,
    total_value_out: f64,
    /// Event emission patterns.
    event_counts: HashMap<String, u64>,
}

struct PatternRule {
    name: String,
    entity_type: EntityType,
    /// Minimum observation period before this rule can fire.
    min_observation_period: Duration,
    /// Minimum transactions before this rule can fire.
    min_txs: u64,
    /// Predicate that checks whether the observation matches this pattern.
    matches: Box<dyn Fn(&ContractObservation) -> bool + Send + Sync>,
    /// Base confidence, scaled by observation duration.
    base_confidence: f64,
}
```

Built-in pattern rules:

- **DEX pool**: high-frequency swap calls, exactly 2-3 token addresses in events, balanced value in/out.
- **Lending protocol**: supply/borrow/repay/liquidate cycle, utilization ratio changes, interest accrual events.
- **Bridge**: lock events on source chain correlated with mint events on destination chain (cross-chain correlation requires WorldGraph).
- **Token**: high transfer count, many unique callers, approval events.
- **Factory**: low tx count but emits PairCreated/PoolCreated events.

### Layer 4: factory contract tracking

Monitor known factories for new contract deployments:

```rust
/// Layer 4: Factory contract tracking.
///
/// Watches known factory contracts for deployment events.
/// Newly deployed contracts inherit their parent factory's classification.
///
/// Known factories:
/// - Uniswap V2 Factory: PairCreated(token0, token1, pair, allPairs.length)
/// - Uniswap V3 Factory: PoolCreated(token0, token1, fee, tickSpacing, pool)
/// - Balancer Vault: PoolRegistered(poolId, poolAddress, specialization)
/// - Aave PoolAddressesProvider: various registration events
pub struct FactoryTracker {
    /// Known factory addresses and their deployment event signatures.
    factories: HashMap<String, FactorySpec>,
}

struct FactorySpec {
    name: String,
    /// The entity type that deployed contracts inherit.
    child_type: EntityType,
    /// Event topic that signals a new deployment.
    deployment_event: [u8; 32],
    /// Which log parameter contains the new contract address.
    address_param_index: usize,
}

impl FactoryTracker {
    /// Check if a log event is a factory deployment.
    /// If so, return a classification for the newly deployed contract.
    pub fn check_deployment(&self, log: &DecodedLog) -> Option<(String, Classification)> {
        let factory_spec = self.factories.get(&log.address)?;

        if log.topics.first()
            .map_or(false, |t| t == &hex::encode(factory_spec.deployment_event))
        {
            let new_address = extract_address_param(
                log,
                factory_spec.address_param_index,
            )?;

            Some((new_address.clone(), Classification {
                entity_type: factory_spec.child_type.clone(),
                confidence: 0.95,
                source: ClassificationSource::Factory,
                details: ClassificationDetails::Factory {
                    factory_name: factory_spec.name.clone(),
                    factory_address: log.address.clone(),
                },
            }))
        } else {
            None
        }
    }
}
```

### Layer 5: cross-agent stigmergy

Query the InsightStore (PRD-05, section 3) for contract classifications posted by other agents. If multiple independent agents classified the same contract identically, confidence is high.

```rust
/// Layer 5: Cross-agent stigmergy via InsightStore.
///
/// Query the on-chain knowledge substrate for contract classifications
/// posted by other agents. Multiple independent classifications of the
/// same contract increase confidence.
///
/// Privacy note: classifications are stored as HDC vectors, not plain text.
/// The query uses cosine similarity in HDC space. The actual contract type
/// name is encoded in the vector but not directly readable from the chain.
pub async fn query_stigmergy(
    insight_store: &InsightStoreClient,
    address: &str,
    chain: &ChainId,
) -> Option<Classification> {
    // Encode the query as an HDC vector.
    let query_hdv = encode_contract_query(address, chain);

    // Search InsightStore for matching entries.
    let results = insight_store
        .query(query_hdv, 10, InsightKind::ContractClassification)
        .await
        .ok()?;

    if results.is_empty() {
        return None;
    }

    // Decode classifications from HDC vectors.
    let mut type_votes: HashMap<EntityType, u64> = HashMap::new();
    for result in &results {
        if let Some(entity_type) = decode_entity_type_from_hdv(&result.hdv) {
            *type_votes.entry(entity_type).or_insert(0) += 1;
        }
    }

    // Majority vote with confidence proportional to agreement.
    let (entity_type, count) = type_votes.into_iter()
        .max_by_key(|(_, c)| *c)?;

    let agreement_ratio = count as f64 / results.len() as f64;

    Some(Classification {
        entity_type,
        confidence: agreement_ratio * 0.85, // Cap at 0.85 for stigmergic sources
        source: ClassificationSource::Stigmergy,
        details: ClassificationDetails::Stigmergy {
            source_count: results.len(),
            agreement_ratio,
        },
    })
}
```

### Layer composition

All five layers compose into a single classification pipeline:

```rust
impl ContractDiscovery {
    /// Classify a contract address using all available layers.
    ///
    /// Layers run in order (0 through 4 locally, 5 from InsightStore).
    /// If Layer 0 (ERC-165) returns a high-confidence result, later layers
    /// are skipped. Otherwise, results from all layers are combined
    /// using a weighted vote.
    pub async fn classify(
        &mut self,
        connector: &dyn ChainConnector,
        insight_store: &InsightStoreClient,
        address: &str,
        chain: &ChainId,
    ) -> Classification {
        let mut classifications: Vec<Classification> = Vec::new();

        // Layer 0: ERC-165 (authoritative for compliant contracts).
        if let Some(c) = self.erc165.detect(connector, address).await {
            if c.confidence > 0.9 {
                return c; // High-confidence ERC-165 result, skip other layers.
            }
            classifications.push(c);
        }

        // Layer 1: Selector fingerprinting.
        if let Ok(bytecode) = connector.call(
            ContractCall { to: address.into(), data: vec![] },
            None,
        ).await {
            if let Some(c) = self.selector_db.classify(&bytecode) {
                classifications.push(c);
            }
            // Layer 2: Bytecode similarity.
            if let Some(c) = self.bytecode_db.classify(&bytecode) {
                classifications.push(c);
            }
        }

        // Layer 3: Transaction patterns (uses accumulated observations).
        if let Some(c) = self.pattern_analyzer.classify(address) {
            classifications.push(c);
        }

        // Layer 5: Cross-agent stigmergy.
        if let Some(c) = query_stigmergy(insight_store, address, chain).await {
            classifications.push(c);
        }

        // Combine classifications via confidence-weighted vote.
        combine_classifications(&classifications)
    }
}

/// Combine classifications from multiple layers.
///
/// Each layer votes for an entity type weighted by its confidence.
/// The type with the highest total weight wins. Final confidence
/// is the weighted average of contributing layers' confidences.
fn combine_classifications(cs: &[Classification]) -> Classification {
    if cs.is_empty() {
        return Classification {
            entity_type: EntityType::Unknown,
            confidence: 0.0,
            source: ClassificationSource::Selector,
            details: ClassificationDetails::None,
        };
    }

    let mut votes: HashMap<EntityType, (f64, Vec<&Classification>)> = HashMap::new();
    for c in cs {
        let entry = votes.entry(c.entity_type.clone()).or_insert((0.0, vec![]));
        entry.0 += c.confidence;
        entry.1.push(c);
    }

    let (entity_type, (total_weight, sources)) = votes.into_iter()
        .max_by(|a, b| a.1.0.partial_cmp(&b.1.0).unwrap())
        .unwrap();

    let avg_confidence = total_weight / sources.len() as f64;
    let best_source = sources.iter()
        .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
        .unwrap();

    Classification {
        entity_type,
        confidence: avg_confidence.min(0.99),
        source: best_source.source,
        details: best_source.details.clone(),
    }
}
```

---

## 15. Dynamic worldview building

### The WorldGraph

The WorldGraph is a living, evolving graph of entities and relationships that the agent discovers through foraging. Nothing is hardcoded. The agent starts with chain subscriptions and a strategy. Everything else -- which contracts matter, how they relate, what they do -- emerges through observation and classification.

```rust
/// A living graph of entities and relationships discovered through
/// foraging and contract classification.
///
/// The WorldGraph is the agent's understanding of its environment.
/// It grows through observation, refines through classification,
/// prunes through attention allocation, and shares through stigmergy.
///
/// The graph serves three functions:
/// 1. Context assembly: relevant subgraphs are injected into the
///    agent's context window via VCG bidding.
/// 2. Attention guidance: the foraging model uses graph topology
///    to discover new entities (neighbors of high-value entities).
/// 3. Strategy evolution: patterns in the graph inform playbook
///    updates during dream consolidation.
pub struct WorldGraph {
    /// All known entities, keyed by EntityId.
    entities: HashMap<EntityId, Entity>,

    /// Directed edges between entities.
    relationships: Vec<Relationship>,

    /// Adjacency list for fast neighbor lookup.
    adjacency: HashMap<EntityId, Vec<(EntityId, RelationshipType)>>,

    /// Per-entity classification results (may have multiple from different layers).
    classifications: HashMap<EntityId, Vec<Classification>>,

    /// Per-entity attention budgets from the foraging model.
    attention_budgets: HashMap<EntityId, f64>,

    /// HDC fingerprint encoding the entire worldview.
    /// Used for cross-agent comparison: agents with similar fingerprints
    /// have similar worldviews (monitor similar entities).
    hdv_fingerprint: HdcVector,

    /// Metrics for observability.
    entity_count: usize,
    relationship_count: usize,
    last_updated: DateTime<Utc>,
}

/// An entity in the WorldGraph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Unique identifier (chain + address).
    pub id: EntityId,

    /// Which chain this entity lives on.
    pub chain: ChainId,

    /// On-chain address.
    pub address: String,

    /// Classified entity type. Starts as Unknown, evolves through
    /// the contract discovery pipeline.
    pub entity_type: EntityType,

    /// Classification confidence (0.0 to 1.0).
    pub confidence: f64,

    /// Which discovery layer first identified this entity.
    pub discovery_source: ClassificationSource,

    /// When the entity was first observed.
    pub first_seen: DateTime<Utc>,

    /// When the entity last produced a relevant event.
    pub last_active: DateTime<Utc>,

    /// Events per second relevant to the agent's strategy.
    /// Exponentially weighted moving average.
    pub info_rate: f64,

    /// HDC encoding of this entity's behavioral pattern.
    /// Used for similarity queries against the InsightStore.
    pub hdv: HdcVector,

    /// Cached state snapshot (protocol-specific).
    /// For a DEX pool: reserves, price, volume.
    /// For a lending vault: utilization, supply rate, borrow rate.
    pub cached_state: Option<serde_json::Value>,
}

/// A directed relationship between two entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub from: EntityId,
    pub to: EntityId,
    pub relationship_type: RelationshipType,
    pub confidence: f64,
    pub discovered_at: DateTime<Utc>,
}

/// Types of relationships between entities.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RelationshipType {
    /// Pool contains this token (DEX pool -> token).
    PoolContainsToken,
    /// Vault borrows from this pool (lending vault -> pool).
    VaultBorrowsFrom,
    /// Bridge connects two chains (bridge -> chain entity).
    BridgeConnects,
    /// Factory deployed this contract (factory -> child).
    FactoryDeployed,
    /// Router routes through this pool (router -> pool).
    RouterUsesPool,
    /// Token wraps another token (wrapped -> underlying).
    Wraps,
    /// Oracle provides price feed for this pair (oracle -> token pair).
    OracleFeedFor,
    /// Cross-chain: same protocol on different chains.
    CrossChainCounterpart,
    /// Generic: detected from correlated transaction patterns.
    CorrelatedActivity { correlation: f64 },
}

/// Unique entity identifier: chain + address.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct EntityId {
    pub chain: ChainId,
    pub address: String,
}
```

### WorldGraph evolution

The WorldGraph evolves through seven stages, each running continuously:

**Stage 1: Bootstrap.** The agent starts with its configured chains and a strategy definition. The WorldGraph is empty.

**Stage 2: Discovery.** As canonical events arrive from chain actors, new contract addresses enter the WorldGraph as `Unknown` entities with high uncertainty (and therefore high Gittins index -- the foraging model will explore them):

```rust
impl WorldGraph {
    /// Process a canonical event and update the graph.
    pub fn process_event(&mut self, event: &CanonicalEvent) {
        // Extract entity addresses from the event.
        let addresses = extract_addresses(event);

        for address in addresses {
            let entity_id = EntityId {
                chain: event.chain.clone(),
                address: address.clone(),
            };

            if !self.entities.contains_key(&entity_id) {
                // New entity discovered. Add with Unknown type.
                self.entities.insert(entity_id.clone(), Entity {
                    id: entity_id.clone(),
                    chain: event.chain.clone(),
                    address,
                    entity_type: EntityType::Unknown,
                    confidence: 0.0,
                    discovery_source: ClassificationSource::TransactionFlow,
                    first_seen: event.block_time,
                    last_active: event.block_time,
                    info_rate: 0.0,
                    hdv: HdcVector::zero(),
                    cached_state: None,
                });
                self.entity_count += 1;
            }

            // Update last_active and info_rate.
            if let Some(entity) = self.entities.get_mut(&entity_id) {
                entity.last_active = event.block_time;
                let alpha = 0.01;
                entity.info_rate = alpha + (1.0 - alpha) * entity.info_rate;
            }
        }

        self.last_updated = Utc::now();
    }
}
```

**Stage 3: Classification.** The contract discovery pipeline (section 14) processes Unknown entities and assigns types. As classification confidence grows, the entity's type solidifies.

**Stage 4: Relationship extraction.** Relationships are detected from event patterns:
- A Swap event on a pool that references two token addresses creates `PoolContainsToken` edges.
- A Deposit event on a vault that calls a lending pool creates a `VaultBorrowsFrom` edge.
- A factory's `PairCreated` event creates a `FactoryDeployed` edge.

```rust
impl WorldGraph {
    /// Extract relationships from a decoded event.
    pub fn extract_relationships(&mut self, event: &CanonicalEvent) {
        if let EventPayload::Log(ref log) = event.payload {
            if let Some(ref decoded) = log.decoded {
                match decoded.event_name.as_str() {
                    "Swap" | "Sync" => {
                        // Pool -> Token relationships.
                        let pool_id = EntityId {
                            chain: event.chain.clone(),
                            address: log.address.clone(),
                        };
                        for param in &decoded.params {
                            if param.0.contains("token") || param.0.contains("Token") {
                                if let Some(addr) = param.1.as_str() {
                                    let token_id = EntityId {
                                        chain: event.chain.clone(),
                                        address: addr.into(),
                                    };
                                    self.add_relationship(Relationship {
                                        from: pool_id.clone(),
                                        to: token_id,
                                        relationship_type: RelationshipType::PoolContainsToken,
                                        confidence: 0.8,
                                        discovered_at: event.block_time,
                                    });
                                }
                            }
                        }
                    }
                    "PairCreated" | "PoolCreated" => {
                        // Factory -> Child relationship.
                        // Handled by FactoryTracker (layer 4), but we also
                        // record the relationship in the graph.
                        let factory_id = EntityId {
                            chain: event.chain.clone(),
                            address: log.address.clone(),
                        };
                        for param in &decoded.params {
                            if param.0 == "pair" || param.0 == "pool" {
                                if let Some(addr) = param.1.as_str() {
                                    let child_id = EntityId {
                                        chain: event.chain.clone(),
                                        address: addr.into(),
                                    };
                                    self.add_relationship(Relationship {
                                        from: factory_id.clone(),
                                        to: child_id,
                                        relationship_type: RelationshipType::FactoryDeployed,
                                        confidence: 0.95,
                                        discovered_at: event.block_time,
                                    });
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn add_relationship(&mut self, rel: Relationship) {
        // Avoid duplicate relationships.
        let key = (&rel.from, &rel.to, &rel.relationship_type);
        if !self.relationships.iter().any(|r|
            (&r.from, &r.to, &r.relationship_type) == key
        ) {
            self.adjacency
                .entry(rel.from.clone())
                .or_default()
                .push((rel.to.clone(), rel.relationship_type.clone()));
            self.relationships.push(rel);
            self.relationship_count += 1;
        }
    }
}
```

**Stage 5: Strategy alignment.** The foraging model evaluates which entities are relevant to the agent's strategy. A "delta neutral market making" strategy assigns high relevance to DEX pools and lending rates, moderate relevance to bridges (for cross-chain flow), and low relevance to NFT contracts.

**Stage 6: Gittins index update.** Entity relevance, information rate, and uncertainty compose into the Gittins index (section 13). Entities that consistently produce strategy-relevant events get more attention. Irrelevant entities fade. The foraging model drops entities that fall below the average index (Marginal Value Theorem).

**Stage 7: Cross-agent pollination.** The agent periodically publishes its WorldGraph fingerprint (HDC vector) to the InsightStore and queries for similar agents' discoveries. When another agent has classified a contract that this agent has observed but not yet classified, the stigmergy layer (layer 5) provides the classification.

### Strategy evolution through dreams

The strategy itself evolves during dream consolidation (PRD-03, section on delta cycles). When the agent enters a dream cycle:

1. **Episode replay.** High-surprise episodes are replayed. The agent re-examines events that caused unexpected outcomes.
2. **Pattern extraction.** The WorldGraph's topology is analyzed for recurring patterns. Example: certain pool pairs on Base consistently show price divergence from their Ethereum counterparts.
3. **Counterfactual simulation.** The agent asks: "What would have happened if I had monitored entity X more closely?" or "What if I had included cross-chain price data in my context?"
4. **Playbook update.** If pattern extraction reveals a consistent opportunity, the agent's playbook (PRD-05, section on procedural memory) is updated. Example: "delta neutral market making" evolves to "delta neutral market making WITH cross-chain Base/ETH arbitrage."

The dream cycle does not modify the WorldGraph directly. It modifies the strategy, which changes how the foraging model evaluates relevance, which changes attention budgets, which changes what the agent observes, which changes the WorldGraph. The loop is indirect and self-correcting.

### Context injection

The WorldGraph injects relevant subgraphs into the agent's context window through the VCG auction (PRD-04):

```rust
impl WorldGraph {
    /// Generate a context section for the VCG auction.
    ///
    /// Extracts the most relevant subgraph and formats it for
    /// inclusion in the agent's prompt.
    pub fn generate_context_bid(
        &self,
        strategy: &dyn StrategyRelevance,
        max_entities: usize,
    ) -> ContextBid {
        // Select top entities by attention budget.
        let mut ranked: Vec<(&EntityId, &Entity)> = self.entities.iter()
            .filter(|(_, e)| e.entity_type != EntityType::Unknown)
            .collect();
        ranked.sort_by(|a, b|
            self.attention_budgets.get(b.0).unwrap_or(&0.0)
                .partial_cmp(self.attention_budgets.get(a.0).unwrap_or(&0.0))
                .unwrap()
        );
        ranked.truncate(max_entities);

        // Format the subgraph as structured text.
        let mut content = String::from("## Active worldview\n\n");
        for (id, entity) in &ranked {
            content.push_str(&format!(
                "- **{}** on {} ({}): confidence {:.0}%, info rate {:.2}/s\n",
                entity.entity_type.label(),
                entity.chain,
                &entity.address[..10],
                entity.confidence * 100.0,
                entity.info_rate,
            ));

            // Include relationships.
            if let Some(neighbors) = self.adjacency.get(*id) {
                for (neighbor_id, rel_type) in neighbors {
                    if let Some(neighbor) = self.entities.get(neighbor_id) {
                        content.push_str(&format!(
                            "  -> {} {} ({})\n",
                            rel_type.label(),
                            neighbor.entity_type.label(),
                            &neighbor.address[..10],
                        ));
                    }
                }
            }

            // Include cached state if available.
            if let Some(ref state) = entity.cached_state {
                content.push_str(&format!("  State: {}\n", state));
            }
        }

        // Compute bid value proportional to strategy relevance.
        let avg_relevance: f64 = ranked.iter()
            .map(|(_, e)| strategy.score(&e.entity_type, &EventPayload::Block(Default::default())))
            .sum::<f64>()
            / ranked.len().max(1) as f64;

        ContextBid {
            key: "worldgraph".into(),
            content,
            value: avg_relevance.min(0.9),
            category: ContextCategory::Strategy,
            ttl: Duration::from_secs(30),
        }
    }
}
```

---

## 16. Active inference for attention allocation

### Free energy minimization

Active inference (Friston, 2010) provides a principled framework for attention allocation that subsumes the foraging model. The core idea: agents minimize variational free energy, which is the divergence between their predictions and their observations. Attention goes where prediction error is highest, because that is where the agent's model is most wrong.

The free energy for a monitored entity i decomposes into:

```
F_i = D_KL(q(s_i) || p(s_i | o_i))  +  H(o_i | s_i)
       ~~~~~~~~~~~~~~~~~~~~~~~~~~~~     ~~~~~~~~~~~~~~~~
       Complexity (how far beliefs       Accuracy (prediction
       shifted from prior)               error on observations)
```

Where:
- `q(s_i)` is the agent's current belief about entity i's state (e.g., "this is a Uniswap V3 pool with ~$5M TVL and high swap frequency").
- `p(s_i | o_i)` is the posterior belief after observing new data.
- `o_i` is the actual observation.
- `H(o_i | s_i)` is the conditional entropy -- how surprising the observation is given the current belief.

### Connecting free energy to attention

High free energy on entity i means one of two things:
1. **High complexity**: the agent's beliefs shifted a lot after the last observation. The entity is behaving unexpectedly. Allocate more attention.
2. **High accuracy loss**: the agent's predictions about the entity are consistently wrong. The model needs updating. Allocate more attention.

Low free energy means the agent's model is accurate and stable. Reduce attention.

```rust
/// Active inference attention allocation.
///
/// Augments the Gittins-index foraging model with free energy
/// computation. Entities with high free energy receive attention
/// beyond what the Gittins index alone would allocate.
///
/// This drives exploration naturally: uncertain entities have high
/// free energy (their predictions are unreliable), so the agent
/// explores them even without an explicit exploration bonus.
pub struct ActiveInferenceAttention {
    /// Per-entity generative model (predicts next observation).
    models: HashMap<EntityId, EntityModel>,

    /// Per-entity free energy history (for trend detection).
    free_energy_history: HashMap<EntityId, VecDeque<f64>>,

    /// Weight of free energy vs Gittins index in final attention budget.
    /// 0.0 = pure Gittins, 1.0 = pure free energy.
    free_energy_weight: f64,
}

/// Simple generative model for an entity.
///
/// Predicts the next observation based on exponentially weighted
/// moving statistics. Not a neural network -- this runs at T0.
struct EntityModel {
    /// Predicted event rate (events per second).
    predicted_rate: f64,
    /// Observed event rate.
    observed_rate: f64,
    /// Predicted distribution of event types.
    predicted_type_dist: HashMap<String, f64>,
    /// Observed distribution of event types.
    observed_type_dist: HashMap<String, f64>,
    /// Number of observations.
    n: u64,
}

impl EntityModel {
    /// Compute free energy after a new observation.
    fn free_energy(&self) -> f64 {
        // Accuracy: squared prediction error on event rate.
        let accuracy_loss =
            (self.predicted_rate - self.observed_rate).powi(2);

        // Complexity: KL divergence between predicted and observed type distributions.
        let complexity = kl_divergence(
            &self.observed_type_dist,
            &self.predicted_type_dist,
        );

        accuracy_loss + complexity
    }

    /// Update predictions toward observations (belief update).
    fn update(&mut self, alpha: f64) {
        self.predicted_rate =
            alpha * self.observed_rate + (1.0 - alpha) * self.predicted_rate;

        for (key, obs_prob) in &self.observed_type_dist {
            let pred = self.predicted_type_dist
                .entry(key.clone())
                .or_insert(0.0);
            *pred = alpha * obs_prob + (1.0 - alpha) * *pred;
        }

        self.n += 1;
    }
}

/// KL divergence D_KL(P || Q) between two discrete distributions.
fn kl_divergence(
    p: &HashMap<String, f64>,
    q: &HashMap<String, f64>,
) -> f64 {
    let epsilon = 1e-10;
    p.iter()
        .map(|(k, p_k)| {
            let q_k = q.get(k).copied().unwrap_or(epsilon);
            if *p_k > epsilon {
                p_k * (p_k / q_k).ln()
            } else {
                0.0
            }
        })
        .sum()
}

impl ActiveInferenceAttention {
    /// Combine Gittins indices with free energy for final attention budgets.
    pub fn allocate(
        &self,
        gittins_budgets: &HashMap<EntityId, f64>,
    ) -> HashMap<EntityId, f64> {
        let mut combined = HashMap::new();

        for (id, gittins_budget) in gittins_budgets {
            let fe_budget = self.models.get(id)
                .map(|m| m.free_energy().min(1.0))
                .unwrap_or(1.0); // Unknown entities get max free energy

            let budget = (1.0 - self.free_energy_weight) * gittins_budget
                + self.free_energy_weight * fe_budget;

            combined.insert(id.clone(), budget.min(1.0));
        }

        combined
    }
}
```

### Why free energy, not just Gittins

The Gittins index is a reward-maximizing policy. It allocates attention proportional to expected information gain. This works well when the agent's model of each entity is approximately correct.

Free energy adds a correction for model quality. An entity might have low expected information rate (low Gittins) but high prediction error (high free energy). The Gittins-only agent ignores it. The free-energy-augmented agent pays attention because its model is wrong -- and a wrong model is a liability.

In practice, this matters for regime changes. When a lending protocol shifts from low utilization to high utilization (rate spike), the Gittins index updates slowly because the moving average lags. Free energy spikes immediately because the prediction error jumps. The agent notices the regime change faster and reallocates attention before the Gittins index catches up.

---

## 17. The `roko publish` ecosystem

### Publishing a cognitive extension

```bash
cd my-extension/
# Cargo.toml has [package.metadata.roko] section configured.

# Verify the extension compiles and passes tests.
cargo test

# Dry run: validate manifest, check dependencies, verify ABI version.
roko publish --dry-run

# Publish to the roko registry.
roko publish
```

The `roko publish` command:

1. Reads `[package.metadata.roko]` from `Cargo.toml`.
2. Verifies the crate compiles (`cargo check`).
3. Checks ABI version compatibility.
4. Validates the manifest (required fields, valid type, valid capabilities).
5. Builds the crate in release mode.
6. Packages the binary artifact, manifest, and gallery assets.
7. Uploads to the roko registry.
8. The registry indexes the package for search.

### Publishing a chain connector

```bash
cd roko-chain-solana/
roko publish --type chain-connector

# The registry verifies:
# - Implements ChainConnector trait
# - Declares chain_type, block_time_ms, finality_mode in manifest
# - Includes at least one integration test
```

### Publishing a domain profile

```bash
cd my-quant-profile/
# Contains profile.toml + optional bundled skills and prompts.
roko publish --type profile
```

### Publishing a Pi-compatible extension

```bash
cd my-pi-extension/
# package.json has "pi" key.
npm publish                   # Publish to npm (for Pi users)
roko publish --npm            # Also publish to roko registry
```

### The registry

The roko registry is a crates.io-compatible Rust registry (built on Kellnr or a custom implementation) extended with:

- **Gallery metadata**: images, videos, tags for marketplace browsing.
- **Type classification**: extensions, skills, profiles, arenas, chain connectors.
- **Compatibility matrix**: which roko versions each package supports.
- **Usage statistics**: install counts, active agent counts.
- **npm mirror**: Pi-compatible packages are also indexed from npm for unified search.

Search:

```bash
roko search "defi"
# Results:
# crate:roko-ext-defi         Cognitive extension  DeFi position monitoring      v0.3.0  1.2K installs
# crate:roko-ext-yield        Cognitive extension  Yield farming optimization    v0.1.0  340 installs
# npm:@pi/defi-tools          Extension (Tier 1)   Basic DeFi tools for Pi/Roko  v2.1.0  5.4K installs
# crate:roko-profile-quant    Domain profile       Quantitative trading agent    v0.2.0  890 installs
# crate:roko-chain-hyperliquid Chain connector     Hyperliquid integration       v0.1.0  230 installs
```

### Marketplace TUI

`roko market` opens a ratatui-based package browser:

```
+-------------------------------------------------------------------+
| Roko Package Marketplace                          [Search: defi]  |
+-------------------------------------------------------------------+
| Type: All | Extension | Skill | Profile | Arena | Chain           |
+-------------------------------------------------------------------+
| roko-ext-defi                               v0.3.0  1.2K installs|
| DeFi position monitoring and yield optimization                   |
| Tags: defi, yield, blockchain     Type: Cognitive Extension       |
|                                                                   |
| roko-profile-quant                          v0.2.0    890 installs|
| Quantitative trading agent profile                                |
| Tags: quant, trading, blockchain  Type: Domain Profile            |
|                                                                   |
| roko-chain-hyperliquid                      v0.1.0    230 installs|
| Hyperliquid blockchain integration                                |
| Tags: hyperliquid, chain          Type: Chain Connector           |
+-------------------------------------------------------------------+
| [Enter] Install  [i] Info  [d] Dependencies  [q] Quit            |
+-------------------------------------------------------------------+
```

---

## 18. HuggingFace integration (Stream C)

### The five API layers

HuggingFace is not one API. It is five, each serving a different function in the agent improvement loop.

| Layer | HF API | What Roko uses it for | Crate module |
|-------|--------|----------------------|--------------|
| **Inference Providers** | `/models/{id}` with provider routing | Production inference for specialized models (code review, security analysis, domain-specific tasks) that CascadeRouter selects as arms | `roko-hf::inference` |
| **Hub** | `/api/models`, `/api/datasets` | Model and dataset discovery, download, metadata queries. Enables cross-instance model sharing. | `roko-hf::hub` |
| **Datasets** | `/api/datasets/{id}`, streaming API | Training data management. Episodes become training rows. Published datasets feed other Roko instances. | `roko-hf::datasets` |
| **Endpoints** | `/api/endpoints` | Dedicated inference endpoints for high-throughput or latency-sensitive workloads. CascadeRouter can route to a dedicated endpoint when volume justifies the cost. | `roko-hf::endpoints` |
| **AutoTrain** | `/api/autotrain` | Fine-tuning without MLOps. Feed episodes as training data, get a fine-tuned model back. The model becomes a new CascadeRouter arm. | `roko-hf::autotrain` |

### The exponential fine-tuning loop

The loop has six stages. Each stage feeds the next. The cycle time is bounded by fine-tuning duration (hours to days), but the data collection is continuous.

```
Stage 1: Episodes accumulate
  Agent executes tasks. Each task produces an episode:
  prompt, response, tool calls, gate results, CRPS scores, latency.
  Episodes log to .roko/episodes.jsonl (already wired).

Stage 2: Episodes become training data
  A nightly job (or on-demand trigger) transforms episodes into
  training rows. High-gate-pass episodes become positive examples.
  Low-gate-pass episodes become negative examples (with the correct
  output from retry or human fix as the positive).

  Format: instruction-response pairs with metadata tags
  (domain, task_type, complexity, gate_scores).

Stage 3: Training data uploads to HuggingFace Datasets
  roko-hf::datasets pushes the training set to a HF dataset repo.
  The dataset is versioned (one commit per upload).
  Optional: make the dataset public for cross-instance sharing.

Stage 4: AutoTrain fine-tunes a model
  roko-hf::autotrain kicks off a fine-tuning job:
  - Base model: configurable (default: a small open model like Llama 3.1 8B)
  - Training data: the dataset from stage 3
  - Method: LoRA or full fine-tuning depending on base model size
  - Output: a fine-tuned model hosted on HuggingFace

Stage 5: Fine-tuned model becomes a CascadeRouter arm
  CascadeRouter adds the new model as an arm with an initial
  Thompson Sampling prior (alpha=1, beta=1 -- uninformative).
  The arm competes against existing arms (Claude, GPT, base models)
  for task routing.

Stage 6: Explore and feedback
  CascadeRouter routes a fraction of tasks to the new arm
  (exploration rate controlled by Thompson Sampling).
  Gate results update the arm's posterior. If the fine-tuned model
  outperforms on certain task types, it wins more routing share.
  If it underperforms, it gets routed less.

  The cycle restarts: new episodes from the fine-tuned model
  feed stage 1.
```

Each cycle through this loop produces a model that is slightly better at the specific tasks this Roko instance handles. Over multiple cycles, the model specializes. A Roko instance that mostly does code review fine-tunes toward code review. One that mostly does DeFi analysis fine-tunes toward DeFi. The generic base model becomes a specialist.

### Network effects through cross-instance model sharing

When a Roko instance publishes its fine-tuned model to HuggingFace Hub (opt-in, not default), other instances can discover and use it:

1. **Discovery.** `roko search --type model --domain blockchain` queries the Hub for models tagged with Roko metadata.
2. **Evaluation.** The discovering instance runs the model through its local arena suite to estimate performance before committing.
3. **Adoption.** If the model passes a configurable performance threshold, it is added as a CascadeRouter arm.
4. **Feedback.** Usage data from the adopting instance feeds back to the model's Hub page as community metrics.

This creates a network effect: more Roko instances means more fine-tuned models, which means each instance has a larger pool of specialized models to draw from. A new Roko instance bootstraps faster because it can adopt models that other instances have already fine-tuned.

### roko-hf crate structure

```
crates/roko-hf/
  src/
    lib.rs              # Crate root, re-exports
    inference.rs        # Inference Providers API client
    hub.rs              # Hub API client (model/dataset discovery)
    datasets.rs         # Datasets API client (upload, stream, version)
    endpoints.rs        # Endpoints API client (create, manage, delete)
    autotrain.rs        # AutoTrain API client (create job, poll status, download)
    episode_transform.rs  # Transform .roko/episodes.jsonl into training data format
    config.rs           # HF API token, endpoint URLs, defaults
  Cargo.toml
```

Each module wraps the corresponding HF API with typed Rust clients. The crate depends on `reqwest` for HTTP and `serde` for JSON serialization. No HuggingFace SDK dependency -- the APIs are stable REST endpoints.

---

## 19. SWE-bench and arena integration

### Arenas as packages

Arena definitions are a first-class package type (section 3, Tier 3). This means arenas install, update, and remove through the same `roko install` command as extensions and profiles:

```bash
# Install a SWE-bench arena
roko install crate:roko-arena-swe-bench

# Install a HumanEval arena
roko install crate:roko-arena-humaneval

# Install a custom domain-specific arena
roko install git:github.com/org/roko-arena-defi-audit
```

The arena manifest declares the `Arena` trait implementation, scoring function, and dataset source:

```toml
[package.metadata.roko]
type = "arena"
arena_name = "swe-bench"
dataset_source = "princeton-nlp/SWE-bench_Lite"
scoring = "pass@1"
roko_version = ">=0.5.0"
abi_version = 1
```

### The arena package type alongside others

The package taxonomy includes six types. Arenas sit alongside extensions, skills, profiles, chain connectors, and InsightStore modules:

| Package type | What it provides | Install example |
|-------------|-----------------|-----------------|
| Extension | Tools and hooks | `roko install crate:roko-ext-defi` |
| Skill | Markdown knowledge | `roko install npm:@pi/code-review` |
| Domain profile | Agent configuration | `roko install crate:roko-profile-quant` |
| Chain connector | Blockchain integration | `roko install crate:roko-chain-solana` |
| InsightStore module | Knowledge query strategy | `roko install crate:roko-insight-defi` |
| **Arena** | **Benchmark protocol** | **`roko install crate:roko-arena-swe-bench`** |

### The perpetual grinder pattern

Arenas are not one-shot benchmarks. The `roko bench` command supports continuous, repeating evaluation:

```bash
# Run SWE-bench once
roko bench arena --name swe-bench

# Run SWE-bench continuously (repeat=0 means infinite)
roko bench arena --name swe-bench --repeat 0

# Run with a specific model arm
roko bench arena --name swe-bench --model claude-opus-4-6 --repeat 10

# Run across all installed arenas
roko bench arena --all --repeat 0
```

The perpetual grinder (`--repeat 0`) runs indefinitely:

1. Pick a problem from the arena's dataset.
2. Dispatch an agent to solve it.
3. Score the result using the arena's scoring function.
4. Log the episode (prompt, response, score, latency, model used).
5. Update the CascadeRouter arm posterior for the model that solved it.
6. Loop.

This pattern produces three outputs simultaneously:
- **Benchmark scores** that track agent capability over time.
- **Training data** for the HuggingFace fine-tuning loop (section 18). Every grinder episode is a training row.
- **CascadeRouter calibration** data. The router learns which models perform best on which arena problem types.

The grinder runs as a background process managed by the ProcessSupervisor. It yields to higher-priority work (user tasks, plan execution) and resumes during idle time.

---

## 20. Integration with prior PRDs

### PRD-01: Platform overview and ecosystem growth

The package system is the growth engine. Every package -- extension, skill, profile, arena, chain connector -- is a unit of ecosystem expansion. More packages means more capable agents. More capable agents produce better outcomes. Better outcomes attract more contributors who publish more packages. The flywheel spins on three axes:

- **Horizontal growth.** Each new extension adds capabilities (DeFi monitoring, security scanning, research synthesis). More capabilities means agents handle more task types.
- **Vertical growth.** Each new chain connector adds data sources. Each new arena adds evaluation depth. Agents get both broader and deeper.
- **Cross-pollination.** A chain connector published by one team feeds data to extensions published by another team. Neither team coordinated. The package system's composition model creates emergent integrations.

PRD-01's vision of a self-improving ecosystem depends on the package system to lower the barrier from "modify the monorepo" to "publish a crate."

### PRD-02: Agent runtime

Chain actors are managed by the `ProcessSupervisor`. When the heartbeat pipeline's provisioning phase detects blockchain capabilities in the composed profile, it calls `spawn_chain_actors()` to start one actor per configured chain. The supervisor monitors actor health and restarts crashed actors.

The WorldGraph is agent-level state stored in `CorticalState`. Extensions access it through the `ExtensionContext`. The graph persists across heartbeat ticks and survives agent restarts through checkpoint serialization.

### PRD-03: Cognitive engine and inference routing

The foraging model feeds the prediction error computation. When an entity's free energy spikes, the cognitive engine observes higher prediction error on chain-related probes, which may escalate the tier from T0 to T1 or T2. Conversely, when the WorldGraph stabilizes and free energy drops across all entities, more ticks resolve at T0.

R0 processing is always T0. The fast-path detectors run pure Rust pattern matching with no LLM involvement. R1 processing is T0 or T1. R2 and R3 may escalate to T2 for strategic decisions.

The inference gateway routes by intent. When a foraging model decides an entity needs deeper analysis, the gateway selects the model best suited to the task type (code analysis, rate prediction, research synthesis) using CascadeRouter's Thompson Sampling posteriors. HuggingFace fine-tuned models (section 18) appear as CascadeRouter arms alongside Claude, GPT, and open-weight models.

### PRD-04: Context engineering and VCG auction

The WorldGraph injects context through the VCG auction (section 15, context injection). The `worldgraph` context category competes with other categories (code intelligence, knowledge entries, task description) for budget in the context window. The foraging model's strategy relevance scores determine the bid value.

WorldGraphBidder participates as the 9th VCG auction bidder (after the 8 standard bidders: task, code, knowledge, playbook, episode, research, neuro, affect). Its bid value scales with the combined free energy of WorldGraph entities -- high free energy means the worldview is uncertain, which means context about observed entities is more valuable for the next inference.

InsightStore bidder uses chain queries from the multi-chain architecture to fetch relevant knowledge entries. When a ChainActor detects a contract interaction matching an InsightStore query, the bidder's valuation increases for that context slot.

### PRD-05: Knowledge and stigmergy

WorldGraph entities become InsightStore entries through PP-HDC encoding (section 14, layer 5). Each entity's behavioral pattern is encoded as a 10,240-bit hyperdimensional vector and posted to the InsightStore. Other agents query these vectors to discover entities they have not observed directly.

The PP-HDC 7-layer defense pipeline (permutation-projection hyperdimensional computing) encodes WorldGraph entities as vectors that preserve semantic similarity while resisting adversarial manipulation. Layer 1 (binding) combines entity attributes. Layer 2 (bundling) aggregates temporal observations. Layers 3-5 (protection) apply random permutations and projections to prevent reverse engineering. Layers 6-7 (verification) compute similarity checksums for integrity validation.

The dream consolidation cycle (section 15, strategy evolution) promotes patterns discovered in the WorldGraph to semantic memory entries in the Neuro store. A recurring cross-chain price divergence becomes a stored `Heuristic` that the agent can retrieve without re-discovering the pattern.

### PRD-06: Domains and arenas

Multi-domain composition (section 8) extends PRD-06's `DomainProfile` system. The `ComposedProfile` struct wraps multiple `FullDomainProfile` instances and merges them using configurable strategies. The runtime sees a flat profile after `flatten()` -- no changes needed to the heartbeat pipeline.

The multi-chain architecture (section 9) is how the blockchain domain profile works in practice. PRD-06 described the blockchain profile's tick timing and event subscriptions. This PRD specifies the machinery that produces those events.

Arenas are installable packages (section 19). This connects PRD-06's arena framework to the package system. Arena definitions ship as crates that implement the `Arena` trait. Any Roko instance can install any published arena and run it locally. The perpetual grinder pattern produces training data that feeds back into CascadeRouter calibration.

### PRD-07: ISFR and instruments

The ISFR oracle is one specific chain actor reading from the Korai chain. ISFRUpdate is a variant of EventPayload (section 9). When the ISFR precompile publishes a new rate, the ChainActor for Korai emits a CanonicalEvent with `EventPayload::ISFRUpdate`, which the heartbeat pipeline processes like any other chain event.

Multi-chain ISFR (reading rates from multiple chains for cross-chain benchmarking) is a natural application of the actor-per-chain model. Each chain actor reads that chain's lending rates (Aave on Base, Compound on Arbitrum). The aggregation happens in the TemporalAggregator (section 10) at R2 resolution.

ClearingInsights from cooperative clearing rounds flow into the WorldGraph as market entity updates. The WorldGraph accumulates clearing price history, surplus values, and imbalance ratios for the ISFR perpetual market entity. This data feeds the foraging model and the context injection layer.

### PRD-08: User experience

The `roko install` command (section 4) is the primary package management surface. The package browser TUI (`roko market`, section 17) provides visual discovery. Persistent chat (`roko chat`) shows WorldGraph state alongside conversation, giving users real-time visibility into what the agent observes across chains.

The HuggingFace model browser integrates with `roko market`: users can discover and install community fine-tuned models alongside extensions and profiles.

---

## 21. Synergistic scaling properties

The components described in this PRD and across the PRD set do not scale independently. They amplify each other. Each new dimension of growth creates cross-connections with every other dimension, producing superlinear returns.

### Packages amplify everything

More packages means more extensions. More extensions means each agent can handle more task types. More task types means more episodes. More episodes means more training data. More training data means better fine-tuned models. Better models means higher arena scores. Higher arena scores means more users. More users means more package authors.

The critical transition: when the package ecosystem reaches ~50 published packages across 3+ types (extensions, profiles, chain connectors), a new Roko instance can bootstrap a capable agent without writing any custom code. The package ecosystem becomes the default way to configure agents, not an add-on.

### Chain connectors amplify WorldGraph

Each new chain connector adds a data source to the multi-chain architecture. More data sources means the WorldGraph discovers more entities. More entities means richer context for every agent, regardless of which chains they primarily monitor.

A Solana chain connector does not help only Solana-focused agents. It adds Solana DEX entities to the WorldGraph, which Ethereum-focused agents can query when they detect cross-chain arbitrage patterns. The WorldGraph is a shared substrate -- every chain connector enriches it for all agents.

### Arenas amplify model quality

More arenas means more diverse evaluation. More diverse evaluation means the CascadeRouter's Thompson Sampling posteriors have finer resolution -- the router learns not only which model is best overall, but which model is best for code review vs. DeFi analysis vs. security auditing vs. research synthesis.

More arenas also means more training data for the HuggingFace fine-tuning loop. Each arena produces episodes with structured scoring. These scored episodes are high-quality training rows because the scoring function provides an objective quality signal (pass/fail, CRPS, arena-specific metrics). A Roko instance running 5 arenas generates 5x the training data of one running a single arena, and the diversity of that data produces more generalizable fine-tuned models.

### Agents amplify ISFR

More agents means more CRPS predictions. More predictions means tighter calibration statistics. Tighter calibration means more institutional confidence in ISFR as a benchmark. More institutional confidence means more clearing volume. More clearing volume means more surplus for solvers, which attracts more solvers, which improves clearing quality, which attracts more traders.

The epistemic reputation system creates a quality ratchet: agents with better CRPS scores get more knowledge access (higher query quotas), which makes their predictions better, which raises their scores further. Bad predictors lose access, reducing noise. The system self-selects for quality.

### Knowledge creates O(N^2) cross-connections

Each of the five dimensions -- packages, chains, arenas, agents, knowledge -- creates connections with every other dimension:

| Dimension A | Dimension B | Cross-connection |
|-------------|-------------|------------------|
| Packages | Chains | A new chain connector package adds multi-chain data for all installed extensions |
| Packages | Arenas | A new arena package provides evaluation for all installed model arms |
| Packages | Agents | A new extension package adds capabilities to all agents using that domain profile |
| Packages | Knowledge | A new InsightStore module package improves knowledge queries for all agents |
| Chains | Arenas | Multi-chain data improves performance on cross-chain arenas |
| Chains | Agents | More chain data means richer WorldGraph context for all agents |
| Chains | Knowledge | Chain events become InsightStore entries available to all agents |
| Arenas | Agents | More arena scores mean better CascadeRouter calibration for all task types |
| Arenas | Knowledge | Arena episodes become training data and InsightStore entries |
| Agents | Knowledge | More agents produce more episodes, more episodes produce more knowledge, more knowledge improves all agents |

With N dimensions and M items per dimension, the system has O(N^2 * M) cross-connections. Adding one item to any dimension creates connections with items in every other dimension. This is why the system's value grows faster than the sum of its parts.

### The compounding loop

The full compounding loop, traced from a single starting point:

1. A contributor publishes a new chain connector for Arbitrum.
2. Roko instances install it. Their ChainActors now ingest Arbitrum events.
3. WorldGraphs across instances discover new entities on Arbitrum.
4. Foraging models allocate attention to high-value Arbitrum sources.
5. Agents produce episodes involving Arbitrum data.
6. Episodes feed the HuggingFace fine-tuning loop.
7. Fine-tuned models improve at Arbitrum-related tasks.
8. CascadeRouter learns to route Arbitrum tasks to the fine-tuned model.
9. Better Arbitrum task performance attracts more users with Arbitrum use cases.
10. More users means more episodes, which feeds step 6 again.
11. One of those users publishes an Arbitrum-specific extension package.
12. The extension package is available to all instances, returning to step 2.

Each pass through this loop produces a system that is measurably better at the task type the new component introduced. The improvement compounds because every pass adds training data, model calibration, WorldGraph entities, and package options that persist across future passes.

---

## 22. Implementation phasing

### Phase 1: Package ecosystem foundation

**Crates**: `roko-ext-registry`
**Duration**: 2-3 weeks

- Package manifest parsing (`[package.metadata.roko]` and `package.json`)
- `roko install` for Cargo crates (dynamic loading via `libloading`)
- `roko install` for local paths
- `roko remove`, `roko list`
- Lockfile generation and reproducible installs
- Extension chain assembly with installed extensions
- Skill and prompt discovery and indexing

### Phase 2: Pi compatibility

**Crates**: `roko-quickjs`
**Duration**: 2-3 weeks

- QuickJS integration via `rquickjs`
- Pi API bridge (`pi.registerTool`, `pi.on`)
- `roko install` for npm packages
- Security sandbox (memory limits, time limits, capability checks)
- Tool registration bridge (JS tools callable from Rust agents)

### Phase 3: Multi-domain agents

**Crates**: extend `roko-core`
**Duration**: 1-2 weeks

- `ComposedProfile` struct and merge strategies
- `--profile a,b` CLI flag
- Profile composition in `roko.toml`
- Tests for Union and PrimarySecondary merge behaviors

### Phase 4: Multi-chain ingestion

**Crates**: `roko-chain-ingest`
**Duration**: 3-4 weeks

- `ChainConnector` trait
- `EvmConnector` implementation via Alloy
- `ChainActor` struct and main loop
- Reorg detection and handling
- `CanonicalEvent` schema and `CanonicalEventBus`
- R0 fast-path with built-in detectors
- Hierarchical temporal aggregation (R1-R3)
- `FinalityTracker` for downstream reorg handling
- Integration with ProcessSupervisor

### Phase 5: Predictive foraging

**Crates**: `roko-foraging`
**Duration**: 2-3 weeks

- `GittinsIndex` computation
- `ForagingModel` with attention allocation
- Active inference augmentation (`ActiveInferenceAttention`)
- Integration with chain actors (attention budget -> event filtering)
- Patch switching (Marginal Value Theorem)
- Configuration in domain profiles

### Phase 6: WorldGraph and contract discovery

**Crates**: `roko-worldgraph`
**Duration**: 3-4 weeks

- `WorldGraph` struct with entity and relationship management
- Five-layer contract discovery pipeline (ERC-165, selectors, bytecode, patterns, factory)
- Layer 5 stigmergy (InsightStore integration)
- Context injection via VCG bidding
- HDC fingerprint computation for worldview sharing
- Dream cycle integration (strategy evolution from WorldGraph patterns)

### Phase 7: Ecosystem completion

**Crates**: extend `roko-cli`
**Duration**: 2-3 weeks

- `roko publish` command
- `roko search` command
- `roko market` TUI browser
- Roko-enhanced API surface (Tier 2 JS hooks)
- Registry backend (Kellnr-based or custom)
- Documentation and example packages

---

## 23. References

1. Charnov, E.L. (1976). "Optimal Foraging, the Marginal Value Theorem." *Theoretical Population Biology*, 9(2), 129-136.

2. Gittins, J.C. & Jones, D.M. (1974). "A Dynamic Allocation Index for the Sequential Design of Experiments." In *Progress in Statistics*, 241-266.

3. Frazier, P. & Yu, A.J. (2013). "Sequential Hypothesis Testing under Stochastic Deadlines." *Proceedings of the Allerton Conference on Communication, Control, and Computing*.

4. Friston, K. (2010). "The Free-Energy Principle: A Unified Brain Theory?" *Nature Reviews Neuroscience*, 11(2), 127-138.

5. Thompson, W.R. (1933). "On the Likelihood that One Unknown Probability Exceeds Another in View of the Evidence of Two Samples." *Biometrika*, 25(3-4), 285-294.

6. Whittle, P. (1988). "Restless Bandits: Activity Allocation in a Changing World." *Journal of Applied Probability*, 25(A), 287-298.

7. Pirolli, P. & Card, S.K. (1999). "Information Foraging." *Psychological Review*, 106(4), 643-675.

8. EIP-165: Standard Interface Detection. Ethereum Improvement Proposals. https://eips.ethereum.org/EIPS/eip-165

9. Hu, S. et al. (2024). "Esim: Efficient Smart Contract Similarity Detection via Semantic Similarity Graph." arXiv:2511.12971.

10. Huang, T. et al. (2021). "Hunting Vulnerable Smart Contracts via Graph Neural Networks." arXiv:2106.15497.

11. Chen, S. et al. (2025). "ChronoWave-GNN: A Multi-Scale Temporal Framework for Blockchain Transaction Pattern Analysis." *Nature*.

12. Li, Z. et al. (2025). "Autonomous Agents on Blockchains: A Comprehensive Survey." arXiv:2601.04583.

13. Da Costa, L. et al. (2025). "Orchestrator: Multi-Agent Active Inference." arXiv:2509.05651.

14. Chainlink. "CCIP Lane Architecture." Chainlink Documentation.

15. Apache Flink. "Event Time and Watermarks." Apache Flink Documentation.

16. Kahaneman, D. (2011). *Thinking, Fast and Slow*. Farrar, Straus and Giroux.

17. Surowiecki, J. (2004). *The Wisdom of Crowds*. Doubleday.

18. Lee, Y. et al. (2026). "Meta-Harness: Optimizing LLM Scaffolding Generalizes Across Models." arXiv:2603.28052.

19. Sumers, T.R. et al. (2023). "Cognitive Architectures for Language Agents." arXiv:2309.02427.

20. TRM Labs. "Multi-Chain Ingestion Architecture." TRM Labs Engineering Blog.
