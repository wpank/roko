# Crate Map

> Authoritative mapping of Roko's architectural concepts to their implementing crates.
> Every shipping or built crate is listed with its status, responsibility, known test count,
> and layer position. Test counts and LOC figures are as of 2026-04-17 per
> [`status/status.md`](../status/status.md).

**Status**: Written (conceptual map — see `operations/` for workspace layout)
**Crate**: — (cross-crate reference document)
**Depends on**: [`status/vision.md`](../status/vision.md), [`GLOSSARY.md`](../GLOSSARY.md)
**Last reviewed**: 2026-04-17

---

## TL;DR

Roko's workspace has 36 members (~322K Rust LOC, 3,761 tests as of 2026-04-17). The crates
organize into five layers (L0–L4) with strictly downward dependencies, plus three cross-cut
crates that span multiple layers via trait injection.

---

## Layer Map Overview

```
L4 Orchestration     roko-orchestrator    roko-cli
L3 Harness           roko-conductor       roko-chain        roko-std      roko-serve
L2 Scaffold          roko-compose         roko-learn        roko-daimon   roko-dreams
L1 Framework         roko-agent           roko-gate         roko-fs       roko-neuro
L0 Runtime           roko-core            roko-runtime
```

Cross-cuts (inject into multiple layers via trait objects):
```
roko-neuro     (knowledge)   L1, injected into L2/L3/L4
roko-daimon    (affect)      L2, injected into L3/L4
roko-dreams    (consolidation) L2, offline — no injection path yet
```

---

## L0 — Runtime Layer

The kernel. Every other crate depends on these two.

| Crate | Status | Responsibility | Tests (as of 2026-04-17) | Dependents |
|---|---|---|---|---|
| `roko-core` | **Shipping** | `Engram` type + 6 Synapse traits (`Substrate`, `Scorer`, `Gate`, `Router`, `Composer`, `Policy`), `Score`, `Decay`, `Provenance`, `ContentHash`, `Kind`, `Body`, `HDC` fingerprint, `Taint`, `Attestation` | 376 | All crates |
| `roko-runtime` | **Shipping** | `ProcessSupervisor`, `EventBus<E>`, cancellation tokens, async task lifecycle | — (as of 2026-04-17 per status.md) | L1–L4 crates |

### `roko-core` — Detail

`roko-core` is the kernel. It defines the two mediums and the six operator traits, but contains
no implementations — only the trait definitions and the `Engram` data type. This is a deliberate
design constraint: the kernel must remain implementation-free so that multiple backends can
be plugged in without depending on each other.

Key types:
- `Engram` — content-addressed durable record
- `Score` — 7-axis appraisal (confidence, novelty, utility, reputation + precision, salience, coherence)
- `Decay` — four variants (balance/demurrage, reinforcement, novelty weighting, cold-tier freeze/thaw)
- `Provenance` — audit context
- `ContentHash` — BLAKE3-derived identity
- `Kind` — semantic category enum
- `Body` — payload variant enum
- `Taint` — one-way contamination flag
- `Attestation` — trust level enum (LocalAgent → ChainWitness)

### `roko-runtime` — Detail

`roko-runtime` provides the infrastructure plumbing that all other crates share: process
supervision, the live `EventBus<E>` transport (to be superseded by `Bus`), cancellation
token propagation, and async task lifecycle management. It has no business logic.

---

## L1 — Framework Layer

Implementations of the core traits. These crates depend on L0 and nothing else above them.

| Crate | Status | Responsibility | Tests (as of 2026-04-17) | Dependents |
|---|---|---|---|---|
| `roko-agent` | **Shipping** | 5 LLM backends, `CascadeRouter` (Static → Confidence → UCB), MCP tool integration, 7-step safety pipeline | 346 | L2–L4 crates |
| `roko-gate` | **Shipping** | 11-gate, 7-rung pipeline (Compile → Lint → Test → Symbol → GeneratedTest → PropertyTest → Integration), adaptive thresholds, process reward models, forensic replay | 200 | L2–L4 crates |
| `roko-fs` | **Shipping** | `FileSubstrate` (JSONL), garbage collection | 37 | `roko-orchestrator`, `roko-cli` |
| `roko-neuro` | **Built** | 6 knowledge types × 4 validation tiers, HDC 10,240-bit encoding, sub-millisecond similarity search | — (as of 2026-04-17 per status.md) | `roko-learn`, `roko-conductor` |

### `roko-agent` — Detail

The L1 LLM interface crate. Its `CascadeRouter` is Roko's implementation of the
FrugalGPT cascade principle: try cheap models first, escalate to expensive ones only when
confidence thresholds are not met. UCB (Upper Confidence Bound) bandits learn per-task
model performance over time.

LLM backends:
1. Claude CLI (subprocess)
2. Anthropic API (HTTP)
3. OpenAI-compatible API (HTTP)
4. Cursor ACP (IDE integration)
5. Ollama (local models)

### `roko-gate` — Detail

The L1 verification crate. Its 11-gate pipeline applies strictly in order within each rung.
Gate failure is a verdict, not an error: a failing gate terminates the pipeline at the
minimum necessary rung. Adaptive thresholds use EMA over observed pass rates to avoid
both under- and over-gating.

Gate pipeline stages:
1. Compile — code compiles
2. Lint — passes `clippy` and style checks
3. Test — existing test suite passes
4. Symbol — referenced symbols exist
5. GeneratedTest — generated tests exercise the change
6. PropertyTest — property-based tests pass
7. Integration — integration tests pass

### `roko-neuro` — Detail

The L1 knowledge crate. Not yet wired to the runtime. Implements:

- 6 knowledge types: Insight, Heuristic, Warning, CausalLink, StrategyFragment, AntiKnowledge
- 4 validation tiers: Transient → Working → Consolidated → Persistent
- HDC encoding: 10,240-bit binary vectors, Hamming distance similarity, sub-millisecond search

The HDC encoding enables similarity search without neural network inference — a significant
operational advantage for a system that may run on resource-constrained hardware.

---

## L2 — Scaffold Layer

Context assembly, learning, and cognitive modulation. Depend on L0 and L1.

| Crate | Status | Responsibility | Tests (as of 2026-04-17) | Dependents |
|---|---|---|---|---|
| `roko-compose` | **Shipping** | `SystemPromptBuilder` (7-layer, 12 role templates), token budget management, Liu et al. U-shape placement, 13-step enrichment pipeline | 36+ (as of 2026-04-17 per status.md) | `roko-orchestrator`, `roko-cli` |
| `roko-learn` | **Shipping** | Episode logger, cost tracker, playbook, skill library, pattern miner, cascade router updates, C-Factor, regression detector, bandit experiments, efficiency events; 42 modules | 101 | `roko-orchestrator` |
| `roko-daimon` | **Built** | PAD affect vectors, 6 behavioral states (Engaged/Struggling/Coasting/Exploring/Focused/Resting), somatic marker hypothesis implementation | — (as of 2026-04-17 per status.md) | Planned: `roko-orchestrator` |
| `roko-dreams` | **Scaffold** | NREM replay, REM imagination, slow consolidation (stubs only) | — (as of 2026-04-17 per status.md) | Planned: `roko-orchestrator` |

### `roko-learn` — Detail

35,847 LOC (as of 2026-04-17 per status.md). The largest crate by lines. Updates 10+
learning subsystems on every agent turn:

1. Episode logger — records full execution trace
2. Cost tracker — tracks token costs per task type
3. Playbook — extracts actionable heuristic rules
4. Skill library — stores reusable execution patterns
5. Pattern miner — detects recurring task structures
6. Cascade router update — bandit update for model selection
7. C-Factor metric — collective intelligence quotient
8. Regression detector — flags performance degradation
9. Experiments — A/B test tracking
10. Efficiency events — LOC/cost/time efficiency signals

Three bandit algorithms: UCB1 (pure exploration), LinUCB (contextual), Track-and-Stop
(best-arm identification).

### `roko-compose` — Detail

Implements the Liu et al. 2023 "lost in the middle" finding: important context belongs at
the beginning and end of the context window, not in the middle. The `SystemPromptBuilder`
enforces U-shape placement automatically. Cache-aligned prompt assembly reduces LLM API
costs by maximizing prompt cache hit rates.

---

## L3 — Harness Layer

Integration, supervision, and external interfaces. Depend on L0–L2.

| Crate | Status | Responsibility | Tests (as of 2026-04-17) | Dependents |
|---|---|---|---|---|
| `roko-conductor` | **Built** | Cybernetic regulator: 10 watchers, graduated interventions (Continue/Restart/Fail), stuck detection, circuit breakers, EWMA anomaly detection, Yerkes-Dodson pressure dynamics | — (as of 2026-04-17 per status.md) | Planned: `roko-orchestrator` |
| `roko-chain` | **Built** | Korai EVM: soulbound identity passports, 7-domain reputation with EMA, Spore/Sparrow job marketplace, HDC precompile (~400 gas), KORAI/DAEJI tokens with 1% annual demurrage, ISFR clearing with KKT certificates | 52 | Planned: chain deployment |
| `roko-std` | **Shipping** | 19 built-in tools: file I/O, shell execution, search, MCP client, and more | 96 | `roko-orchestrator`, `roko-cli` |
| `roko-serve` | **Shipping** | HTTP control plane: 200+ routes, SSE, WebSocket | — (as of 2026-04-17 per status.md) | `roko-cli` |

### `roko-conductor` — Detail

Built but not yet called from the orchestrator. Implements the Good Regulator Theorem
(Conant & Ashby 1970): for a system to regulate well, it must contain a model of what it
is regulating. The Conductor maintains a model of agent behavior and intervenes when
that behavior deviates from expected bounds.

Graduated interventions:
1. **Continue** — log observation, no action
2. **Restart** — restart the current task with a modified context
3. **Fail** — terminate the task and escalate

### `roko-chain` — Detail

Blocked by chain deployment. The Korai EVM is a dedicated blockchain for agent coordination.
Key features:
- Soulbound identity passports (ERC-8004 agent standard)
- 7-domain reputation with EMA decay
- Spore (job posting) / Sparrow (job acceptance) marketplace
- HDC precompile at ~400 gas (sub-millisecond vector similarity on-chain)
- 1% annual demurrage on KORAI/DAEJI tokens (prevents liquidity hoarding)
- ISFR clearing with KKT certificates for interest rate derivative settlement

---

## L4 — Orchestration Layer

Top-level coordination and user interfaces. Depend on all lower layers.

| Crate | Status | Responsibility | Tests (as of 2026-04-17) | Dependents |
|---|---|---|---|---|
| `roko-orchestrator` | **Shipping** | `ParallelExecutor` (pure state machine), cross-plan DAG scheduling, git worktree isolation, file-conflict-aware merge queue, hash-chained event-log crash recovery | 158 | `roko-cli` |
| `roko-cli` | **Shipping** | `roko prd` lifecycle (38 tests), `roko plan run`, `roko re` research, `roko dashboard` (ratatui TUI, F1-F7 tabs), `roko serve` | 38+ (as of 2026-04-17 per status.md) | — (top-level) |

### `roko-orchestrator` — Detail

Implements the self-hosting loop at the orchestration level. Key design constraint: the
`ParallelExecutor` is a pure state machine — no I/O, no side effects. All side effects are
pushed to the edges. This makes the executor trivially testable and crash-recoverable via
hash-chained event-log replay.

Cross-plan DAG scheduling enables tasks from multiple concurrent PRDs to share resources
and avoid conflicts. Git worktree isolation gives each agent an independent working
directory, preventing merge conflicts during parallel execution.

---

## Workspace Totals

| Metric | Value (as of 2026-04-17 per status.md) |
|---|---|
| Workspace members | 36 |
| Total Rust LOC | ~322K |
| Total test functions | 3,761 |
| Shipping crates | 8 |
| Built crates (code exists, not fully wired) | 4 |
| Scaffold crates (stubs only) | 3+ |

---

## Crate Dependency Graph (simplified)

```
roko-cli ────────────────────────────────────────────────── L4
  └── roko-orchestrator ─────────────────────────────────── L4
        ├── roko-compose ────────────────────────────────── L2
        ├── roko-learn ──────────────────────────────────── L2
        ├── roko-conductor (planned wiring) ─────────────── L3
        ├── roko-std ────────────────────────────────────── L3
        ├── roko-serve ──────────────────────────────────── L3
        ├── roko-agent ──────────────────────────────────── L1
        │     └── roko-core ──────────────────────────────── L0
        ├── roko-gate ───────────────────────────────────── L1
        │     └── roko-core ──────────────────────────────── L0
        ├── roko-fs ─────────────────────────────────────── L1
        │     └── roko-core ──────────────────────────────── L0
        └── roko-runtime ────────────────────────────────── L0
              └── roko-core ──────────────────────────────── L0
```

Cross-cuts (trait injection, not listed in graph above):
- `roko-neuro` injected into `roko-orchestrator` (planned wiring)
- `roko-daimon` injected into `roko-orchestrator` (planned wiring)
- `roko-dreams` injected into `roko-orchestrator` (planned wiring, scaffold)

---

## Planned Future Crates

These are mentioned in the architecture but do not yet exist as workspace members:

| Planned crate | Layer | Purpose |
|---|---|---|
| `roko-bus` | L0/L1 | `Bus` trait abstraction + `Topic`/`TopicFilter` types |
| `roko-mesh` | L3 | Agent-to-agent networking (`Mesh` layer) |
| `roko-index` | L1 | Code intelligence index |
| `roko-lang-*` | L1 | Language-specific intelligence backends (one per language) |

---

## See Also

- [`status/status.md`](../status/status.md) — master implementation matrix (source for test counts and LOC)
- [`status/vision.md`](../status/vision.md) — why this architecture exists
- [`reference/12-design-principles.md`](12-design-principles.md) — design constraints that shaped these crate boundaries

## Open Questions

- At what point does `roko-fs` promote to `roko-substrate` to house multiple backend implementations?
- Should `roko-bus` be extracted from `roko-runtime` as a separate L0/L1 crate when `Bus` ships?
- The LOC figures for individual crates other than `roko-learn` are not tracked per-crate in the
  current status matrix. A per-crate LOC audit would improve this table.
