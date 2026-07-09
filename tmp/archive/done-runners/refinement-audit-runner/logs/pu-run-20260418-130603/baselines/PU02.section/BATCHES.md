# Batch Execution Contract

8 batches ordered for unattended execution. The goal is not just to "cover the docs", but to let an agent harden the shared agent layer without needing hidden project context.

---

## Batch Posture

- Default strategy: **activate existing agent infrastructure before inventing new agent frameworks**.
- Treat `crates/roko-cli/src/orchestrate.rs`, `crates/roko-agent/src/provider/mod.rs`, and the response-type surface as conflict hotspots.
- If a task starts requiring verification-policy, learning-policy, or orchestration-policy redesign, record the seam and stop.
- Every completed batch should leave behind:
  - code changes,
  - verification command output,
  - explicit deferrals,
  - and any follow-on dependency it creates.

Required reads for every batch:

- [00-INDEX.md](00-INDEX.md)
- the owning section file(s) named below
- [SOURCE-INDEX.md](SOURCE-INDEX.md)
- [context-pack/agent-runbook.md](context-pack/agent-runbook.md)
- [context-pack/carry-forward-map.md](context-pack/carry-forward-map.md)

---

## Recommended Serial Order

For a single long-running agent run, prefer:

`G6 -> G1 -> G3 -> G4 -> G5 -> G2 -> G7 -> G8`

This order closes the narrowest safety gap first, removes local duplication before cross-crate migration, unblocks runtime tool universality, then lands the temperament foundation after the main runtime path is healthier.

---

## Batch Overview

| Batch | Tasks | Purpose | Primary Write Scope | Verify Focus | Est. LOC |
|-------|-------|---------|---------------------|--------------|----------|
| G1 | A.05, A.07 | Canonicalize response types inside `roko-agent` | `roko-agent` chat/translate modules | `cargo test -p roko-agent` | 120 |
| G2 | A.03, A.08 | Move shared response surface into `roko-core` | `roko-core`, `roko-agent`, `roko-compose` | `cargo test -p roko-core -p roko-agent -p roko-compose` | 220 |
| G3 | C.13a | Close Anthropic HTTP tool-loop backend gap | `roko-agent` tool-loop backends | `cargo test -p roko-agent tool_loop` | 180 |
| G4 | C.17, E.18a, F.07b | Route at least one orchestrator path through ToolDispatcher + ToolLoop | `roko-cli`, `roko-agent` | `cargo test -p roko-cli -p roko-agent` | 260 |
| G5 | C.36 | Enforce `max_tools` / degradation caps | `roko-agent`, `roko-core` tool capability surfaces | `cargo test -p roko-agent -p roko-core` | 80 |
| G6 | D.14b | Eliminate remaining direct safety-bypass creation sites | `roko-cli` entrypoints | `cargo test -p roko-cli -p roko-agent` | 60 |
| G7 | E.10 | Add typed temperament foundation | `roko-core`, `roko-agent`, `roko-cli` | `cargo test -p roko-core -p roko-agent -p roko-cli` | 140 |
| G8 | E.11, E.13, E.14 | Propagate temperament into runtime behavior | `roko-agent`, `roko-learn`, `roko-cli` | `cargo test -p roko-agent -p roko-learn -p roko-cli -p roko-core` | 220 |

---

## Dependency Graph

| Batch | Depends on |
|-------|------------|
| G1 | — |
| G2 | G1 |
| G3 | — |
| G4 | G3 |
| G5 | G4 |
| G6 | — |
| G7 | — |
| G8 | G7, G4 |

Why `G4 -> G5`:

- the best place to enforce `max_tools` is clearer once the production orchestrator path actually uses the tool-loop stack.

Why `G7 -> G8`:

- propagation work without a shared temperament type and config field will devolve into ad-hoc string plumbing.

Parallel-safe groups:

- `{G1, G3, G6, G7}` can start immediately.
- `G2` should wait for `G1`.
- `G4` should wait for `G3`.
- `G5` should wait for `G4`.
- `G8` should wait for `G7` and benefits from `G4`.

Conflict groups:

| Group | Crates / Files | Batches |
|-------|----------------|---------|
| response-surface | `crates/roko-agent/src/chat_types.rs`, `translate/mod.rs`, `usage.rs`, `crates/roko-core/src/*` | G1, G2 |
| provider-toolloop | `crates/roko-agent/src/provider/mod.rs`, `tool_loop/backends/*` | G3, G4 |
| cli-agent-path | `crates/roko-cli/src/orchestrate.rs`, `run.rs`, `agent_spawn.rs`, `main.rs` | G4, G6, G8 |
| temperament | `crates/roko-core/src/config/schema.rs`, `crates/roko-agent/src/introspection.rs`, `crates/roko-learn/src/*router*` | G7, G8 |

---

## Batch Details

### G1 — Canonicalize Response Types Inside `roko-agent`

**Owns**: `A.05.1`, `A.07.1`

**Read first**:
- [A-core-abstractions.md](A-core-abstractions.md)
- [C-tool-loop.md](C-tool-loop.md)

**Problem**: `ChatResponse` and `ResponseMetadata` exist in both `chat_types.rs` and `translate/mod.rs`. The richer version appears to be the real one, but the duplicated surface makes future crate extraction risky.

**Scope**:
1. Choose one canonical definition location inside `roko-agent`.
2. Remove divergent duplicate struct definitions or reduce them to re-exports/type aliases.
3. Update translator/tool-loop/users to consume the same type.
4. Add focused tests for serde or construction paths if the move changes imports.

**Out of scope**:
- moving the types to `roko-core`,
- redesigning the full response schema,
- changing runtime semantics of `raw_assistant_message` or `session`.

**Files**:
- `crates/roko-agent/src/chat_types.rs`
- `crates/roko-agent/src/translate/mod.rs`
- `crates/roko-agent/src/lib.rs`

**Verify**:
```bash
cargo test -p roko-agent
cargo clippy -p roko-agent --no-deps -- -D warnings
```

**Acceptance criteria**:
- only one real `ChatResponse` definition remains in `roko-agent`,
- only one real `ResponseMetadata` definition remains in `roko-agent`,
- translators and tool-loop code compile against the canonical type,
- no compatibility wrapper silently preserves duplicate ownership.

**Handoff note**:
- record whether moving `Usage` alongside `ChatResponse` is required for `G2`.

---

### G2 — Move Shared Response Types Into `roko-core`

**Owns**: `A.03.1`, `A.08`

**Read first**:
- [A-core-abstractions.md](A-core-abstractions.md)
- [context-pack/agents-summary.md](context-pack/agents-summary.md)

**Problem**: the broader codebase cannot rely on agent-owned response types. If `ChatResponse` becomes a shared contract, it needs a stable home outside `roko-agent`.

**Scope**:
1. Create a core-owned response surface for the minimum shared types.
2. Resolve the `Usage` ownership question cleanly so the moved response type does not depend on `roko-agent`.
3. Add compatibility re-exports in `roko-agent` where helpful.
4. Migrate at least `roko-agent` and one downstream consumer to the new location.

**Out of scope**:
- giant workspace-wide serde refactors,
- changing every old import in one sweep if compatibility re-exports are safer,
- expanding into provider-specific response payload redesign.

**Files**:
- `crates/roko-core/src/`
- `crates/roko-agent/src/chat_types.rs`
- `crates/roko-agent/src/usage.rs`
- `crates/roko-compose/src/` if it starts consuming the shared types directly

**Verify**:
```bash
cargo test -p roko-core -p roko-agent -p roko-compose
cargo clippy -p roko-core -p roko-agent --no-deps -- -D warnings
```

**Acceptance criteria**:
- the shared response surface is defined in `roko-core`,
- `roko-agent` no longer owns the canonical type definitions,
- at least one downstream crate uses the core-owned types,
- `Usage` includes model attribution or an explicitly equivalent shared mechanism.

---

### G3 — Anthropic HTTP Tool-Loop Backend Coverage

**Owns**: `C.13a`

**Read first**:
- [C-tool-loop.md](C-tool-loop.md)
- [B-provider-system.md](B-provider-system.md)

**Problem**: backend factory coverage is not fully aligned with the provider surface. That weakens the claim that tool-loop execution is a universal HTTP-model path.

**Scope**:
1. Decide the smallest correct Anthropic HTTP tool-loop backend path.
2. Implement the backend or an explicit adapter path into the backend abstraction.
3. Add tests proving backend creation and at least one happy-path turn.
4. Keep the implementation consistent with existing translator/dispatcher behavior.

**Out of scope**:
- provider-wide streaming redesign,
- speculative multi-provider hedging redesign,
- adding research reasoning strategies.

**Files**:
- `crates/roko-agent/src/tool_loop/backends/mod.rs`
- new or existing Anthropic backend module under `crates/roko-agent/src/tool_loop/backends/`
- `crates/roko-agent/src/provider/mod.rs` if factory integration is needed

**Verify**:
```bash
cargo test -p roko-agent tool_loop
cargo clippy -p roko-agent --no-deps -- -D warnings
```

**Acceptance criteria**:
- Anthropic HTTP models are no longer a backend-factory hole,
- backend creation is covered by tests,
- the implementation uses the existing translator/dispatcher stack rather than inventing a side path.

---

### G4 — Universal Tool Path In The Orchestrator

**Owns**: `C.17`, `E.18a`, `F.07b`

**Read first**:
- [C-tool-loop.md](C-tool-loop.md)
- [E-routing-temperament.md](E-routing-temperament.md)
- [D-lifecycle-infrastructure.md](D-lifecycle-infrastructure.md)

**Problem**: `roko run` uses the good agent path; `orchestrate.rs` does not. That means plan execution bypasses the dispatcher, the safety layer, and the monitorable tool loop.

**Scope**:
1. Identify one production plan-execution path in `orchestrate.rs` that should use `ToolLoopAgent` / `ToolDispatcher`.
2. Route tool-capable HTTP-backed agents through the same safety/dispatcher stack already used in `run.rs`.
3. Attach `MetacognitiveMonitor` on that path if the `ToolLoop` is now the owner.
4. Preserve compatibility for paths that should stay direct, such as CLI-native subprocess agents, unless a small safe bridge exists.

**Out of scope**:
- forcing every provider into one identical path,
- rewriting all of `orchestrate.rs`,
- removing Claude CLI direct execution if that widens scope.

**Files**:
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-cli/src/agent_spawn.rs`
- `crates/roko-agent/src/provider/mod.rs`
- `crates/roko-agent/src/tool_loop/agent_wrapper.rs`

**Verify**:
```bash
cargo test -p roko-cli -p roko-agent
cargo run -p roko-cli -- plan run plans/ --dry-run
rg -n "ToolDispatcher|ToolLoopAgent|MetacognitiveMonitor" crates/roko-cli crates/roko-agent
```

**Acceptance criteria**:
- at least one production orchestrator path uses `ToolDispatcher`,
- tool execution on that path passes through the safety layer,
- `ToolLoopAgent` is runtime-reachable from plan execution,
- `MetacognitiveMonitor` is attached there or the exact reason it remains deferred is documented.

---

### G5 — Tool Count Hardening

**Owns**: `C.36`

**Read first**:
- [C-tool-loop.md](C-tool-loop.md)

**Problem**: model profiles expose `max_tools` / degrade caps, but the runtime never enforces them. Smaller models can therefore be handed tool sets the code already knows are harmful.

**Scope**:
1. Choose the enforcement point for tool-count caps.
2. Enforce the cap deterministically.
3. Add tests for cap propagation and truncation.
4. Document what happens when more tools are available than allowed.

**Out of scope**:
- semantic tool ranking,
- ToolRAG,
- graph-based tool recommendation.

**Files**:
- `crates/roko-agent/src/translate/capability.rs`
- `crates/roko-core/src/tool/discovery.rs`
- any `ToolLoop` or translator call sites that need cap enforcement

**Verify**:
```bash
cargo test -p roko-agent -p roko-core
cargo clippy -p roko-agent -p roko-core --no-deps -- -D warnings
```

**Acceptance criteria**:
- configured max-tool caps affect runtime behavior,
- the truncation order is deterministic and tested,
- the batch does not claim semantic relevance ranking if it only enforces a cap.

---

### G6 — Creation-Site Safety Cleanup

**Owns**: `D.14b`

**Read first**:
- [D-lifecycle-infrastructure.md](D-lifecycle-infrastructure.md)

**Problem**: most creation sites now route through scoped helpers, but the research entrypoints still call `create_agent_for_model` directly.

**Scope**:
1. Replace the remaining direct research-path creation calls with `spawn_agent_scoped` or an equivalent scoped helper.
2. Audit remaining `create_agent_for_model` call sites in CLI entrypoints.
3. Leave specialty non-Agent HTTP clients alone unless they are clearly incorrect.

**Out of scope**:
- wiring pools into runtime,
- broad entrypoint redesign,
- converting specialty embed/search clients into general agents.

**Files**:
- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/agent_spawn.rs`

**Verify**:
```bash
cargo test -p roko-cli -p roko-agent
rg -n "create_agent_for_model\\(" crates/roko-cli/src
```

**Acceptance criteria**:
- the remaining research agent creation paths use scoped safety helpers,
- any intentional direct calls are documented and justified,
- the patch does not widen into pool or orchestrator ownership work.

---

### G7 — Typed Temperament Foundation

**Owns**: `E.10`

**Read first**:
- [E-routing-temperament.md](E-routing-temperament.md)
- [A-core-abstractions.md](A-core-abstractions.md)

**Problem**: temperament exists in docs and as a string field in `AgentIdentity`, but there is no shared typed config/runtime contract.

**Scope**:
1. Add a shared `Temperament` enum in a crate that downstream users can depend on.
2. Add a config field with a clear defaulting strategy.
3. Replace free-form string temperament usage where practical.
4. Thread the type through agent identity / spawn surfaces without changing behavior yet.

**Out of scope**:
- routing or gate tuning logic,
- per-provider temperature policy redesign,
- opinionated temperament presets for every role.

**Files**:
- `crates/roko-core/src/`
- `crates/roko-core/src/config/schema.rs`
- `crates/roko-agent/src/introspection.rs`
- `crates/roko-cli/src/*` where agent options are assembled

**Verify**:
```bash
cargo test -p roko-core -p roko-agent -p roko-cli
cargo clippy -p roko-core -p roko-agent -p roko-cli --no-deps -- -D warnings
```

**Acceptance criteria**:
- a shared `Temperament` type exists,
- config can express it,
- `AgentIdentity` and agent creation surfaces no longer rely on an unstructured string,
- existing configs still load or fail with a clear migration message.

---

### G8 — Temperament Runtime Propagation

**Owns**: `E.11`, `E.13`, `E.14`

**Read first**:
- [E-routing-temperament.md](E-routing-temperament.md)
- [B-provider-system.md](B-provider-system.md)
- [C-tool-loop.md](C-tool-loop.md)

**Problem**: a typed temperament only matters if it changes runtime behavior in bounded, testable ways.

**Scope**:
1. Map temperament to at least one model-parameter or effort-setting decision.
2. Map temperament to at least one tool-selection or tool-cap behavior.
3. Map temperament to at least one router behavior, such as exploration strength or escalation aggressiveness.
4. Add tests proving the behavior changes.

**Out of scope**:
- full gate-threshold semantics by temperament,
- large review-depth redesign,
- meta-routing and anti-monoculture systems.

**Files**:
- `crates/roko-agent/src/provider/mod.rs`
- `crates/roko-agent/src/translate/capability.rs`
- `crates/roko-learn/src/model_router.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-cli/src/orchestrate.rs`

**Verify**:
```bash
cargo test -p roko-agent -p roko-learn -p roko-cli -p roko-core
cargo clippy -p roko-agent -p roko-learn -p roko-core --no-deps -- -D warnings
```

**Acceptance criteria**:
- temperament affects at least one live model-setting decision,
- temperament affects at least one live tool or routing decision,
- tests make the behavior discoverable,
- gate strictness work that really belongs in `04` is explicitly deferred rather than improvised.
