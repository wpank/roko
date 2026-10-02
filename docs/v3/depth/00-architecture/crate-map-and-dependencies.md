# 00-ARCH -- Crate Map and Dependencies

> **Parent**: [00-ARCHITECTURE](../../00-ARCHITECTURE.md)
>
> Roko is a 39-member Cargo workspace (~1M LOC, 10,300+ tests). This depth file
> catalogs every active crate with lines of code, test count, dependency tier,
> status, and inter-crate dependency edges. Written fresh for v3 on 2026-09-15.

---

## 1. Workspace Overview

| Metric | Value |
|---|---|
| Workspace members | 39 |
| Total Rust LOC (src/) | ~1,010,000 |
| Total `#[test]` functions | ~10,300 |
| Minimum rustc version | 1.91 (alloy dependency) |
| Green release checkpoint rustc | 1.96.1 (2026-08-16) |
| Accepted epics | 48/48 |
| Executable tasks complete | Withdrawn 2026-09-29: stale count (`work/history/claude-md-status-2026-09-28.md`) |

---

## 2. Dependency Tiers

Crates are organized into five tiers based on their workspace-internal dependency
depth. The invariant is strict: no crate depends on a crate in a higher tier.

| Tier | Description | Member count |
|---|---|---|
| **T0 -- Leaf** | No workspace dependencies | 5 |
| **T1 -- Foundation** | Depends only on T0 crates | 7 |
| **T2 -- Service** | Depends on T0 and T1 | 9 |
| **T3 -- Integration** | Depends on T0-T2 | 9 |
| **T4 -- Application** | Depends on T0-T3 | 8+ |

---

## 3. Complete Crate Catalog

### 3.1 T0 -- Leaf Crates

No workspace dependencies. These are the foundation atoms.

| Crate | Path | LOC | Tests | Status | Description |
|---|---|---|---|---|---|
| roko-primitives | `crates/roko-primitives/` | ~5,400 | 142 | Stable | HDC vectors (10,240-bit), tier routing, binary fingerprints |
| roko-mcp-stdio | `crates/roko-mcp-stdio/` | ~250 | 2 | Stable | MCP stdio transport adapter |
| roko-demo | `crates/roko-demo/` | ~5,800 | 6 | Built | Demo/example binary for showcasing features |
| speed-test | `demo/speed-test/` | -- | -- | Dev | Speed benchmarking tool |
| roko-chain-watcher | `apps/roko-chain-watcher/` | -- | -- | Stub | Chain event watcher application |

### 3.2 T1 -- Foundation Crates

Depend only on T0 crates. Provide the kernel contracts, language support, and MCP
integrations.

| Crate | Path | LOC | Tests | Status | Description |
|---|---|---|---|---|---|
| roko-core | `crates/roko-core/` | ~96,000 | 1,849 | Kernel, stable | Signal + 12 traits (Store, ColdStore, Score, Verify, Route, Compose, React, Bus, Observe, Connect, Trigger, Substrate), types, config (E42 8/8), tools, errors. 139 items encapsulated as pub(crate) |
| roko-lang-rust | `crates/roko-lang-rust/` | ~1,400 | 46 | Built | Rust language support for code intelligence |
| roko-lang-typescript | `crates/roko-lang-typescript/` | ~940 | 33 | Built | TypeScript language support |
| roko-lang-go | `crates/roko-lang-go/` | ~670 | 25 | Built | Go language support |
| roko-mcp-github | `crates/roko-mcp-github/` | ~4,300 | 39 | Partial | GitHub MCP integration |

### 3.3 T2 -- Service Crates

Depend on T0 and T1. Provide the main domain services, tool definitions, and
computation engines.

| Crate | Path | LOC | Tests | Status | Description |
|---|---|---|---|---|---|
| roko-agent | `crates/roko-agent/` | ~116,600 | 1,721 | Wired | 12 LLM provider kinds (AnthropicApi, ClaudeCli, CodexCli, OpenAiCompat, CursorAcp, CursorCli, PerplexityApi, GeminiApi, GeminiCli, CerebrasApi, Hermes, OpenClaw), pools, MCP, tool loop, safety |
| roko-chain | `crates/roko-chain/` | ~29,800 | 340 | Local tranche | Optional chain client, transferable identity/delegation, challengeable knowledge, TraceRank, local registry, marketplace, arena, DeFi state machines |
| roko-daimon | `crates/roko-daimon/` | ~8,600 | 104 | Wired | Affect engine, PAD vector, somatic markers, 6 behavioral states, dispatch modulation |
| roko-eval | `crates/roko-eval/` | ~2,200 | 29 | Built | Benchmark evaluation harness (SWE-bench, custom) |
| roko-fs | `crates/roko-fs/` | ~12,000 | 158 | Stable | FileSubstrate (JSONL), GC, layout, rotation-bounded generations |
| roko-graph | `crates/roko-graph/` | ~27,000 | 400 | Sole engine | Graph engine (#260 default, #276 retired WorkflowEngine), DAG cells, topology, cost state, bounded parallel waves, conditional routing, immune decision Graph, resume-durable schema-v2 |
| roko-plugin | `crates/roko-plugin/` | ~5,800 | 92 | 8/8 (E32) | Plugin manifests, signed dependency graphs, bounded typed WASM hooks, strict admission, verified relay/install, canonical tier/capability policy |
| roko-std | `crates/roko-std/` | ~11,000 | 218 | Partial | 35 default tool definitions (16 executable local + 19 GitHub MCP); 52 with typed optional-chain placeholders |
| roko-index | `crates/roko-index/` | ~5,800 | 88 | Built | Parser + graph + HDC indexing for code intelligence |

### 3.4 T3 -- Integration Crates

Depend on T0-T2. Provide prompt assembly, verification, learning, knowledge
management, and runtime infrastructure.

| Crate | Path | LOC | Tests | Status | Description |
|---|---|---|---|---|---|
| roko-compose | `crates/roko-compose/` | ~33,000 | 419 | Wired | Prompt assembly, 11 role templates, 9-layer SystemPromptBuilder, E44 cross-cut functors (4 functors, 6 natural transforms), VCG attention auction, EnrichedCell |
| roko-conductor | `crates/roko-conductor/` | ~10,800 | 320 | Wired | 12 watchers, circuit breaker, diagnosis. E33 telemetry producer |
| roko-dreams | `crates/roko-dreams/` | ~14,700 | 89 | Wired | Offline consolidation: hypnagogia, imagination, NREM/REM/integration cycle, resident daemon scheduling (idle/cron/episode-count triggers) |
| roko-gate | `crates/roko-gate/` | ~30,100 | 546 | Wired | 19 gates, 7-rung pipeline, adaptive EMA thresholds, enriched rung inputs |
| roko-gateway | `crates/roko-gateway/` | ~5,200 | 26 | Complete (E26 12/12) | Nine-stage inference gateway: routing/fallback, exact/semantic caches, tool/output/thinking controls, convergence, cost accounting, key rotation, backpressure, batches |
| roko-learn | `crates/roko-learn/` | ~74,400 | 954 | Wired | Episodes, playbooks, bandits, model routing, A/B experiments, efficiency events, HDC consolidation, hindsight adjustments, c-factor governance, significance/early stopping, Variance Inequality, autocatalytic metrics |
| roko-neuro | `crates/roko-neuro/` | ~22,700 | 243 | Wired | Durable knowledge store, distillation, tier progression, temporal query/GC, cross-domain transfer |
| roko-runtime | `crates/roko-runtime/` | ~32,400 | 285 | Wired | ProcessSupervisor, event bus, cancellation, workflow contract, agent lifecycle |
| roko-mcp-code | `crates/roko-mcp-code/` | ~2,300 | 16 | Wired | Code-intelligence MCP server for editor integration |

### 3.5 T4 -- Application Crates

Depend on T0-T3. These are the user-facing entry points, HTTP surfaces, and
integration test harnesses.

| Crate | Path | LOC | Tests | Status | Description |
|---|---|---|---|---|---|
| roko-cli | `crates/roko-cli/` | ~279,400 | 3,109 | Main entry point | CLI commands, plan DAG/runner, Graph execution, merge queue, worktree manager, ratatui TUI (F1-F10), chat, research, status, doctor, dashboard |
| roko-serve | `crates/roko-serve/` | ~107,400 | 462 | Wired | HTTP control plane: REST routes (counts in `tools/http_route_inventory.snapshot.json`) + SSE + WebSocket on :6677 |
| roko-acp | `crates/roko-acp/` | ~22,100 | 120 | 8/8 (E17) | ACP (Agent Client Protocol) server for Cursor/external editor integration. 180 ACP tests pass |
| roko-agent-server | `crates/roko-agent-server/` | ~7,700 | 21 | Wired | Per-agent HTTP sidecar: 14 routes including `/message` (real LLM dispatch), `/stream` WS, `/predictions`, `/research`, `/tasks` |
| roko-execution | `crates/roko-execution/` | ~12,000 | 239 | Wired | RuntimeServices builder (#243), diagnostic service, execution control, feedback settlement. Profile-driven shared service facade for CLI/serve/ACP |
| roko-tests | `tests/` | -- | -- | CI | Cross-crate integration tests |
| agent-relay | `apps/agent-relay/` | -- | -- | Wired (E29) | Supervised HTTP JSON relay adapter, bounded canonical-envelope delivery, atomic cursor restore, ACK-after-durable subscription |
| mirage-rs | `apps/mirage-rs/` | -- | -- | Built | In-process chain simulation tool |

---

## 4. Dependency Graph

### 4.1 Simplified dependency DAG

```
roko-primitives (T0)
    |
    v
roko-core (T1) -----------+-------+-------+-------+-------+
    |                      |       |       |       |       |
    v                      v       v       v       v       v
roko-agent (T2)       roko-fs  roko-graph roko-chain roko-daimon roko-plugin
    |                      |       |                              |
    v                      v       v                              v
roko-compose (T3) <-- roko-neuro <-- roko-learn <---------- roko-std
    |                                    |
    v                                    v
roko-gate   roko-runtime   roko-dreams  roko-gateway
    |              |            |
    +-------+------+------+-----+
            |             |
            v             v
       roko-execution  roko-conductor
            |
            v
    roko-cli    roko-serve    roko-acp    roko-agent-server
```

### 4.2 Key dependency edges

| From | To | Purpose |
|---|---|---|
| roko-core -> roko-primitives | HDC vectors, binary fingerprints |
| roko-agent -> roko-core | Signal types, provider traits, tool definitions |
| roko-agent -> roko-std | Default tool registry |
| roko-compose -> roko-agent | Safety contracts for SafetyFunctor (E44) |
| roko-compose -> roko-neuro | KnowledgeStore for MemoryFunctor (E44) |
| roko-compose -> roko-daimon | DaimonState for DaimonFunctor (E44) |
| roko-compose -> roko-dreams | DreamCycleReport for DreamsFunctor (E44) |
| roko-compose -> roko-learn | CascadeRouter for dream routing |
| roko-learn -> roko-daimon | Affect-coupled learning loops (E25) |
| roko-dreams -> roko-neuro | Knowledge consolidation |
| roko-graph -> roko-core | Cell/Graph primitives |
| roko-cli -> (most crates) | Application-level integration |
| roko-serve -> (most crates) | HTTP control plane integration |

---

## 5. LOC Distribution

```
roko-cli          279,400  (27.6%)  ████████████████████████████
roko-agent        116,600  (11.5%)  ████████████
roko-serve        107,400  (10.6%)  ███████████
roko-core          96,000   (9.5%)  ██████████
roko-learn         74,400   (7.4%)  ████████
roko-compose       33,000   (3.3%)  ████
roko-runtime       32,400   (3.2%)  ████
roko-gate          30,100   (3.0%)  ███
roko-chain         29,800   (2.9%)  ███
roko-graph         27,000   (2.7%)  ███
roko-neuro         22,700   (2.2%)  ███
roko-acp           22,100   (2.2%)  ███
roko-dreams        14,700   (1.5%)  ██
roko-execution     12,000   (1.2%)  ██
roko-fs            12,000   (1.2%)  ██
roko-std           11,000   (1.1%)  ██
roko-conductor     10,800   (1.1%)  ██
Others (22 crates) ~58,000  (5.7%)  ██████
```

The top four crates (roko-cli, roko-agent, roko-serve, roko-core) account for 59.2%
of all code. roko-cli alone is 27.6% -- this reflects its role as the primary
application assembly point containing the runner event loop, plan execution, TUI, and
all CLI command implementations.

---

## 6. Test Distribution

```
roko-cli           3,109  (30.1%)  ████████████████
roko-core          1,849  (17.9%)  ██████████
roko-agent         1,721  (16.7%)  █████████
roko-learn           954   (9.2%)  █████
roko-gate            546   (5.3%)  ███
roko-serve           462   (4.5%)  ███
roko-compose         419   (4.1%)  ███
roko-graph           400   (3.9%)  ██
roko-chain           340   (3.3%)  ██
roko-conductor       320   (3.1%)  ██
roko-runtime         285   (2.8%)  ██
roko-neuro           243   (2.4%)  ██
roko-execution       239   (2.3%)  ██
roko-std             218   (2.1%)  ██
roko-primitives      142   (1.4%)  █
roko-acp             120   (1.2%)  █
roko-daimon          104   (1.0%)  █
roko-plugin           92   (0.9%)  █
roko-dreams           89   (0.9%)  █
roko-index            88   (0.9%)  █
Others              ~380   (3.7%)  ██
```

---

## 7. Architectural Invariants

1. **No upward dependencies**: T0 crates never depend on T1+. T1 never depends on
   T2+. This is the foundational invariant of the workspace.

2. **roko-core is the hub**: Every workspace crate except roko-primitives and leaf
   utilities depends on roko-core, directly or transitively.

3. **roko-primitives is the leaf**: Contains only HDC operations and basic numeric
   types. No workspace dependencies.

4. **Application crates are sinks**: roko-cli and roko-serve depend on nearly
   everything but nothing depends on them.

5. **Feature gating**: Optional dependencies (e.g., `hdc` feature in roko-compose,
   chain features in roko-std) keep the default compilation lean.

6. **Cross-cuts stay cross-cut**: roko-neuro, roko-daimon, and roko-dreams do not
   define new kernel types. They inject behavior through the E44 functor model.

7. **Graph is the sole engine**: `roko-graph` is the only execution engine since
   #260 (default) and #276 (retired WorkflowEngine). Runner-v2 retained as
   `--engine legacy` for one release cycle.

---

## 8. Stability Tiers

| Tier | Stability | Examples |
|---|---|---|
| **Core** | Semver-major-only breaks | roko-core, roko-primitives |
| **Extended** | Minor-version breaks with notice | roko-compose, roko-gate, roko-learn, roko-std |
| **Experimental** | Anything goes behind feature flags | chain features, dream scheduling, host-specific boundaries |

---

## Cross-References

- [00-ARCHITECTURE](../../00-ARCHITECTURE.md) -- Parent chapter with key crate table
- `Cargo.toml` (workspace root) -- Canonical member list
- `crates/roko-core/Cargo.toml` -- Foundation crate dependencies
- [design-principles-frontier-summary.md](design-principles-frontier-summary.md) -- P1 composition principles
