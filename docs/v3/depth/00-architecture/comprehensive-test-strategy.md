# Comprehensive Test Strategy

> **v3 depth file** -- `/docs/v3/depth/00-architecture/comprehensive-test-strategy.md`
> Canonical source: v1 `docs/v1/00-architecture/32-comprehensive-test-strategy.md`
> Status: **Updated for 10,300+ tests** -- 39 workspace members, ~1M LOC. The v1 audit
> covered 3,761 tests across 36 members. This revision reflects the 2026-09-15 test
> reality, including the accepted epics, Graph engine convergence, and TUI parity.

---

## 1. Purpose

Roko is a self-improving agent system: it turns requests (prompts or written specs) into
plans, executes them, validates with gates, and persists results. A self-improving system can silently
regress in ways that static software cannot -- an agent that modifies its own prompt
templates, learning weights, or gate thresholds can degrade capabilities while all unit
tests still pass. This document specifies the complete testing strategy.

The test strategy must also verify the observability claim. If logs, metrics, traces,
Pulses, replay outputs, or StateHub projections drift from their contracts, Roko becomes
hard to debug, hard to audit, and impossible to trust in production.

### The Testing Paradox of Self-Improving Systems

Classical testing assumes a fixed program. Roko violates this assumption in three ways:

1. **Prompt evolution**: SystemPromptBuilder templates, playbook injection, and experiment
   assignment change the effective program at runtime without changing source code.
2. **Threshold drift**: Adaptive gate thresholds (EMA alpha=0.1), CascadeRouter bandit
   arms, and efficiency metrics evolve continuously based on execution history.
3. **Knowledge accumulation**: Neuro knowledge tiers, episode logs, and pattern extraction
   create emergent behaviors not present at initial deployment.

Testing must therefore operate at three levels: **static** (source code correctness),
**behavioral** (system behavior under fixed inputs), and **evolutionary** (capability
preservation across self-modification cycles).

---

## 2. Current State (2026-09-15)

### 2.1 Test Count by Crate

| Crate | Layer | Tests (approx) | Change from v1 | Status |
|---|---|---|---|---|
| `roko-core` | L0 Kernel | 1,200+ | +824 | High coverage, pub(crate) encapsulation |
| `roko-agent` | L1 Framework | 1,100+ | +754 | 12 provider kinds, MCP, safety |
| `roko-serve` | L4 Application | 800+ | +774 | canonical routes (counts in `tools/http_route_inventory.snapshot.json`) tested |
| `roko-graph` | L1 Engine | 600+ | New | Sole execution engine |
| `roko-learn` | Cross-cut | 600+ | +499 | Experiments, playbooks, bandits |
| `roko-gate` | L3 Harness | 500+ | +300 | 19 gates, 7-rung pipeline |
| `roko-cli` | L4 Application | 500+ | +462 | TUI parity, plan commands |
| `roko-conductor` | L1 Framework | 350+ | +220 | 12 watchers, circuit breaker |
| `roko-daimon` | Cross-cut | 300+ | +300 | E23 10/10, GoalTree, energy |
| `roko-chain` | L1 Framework | 300+ | +290 | Local tranche: 4 state machines |
| `roko-compose` | L2 Scaffold | 300+ | +277 | 9-layer builder, enrichment |
| `roko-std` | L1 Framework | 250+ | +130 | 35 definitions, tool policy |
| `roko-neuro` | Cross-cut | 200+ | +197 | Durable knowledge, tiers |
| `roko-plugin` | L1 Framework | 200+ | New | E32 8/8, signed deps, WASM hooks |
| `roko-agent-server` | L4 Application | 200+ | New | Sidecar, LLM dispatch |
| `roko-acp` | L4 Application | 180+ | New | ACP server, 180 tests |
| `roko-dreams` | Cross-cut | 150+ | +143 | Consolidation, scheduling |
| `roko-fs` | L3 Harness | 150+ | +90 | JSONL substrate, GC |
| `roko-index` | L2 Scaffold | 100+ | +68 | Parser + graph + HDC |
| `roko-execution` | L1 Framework | 100+ | New | RuntimeServices builder |
| `roko-lang-* (3)` | L2 Scaffold | 100+ | +100 | Rust, TypeScript, Go |
| `roko-mcp-code` | L1 Framework | 50+ | New | Code intelligence MCP |
| `roko-runtime` | L0 Runtime | 80+ | +74 | ProcessSupervisor, event bus |
| `roko-primitives` | L1 Framework | 60+ | +42 | HDC vectors, tier routing |
| `roko-demo` | L4 Application | 30+ | New | Scenario demos |
| `roko-mcp-* (other)` | L1 Framework | 80+ | New | GitHub, Slack, Scripts, Stdio |
| **Workspace tests** | Cross-crate | 100+ | +81 | e2e, tool_replay, integration |
| **Total** | | **10,300+** | **+6,539** | |

### 2.2 Infrastructure Status

| Capability | v1 Status | Current Status |
|---|---|---|
| Unit tests | Good coverage in core/agent/gate | Comprehensive across all 39 crates |
| Integration tests | Partial (9 crates) | 25+ crates have `tests/` |
| Property-based tests | Declared, unused | proptest used in core, gate, primitives |
| Benchmarks | Missing entirely | Partial (HDC ops, gate pipeline) |
| Fuzzing | Missing entirely | Partial (config parser) |
| Mutation testing | Missing entirely | Not yet |
| Observability contract tests | Missing entirely | E33 lens executor tests, StateHub tests |
| Mocks | Partial | Comprehensive (`MockAgent`, `MockProvider`, wiremock) |
| Fixtures | Partial (15 files) | 50+ fixture files across crates |
| CI matrix | Not in repo | Pre-commit checks (fmt, clippy, test) |

---

## 3. Test Philosophy

### 3.1 Three Properties Per Crate

Each crate's tests verify:

1. **Correctness**: Does the implementation match the spec?
2. **Contract adherence**: Does the implementation satisfy its trait contract?
3. **Boundary behavior**: What happens at limits (empty input, max values, NaN, concurrent access)?

### 3.2 Three Testing Levels for Self-Improvement

| Level | What | How | Frequency |
|---|---|---|---|
| **Static** | Source code correctness | Unit tests, property tests, clippy | Every commit |
| **Behavioral** | System behavior under fixed inputs | Integration tests, e2e tests, fixture replay | Every PR |
| **Evolutionary** | Capability preservation | Regression gates, prompt snapshot tests, learning drift detection | Per release |

---

## 4. Per-Crate Test Specifications

### 4.1 roko-core (L0 Kernel) -- 1,200+ tests

| Module | What to test | Tests |
|---|---|---|
| `engram.rs` (Signal) | Builder, ContentHash uniqueness, lineage DAG, serde round-trip, clone independence | 120+ |
| `score.rs` | All 7 axes in range, effective_score formula, arithmetic, NaN rejection | 80+ |
| `decay.rs` | Four variants at t=0, t=half_life, t=inf; negative time; overflow | 70+ |
| `kind.rs` | All Kind variants serialize/deserialize, display, equality | 40+ |
| `body.rs` | All Body variants, JSON round-trip, large payload, empty payload | 35+ |
| `provenance.rs` | Constructors, taint propagation, trust range enforcement | 30+ |
| `config/` | 60+ parameter validation, 3-level override, defaults, serde, migration | 200+ |
| `tools/` | Tool policy, allowlists, deny-wins, unknown roles | 100+ |
| Property-based | Score commutativity, decay monotonicity, hash determinism | 40+ |

### 4.2 roko-agent (L1 Framework) -- 1,100+ tests

| Module | What to test | Tests |
|---|---|---|
| `provider/` | 12 backends: response parsing, error mapping, streaming, retry, timeout | 300+ |
| `harness/` | ACP client, tool loop, tool dispatch, result re-prompt cycle | 150+ |
| `safety/` | Role authorization (roles x tools matrix), pre-check, post-check, taint | 100+ |
| `dispatcher/` | Provider selection, pool management, MCP resolution | 100+ |
| `ollama/` | Ollama adapter, model discovery, streaming | 50+ |
| `cursor_cli_agent.rs` | Cursor CLI integration, subprocess management | 30+ |
| Mock infrastructure | MockAgent, mock_provider, wiremock scenarios | 50+ |
| Integration | Cross-provider fixture round-trips, MCP end-to-end | 80+ |

### 4.3 roko-graph (L1 Engine) -- 600+ tests

| Module | What to test | Tests |
|---|---|---|
| Cells (7 cognitive + 5 verify) | Cell execution, input/output contracts, error handling | 150+ |
| Topology | DAG construction, cycle detection, parallel waves, conditional routing | 100+ |
| Cost state | Atomic reservations, schema-v2 persistence, paid-failure handling | 80+ |
| Execution | Live provider dispatch, resume-durable checkpoints, GuaranteedFinally | 100+ |
| Immune graph | Five-stage decision graph, taint propagation, quarantine | 80+ |
| ProductionPlanTopology | Plan-to-graph conversion, task dependencies | 50+ |
| Integration | Full graph execution with mock providers | 40+ |

### 4.4 roko-gate (L3 Harness) -- 500+ tests

| Module | What to test | Tests |
|---|---|---|
| 19 gate implementations | Each gate: valid input, invalid input, timeout, verdict | 200+ |
| `gate_pipeline.rs` | Short-circuit, full execution, verdict aggregation, rung dispatch | 50+ |
| `ratchet.rs` | Monotonicity, per-plan isolation, rung 0 edge, full pass sequence | 30+ |
| `adaptive_threshold.rs` | EMA update, retry suggestion, skip advisory, persistence | 40+ |
| Rung oracles (4-6) | build_rung_execution_inputs, enriched inputs | 30+ |
| Property-based | GateRatchet monotonicity, threshold boundedness | 20+ |
| Integration | Real `cargo build` against tempdir fixtures | 30+ |

### 4.5 roko-serve (L4 Application) -- 800+ tests

| Module | What to test | Tests |
|---|---|---|
| Canonical routes (counts in `tools/http_route_inventory.snapshot.json`) | Request validation, response format, auth, error handling | 400+ |
| StateHub | Push-based projections, SSE delivery, cursor atomicity | 80+ |
| WebSocket | Agent streaming, bidirectional control | 40+ |
| Marketplace routes | Artifact/package/publish/economics/fork | 60+ |
| Telemetry routes | Lens queries, retention, resolution | 50+ |
| Named surfaces | Workbench/Inbox/Canvas/Minimap/Autonomy projections | 50+ |
| Auth middleware | Worker callback auth, serve-auth, opaque IDs | 40+ |
| Integration | Multi-route scenarios, concurrent requests | 80+ |

---

## 5. Property-Based Testing Strategy

### 5.1 High-Value Property Candidates

#### Category A: Algebraic Properties

| Crate | Property | Expression |
|---|---|---|
| `roko-core` | Score effective always in [-1, 1] | `forall s: Score. -1.0 <= s.effective() <= 1.0` |
| `roko-core` | Score weighted_merge commutative at w=0.5 | `merge(a, b, 0.5) ~ merge(b, a, 0.5)` |
| `roko-core` | Decay weight_at monotonically non-increasing | `forall t1 <= t2. decay.weight_at(t1) >= decay.weight_at(t2)` |
| `roko-core` | ContentHash deterministic | `forall data. blake3(data) == blake3(data)` |
| `roko-core` | Signal serialization round-trip | `forall s. deserialize(serialize(s)) == s` |
| `roko-primitives` | HDC bind is self-inverse | `forall a, b. bind(bind(a, b), b) ~ a` |
| `roko-primitives` | HDC bundle preserves similarity | `forall a, b. sim(bundle(a, b), a) > 0` |
| `roko-primitives` | HDC permute is invertible | `forall a, k. unpermute(permute(a, k), k) == a` |
| `roko-gate` | Verdict aggregation: all-pass -> pass | `forall vs. vs.all(passed) => aggregate(vs).passed` |
| `roko-gate` | Verdict aggregation: any-fail -> fail | `forall vs. vs.any(!passed) => !aggregate(vs).passed` |

#### Category B: Stateful Properties

| Crate | Property | State machine |
|---|---|---|
| `roko-gate` | GateRatchet monotonicity | Operations: `record_pass(plan, rung)`. Invariant: `highest_pass` never decreases |
| `roko-gate` | AdaptiveThresholds EMA bounded | Invariant: `ema_pass_rate in [0, 1]` always |
| `roko-learn` | Episode log append-only | Invariant: `len()` monotonically increases |
| `roko-learn` | Bandit arm explores all arms | After N > K*10 selections, every arm selected |
| `roko-fs` | FileSubstrate idempotent write | Writing same signal twice does not duplicate |
| `roko-graph` | Cost state atomic reservations | Concurrent reservations never exceed budget |

#### Category C: Metamorphic Relations

| Crate | Metamorphic relation | Transformation |
|---|---|---|
| `roko-compose` | Prompt stability under reordering | Reorder sections -> same prompt except order |
| `roko-compose` | Prompt monotonicity under enrichment | Add context -> token count increases |
| `roko-gate` | Gate pipeline determinism | Same input -> same verdict |
| `roko-agent` | Safety layer consistency | Denied tool remains denied under reformulation |
| `roko-core` | Query filter monotonicity | Broader filter -> superset of results |

---

## 6. Integration Test Strategy

### 6.1 Cross-Crate Integration Matrix

| Test scenario | Crates involved | Status | Priority |
|---|---|---|---|
| **Full self-hosting loop** | cli -> graph -> agent -> gate -> fs -> learn | Wired | P0 |
| **Prompt -> Plan -> Execute** | cli -> agent -> graph -> gate | Wired | P0 |
| **Gate pipeline -> Adaptive thresholds** | gate + learn | Wired | P1 |
| **Agent -> Safety -> Tool dispatch** | agent + std | Wired | P1 |
| **CascadeRouter -> Model selection** | agent + learn | Wired | P1 |
| **SystemPromptBuilder -> Agent dispatch** | compose + agent | Wired | P1 |
| **Episode logging -> Skill extraction** | learn + compose | Wired | P2 |
| **Telemetry contract verification** | graph -> agent -> gate -> Bus -> StateHub | Partial | P0 |
| **Signal write -> Query -> Decay -> GC** | core -> fs | Wired | P2 |
| **ProcessSupervisor -> Agent lifecycle** | runtime -> agent -> graph | Wired | P2 |
| **Conductor watchers -> Circuit breaker** | conductor -> agent | Wired | P2 |
| **Daimon affect -> Router tier selection** | daimon -> agent | Wired | P2 |
| **Dreams -> Knowledge consolidation** | dreams -> neuro -> fs | Wired | P2 |
| **Graph -> Immune decision graph** | graph -> safety | Wired | P1 |
| **ACP -> Provider dispatch** | acp -> agent -> gate | Wired (180 tests) | P1 |

### 6.2 End-to-End Test Scenarios

#### E2E-1: Minimal Self-Hosting Loop

```rust
/// Exercises: init -> plan create -> plan run -> status
#[tokio::test]
async fn test_minimal_self_hosting_loop() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("roko").unwrap()
        .args(["init"]).current_dir(&dir).assert().success();
    Command::cargo_bin("roko").unwrap()
        .args(["plan", "create", "hello-world", "--title", "Add a hello world function"])
        .current_dir(&dir).assert().success();
    assert!(dir.path().join(".roko").exists());
    assert!(dir.path().join("roko.toml").exists());
}
```

#### E2E-2: Graph Engine Resume

```rust
/// Exercises: plan run -> interrupt -> plan run --resume -> completion
#[tokio::test]
async fn test_graph_resume_after_interruption() {
    // Setup: create a plan with 3 tasks, execute first 2
    // Kill: simulate interruption after task 2
    // Resume: only task 3 executes, final status shows all 3 complete
}
```

#### E2E-3: ACP Integration

```rust
/// Exercises: ACP server start -> message -> tool call -> response
#[tokio::test]
async fn test_acp_full_cycle() {
    // Start ACP server
    // Send message via ACP protocol
    // Verify tool dispatch and response
}
```

---

## 7. Performance Testing

### 7.1 Critical Performance Targets

| Operation | Target | Current | Status |
|---|---|---|---|
| HDC hamming_distance (10,240-bit) | < 100ns | ~50ns | Met |
| HDC bind (XOR) | < 10ns | ~5ns | Met |
| Signal serialization | < 1us | ~800ns | Met |
| Gate pipeline (7 rungs) | < 5s | ~3s | Met |
| Graph checkpoint write | < 50ms | ~30ms | Met |
| Config load + validation | < 100ms | ~60ms | Met |
| Somatic landscape query (10K markers) | < 100us | Not benchmarked | Needed |
| StateHub projection update | < 1ms | ~500us | Met |

### 7.2 Benchmark Candidates

| Benchmark | What | Where |
|---|---|---|
| `bench_hdc_ops` | bind, bundle, permute, hamming at scale | `roko-primitives/benches/` |
| `bench_gate_pipeline` | Full 7-rung pipeline with real gates | `roko-gate/benches/` |
| `bench_compose_assembly` | 9-layer prompt assembly with enrichment | `roko-compose/benches/` |
| `bench_graph_execution` | Graph with 20 cells, parallel waves | `roko-graph/benches/` |
| `bench_substrate_query` | JSONL query at 100K signals | `roko-fs/benches/` |
| `bench_config_load` | Full config load with migration | `roko-core/benches/` |

---

## 8. Safety and Adversarial Testing

### 8.1 Safety Test Matrix

| Category | Tests | Coverage |
|---|---|---|
| Taint propagation | Taint never silently clears, propagates through composition | Wired (E34) |
| Quarantine | Quarantined signals excluded from default query and compose | Wired |
| Role authorization | 12 provider kinds x tool matrix, deny-wins | Wired |
| Sandbox policy | Five-level sandbox (Unrestricted through Deny) | Wired |
| Corrigibility | Five-head ordering respected under all conditions | Wired |
| Tool cooldown/isolation | Provider isolation, tool cooldown enforcement | Wired |
| Budget enforcement | USD budgets persisted and enforced across sessions | Wired (ACP) |
| Immune graph | Five-stage decision graph screens all host-visible tool results | Wired |

### 8.2 Adversarial Test Candidates

| Test | What | Priority |
|---|---|---|
| Prompt injection resistance | Tool output containing instructions does not alter behavior | P0 |
| Memory poisoning detection | False claims quarantined before reaching durable state | P0 |
| Plugin sandbox escape | Plugin exceeding capabilities is contained | P1 |
| Budget exhaustion graceful | Zero budget produces graceful degradation, not crash | P1 |
| Cross-tenant isolation | Multi-tenant deployment prevents data leakage | P2 |

---

## 9. Test Infrastructure

### 9.1 Pre-Commit Checks (Mandatory)

```bash
cargo +nightly fmt --all                              # Format
cargo clippy --workspace --no-deps -- -D warnings     # Lint
cargo test --workspace                                # Tests
```

### 9.2 Extended Validation (Pre-Release)

```bash
# Full workspace check including all targets and features
cargo clippy --workspace --all-targets --all-features --no-deps -- -D warnings
# Release build verification
cargo build --release --workspace
# CLI startup verification
cargo run -p roko-cli -- doctor
cargo run -p roko-cli -- plan validate plans/
```

### 9.3 Test Organization Convention

```
crates/<crate>/
  src/
    lib.rs          # Inline unit tests (#[cfg(test)] mod tests)
    module.rs       # Module-level inline tests
  tests/
    integration.rs  # Cross-module integration tests
    fixtures/       # Test data files
  benches/
    benchmark.rs    # Criterion benchmarks
```

---

## 10. Evolution Testing

### 10.1 Prompt Snapshot Tests

Prompt templates change over time. Snapshot tests detect unintended regressions:

```rust
#[test]
fn test_prompt_template_stability() {
    let builder = SystemPromptBuilder::new(Role::Coder);
    let prompt = builder.build(&context);
    // Compare against stored snapshot
    insta::assert_snapshot!(prompt);
}
```

### 10.2 Learning Drift Detection

```rust
#[test]
fn test_cascade_router_does_not_drift() {
    let router = CascadeRouter::load(".roko/learn/cascade-router.json")?;
    // Verify arm weights are within expected bounds
    for arm in router.arms() {
        assert!(arm.mean_reward > 0.0 && arm.mean_reward <= 1.0);
        assert!(arm.pull_count > 0); // No completely unexplored arms
    }
}
```

### 10.3 Gate Threshold Stability

```rust
#[test]
fn test_adaptive_thresholds_bounded() {
    let thresholds = AdaptiveThresholds::load(".roko/learn/gate-thresholds.json")?;
    for (rung, ema) in thresholds.ema_values() {
        assert!(ema >= 0.0 && ema <= 1.0,
            "Rung {} EMA {} out of bounds", rung, ema);
    }
}
```

---

## Cross-References

- [Implementation readiness audit](./implementation-readiness-audit.md) -- Gap analysis
- [Cross-section integration map](./cross-section-integration-map.md) -- Integration wiring
- [Cognitive immune system](./cognitive-immune-system.md) -- Safety testing foundations
- [Cross-pollination innovations](./cross-pollination-innovations.md) -- Test criteria per innovation
