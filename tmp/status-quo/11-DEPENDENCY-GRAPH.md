# 11 · Workspace Dependency Graph & Layering Violations

> Status-quo audit · **deeper second pass** · verified **HEAD `5852c93c05`** · `main` · **2026-07-08**
> Method: parsed every `[dependencies]`/`[dev-dependencies]` section of all 35 member manifests
> (`crates/*/Cargo.toml`, `apps/*/Cargo.toml`, `tests/Cargo.toml`), read `[package.metadata.roko].layer`
> from each, and read the enforcement source `scripts/layer_check.rs` (`check_layers`, `fn run_layer_check`).
> Layer rule confirmed from source: a violation is any edge where **`from_layer < to_layer`** (strictly less;
> same-layer edges are allowed). `check_layers` iterates `package.dependencies` from `cargo metadata --no-deps`
> and **does not filter `dep.kind` or `optional`** — so dev-deps and optional deps count as violations too.

---

## 1. Layer assignments (from `[package.metadata.roko].layer`)

| Layer | Crates | Intent |
|:--:|---|---|
| **L0** | `roko-primitives` | Pure primitives (HDC vectors, tier routing). Depends on nothing. |
| **L1** | `roko-core`, `roko-runtime` | Kernel (Signal + 6 traits) + process/event runtime. |
| **L2** | `roko-std`, `roko-fs`, `roko-chain`, `roko-agent`, `roko-learn`, `roko-neuro`, `roko-compose`, `roko-daimon`, `roko-dreams`, `roko-plugin`, `roko-graph`, `roko-index`, `roko-lang-{rust,typescript,go}`, `roko-mcp-{stdio,code,github,slack,scripts}`, `roko-demo` | Domain implementations. |
| **L3** | `roko-gate`, `roko-orchestrator`, `roko-conductor` | Orchestration + verification. |
| **L4** | `roko-cli`, `roko-serve`, `roko-acp`, `roko-agent-server` | Surfaces (CLI, HTTP, editor protocol, sidecar). |
| **—** | `mirage-rs`, `agent-relay`, `roko-chain-watcher` (apps), `roko-tests` | No layer metadata → **exempt from the layer gate** (see violation V6). |

**31 of 35 members carry layer metadata** (all `crates/*`). The **4 unlabeled members** — `mirage-rs`, `agent-relay`, `roko-chain-watcher` (apps) and `roko-tests` — are invisible to `check_layers`, so violation V6 (`agent-server → agent-relay`) slips past the gate.

---

## 2. Intended layering (ASCII) — arrows read "is allowed to depend on"

```
                         ┌──────────────────────────────────────────────┐
   L4  SURFACES          │  roko-cli   roko-serve   roko-acp   roko-agent-server │
                         └───────────────┬──────────────────────────────┘
                                         │ (may depend on L3,L2,L1,L0)
                         ┌───────────────▼──────────────────────────────┐
   L3  ORCH + VERIFY     │  roko-gate    roko-orchestrator    roko-conductor      │
                         └───────────────┬──────────────────────────────┘
                                         │ (may depend on L2,L1,L0)
                         ┌───────────────▼──────────────────────────────┐
   L2  DOMAIN            │  std fs chain agent learn neuro compose daimon dreams  │
                         │  plugin graph index lang-* mcp-* demo                  │
                         └───────────────┬──────────────────────────────┘
                                         │ (may depend on L1,L0)
                         ┌───────────────▼──────────────────────────────┐
   L1  KERNEL+RUNTIME    │  roko-core            roko-runtime                     │
                         └───────────────┬──────────────────────────────┘
                                         │ (may depend on L0)
                         ┌───────────────▼──────────────────────────────┐
   L0  PRIMITIVES        │  roko-primitives                                       │
                         └────────────────────────────────────────────────────────┘
```

## 3. ACTUAL layering — with the violation edge drawn upward (✗)

```
   L4   cli ─┬─► serve ─► agent-server ══╗ (V6: crates→apps)
             │                           ╚═► agent-relay  [apps/, no layer]
             └─► acp
                    │  all L4 → L3/L2/L1/L0 ......................... OK
   L3   gate ─► agent      (L3→L2 OK by number, design-smell V3)
        orchestrator ─► gate, conductor   (L3→L3 same-layer, allowed)
        conductor ─► learn (L3→L2 OK by number, design-rule-break V4)
                    ▲
                    │  ✗✗✗  ONE UPWARD EDGE FAMILY, ALL FROM roko-runtime  ✗✗✗
   L1   runtime ════╪═══════════════════════════► gate   (V1  L1→L3  NORMAL dep)
        runtime ────┼──► compose                          (V2  L1→L2  dev-dep)
        runtime ────┴──► learn                            (V2  L1→L2  dev-dep)
        core ─► primitives ................................ OK
   L0   primitives ── (nothing) .............................. OK
```

**There is exactly one layer-violating crate: `roko-runtime`.** Every upward (`from_layer < to_layer`) edge in the whole workspace originates in `roko-runtime`. No other crate has one. `roko-cli` is depended on by nothing (clean leaf ✅).

---

## 4. Violations table (exhaustive, with offending manifest line)

| # | Edge | Layers | Kind | Offending line | Why it exists | CI-gate flags? |
|--:|---|:--:|---|---|---|:--:|
| **V1** | `roko-runtime → roko-gate` | **L1 → L3** | normal | `crates/roko-runtime/Cargo.toml:27` | `WorkflowEngine`/`EffectDriver` live in runtime but call `GateService`/`GateRegistry` — see `src/effect_driver.rs:21` (`use roko_gate::GateRegistry`), `src/workflow_engine.rs:1327` (`roko_gate::GateService::new()`) | **YES — exit 1** |
| **V2a** | `roko-runtime → roko-compose` | L1 → L2 | **dev** | `crates/roko-runtime/Cargo.toml` `[dev-dependencies]` | test-only, but `check_layers` ignores `dep.kind` | **YES** |
| **V2b** | `roko-runtime → roko-learn` | L1 → L2 | **dev** | `crates/roko-runtime/Cargo.toml` `[dev-dependencies]` | test-only, but counted | **YES** |
| **V3** | `roko-gate → roko-agent` | L3 → L2 | normal | `crates/roko-gate/Cargo.toml:15` | agent-executed gates need the full LLM backend | no (number OK) — **design smell**: gate is un-reusable without the 87 K-LOC agent stack |
| **V4** | `roko-conductor → roko-learn` | L3 → L2 | normal | `crates/roko-conductor/Cargo.toml:15` | conductor imports learn internals directly | no (number OK) — **breaks designed rule** ("conductor must react via Bus, not import roko-learn", `docs/v1/00-architecture/15-crate-map.md`) |
| **V5** | `roko-std → roko-chain` | L2 → L2 | normal | `crates/roko-std/Cargo.toml:15` | builtin `CHAIN_DOMAIN_TOOLS` in std | no (same-layer OK) — **kernel-purity smell**: every std consumer (agent, acp, serve, cli) transitively compiles chain |
| **V6** | `roko-agent-server → agent-relay` | L4 → *app* | normal | `crates/roko-agent-server/Cargo.toml:16` | `agent-relay` is a library that lives under `apps/` | no (relay has no layer) — **topology inversion**: a `crates/` member depends on an `apps/` member |

**Only V1, V2a, V2b make `layer-check` return exit 1** (all three are `roko-runtime` edges). V3–V6 pass the numeric gate but are genuine architectural smells; V4 additionally violates the project's own written crate-map rule.

**Same-layer edges (allowed, listed for cycle-risk awareness):** `roko-std → roko-chain` (L2→L2), `roko-orchestrator → roko-gate` (L3→L3), `roko-orchestrator → roko-conductor` (L3→L3). None form a cycle (gate/conductor/chain do not depend back).

**No hard dependency cycles exist.** The apparent `agent↔learn`, `agent↔compose`, `learn↔compose`, `learn↔neuro` couplings are one-directional in `[dependencies]`; the reverse edges are `[dev-dependencies]` only (legal, no compile cycle). Real normal direction: `compose → learn → agent`, `neuro → learn`.

---

## 5. Complete adjacency (normal `[dependencies]`; dev/optional flagged)

Edges to `roko-core` (24 crates) and `roko-primitives` are the norm; the "→ deps" column omits them where noted for legibility but the counts include them.

| Crate | L | Intra deps (normal) | Dev-only intra | Depended-on-by (normal crates) | #rev |
|---|:-:|---|---|---|:-:|
| roko-primitives | 0 | — | — | core, dreams, learn, runtime **(+opt: fs, compose, neuro, serve; +mirage)** | 4 |
| roko-core | 1 | primitives | — | 24 crates (all except primitives, mcp-{stdio,github,slack,scripts}, agent-relay, demo→no… actually demo excl) **+ chain-watcher** | 24 |
| **roko-runtime** | 1 | core, primitives, **gate ✗** | **compose ✗, learn ✗** | acp, cli, orchestrator, serve **(+mirage)** | 4 |
| roko-std | 2 | core, **chain** | — | agent, acp, cli, serve | 4 |
| roko-fs | 2 | core (+primitives opt) | — | agent, cli, learn, neuro, serve | 5 |
| roko-chain | 2 | core (+alloy opt) | — | agent-server, cli, demo, serve, std **(+chain-watcher)** | 5 |
| roko-agent | 2 | core, fs, std | learn, std | acp, agent-server, cli, compose, dreams, gate, learn, neuro, orchestrator, serve | **10** |
| roko-learn | 2 | core, agent, daimon, fs, primitives | compose, neuro | acp, agent-server, cli, compose, conductor, dreams, neuro, orchestrator, serve (+runtime dev) | **9** |
| roko-neuro | 2 | core, fs, agent, learn (+prim opt) | — | acp, agent-server, cli, compose, dreams, orchestrator, serve | 7 |
| roko-compose | 2 | core, agent, learn, neuro (+prim opt) | std | acp, cli, orchestrator, serve (+runtime dev) | 4 |
| roko-daimon | 2 | core | — | cli, learn, orchestrator, serve | 4 |
| roko-dreams | 2 | core, neuro, learn, agent, primitives | — | acp, cli, serve | 3 |
| roko-plugin | 2 | core | — | cli, serve | 2 |
| roko-graph | 2 | core | — | cli | 1 |
| roko-index | 2 | core, lang-{rust,typescript,go} | — | cli, mcp-code | 2 |
| roko-lang-{rust,ts,go} | 2 | core | — | index | 1 |
| roko-mcp-stdio | 2 | — | — | mcp-{code,github,slack,scripts} | 4 |
| roko-mcp-code | 2 | core, index, mcp-stdio | — | — (leaf bin) | 0 |
| roko-mcp-{github,slack,scripts} | 2 | mcp-stdio | — | — (leaf bins) | 0 |
| roko-demo | 2 | chain (alloy) | — | — (leaf bin, app-in-crates) | 0 |
| roko-gate | 3 | core, **agent** | std | **runtime (upward ✗)**, acp, cli, orchestrator, serve | 5 |
| roko-orchestrator | 3 | core, agent, compose, conductor, daimon, gate, learn, neuro, runtime | — | acp, cli, serve | 3 |
| roko-conductor | 3 | core, **learn** | — | cli, orchestrator, serve | 3 |
| roko-acp | 4 | core, runtime, agent, gate, compose, orchestrator, learn, dreams, neuro, std | — | cli | 1 |
| roko-agent-server | 4 | **agent-relay (app ✗)**, agent, chain, core, learn, neuro | — | cli, serve | 2 |
| roko-serve | 4 | 16 crates (core, agent, agent-server, chain, learn, neuro, dreams, gate, fs, compose, std, orchestrator, conductor, plugin, daimon, runtime; +prim opt) | core, runtime | cli | 1 |
| roko-cli | 4 | **20 crates** (all except demo, mcp-*, lang-*, primitives-direct, mirage, watcher) | agent-relay | **— (nothing; clean leaf ✅)** | 0 |
| mirage-rs (app) | — | runtime (+core opt, +primitives opt) | — | — | 0 |
| agent-relay (app) | — | — | — | **agent-server ✗** (+cli dev) | 1 |
| roko-chain-watcher (app) | — | core, chain | — | — | 0 |
| roko-tests | — | — | core, std, fs, gate, compose, agent | — | 0 |

**Fan-in leaders:** `roko-core` (24) ≫ `roko-agent` (10) > `roko-learn` (9) > `roko-neuro` (7). **Fan-out leader:** `roko-cli` (20 intra deps) — the most-coupled crate, the god-surface.

**"Should be leaves but aren't":**
- `agent-relay` is a *library* (54 pub decls) placed in `apps/` yet has a reverse dep (agent-server) → V6.
- `roko-demo` is an app *binary* placed in `crates/` at L2 — correctly a leaf (0 rev-deps) but mis-homed.
- `roko-gate` should be a low-fan-out verification leaf; instead it fans **out** to `roko-agent` (V3) and is depended on **upward** by runtime (V1) — it is entangled on both sides.

---

## 6. What the CI `layer-check` job actually enforces

`.github/workflows/ci.yml:37-48` runs `cargo run -p roko-cli -- layer-check` → `run_layer_check()`. Beyond the numeric layer rule it runs **5 negative-pattern checks** (all fail CI) + 1 warning:

| Check (fn) | Target | Fails on |
|---|---|---|
| `check_layers` | all edges | `from_layer < to_layer` → **V1, V2a, V2b today** |
| `check_duplicate_foundation_traits` | crates ≠ core | `AffectPolicy`/`DispatchModulation`/`AffectContext` defined outside roko-core |
| `check_debug_event_logging` | `roko-runtime/src/jsonl_logger.rs` | `:?` debug-format of runtime events |
| `check_direct_model_subprocess` | `crates/**` | `Command::new("claude"/"codex")` in un-`legacy-orchestrate`-gated code |
| `check_noop_gates` | `roko-gate/src/gate_service.rs` | `passed: true` near stub/noop/always language |
| `check_empty_event_fields` | `roko-runtime/src` | `agent_id/model: String::new()` placeholders |
| `check_path_shared_modules` (warn) | cli vs serve `#[path]` | shared `#[path=...]` modules (should be a crate API) |

**Open question stands:** because V1/V2 exist, `layer-check` should return exit 1 on `main`. Either the CI job is currently red, is being skipped, or `cargo metadata` resolves something the static manifests don't. **Run `cargo run -p roko-cli -- layer-check` to settle it** (checklist P0).

---

## 7. Fix directions

- **V1 (the worst):** hoist gate-consuming code out of `roko-runtime`. `WorkflowEngine`/`EffectDriver` should live in `roko-orchestrator` (L3, already depends on both runtime and gate) or `roko-graph`, or gate should expose a runtime-safe L1 trait that runtime depends on instead of the concrete `GateService`. This is the single change that turns `layer-check` green (modulo V2).
- **V2:** either accept dev-deps (patch `check_layers` to skip `dep.kind == DependencyKind::Development`), or move runtime's compose/learn-using tests into `tests/`.
- **V4:** route `roko-conductor` through the event Bus instead of importing `roko-learn`.
- **V5:** put chain tools behind a `chain-tools` feature in `roko-std` so kernel consumers don't drag in chain.
- **V6:** move `agent-relay` from `apps/` into `crates/` (or split its lib target).

---

## 8. Checklist / roadmap

- [ ] **[P0]** Run `cargo run -p roko-cli -- layer-check` and record actual exit code — confirms whether V1/V2 turn CI red today.
- [ ] **[P0]** Resolve **V1** `roko-runtime(L1) → roko-gate(L3)` by re-homing `WorkflowEngine`/`EffectDriver` to L3 — verify: `grep -n roko-gate crates/roko-runtime/Cargo.toml` (expect gone).
- [ ] **[P1]** Resolve **V2**: move runtime dev-deps' tests to `tests/`, or make `check_layers` skip dev-deps (`scripts/layer_check.rs:73`).
- [ ] **[P1]** Fix **V6**: relocate `agent-relay` into `crates/` — verify: `grep -n agent-relay crates/roko-agent-server/Cargo.toml` points to `../agent-relay`.
- [ ] **[P2]** Address **V4** (`conductor→learn`) per crate-map rule — verify: `grep -n roko-learn crates/roko-conductor/Cargo.toml` (expect gone).
- [ ] **[P2]** Address **V5**: gate `roko-std → roko-chain` behind a feature — verify: `grep -n roko-chain crates/roko-std/Cargo.toml`.
- [ ] **[P2]** Reduce `roko-cli` fan-out (20 deps) by extracting the TUI / runner into their own crates.
- [ ] **[P3]** Add layer metadata to the 3 apps + `tests/` so the gate sees V6, or explicitly document them as gate-exempt.
- [ ] **[P3]** Decide **V3** (`gate→agent`): split gate into a kernel-only core + an agent-executed-gate extension, so verification is reusable without the LLM stack.

## 9. Open questions

1. **Is CI `layer-check` green on `main` right now?** Static analysis says no (V1/V2). Settle by running it.
2. **Three execution engines coexist** — orchestrator's plan DAG, roko-graph's cell DAG, and `WorkflowEngine` inside runtime (the thing dragging gate into L1). Which is canonical, and should `WorkflowEngine` move to break V1?
3. **Should dev-deps count as layer violations?** The gate currently says yes (unfiltered `dep.kind`); the fix for V2 is a policy decision.
4. **Is `roko-gate → roko-agent` (V3) intended?** It makes verification depend on 87 K LOC of LLM backends.

See also: **doc 16** (codebase inventory), **doc 03** (crate audit), **doc 59** (API route ledger).
