# Refinement Audit Runner — Batch AUD07

Run id: run-20260417-214125
Attempt: 1
Model: gpt-5.4
Reasoning: high

## Shared Context Pack

### 00-AUDIT-RULES

# Audit Application Rules

You are applying refinement-audit critiques to Roko's documentation and tooling.
The audit found that the refinements were "directionally correct but 5-10x overscoped."

## Core Principles

1. **The diagnosis is correct, the prescription was overscoped.** Ship what matters.
2. **Split "exists" from "planned."** Never describe unbuilt features in present tense.
3. **Narrow, don't delete.** Move overscoped content to "future work" sections.
4. **Fix factual errors.** Update LOC counts, route counts, crate counts, status labels.
5. **Reduce jargon inflation.** If a concept has 0 lines of code, it's a research hypothesis.

## Verdicts to Apply

- `keep` → Polish wording. Strengthen evidence. Keep it.
- `narrow` → Reduce scope. Add "aspirational" or "target-state" caveats.
- `defer` → Move to explicit future-work section with a clear label.
- `rewrite` → Reframe per the audit's specific guidance. Don't just edit — rethink.

## Factual Corrections (from codebase reality check)

- Total Rust LOC: 322,088 (not 177K)
- Workspace members: 36 (not 18)
- roko-serve routes: 200+ (not ~85)
- TUI: 58K LOC (wired, not "text-mode only")
- roko-learn: 42 modules, 35,847 LOC
- Event bus event types: exactly 2 (PlanRevision, PrdPublished)
- Pulse/Datum/Demurrage/Worldview/Custody: 0 lines of code each

## 5 Aspirational Concepts with 0 Code

These MUST be labeled as "target-state" or "planned" in docs, never described as existing:
1. Pulse (ephemeral event type)
2. Datum (medium polymorphism enum)
3. Demurrage (knowledge decay economic model)
4. Worldview (heuristic cluster)
5. Custody (chain-of-custody record)

### 01-PRIORITY-QUEUE

# Priority Queue

From the audit master summary — this is the recommended priority order.

## Ship Now (1-2 weeks total)

1. Add HDC fingerprint field to Engram — `roko-core/src/engram.rs` — 1 day
2. Unify event enums into `RokoEvent` — across 4 crates — 1 week
3. Add generic `Bus<E>` trait to roko-core — ~100 lines — 2-3 days
4. Clean up stale "Signal" references — traits.rs, README, kind.rs — 1 hour
5. Fix architecture INDEX status — `docs/00-architecture/INDEX.md` — 30 min

## Ship Soon (next month)

6. CLI parity / muscle memory (REF28)
7. StateHub hardening (REF26)
8. Heuristic calibration struct (REF14)
9. Safety: extend Attestation + expand taint (REF32)
10. Threat model doc (REF32 §13)

## Defer

- Pulse type, Datum enum, Operator generalization
- Demurrage, Plugin SPI tiers 4-5, 3 new kernel crates
- All 5 rewrite candidates, SvelteKit web UI, gRPC
- 12-month roadmap timeline

## Wrong (needs correction in docs)

- Synergy matrix (7/10 primitives don't exist)
- REF32 ignores existing safety system
- Glossary marks EventBus as "retired" (it's the only live transport)
- "Moat" framing (2/10 components exist fully)
- Doc INDEX says serve/TUI "not wired" (both definitively wired)

### 02-DOCS-TREE-MAP

# Docs Tree Map

The canonical documentation lives at `docs/`. Here is the full structure:

```
docs/
├── 00-architecture/        # 33+ files; kernel + trait system + analysis + design principles
├── 01-orchestration/       # Plan DAG, execution, plan runner
├── 02-agents/              # Agent dispatch, backends, sidecar
├── 03-composition/         # Prompts, context assembly, templates, budgets
├── 04-verification/        # Gates, validation, 7-rung pipeline
├── 05-learning/            # Self-learning loops, episodes, playbooks, experiments
├── 06-neuro/               # HDC, knowledge store, distillation, tier progression
├── 07-conductor/           # Event watchers, circuit breaker, diagnosis
├── 08-chain/               # On-chain primitives, ChainBus (Phase 2+)
├── 09-daimon/              # Behavior primitives (Phase 2+)
├── 10-dreams/              # Sleep-time compute, consolidation (Phase 2+)
├── 11-safety/              # Role auth, provenance, attestation, taint
├── 12-interfaces/          # CLI, HTTP API, TUI, Web UI, chat
├── 13-coordination/        # Stigmergy, coordination theory, c-factor
├── 14-identity-economy/    # Identity, economic models
├── 15-code-intelligence/   # Parser, indexing, HDC graphs
├── 16-heartbeat/           # Reactive/reflective loops, timing, CoALA mapping
├── 17-lifecycle/           # Agent lifecycle, shutdown
├── 18-tools/               # Tool system, plugin SPI
├── 19-deployment/          # Containers, orchestration, observability
├── 20-technical-analysis/  # Architecture audit, moat analysis, innovations
├── 21-references/          # Bibliography, research papers
├── INDEX.md                # Top-level index
├── STATUS.md               # Current wiring status
├── BENCHMARKS.md           # Performance data
└── CLI-REFERENCE.md        # Command documentation
```

## Key files you'll likely need to edit

- `docs/00-architecture/INDEX.md` — master architecture index (stale status claims)
- `docs/00-architecture/01-naming-and-glossary.md` — canonical glossary
- `docs/00-architecture/15-crate-map.md` — crate dependency graph
- `docs/00-architecture/31-implementation-readiness-audit.md` — readiness status
- `docs/INDEX.md` — top-level doc index
- `docs/STATUS.md` — current wiring status table

## What the refinements-runner already changed

The first pass (`tmp/refinements-runner/`) landed 35 batches (REF01-REF35) that introduced
new concepts (Pulse, Bus, Datum, demurrage, etc.) into the docs. Many of these concepts
have ZERO lines of code. The audit found that the docs now describe aspirational
architecture as if it exists. Your job is to fix that.

### 03-WORKSPACE-TOPOLOGY

# Workspace Topology

Roko is a Rust workspace at `/Users/will/dev/nunchi/roko/roko/`.

## Crate map (36 workspace members)

| Crate | Path | LOC | Status |
|---|---|---|---|
| roko-core | `crates/roko-core/` | kernel | Stable — Engram + 6 traits + config + tools |
| roko-agent | `crates/roko-agent/` | large | 8 LLM backends, pools, MCP, tool loop, safety |
| roko-agent-server | `crates/roko-agent-server/` | medium | Per-agent HTTP sidecar, real LLM dispatch |
| roko-serve | `crates/roko-serve/` | 30K | HTTP control plane, 200+ routes, SSE, WebSocket |
| roko-orchestrator | `crates/roko-orchestrator/` | medium | Plan DAG, parallel executor, merge queue |
| roko-gate | `crates/roko-gate/` | medium | 11 gates, 7-rung pipeline, adaptive thresholds |
| roko-compose | `crates/roko-compose/` | medium | Prompt assembly, 9 templates, enrichment |
| roko-conductor | `crates/roko-conductor/` | medium | 10 watchers, circuit breaker, diagnosis |
| roko-learn | `crates/roko-learn/` | 36K | 42 modules: episodes, playbooks, bandits, routing, experiments |
| roko-cli | `crates/roko-cli/` | 17K+ | CLI binary + ratatui TUI (58K LOC total) |
| roko-fs | `crates/roko-fs/` | small | FileSubstrate (JSONL), GC, layout |
| roko-std | `crates/roko-std/` | medium | Defaults, 19 builtin tools, mock dispatcher |
| roko-runtime | `crates/roko-runtime/` | medium | ProcessSupervisor, event bus, cancellation |
| roko-primitives | `crates/roko-primitives/` | small | HDC vectors (10,240-bit), tier routing |
| roko-neuro | `crates/roko-neuro/` | medium | Durable knowledge store, distillation, tiers |
| roko-mcp-code | `crates/roko-mcp-code/` | medium | Code-intelligence MCP server |
| roko-index | `crates/roko-index/` | medium | Parser + graph + HDC indexing |
| roko-lang-* | `crates/roko-lang-*/` | small | Language support (rust, typescript, go) |
| roko-dreams | `crates/roko-dreams/` | small | Offline consolidation (Phase 2+) |
| roko-daimon | `crates/roko-daimon/` | small | Behavior primitives (Phase 2+) |
| roko-chain | `crates/roko-chain/` | small | Chain witness primitives (Phase 2+) |

## Key numbers (from codebase audit)

- Total Rust LOC: 322,088
- Workspace members: 36
- Test functions: 3,761
- orchestrate.rs: 17,087 lines
- Event bus event types: exactly 2 (PlanRevision, PrdPublished)
- Signal→Engram rename: 99.6% complete

## Concepts with 0 lines of code

These exist ONLY in docs, not in any crate:
- Pulse, Datum, Demurrage, Worldview, Custody
- roko-bus, roko-hdc (as separate crate), roko-spi
- Bus trait (as a formalized kernel trait)

### 04-DELEGATION-GUIDANCE

# Delegation Guidance

You are explicitly authorized to use multiple subagents for this batch.
Use them where it helps, but keep the immediate blocking work local.

## Required delegation behavior

- Before editing, form a short plan and identify 2-4 concrete subtasks.
- Spawn explorers for targeted codebase/docs reads and workers for bounded edits.
- Give each worker a disjoint write scope — no two workers edit the same file.
- Do not wait idly for subagents if you can progress locally.
- If subagents are unavailable in this environment, continue locally without failing.

## Reading files

Before editing any file, READ IT FIRST. You are working in a git worktree
that contains the full repository. Use your file-reading capabilities to
inspect the current state of any file before modifying it.

## Phase-specific guidance

### Phase 1 (AUD* batches) — Docs only
- Only edit files under `docs/`. Never touch `crates/`, `tmp/`, or `src/`.
- Read the target docs before editing to understand their current state.
- The refinements-runner already made changes — you are refining those changes.

### Phase 2 (PU* batches) — Parity content refresh
- Only edit files under `tmp/docs-parity/NN/`.
- Read the current `docs/` tree first to understand what the audit pass changed.
- Update context-pack/, BATCHES.md, 00-INDEX.md, and all batch detail .md files.
- Update the run-docs-parity.sh script if its batch descriptions or verify
  commands reference stale content.

### Phase 3 (PE* batches) — Code execution
- Edit files under `crates/` to implement what the parity docs describe.
- Read BATCHES.md and 00-INDEX.md from the parity section FIRST.
- Search before writing: `grep -rn 'Name' crates/ --include='*.rs' | grep -v target/`
- Wire existing code — do not reimplement what already exists.
- Run `cargo check` after changes to verify compilation.

## Audit Source Files

These are the critique/triage documents that drive your edits.
Read them carefully — they contain specific verdicts (keep/narrow/defer/rewrite)
and codebase reality checks.

--- BEGIN 06-codebase-reality-check.md ---

# Codebase Reality Check: Refinements vs. Actual Code

**Date**: 2026-04-17
**Auditor**: Automated code audit (Opus 4.6)
**Method**: Grep, read, and compile-check across all 32 workspace crates (~322K LOC, 3,761 tests)
**Build status**: `cargo check --workspace` passes cleanly (one unused-mut warning in mirage-rs)

---

## 1. Signal -> Engram Rename

**Refinement claim**: "877:5 ratio, essentially complete."

**Reality**: **Confirmed, mostly accurate.** The actual ratio today is ~926 Engram occurrences
across 134 files vs. 11 "Signal" occurrences across 6 files. The remaining 11 Signal references
break down as:

| File | Count | What it actually is |
|------|-------|---------------------|
| `roko-runtime/src/process.rs` | 5 | Unix `nix::Signal` (SIGTERM/SIGKILL) -- unrelated to the rename |
| `roko-neuro/src/context.rs` | 2 | Enum variant `SourceFamily::Signal` and match arm -- **straggler** |
| `roko-compose/src/system_prompt_builder.rs` | 1 | String literal `"Signal"` in a prompt template -- **straggler** |
| `roko-compose/src/context_provider.rs` | 1 | String literal `"Signal"` in a prompt template -- **straggler** |
| `roko-cli/src/plan_generate.rs` | 1 | Test assertion: `prompt.contains("Signal -> Engram")` -- meta-reference |
| `roko-cli/src/tui/views/dashboard_view.rs` | 1 | UI column header `"Signal"` -- **straggler** |

**Verdict**: 4 real stragglers out of 926+ usages. The rename is ~99.6% complete. The 4
stragglers are cosmetic (string literals in prompts and UI headers), not structural. No
`struct Signal` or `type Signal` remains anywhere in the codebase. The core type in
`roko-core/src/engram.rs` is `pub struct Engram` with BLAKE3 content-hashing, decay functions,
lineage DAG, multi-axis scoring, and attestation support (451 lines, 15 tests). This is a
real, fully-fleshed data type, not a stub.

---

## 2. Event Bus

**Refinement claim**: Bus should be promoted to a kernel trait as a "second fabric" alongside
Substrate. Refinement 03 proposes `trait Bus: Send + Sync` as a first-class kernel operator.

**Reality**: The event bus at `crates/roko-runtime/src/event_bus.rs` (428 lines, 9 tests) is:

- A **generic typed broadcast** using `tokio::sync::broadcast` + a bounded `VecDeque` replay ring
- Parameterized over `E: Clone + Send + Sync + 'static`
- Has `emit()`, `subscribe()`, `replay_from(seq)`, `sender()` (clonable producer handle)
- Has a concrete `RokoEvent` enum with 2 variants: `PlanRevision` and `PrdPublished`
- Has a `global_event_bus()` singleton via `OnceLock`

**What it actually does today**: It is a simple, well-built pub/sub with replay. It has exactly
two event types, both of which are wired:

1. `PlanRevision` -- emitted when gate failures exhaust retry budget (carries failing verdicts,
   log tail, reason)
2. `PrdPublished` -- emitted when a PRD is promoted to published state (carries slug, path,
   origin)

**Is it a "second fabric"?** No. Today it is a utility for two specific cross-cutting concerns:
plan revision events and PRD lifecycle hooks. It does not carry the volume, variety, or
real-time streaming characteristics that the refinements describe for Pulse traffic. The
refinements envision the Bus carrying agent turn events, gate verdicts, telemetry, heartbeats,
and live observability streams. Today it carries exactly 2 event types.

**Does it need to be a kernel trait?** The honest answer: not yet. What exists is a
well-designed building block (~260 lines of real code), but promoting it to a kernel trait
means every `roko-core` consumer would depend on the bus semantics. Today, `roko-runtime`
depends on `roko-core`, not the reverse. Making `Bus` a kernel trait inverts that
dependency, which is the right direction architecturally but requires moving the bus
implementation into `roko-core` or creating a new `roko-bus` crate that both `roko-core`
and `roko-runtime` depend on.

**Recommendation**: The bus is ready to grow. It is not ready to become a kernel trait in the
current crate graph without restructuring. The refinements' vision is sound but the path
requires either: (a) extracting `EventBus<E>` into a standalone `roko-bus` crate that
`roko-core` can depend on, or (b) moving the bus implementation directly into `roko-core`.
Option (a) is cleaner.

---

## 3. Layer Violations

**Refinement claim**: One confirmed dependency violation exists: `roko-conductor` (L3/L4) depends
on `roko-learn` (L2/cross-cut).

**Reality**: **Confirmed.** `roko-conductor/Cargo.toml` has `roko-learn = { path = "../roko-learn" }`
as a direct dependency. Grepping the conductor source shows the violation is narrow:

```
crates/roko-conductor/src/watchers/context_window_pressure.rs
    use roko_learn::efficiency::AgentEfficiencyEvent;
```

That's it. One import, in one watcher, for one struct. The conductor needs
`AgentEfficiencyEvent` to detect context-window pressure from efficiency telemetry. This is
exactly the kind of coupling the Bus proposal dissolves: the conductor should subscribe to
efficiency events through the bus rather than importing the type directly.

**Severity**: Low. The violation is a single struct import. But it demonstrates the pattern the
refinements correctly identify: without a shared event vocabulary on the bus, crates reach
across layers for type definitions.

The doc `docs/00-architecture/23-architectural-analysis-improvements.md` exists and contains a
thorough analysis (it is the doc that identified this violation). Its Section 2.2 proposes the
Bus-first fix for this exact coupling.

---

## 4. HDC Vectors

**Refinement claim**: 10,240-bit HDC fingerprints for episode similarity, code semantic search,
and associative memory.

**Reality**: The HDC implementation at `crates/roko-primitives/src/hdc.rs` (345 lines, 10 tests)
is **real and complete for its stated scope**:

| Feature | Status | Evidence |
|---------|--------|----------|
| 10,240-bit vector (160 x u64) | Built | `bits: [u64; 160]` |
| XOR bind (involution) | Built + tested | `bind()` + `hdc_bind_involution` test |
| Majority-vote bundle | Built + tested | `bundle()` + `hdc_bundle_tie_rule` test |
| Cyclic permutation | Built | `permute()` for sequence encoding |
| Hamming similarity [0,1] | Built + tested | `similarity()` |
| Deterministic seed expansion | Built + tested | `from_seed()` + FNV-1a hash |
| Serde roundtrip (JSON) | Built + tested | Custom `Serialize`/`Deserialize` impls |
| rkyv zero-copy similarity | Built (feature-gated) | `similarity_archived()` behind `#[cfg(feature = "rkyv")]` |
| `fingerprint(value)` | Built | Serializes any `serde::Serialize` -> HDC vector |
| `text_fingerprint(text)` | Built | Direct text -> HDC vector |

**How far from the refinements' vision?** The low-level math is done. What is NOT built:

1. **HDC-based substrate** (`HdcSubstrate` for nearest-neighbor search over a corpus): The trait
   implementation is mentioned in doc comments but no `HdcSubstrate` struct exists.
2. **HDC indexing for code intelligence**: `roko-index` exists (parser + graph + HDC indexing in
   the Cargo.toml description) but its actual HDC usage needs separate audit.
3. **Per-episode HDC fingerprinting**: **Wired.** `roko-learn/src/hdc_fingerprint.rs` provides
   `fingerprint_episode(prompt, outcome)` and it IS called from `orchestrate.rs` (line 2920).
   Episodes get HDC fingerprints attached before logging.
4. **HDC clustering**: `roko-learn/src/hdc_clustering.rs` exists as a module. Not audited
   deeply here but the file exists in the learning crate.

**Verdict**: The HDC primitives are real, tested, and already wired for episode fingerprinting.
The gap is in using HDC for semantic search/retrieval (the substrate side), not in the vector
math itself.

---

## 5. Learning Subsystem Reality

**Refinement claim**: Prediction errors, active inference, continuous calibration, and
cybernetic feedback loops.

**Reality**: The learning subsystem at `crates/roko-learn/src/` is **the most substantial
subsystem in the project** (35,847 lines across 42 modules). Here is what actually exists:

### What is real and wired

| Module | Lines | Wired? | What it does |
|--------|-------|--------|--------------|
| `cascade_router.rs` | 4,784 | Yes | 3-stage model router (static -> confidence -> UCB1 bandit) |
| `runtime_feedback.rs` | 2,720 | Yes | Post-task learning updates: updates router, playbooks, skills |
| `skill_library.rs` | 2,520 | Yes | Extracts, stores, and queries reusable skills from episodes |
| `model_router.rs` | 2,173 | Yes | LinUCB contextual bandit for model selection |
| `episode_logger.rs` | 1,979 | Yes | Append-only JSONL episode records with usage/verdicts |
| `cfactor.rs` | 1,847 | Yes | Collective intelligence factor: fleet-level quality metric |
| `bandits.rs` | 1,727 | Yes | UCB1 and Thompson sampling bandits |
| `playbook_rules.rs` | 1,355 | Yes | Rule-based playbook matching |
| `costs_db.rs` | 1,224 | Yes | Per-model cost tracking database |
| `efficiency.rs` | 1,176 | Yes | Per-turn efficiency events (tokens, latency, cost) |
| `provider_health.rs` | 1,117 | Yes | Circuit breaker for LLM providers |
| `pattern_discovery.rs` | 977 | Yes | Episode pattern mining |
| `playbook.rs` | 976 | Yes | Playbook storage and retrieval |
| `active_inference.rs` | ~200 | Yes | Bayesian belief state + expected free energy tier selection |
| `prediction.rs` | ~200 | Yes | Prediction records with residual tracking |
| `anomaly.rs` | exists | Yes | Runaway loop, cost spike, quality degradation detection |
| `budget.rs` | exists | Yes | Budget guardrails |
| `hdc_fingerprint.rs` | exists | Yes | Episode HDC fingerprint encoding |
| `hdc_clustering.rs` | exists | Partial | HDC-based episode clustering |
| `drift.rs` | exists | Partial | Distribution drift detection |
| `regression.rs` | exists | Partial | Regression detection |

### Active inference reality

The refinements propose "active inference with prediction errors and continuous model updating."
The actual `active_inference.rs` is ~200 lines implementing:

- A 90-state belief distribution (3 difficulty x 3 skill x 10 confidence levels)
- Bayesian belief update after each task observation (success, cost, latency)
- Expected free energy minimization for tier selection (Fast/Standard/Premium)
- Integration with the cascade router (`select_tier_with_belief`)

This is **real active inference**, not a placeholder. It is not Karl Friston's full Free Energy
Principle, but it is a working Bayesian tier selector that updates its beliefs from outcomes.
The code uses proper likelihood computation, normalization, and expected free energy ranking.

### Prediction error tracking

`prediction.rs` implements `PredictionRecord` with explicit fields for:
- `predicted_success_prob`, `predicted_cost_usd`, `predicted_duration_ms`
- `actual_success`, `actual_cost_usd`, `actual_duration_ms`
- `residual_success`, `residual_cost`, `residual_duration`

This is the prediction-error signal the refinements want. It exists. It is wired through the
routing log and calibration tracker.

### What the refinements propose that does NOT exist yet

1. **Automatic reweighting of scorer axes from prediction residuals** -- the residuals are
   computed but not fed back into scorer weights automatically
2. **Cross-agent prediction markets** -- not built
3. **Dreaming-phase automatic consolidation** -- the DreamRunner exists (5,964 lines) and IS
   wired from `orchestrate.rs` (auto-dream triggers after enough episodes), but the
   "imagination" and "hypnagogia" sub-phases are aspirational

**Verdict**: The learning subsystem is the most sophisticated part of the codebase. The gap
between refinement claims and reality is smaller here than anywhere else. Active inference is
real. Prediction tracking is real. The cascade router is real. The missing pieces are second-order
feedback loops (auto-reweighting, cross-agent prediction markets), not the core learning
machinery.

---

## 6. Safety Layer Reality

**Refinement claim (doc 32)**: Comprehensive safety with sandbox isolation, provenance chains,
formal contracts, and behavioral monitoring.

**Reality**: The safety layer at `crates/roko-agent/src/safety/` (4,465 lines across 10 files)
is **substantive and wired**:

### What actually exists

| Module | What it does | Wired? |
|--------|-------------|--------|
| `mod.rs` (SafetyLayer) | Aggregates all policies, `check_pre_execution()` chains them | Yes |
| `bash.rs` | Command allowlist/denylist for bash/run_tests tools | Yes |
| `git.rs` | Branch-protection policy (blocks force-push to main, etc.) | Yes |
| `network.rs` | Outbound destination allowlist (blocks private IPs, SSRF) | Yes |
| `path.rs` | Worktree-relative path canonicalization, escape prevention | Yes |
| `scrub.rs` | Secret scrubbing from outputs (API keys, tokens) | Yes |
| `rate_limit.rs` | Per-tool, per-role rate limiting | Yes |
| `capabilities.rs` | OCaps-style warrants (`AgentWarrant`, `Capability` enum) | Yes |
| `contract.rs` | Declarative behavioral contracts (invariants, governance, recovery) | Yes |
| `contracts/*.yaml` | Role-specific contracts (implementer, researcher, reviewer) | Yes |

The `SafetyLayer::check_pre_execution()` method chains 5 policy checks in order:
1. Role-local tool whitelist
2. Rate limit
3. OCaps warrant check
4. Bash/git command policy
5. Network/path policy

The `AgentWarrant` system implements proper capability tokens with:
- Unforgeable random IDs (`[u8; 32]`)
- Explicit capability grants (Tool, ReadPath, WritePath, Exec, Network)
- Issuer tracking
- Expiry timestamps
- Delegation depth limiting

The `AgentContract` system implements declarative behavioral contracts with:
- Invariants (MaxTokensPerTurn, RequireGateBeforeCommit)
- Governance rules (MaxToolCallsPerTurn, ForbiddenTools, RequireToolBeforeEdit)
- Recovery actions (Abort on contract violation)
- Role-specific contracts loaded from YAML assets

### What the refinements propose beyond current state

1. **Process-level sandboxing (containers, VMs)**: Not built. Safety is enforcement within the
   Rust process, not OS-level isolation.
2. **Formal verification of safety properties**: Not built. Current safety is runtime checks,
   not compile-time proofs.
3. **Provenance chain on-chain attestation**: The `Attestation` type exists in `roko-core`
   (Ed25519 signatures, `ChainAttestation` with chain_id/tx_hash/block_number) but is not
   wired to an actual chain submission path.
4. **Multi-party approval workflows**: Not built.

**Verdict**: The safety layer is real and enforced. It is not a stub. The OCaps warrant system
and declarative contracts are genuinely sophisticated. The gap is in OS-level sandboxing and
on-chain attestation, not in the application-level safety enforcement.

---

## 7. TUI / Serve Reality

### TUI (`crates/roko-cli/src/tui/`)

**Size**: 58,053 lines across 30+ files (plus subdirectories for views, widgets, pages, modals)

**Structure**: Full ratatui application with:
- `app.rs` -- main event loop
- `tabs.rs` -- F1-F7 tab navigation
- `views/` -- dashboard, verdicts, operations views
- `widgets/` -- reusable UI components
- `pages/` -- full-page views
- `modals/` -- popup dialogs
- `theme.rs` -- theming
- `postfx.rs`, `postfx_pipeline.rs` -- post-processing visual effects
- `atmosphere.rs` -- ambient visual effects
- `ws_client.rs` -- WebSocket client for live updates

This is NOT scaffolding. 58K lines of TUI code with WebSocket integration, post-processing
effects, and multiple view modes is a significant built artifact.

### HTTP Serve (`crates/roko-serve/src/`)

**Size**: 30,652 lines across 18 route modules

**Route count**: ~214 route registrations across 18 modules:
- `status.rs` -- 68 routes (health, metrics, signals, episodes, etc.)
- `aggregator.rs` -- 46 routes (fleet-wide aggregation)
- `webhooks.rs` -- 18 routes
- `learning.rs` -- 15 routes
- `prds.rs` -- 13 routes
- `agents.rs` -- 10 routes
- `providers.rs` -- 7 routes
- `deployments.rs` -- 7 routes
- `plans.rs` -- 6 routes
- `research.rs` -- 6 routes
- plus SSE, WebSocket, config, diagnosis, templates, subscriptions, run

The server uses axum with:
- CORS middleware
- API key authentication (configurable)
- Secret-scrubbing middleware layer
- SSE for live streaming
- WebSocket for bidirectional communication
- OpenAPI spec generation

**Verdict**: Both TUI and serve are substantive, not scaffolded. The "~85 routes" claim in the
CLAUDE.md understates the actual count (closer to 200+). Whether all 200+ routes return real
data vs. stubs would require individual route testing, but the infrastructure is undeniably
built.

---

## 8. Orchestration Loop

**Refinement claim (doc 05)**: The nine-step loop should become seven steps: SENSE, ASSESS,
COMPOSE, ACT, VERIFY, PERSIST/BROADCAST, REACT.

**Reality**: Two loops exist in the codebase:

### Loop 1: `roko-core/src/loop_tick.rs` (the pure kernel loop)

A clean 5-step function (158 lines total, 98 lines of implementation):

```
1. Query substrate for candidates
2. Router selects one
3. Composer builds composed signal
4. Gate verifies
5. If passed: persist + run policy reactions
```

This is the canonical `loop_tick()` function that takes all six traits as parameters. It is
used by `roko run <prompt>` via `run.rs`. It is **pure** -- no I/O decisions, no bus, no
learning feedback. This is genuinely elegant.

### Loop 2: `crates/roko-cli/src/orchestrate.rs` (the plan-driven runtime loop)

A **17,087-line** monster that implements the full plan execution lifecycle:

```
1. Discover plans -> build executor DAG
2. Tick executor -> get ExecutorActions
3. Dispatch actions (SpawnAgent, RunGate, etc.)
4. For SpawnAgent: compose system prompt, spawn agent process, collect output
5. For implementing: run gate pipeline (7-rung dispatch)
6. Record episodes, emit efficiency events, update learning
7. Auto-save state, check shutdown signals
8. Repeat until all plans terminal
```

This is NOT the refinements' seven-step loop. It is a plan-execution runtime that happens to
use the six traits internally. The actual flow is:

- `PlanRunner::run_all()` -- outer loop with tick/dispatch/autosave
- `dispatch_action()` -- big match on ExecutorAction variants
- `handle_implementing()` -- spawns agent, runs gate pipeline
- `run_gate_pipeline()` -- 7-rung gate dispatch with adaptive thresholds
- Episode logging, efficiency events, learning updates at each step
- Dream consolidation (auto-dream) triggered after enough episodes
- Daimon affect updates after each task outcome
- Conductor watcher loop running in parallel

The orchestrate.rs file is also where ALL the cross-crate wiring happens. It imports from
18+ crates and orchestrates their interaction. This is where the "WIRE, don't build"
principle manifests: the individual crates are clean, and orchestrate.rs is the integration
point.

**How does this compare to the refinements' seven-step loop?**

| Refinement Step | Current Implementation |
|-----------------|----------------------|
| 1. SENSE | `Substrate.query` in `loop_tick`; no Bus subscription in the kernel loop |
| 2. ASSESS | `Router.select` in `loop_tick`; cascade router in orchestrate.rs |
| 3. COMPOSE | `Composer.compose` in `loop_tick`; system prompt builder in orchestrate.rs |
| 4. ACT | Agent dispatch in orchestrate.rs (not in kernel loop) |
| 5. VERIFY | `Gate.verify` in `loop_tick`; 7-rung pipeline in orchestrate.rs |
| 6. PERSIST | `Substrate.put` in `loop_tick`; JSONL logging in orchestrate.rs |
| 6b. BROADCAST | Event bus emit in orchestrate.rs (2 event types only) |
| 7. REACT | `Policy.decide` in `loop_tick`; conductor watchers in orchestrate.rs |

**Verdict**: The pure kernel loop (`loop_tick`) is clean and could adopt the seven-step
reframing with minimal changes. The plan-execution loop in `orchestrate.rs` is a different
beast -- it is an integration harness, not a cognitive loop. The refinements should be clear
about which loop they are restructuring. The kernel loop is 98 lines and amendable. The
runtime harness is 17K lines and should NOT be restructured to match a conceptual model.

---

## 9. Crate Count and Dependencies

**Current state**: 32 crates + 3 apps + 1 test crate = 36 workspace members

**Workspace members by category**:
- Kernel: `roko-core`, `roko-primitives`, `roko-runtime` (3)
- Standard impls: `roko-std`, `roko-gate`, `roko-fs`, `roko-compose` (4)
- Agent: `roko-agent`, `roko-agent-server` (2)
- Orchestration: `roko-orchestrator`, `roko-conductor` (2)
- Learning: `roko-learn` (1)
- Knowledge: `roko-neuro` (1)
- Behavioral: `roko-daimon`, `roko-dreams` (2)
- CLI/Server: `roko-cli`, `roko-serve` (2)
- MCP: `roko-mcp-stdio`, `roko-mcp-github`, `roko-mcp-slack`, `roko-mcp-scripts`, `roko-mcp-code` (5)
- Chain: `roko-chain` (1)
- Language: `roko-lang-rust`, `roko-lang-typescript`, `roko-lang-go` (3)
- Indexing: `roko-index` (1)
- Plugin: `roko-plugin` (1)
- Demo: `roko-demo` (1)
- Apps: `mirage-rs`, `agent-relay`, `roko-chain-watcher` (3)
- Tests: `tests` (1)

**Is adding 3 more kernel crates (roko-bus, roko-hdc, roko-spi) sane?**

Adding `roko-bus`: **Sane if extracted from roko-runtime.** The EventBus code is ~260 lines of
real implementation. Extracting it would fix the L3->L2 dependency violation and let `roko-core`
depend on bus semantics without importing the full runtime. Cost: one more crate. Benefit:
clean dependency direction.

Adding `roko-hdc`: **Probably unnecessary.** HDC is already in `roko-primitives` (345 lines).
The primitives crate is tiny (3 files: `lib.rs`, `hdc.rs`, `tier.rs`). Extracting HDC into its
own crate would create a crate with one file. The benefit is unclear unless other crates need
HDC without pulling in the tier router.

Adding `roko-spi` (Service Provider Interface): **Questionable without a clear consumer.** The
current plugin system is in `roko-plugin` (already exists). Adding an SPI crate implies a
stable plugin API, which the project is not yet ready to commit to.

**Verdict**: Extract `roko-bus` -- it solves a real dependency problem. Leave HDC in primitives.
Defer SPI until there are external plugin consumers. The project already has 36 workspace
members; adding crates should solve concrete problems, not satisfy taxonomic completeness.

---

## 10. State of the Art: What Works vs. What's Scaffolding

### Definitively works end-to-end (code exists AND is called from CLI)

| Capability | Evidence |
|------------|----------|
| `roko run "<prompt>"` | `run.rs`: compose -> agent -> gate -> persist -> episode |
| `roko plan run <dir>` | `orchestrate.rs`: discover plans -> DAG executor -> parallel dispatch |
| `roko prd idea/draft/plan` | PRD lifecycle with agent-driven drafting and plan generation |
| `roko research topic/enhance-*` | Research agent with citation tracking |
| `roko dashboard` | 58K-line ratatui TUI with F1-F7 tabs and WebSocket |
| `roko serve` | 200+ HTTP routes on :6677 with axum |
| `roko chat --agent <id>` | Per-agent sidecar communication |
| Plan DAG with parallel execution | `ParallelExecutor` in `roko-orchestrator` |
| 7-rung gate pipeline | `rung_dispatch.rs` with adaptive thresholds |
| 8+ LLM backends | Claude CLI, Claude API, Codex, Cursor, OpenAI-compat, Ollama, Gemini, Perplexity |
| MCP config passthrough | `agent.mcp_config` in roko.toml -> `--mcp-config` flag |
| Episode logging with HDC fingerprints | JSONL append + per-episode HDC vector |
| Cascade model router | 3-stage (static -> confidence -> UCB1) with persistence |
| Safety enforcement | 6 policy families + OCaps warrants + declarative contracts |
| Prompt experiment A/B testing | `ExperimentStore` with static overrides |
| Adaptive gate thresholds | EMA per rung in `.roko/learn/gate-thresholds.json` |
| Conductor watchers | 10 watchers running in parallel during plan execution |
| Daimon affect modulation | PAD state, somatic markers, dispatch parameter tuning |
| Dream consolidation | Auto-triggered after N episodes; clusters episodes into knowledge |
| Active inference tier selection | Bayesian belief state + expected free energy routing |
| Prediction error tracking | Residual computation for success/cost/duration |
| C-Factor fleet metric | Collective intelligence score computed and persisted |

### Built but wiring quality varies

| Capability | Status |
|------------|--------|
| On-chain attestation | `Attestation` type with Ed25519 exists; no chain submission path |
| HDC substrate search | `HdcVector` math complete; no `HdcSubstrate` implementation |
| Code intelligence MCP | `roko-mcp-code` exists as a crate; wiring completeness unclear |
| Language providers | `roko-lang-{rust,typescript,go}` exist; usage from main flow unclear |
| Plugin SDK | `roko-plugin` exists with `EventSource`/`FeedbackCollector`; no known plugins |
| Cross-plan dependencies | Code exists in executor; no evidence of cross-plan dep execution |
| Merge queue / post-merge | `PostMergeRunner` exists and is called from orchestrate.rs |

### Phase 2+ (built structures, not wired to runtime)

| Capability | Status |
|------------|--------|
| `roko-chain` chain witness | Primitives exist; no production chain integration |
| `roko-demo` scenario orchestrator | Scaffolded for demo environments |
| `mirage-rs` EVM simulator | Standalone app, optionally bridges to roko-core |
| Full dream cycle (hypnagogia/imagination) | DreamRunner calls `consolidate_now()`; advanced phases are aspirational |

### The honest summary

The codebase is **far more real than typical architecture documents suggest**. The plan-execute-
gate-persist loop genuinely works. Agents actually spawn and produce output. Gates actually
compile, test, and clippy-check code. Episodes are actually logged with HDC fingerprints.
The cascade router actually routes between models using bandit algorithms. The conductor
actually watches for stuck patterns, cost overruns, and ghost turns. The daimon actually
modulates dispatch parameters based on PAD affect state.

The main risks the refinements should be aware of:

1. **orchestrate.rs is 17K lines.** This is the integration hairball. It works, but it is the
   single-most-fragile file in the project. The refinements' architectural improvements
   should focus on decomposing this file, not adding new abstractions.

2. **The kernel loop (`loop_tick`) and the runtime loop (orchestrate.rs) are different things.**
   The refinements sometimes conflate them. `loop_tick` is 98 lines of pure functional code.
   `orchestrate.rs` is 17K lines of imperative integration. Restructuring the kernel loop is
   easy. Restructuring the runtime loop is a multi-month project.

3. **322K LOC across 32 crates is already substantial.** Adding crates has a real cost in
   compile time, dependency management, and cognitive load. Each proposed new crate should
   justify itself against the alternative of adding a module to an existing crate.

4. **3,761 tests exist but test coverage is uneven.** The kernel, primitives, and learning crates
   are well-tested. The CLI/orchestrate/TUI/serve code has fewer tests relative to its size.

5. **The "built but not wired" pattern is mostly resolved.** The CLAUDE.md says "WIRE, don't
   build" because this was historically the main failure mode. Today, most major subsystems
   ARE wired. The remaining unwired pieces are genuinely Phase 2+ (chain integration, full
   dream cycle, external plugins).

--- END 06-codebase-reality-check.md ---

--- BEGIN 07-doc-quality-audit.md ---

# Doc Quality Audit: Refinements Runner Output

**Auditor:** Claude Opus 4.6
**Date:** 2026-04-17
**Branch:** `agent-refinements` (d29d34cf)
**Scope:** 35 refinement proposals (REF01-REF35) propagated into `docs/`

---

## Executive Summary

The refinements runner produced **high-quality structural documentation** with consistent
terminology, coherent voice, and well-linked cross-references. The strongest chapters --
synergy map, safety spine, observability, StateHub, and the consolidated roadmap -- read as
unified design documents, not pasted-in fragments. However, the audit found **three systemic
issues** and a handful of localized problems that should be addressed before these docs become
the source of truth for implementation.

**Overall score: 3.8 / 5** -- good enough to use as planning material, not yet clean enough
to ship as a developer guide or external spec.

---

## 1. docs/INDEX.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/INDEX.md`

### Assessment

The top-level INDEX is **coherent but excessively front-loaded**. Lines 13-161 form a single
growing paragraph that was appended to by successive refinements. Each REF adds another
sentence or clause pointing to a new doc. The result is a 150-line block quote that no
developer will read.

**Specific issues:**

| Issue | Lines | Severity |
|---|---|---|
| Wall-of-text "Current Framing" block with 15+ REF citations | 171-214 | Medium |
| Absolute paths to the user's local machine in generation notes | 251-253 | Low (cosmetic) |
| Topics list descriptions are inconsistent: some include REF numbers, others do not | 224-245 | Low |

### Scores

| Dimension | Score | Notes |
|---|---|---|
| Consistency | 4/5 | Terms match glossary throughout |
| Coherence | 2/5 | The "Current Framing" block is an accretive wall |
| Accuracy | 4/5 | Cross-references are valid |
| Completeness | 5/5 | Every refinement is cited |
| Terminology hygiene | 4/5 | No retired terms in active use |
| Internal links | 5/5 | All sampled links resolve |

---

## 2. docs/00-architecture/ (Architecture)

### 2.1 INDEX.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/00-architecture/INDEX.md`

The architecture INDEX is the best-structured chapter file in the tree. The reading order in
the Prerequisites section (lines 133-142) is genuinely useful. The contents table (lines
83-122) is complete and each entry's description was updated to reflect refinement content.

**Issues found:**

| Issue | File | Lines | Severity |
|---|---|---|---|
| **STALE STATUS: "roko-serve: HTTP API not wired"** | INDEX.md | 206 | **High** |
| **STALE STATUS: "TUI: Text-mode dashboard only, no interactive terminal UI"** | INDEX.md | 208 | **High** |
| Generated date says "2026-04-11" but Sub-docs says 29 when there are now 36 files | INDEX.md | 224-226 | Medium |
| "roko-agent (346 tests)" -- test count is stale and unverified | INDEX.md | 193 | Low |

The high-severity items directly contradict CLAUDE.md, which marks both `roko-serve` (~85
routes, wired) and the interactive TUI (ratatui, F1-F7 tabs) as **Wired**. The refinements
runner did not update the "Current Status and Implementation Gaps" section at the bottom of
this INDEX to reflect reality. This section was generated on 2026-04-11 and has not been
touched since, even though the code has moved significantly.

### 2.2 01-naming-and-glossary.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/00-architecture/01-naming-and-glossary.md`

This is the strongest doc in the set. The A-Z glossary format works well. Every new term from
REF10-REF35 has a corresponding entry. The "Retired / Deprecated Terms" table (lines 613-633)
is explicit and thorough.

**Minor issues:**

- The "Terms Deliberately Not Defined Here" section (lines 636-645) is a nice touch but could
  also mention `trace`, `span`, and `principal` which are used architecturally but not given
  Roko-specific definitions.
- The glossary links to `tmp/refinements/` files extensively. Those are source proposals, not
  canonical docs. Over time these links will rot if the refinements directory moves.

**Score: 4.5/5** -- excellent vocabulary discipline.

### 2.3 34-synergy-integration-map.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/00-architecture/34-synergy-integration-map.md`

This is one of the best-written docs in the set. The 10-primitive table, the synergy matrix,
and the 10 named synergies section all read as a single coherent voice. The "Non-Synergies
Worth Naming" section (section 8) is unusually honest for generated documentation.

**No issues found.** This doc is ready to use as-is.

**Score: 5/5**

### 2.4 35-consolidated-roadmap.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/00-architecture/35-consolidated-roadmap.md`

Well-structured Q1-Q4 breakdown with clear dependency ladder. The "Not-Doing List" (section 7)
and "One-Year Outcome" (section 8) are useful framing.

**Issues:**

| Issue | Lines | Severity |
|---|---|---|
| Q5-Q6 section header says "Phase 2 Optionality" -- unclear if these are real quarters or metaphorical | 166-178 | Low |
| Team shape section (lines 187-200) assumes 5-7 engineers; unclear if this matches project reality | 187-200 | Low |

**Score: 4/5** -- solid planning doc.

### 2.5 15-crate-map.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/00-architecture/15-crate-map.md`

**Accuracy issue (Medium severity):** The crate map describes `roko-bus`, `roko-hdc`, and
`roko-spi` as "target kernel crates" that do not yet exist. This is **correctly qualified**
throughout: "target boundaries proposed by REF20, not all fully shipped" (line 319), "New
target kernel crate" (lines 76-77). The same is true for `roko-defaults`, `roko-tools`,
`roko-compose-core`, and `roko-templates` -- none of these exist as separate crates yet.

Verified against actual workspace:

- `roko-bus` -- **does not exist**
- `roko-hdc` -- **does not exist**
- `roko-spi` -- **does not exist** (though `roko-plugin` does exist)
- `roko-defaults` -- **does not exist**
- `roko-tools` -- **does not exist**
- `roko-compose-core` -- **does not exist**
- `roko-templates` -- **does not exist**

The doc is honest about this gap. However, other docs reference these crates without the same
qualification (e.g., `docs/00-architecture/12-five-layer-taxonomy.md` line 221: "roko-core,
roko-bus, roko-hdc, and roko-spi are the only kernel-tier crates" -- present tense, not
qualified as target). This creates a consistency problem across the architecture chapter.

**Score: 4/5** -- good doc, but the current-vs-target boundary bleeds in adjacent files.

---

## 3. docs/05-learning/ (Learning)

### 3.1 INDEX.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/05-learning/INDEX.md`

The INDEX reads well as a unified story. The overview (lines 11-19) integrates REF10, REF12,
REF14, and REF16 coherently: prediction loops, demurrage, heuristics, and research-to-runtime
are woven into a single narrative rather than listed as separate additions.

**Issues:**

| Issue | Lines | Severity |
|---|---|---|
| "four durable learning surfaces" (line 19) then lists parenthetical "(episodes -> patterns -> heuristics/worldviews -> playbook projections)" which is actually 4 items, consistent | 19 | None |
| Rust code blocks in cross-cutting concerns section use types like `HdcVector`, `DifficultyModel` that exist only in the PRD, not the codebase | 251-360 | Medium |
| No explicit "future work" disclaimer on the Rust code blocks | 250-360 | Medium |

### 3.2 18-self-learning-cybernetic-loops.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/05-learning/18-self-learning-cybernetic-loops.md`

Reads as one coherent voice. The predict-publish-correct loop is explained clearly. The
per-operator calibration table (lines 36-43) is useful and concise.

**No significant issues.** The integration of REF10 concepts is natural, not pasted in.

**Score: 4.5/5**

### 3.3 19-heuristics-worldviews-and-falsifiers.md

**File:** `/Users/well/dev/nunchi/roko/roko/docs/05-learning/19-heuristics-worldviews-and-falsifiers.md`

Well-structured, reads as one document. The Rust struct definitions for `Heuristic` and
`Calibration` (lines 41-60) are clear specification material.

**Score: 4/5**

---

## 4. docs/12-interfaces/ (Interfaces)

### 4.1 INDEX.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/12-interfaces/INDEX.md`

The overview paragraph (lines 9) is a single **1,500-character sentence** that tries to cite
REF23, REF24, REF25, REF26, REF27, REF28, REF29, and REF30 all in one breath. This is the
worst accretive-citation problem in the tree. The Generation Notes section (lines 104-157)
then repeats most of the same citations in a separate list format.

**Issues:**

| Issue | Lines | Severity |
|---|---|---|
| Overview paragraph is an accretive mega-sentence citing 8 REFs | 9 | Medium |
| Generation Notes duplicates the same REF citations as separate blocks | 115-157 | Medium |
| Sub-Documents table skips #20 (exists: `20-ide-integration-strategy.md`) | 27-51 | Low |
| Prerequisites table (lines 57-70) uses inconsistent topic numbering: "01-core" for architecture, "05-orchestration" for orchestration (should be 01) | 61-64 | Low |

### 4.2 22-statehub-projection-layer.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/12-interfaces/22-statehub-projection-layer.md`

Excellent doc. The Projection trait (lines 28-41) is a clean spec. The canonical projections
table (lines 58-82) is comprehensive and well-scoped. The query+subscribe API examples (lines
89-95) are useful.

REF30 rich-UX primitives are integrated naturally -- the doc explains how projections supply
the typed data that reasoning streams, uncertainty bars, and replay scrubbers need, without
making those UI concepts feel bolted on.

**Score: 4.5/5**

---

## 5. docs/11-safety/ (Safety)

### 5.1 INDEX.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/11-safety/INDEX.md`

Clean, well-organized spine. REF32 is integrated as the through-line rather than an appendix.
The "Chapter Through-Line" section (lines 73-83) is a useful reading guide.

**Score: 4.5/5**

### 5.2 00-defense-in-depth.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/11-safety/00-defense-in-depth.md`

This doc is genuinely good. The shared permission vocabulary (section 1), seven-step loop
safety mapping (section 4), and defense layers stack (section 6) all read as unified design.
The `AuthzDecision` enum (lines 46-53) is a clean, small spec.

**No issues found.** The integration of REF32 concepts (TypedContext, Custody, taint) is
seamless.

**Score: 5/5**

---

## 6. docs/19-deployment/ (Deployment)

### 6.1 INDEX.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/19-deployment/INDEX.md`

Good structure. The five deployment shapes (laptop, single-server, container, clustered, edge)
are consistent throughout. The Key Concepts section (lines 34-42) integrates Engram, Pulse,
Bus, StateHub, and profiles cleanly.

**Issues:**

| Issue | Lines | Severity |
|---|---|---|
| Cross-references at bottom use vague labels like "Agent Types documentation, section 8" without links | 55-60 | Medium |
| "Synapse Architecture: 6-trait composition system" (line 36) lists the original six traits but no longer reflects Bus as a distinct fabric -- the framing is pre-REF03 | 36 | Low |

**Score: 4/5**

### 6.2 14-observability-and-telemetry.md

**File:** `/Users/will/dev/nunchi/roko/roko/docs/19-deployment/14-observability-and-telemetry.md`

One of the best-integrated refinement docs. REF33 content is woven into a coherent operator
story. The Roko-specific metrics table (lines 109-124) is concretely useful. The
"Replay and Time-Travel" section (lines 181-196) connects deployment concerns to the
Bus/Engram model naturally.

**Score: 5/5**

---

## 7. Systemic Issues

### Issue A: "Signal" still appears in active code and docs (HIGH)

The glossary correctly marks `Signal` as retired in favor of `Engram`. However, **`Signal`
still appears as a live Rust type name** in code snippets across at least 8 docs:

| File | Context |
|---|---|
| `docs/12-interfaces/02-roko-new-scaffolders.md` line 65 | `use roko_core::{Context, Gate, Signal, Verdict};` |
| `docs/12-interfaces/02-roko-new-scaffolders.md` line 88 | `output: &Signal` |
| `docs/12-interfaces/02-roko-new-scaffolders.md` line 109 | `let signal = Signal::builder(Kind::AgentOutput)` |
| `docs/07-conductor/01-watcher-ensemble.md` line 17 | `fn decide(&self, stream: &[Signal], ctx: &Context) -> Vec<Signal>;` |
| `docs/07-conductor/INDEX.md` line 102 | `fn decide(&self, stream: &[Signal], ctx: &Context) -> Vec<Signal>;` |
| `docs/11-safety/15-forensic-ai.md` line 248 | `-> Vec<Signal>` |
| `docs/01-orchestration/00-layer-overview.md` line 65 | `roko_core::Signal` |
| `docs/CLI-REFERENCE.md` lines 116, 1043, 1109 | "Signal hash", "Signal trigger", "Signal kind" |

These docs were NOT updated by the refinements runner because they predate the glossary
changes. The scaffolder doc (02-roko-new-scaffolders.md) explicitly notes "will be renamed to
Engram in Tier 0D" (line 128), which is at least honest. But the conductor, orchestration,
forensic, and CLI-REFERENCE docs use `Signal` without any qualification.

**This is the biggest terminology hygiene gap in the tree.** The refinements correctly updated
all newly-written or REF-touched docs, but the pre-existing docs were not swept.

### Issue B: Target crates described as if they exist (MEDIUM)

The docs reference `roko-bus`, `roko-hdc`, `roko-spi`, `roko-defaults`, `roko-tools`,
`roko-compose-core`, and `roko-templates` across at least 7 files. None of these crates exist
in the workspace. The crate-map doc (`15-crate-map.md`) is honest about this gap, but other
docs are not:

- `docs/00-architecture/12-five-layer-taxonomy.md` line 221 says "roko-core, roko-bus,
  roko-hdc, and roko-spi **are** the only kernel-tier crates" (present tense).
- `docs/00-architecture/INDEX.md` line 99 describes the five-layer taxonomy as including
  "target dep graph boundaries for roko-bus, roko-hdc, roko-spi" -- this is qualified, but the
  table entry for the five-layer doc is not.

### Issue C: Stale implementation status (MEDIUM)

The architecture INDEX `Current Status and Implementation Gaps` section (lines 189-218) was
generated on 2026-04-11 and never updated. Two items are factually wrong as of 2026-04-17:

1. `roko-serve: HTTP API not wired` -- **WRONG.** CLAUDE.md marks this as Wired with ~85
   routes.
2. `TUI: Text-mode dashboard only, no interactive terminal UI` -- **WRONG.** CLAUDE.md marks
   this as Wired with ratatui F1-F7 tabs.

The refinements runner focused on propagating new architecture concepts but did not reconcile
the status section against the actual codebase state.

---

## 8. Copy-Paste Artifacts Check

### Duplicate sections

No exact duplicate sections were found across the sampled docs. The refinements runner did a
good job of propagating concepts without literal copy-paste. Each doc adapts the refinement
content to its own context.

### Inconsistent formatting

The only formatting inconsistency is in the INDEX.md files. The top-level `docs/INDEX.md` has
the accretive "Current Framing" block, and `docs/12-interfaces/INDEX.md` has the accretive
overview paragraph. All other INDEXes are clean.

### Context-free paragraphs

None found. Every added paragraph connects to its surrounding doc.

### Over-long docs

The learning INDEX (`docs/05-learning/INDEX.md`) is 400 lines, which is long but justified by
the Rust code blocks in the cross-cutting concerns section. The architecture INDEX is 246
lines, also reasonable given its role.

---

## 9. Dimension Scores

| Dimension | Score | Key observations |
|---|---|---|
| **Consistency** | 4/5 | New docs use canonical terms; pre-existing docs still use `Signal` |
| **Coherence** | 4/5 | Refinement content reads as one voice in individual docs; INDEX accretion is the main weakness |
| **Accuracy** | 3/5 | Stale status section, target crates described in present tense, code snippets with pre-rename types |
| **Completeness** | 4/5 | All 35 refinements are represented; some docs lack future-work disclaimers on unbuilt features |
| **Terminology hygiene** | 3.5/5 | Glossary is excellent; but ~40 `Signal` references remain in non-retired contexts across 8+ files |
| **Internal links** | 5/5 | All 15+ sampled cross-references resolve to existing files |
| **Copy-paste artifacts** | 5/5 | No duplicates, no context-free fragments, no pasted-in blocks |

**Overall: 3.8 / 5**

---

## 10. Recommended Actions

### P0 (fix before using docs as implementation guide)

1. **Update architecture INDEX status section** (`docs/00-architecture/INDEX.md` lines
   204-218) to reflect that `roko-serve` and TUI are wired. Reconcile against CLAUDE.md.

2. **Sweep `Signal` from pre-existing docs** that were not touched by the refinements runner.
   At minimum: `docs/07-conductor/`, `docs/12-interfaces/02-roko-new-scaffolders.md`,
   `docs/01-orchestration/00-layer-overview.md`, `docs/11-safety/15-forensic-ai.md`, and
   `docs/CLI-REFERENCE.md`.

### P1 (fix before external review)

3. **Qualify target crates in `12-five-layer-taxonomy.md`**: change "are the only kernel-tier
   crates" to "are the target kernel-tier crates" on line 221.

4. **Collapse INDEX.md accretive citations**: rewrite the "Current Framing" block in
   `docs/INDEX.md` (lines 171-214) as a structured table or bulleted list instead of a
   growing paragraph.

5. **Fix `12-interfaces/INDEX.md` overview**: break the 1,500-character sentence into a
   paragraph with clear structure.

6. **Add future-work disclaimer to Rust code blocks** in `docs/05-learning/INDEX.md` cross-
   cutting concerns section (lines 250-360) for types like `DifficultyModel`,
   `CurriculumScheduler`, `ToolUsageProfile` that are not yet implemented.

### P2 (cleanup)

7. **Fix deployment INDEX cross-references** (`docs/19-deployment/INDEX.md` lines 55-60):
   replace vague "Agent Types documentation, section 8" with actual links.

8. **Add `20-ide-integration-strategy.md`** to the interfaces sub-documents table.

9. **Stabilize refinement links**: many docs link to `tmp/refinements/` for source proposals.
   These will rot. Consider adding a note that `tmp/refinements/` is frozen source material.

--- END 07-doc-quality-audit.md ---

## Master Summary (reference)

# Refinements Audit — Master Summary

> **Date**: 2026-04-17 | **Auditor**: Claude Opus 4.6 (7 parallel agents)
> **Scope**: All 35 refinement docs + runner infrastructure + landed doc updates + codebase reality check
> **Output**: 7 detailed audits in this directory (01-foundation through 07-doc-quality)

---

## Executive Verdict

**The diagnosis is correct. The prescription is 5-10x overscoped.**

The refinements correctly identify real problems in the codebase (event enum proliferation, a conductor/learn layer violation, stale "Signal" naming, Policy signature mismatch). But they propose a 6-12 month, 5-7 engineer refactoring program for a single-developer project, introducing ~15 types that don't exist yet (Pulse, Datum, Bus trait, TopicFilter, Demurrage, Custody, Worldview, Claim, Paper, TypedContext, etc.) to solve problems that could be fixed in ~1-2 weeks with targeted changes.

---

## The 5 Things to Ship Now

These emerged consistently across all 7 audit workstreams as high-value, low-risk:

| # | What | Where | Effort | Why |
|---|---|---|---|---|
| 1 | **Add HDC fingerprint field to Engram** | `roko-core/src/engram.rs` | 1 day | HdcVector exists (10,240-bit, tested). Episode fingerprinting already works. This is the single highest-value bridge between the learning and memory layers. |
| 2 | **Unify event enums into `RokoEvent`** | Across 4 crates | 1 week | Four incompatible event enums (2x `AgentEvent`, `RokoEvent`, `ServerEvent`) is the real problem. Unify them. |
| 3 | **Add generic `Bus<E>` trait to roko-core** | `roko-core/src/traits.rs` | 2-3 days | ~100 lines. Keep it generic (not Pulse-specific). Solves the layer violation. |
| 4 | **Clean up stale "Signal" references** | traits.rs, README, kind.rs, CLAUDE.md | 1 hour | 40+ stale occurrences across docs and code comments. |
| 5 | **Fix architecture INDEX status** | `docs/00-architecture/INDEX.md` | 30 min | Says "roko-serve: HTTP API not wired" and "TUI: Text-mode dashboard only" — both factually wrong per CLAUDE.md and code (30K LOC serve, 58K LOC TUI). |

---

## The 5 Things to Ship Soon (next month)

| # | What | Source | Effort |
|---|---|---|---|
| 6 | **CLI parity / muscle memory (REF28)** | UX audit | 1-2 weeks |
| 7 | **StateHub hardening (REF26)** | UX audit | 1 week |
| 8 | **Heuristic calibration struct** | Learning audit (REF14) | 3-5 days |
| 9 | **Safety: extend Attestation + expand taint** | Integrator audit (REF32) | 1 week |
| 10 | **Threat model doc** | Integrator audit (REF32 §13) | 2 days |

---

## The 10 Things to Defer

| What | Why defer |
|---|---|
| **Pulse type** (REF02) | Unified `RokoEvent` enum solves the same problem more simply |
| **Datum enum** (REF04) | Premature abstraction; doubles every trait's surface area |
| **Operator generalization** (REF04) | Only Policy actually needs a signature change |
| **Demurrage** (REF12) | Add `last_used + access_count` to Decay first; skip the full economic model |
| **Plugin SPI tiers 4-5** (REF17) | Zero plugin authors exist. WASM host is premature |
| **3 new kernel crates** (REF20) | roko-bus justified, roko-hdc unnecessary (345 LOC), roko-spi premature |
| **All 5 rewrite candidates** (REF21) | Existing code works. Build incrementally |
| **SvelteKit web UI** (REF29) | Zero frontend code exists. Build when someone asks |
| **gRPC wire protocol** (REF27) | No tonic dependency. WebSocket + SSE already work |
| **12-month roadmap timeline** (REF35) | Calibrated for 5-7 engineers, not 1 developer + AI |

---

## The 5 Things That Are Wrong

| What | Issue | Source |
|---|---|---|
| **Synergy matrix** (REF31) | 7 of 10 "load-bearing primitives" don't exist in code. Matrix is aspirational fiction. | Integrator audit |
| **REF32 ignores existing safety system** | The AgentContract/AgentWarrant/Capability system already exists and works. REF32 proposes replacing it without acknowledging it. | Integrator audit |
| **Glossary marks EventBus as "retired"** | `EventBus<E>` is the only live transport code. No Bus trait or Pulse exists. | Integrator audit |
| **"Moat" framing** (REF18) | Of 10 claimed moat components, 2 exist fully, 2 partially, 6 not at all. The moat is aspirational. | Moat audit |
| **Doc INDEX says serve/TUI "not wired"** | serve has 200+ routes (30K LOC), TUI has 58K LOC with WebSocket. Both are definitively wired. | Doc quality + reality check |

---

## Codebase Reality (Key Numbers)

From the reality-check audit:

| What | Reality |
|---|---|
| Total Rust LOC | 322,088 (not 177K as CLAUDE.md says) |
| Workspace members | 36 (not 18) |
| Test functions | 3,761 |
| orchestrate.rs | 17,087 lines (the integration hairball) |
| roko-serve routes | 200+ (not ~85) |
| TUI code | 58K LOC |
| roko-learn modules | 42 modules, 35,847 LOC |
| Signal→Engram rename | 99.6% complete (4 real stragglers) |
| Event bus event types | Exactly 2 (PlanRevision, PrdPublished) |
| Demurrage in code | 0 lines |
| Pulse in code | 0 lines |
| Worldview in code | 0 lines |

---

## Doc Quality Assessment

Overall: **3.8 / 5**

**Good**: No copy-paste artifacts. Glossary is excellent. Synergy map and safety spine read as unified docs. Cross-references resolve.

**Issues**:
1. "Signal" still used in ~40 places across 8+ pre-existing docs
2. Target crates (roko-bus, roko-hdc, roko-spi) described in present tense as if they exist
3. Architecture INDEX has stale status information contradicting CLAUDE.md

---

## Per-Arc Summary

### Foundation (01-09): PARTIALLY AGREE
The diagnosis is correct. The prescription (Pulse, Datum, generalized operators, 7-step TickConfig) is overcomplicated. Fix: unify events, add generic Bus trait, update docs. ~1 week instead of 6-7 weeks.

### Learning (10-16): SIMPLIFY
The docs undercount what already exists. roko-learn has 42 modules and 36K LOC. HDC fingerprint field on Engram is the highest-value change. Demurrage/worldviews/replication-ledger are premature.

### Moat (17-21): DEFER/SKEPTICAL
Zero plugin authors, zero external users. The moat is aspirational. Plugin tier 3 (tool manifests) is useful later. Everything else waits.

### UX (22-30): Pick 3 of 9
Ship REF28 (CLI parity), REF26 (StateHub), and the chat/init subset of REF23. Defer the four-layer SDK, six domain profiles, SvelteKit UI, gRPC, and rich UX primitives.

### Integrators (31-35): Integrate code, not plans
The synergy matrix, glossary, and roadmap are plans connecting to plans. Ship: threat model, glossary (split into "exists" vs "planned"), dependency ordering. Reject: quarterly timeline, synergy matrix of unbuilt features.

---

## Recommended Priority Queue

For a single developer + AI agents:

1. **Close the self-hosting loop** (CLAUDE.md items 10-11: auto plan generation + feedback loop)
2. Ship the 5 "now" items above
3. Ship the 5 "soon" items above
4. Address ux-followup P0 items (67 items in `tmp/ux-followup/`)
5. Decompose `orchestrate.rs` (17K lines is the real tech debt)
6. Everything else goes into "when the system needs it"

---

## Audit Files

| File | What |
|---|---|
| `01-foundation-audit.md` | REF01-09 vs codebase (28K chars) |
| `02-learning-audit.md` | REF10-16 vs codebase (30K chars) |
| `03-moat-audit.md` | REF17-21 vs codebase (25K chars) |
| `04-ux-audit.md` | REF22-30 vs codebase (25K chars) |
| `05-integrator-audit.md` | REF31-35 vs codebase (23K chars) |
| `06-codebase-reality-check.md` | 10 factual claims verified (27K chars) |
| `07-doc-quality-audit.md` | Landed doc updates quality (18K chars) |

## Refinement Matrix (per-REF verdicts)

# Refinement Matrix

Legend:
- `keep`
- `narrow`
- `defer`
- `rewrite`

| Ref | Title | Verdict | Audit note |
|---|---|---|---|
| REF01 | critique one noun | `keep` | The diagnosis is real: transport is under-modeled and the kernel story is too storage-centric. |
| REF02 | Engram vs Pulse | `keep` | `Pulse` is a good transport noun if used to clarify the redesign rather than force a total renaming campaign. |
| REF03 | Bus as first class | `keep` | This is the strongest foundational follow-up: unify and formalize transport. |
| REF04 | operators generalized | `narrow` | Good local idea, bad universal law. Medium polymorphism should be proven operator by operator. |
| REF05 | loop retold | `keep` | Useful as a reference architecture for the redesign, but should guide migration rather than dictate every interface immediately. |
| REF06 | refactoring plan | `keep` | A phased migration plan is appropriate; keep it honest and code-first. |
| REF07 | naming | `narrow` | Good cleanup instinct, but not every proposed term should become top-level canon immediately. |
| REF08 | code sketches | `narrow` | Helpful as exploratory sketches; should not be confused with settled API design. |
| REF09 | phase-2 implications | `narrow` | Good future map, but it should stay downstream of core runtime wins instead of shaping the first redesign pass. |
| REF10 | self-learning loops | `keep` | Strong direction if centered on calibration, contradiction, and adaptation rather than runtime-wide active-inference doctrine. |
| REF11 | HDC substrate | `narrow` | Keep HDC for retrieval/clustering; defer broader semantic-consensus rhetoric. |
| REF12 | knowledge demurrage | `defer` | Interesting hypothesis, but too early to present as the governing memory model. |
| REF13 | c-factor | `defer` | Worth exploring as coordination health, not yet worthy of strong canonical treatment. |
| REF14 | worldview validation | `narrow` | Keep typed heuristics and contradiction tracking; defer full worldview/dissonance stack. |
| REF15 | exponential scaling | `defer` | Too much product-theory confidence for the current maturity level. |
| REF16 | research-to-runtime | `narrow` | Claim registry and provenance-backed defaults are promising; the full paper economy is premature. |
| REF17 | plugin extension architecture | `keep` | Tiered extensibility is the right platform direction if it stays local-first and resists premature ecosystem ambition. |
| REF18 | competitive moat | `defer` | Too much architecture-theater and future-ecosystem assumption. |
| REF19 | net-new innovations | `rewrite` | The catalog format oversells speculative pieces; convert to research hypotheses or remove. |
| REF20 | modularity composability | `keep` | Crate-boundary cleanup and clearer seams are real needs. |
| REF21 | from-scratch redesigns | `narrow` | Useful as a pressure test and cleanup lens, but dangerous as the default implementation mindset. |
| REF22 | developer UX rust | `keep` | Strong redesign target if the SDK is kept crisp and optimized for time-to-first-agent rather than feature taxonomy. |
| REF23 | user UX running agents | `keep` | Strong target-state direction if parity follows a real shared session model instead of surface symmetry for its own sake. |
| REF24 | deployment UX | `keep` | Strong operator-centered direction; needs stricter sequencing and fewer assumptions bundled into the first wave. |
| REF25 | domain-specific agents | `keep` | Domain profiles are a strong packaging abstraction as long as bundles stay ahead of universal type formalism. |
| REF26 | StateHub rearchitecture | `keep` | One of the best proposals. Evolve the existing dashboard hub into real projections. |
| REF27 | realtime event surface | `keep` | Unification is the right target, but the contract should stay small: events, replay, filters, subscriptions. |
| REF28 | CLI parity familiar workflows | `keep` | Familiar-first is right if parity is earned from shared workflow semantics rather than copied command names. |
| REF29 | web UI architecture | `keep` | A web surface is a good redesign goal if it starts as an ops console and grows from projection contracts. |
| REF30 | rich UX primitives | `narrow` | Some primitives are valuable, but only when supported by real shared state and telemetry contracts. |
| REF31 | synergy integration map | `defer` | Fine as internal coherence tooling; too grand as canonical architecture backmatter. |
| REF32 | safety sandbox provenance | `keep` | Strong direction if safety remains a compact enforceable spine rather than an all-at-once governance superstructure. |
| REF33 | observability telemetry | `keep` | Strong direction if the signal set stays operator-useful and avoids speculative overmodeling. |
| REF34 | glossary | `rewrite` | Keep one glossary, but split current canon from target-state proposals. |
| REF35 | consolidated roadmap | `rewrite` | Keep sequencing discipline, but narrow the number of simultaneous deep bets and remove unearned quarter-level certainty. |

## Aggregated view

### Clear keeps

- REF01
- REF02
- REF03
- REF05
- REF06
- REF10
- REF17
- REF20
- REF22
- REF23
- REF24
- REF25
- REF26
- REF27
- REF28
- REF29
- REF32
- REF33

### Strong, but should be narrowed

- REF04
- REF07
- REF08
- REF09
- REF11
- REF14
- REF16
- REF21
- REF30

### Better deferred

- REF12
- REF13
- REF15
- REF18
- REF31

### Need substantive rewrite

- REF19
- REF34
- REF35

## Practical consequence

The refinement set should not be treated as a monolithic "land it all" bundle.
The right next pass is:

1. Preserve the `keep` items.
2. Rewrite the `narrow` items around smaller scope and less doctrinal force.
3. Move the `defer` items into explicit future-work or research-hypothesis sections.
4. Rebuild the `rewrite` items so they stop acting as authority multipliers for
   architecture that is still too speculative or too overloaded.

# Batch AUD07: Fix codebase reality-check errors across ALL docs

**Audit refs**: 06-codebase-reality-check.md (full file), 07-doc-quality-audit.md
(Issue A: Signal references, Issue B: target crates, Issue C: stale status).
This is the broad sweep batch -- it touches many files to fix factual errors
that span the entire docs tree.

Read these files first:

- `tmp/refinement-audit-runner/context-pack/00-AUDIT-RULES.md`
- `tmp/refinements-audit/06-codebase-reality-check.md` (full file -- 10 reality checks)
- `tmp/refinements-audit/07-doc-quality-audit.md` (Issues A, B, C + section 10 Recommended Actions)
- `docs/00-architecture/01-naming-and-glossary.md` (retired terms table)
- `CLAUDE.md` (ground truth for what is wired)

For the Signal sweep, also read:

- `docs/12-interfaces/02-roko-new-scaffolders.md`
- `docs/07-conductor/01-watcher-ensemble.md`
- `docs/07-conductor/INDEX.md`
- `docs/11-safety/15-forensic-ai.md`
- `docs/01-orchestration/00-layer-overview.md`
- `docs/CLI-REFERENCE.md` (if it exists at `docs/CLI-REFERENCE.md`)

## Task

Fix factual errors found by the codebase reality check across the entire docs
tree. Three categories: (1) Replace stale `Signal` type references with
`Engram`, (2) Fix incorrect LOC/crate/route counts wherever they appear,
(3) Mark 0-code concepts as planned wherever they are presented in present
tense.

## Current state (evidence)

### Category 1: Signal -> Engram stragglers

The doc quality audit found `Signal` still used as a live Rust type in code
snippets across at least 8 pre-existing docs that the refinements-runner did
not touch:

| File | Context |
|---|---|
| `docs/12-interfaces/02-roko-new-scaffolders.md` line 65 | `use roko_core::{Context, Gate, Signal, Verdict};` |
| `docs/12-interfaces/02-roko-new-scaffolders.md` line 88 | `output: &Signal` |
| `docs/12-interfaces/02-roko-new-scaffolders.md` line 109 | `let signal = Signal::builder(Kind::AgentOutput)` |
| `docs/07-conductor/01-watcher-ensemble.md` line 17 | `fn decide(&self, stream: &[Signal], ctx: &Context) -> Vec<Signal>;` |
| `docs/07-conductor/INDEX.md` line 102 | `fn decide(&self, stream: &[Signal], ctx: &Context) -> Vec<Signal>;` |
| `docs/11-safety/15-forensic-ai.md` line 248 | `-> Vec<Signal>` |
| `docs/01-orchestration/00-layer-overview.md` line 65 | `roko_core::Signal` |
| `docs/CLI-REFERENCE.md` lines 116, 1043, 1109 | "Signal hash", "Signal trigger", "Signal kind" |

The glossary correctly marks `Signal` as retired. These docs were not swept.

### Category 2: Incorrect numbers

The reality check found these discrepancies:

| Claim in docs | Reality | Where to fix |
|---|---|---|
| ~177K LOC | 322,088 LOC | Any doc citing 177K |
| 18 crates | 36 workspace members | Any doc citing 18 |
| ~85 routes | 200+ routes | Any doc citing 85 |
| 1,568 tests | 3,761 test functions | STATUS.md (done in AUD01, verify others) |

Search ALL docs for these stale numbers and fix them.

### Category 3: 0-code concepts in present tense

The reality check confirmed these have zero code:

| Concept | Lines of code |
|---|---|
| Demurrage | 0 |
| Pulse (struct) | 0 |
| Bus (trait) | 0 (EventBus<E> struct exists) |
| Datum | 0 |
| Worldview | 0 |
| Custody | 0 |
| Claim / Paper | 0 |
| Replication ledger | 0 |
| Plugin SPI / roko-spi | 0 |
| Graduation (Pulse -> Engram) | 0 |

Any doc that describes these in present tense ("Engrams carry demurrage
balance", "Pulse types flow through the Bus") needs qualifying. Previous
batches (AUD02-AUD06) handle specific sections; this batch catches any
remaining instances across the full tree.

## Implementation

### 1. Signal -> Engram sweep

For each file listed in the table above:

- Replace `Signal` with `Engram` in Rust code snippets
- Replace `signal` with `engram` in variable names within code snippets
- Replace `Signal::builder` with `Engram::builder`
- Replace `&[Signal]` with `&[Engram]`
- Replace `Vec<Signal>` with `Vec<Engram>`
- Replace `use roko_core::{..., Signal, ...}` with `use roko_core::{..., Engram, ...}`
- In prose, replace "Signal hash" with "Engram hash", "Signal kind" with
  "Engram kind", etc.
- If a doc has a note like "will be renamed to Engram in Tier 0D", remove that
  note since the rename is complete

Also search for any OTHER docs not in the audit's list that still use `Signal`
as a type name. Run a search across `docs/` for the pattern. Exclude:
- The glossary's "Retired Terms" table (which correctly lists Signal as retired)
- Unix signal references (SIGTERM, SIGKILL) which are unrelated
- Meta-references ("the Signal -> Engram rename is complete")

### 2. Fix stale numbers across all docs

Search `docs/` for:
- `177K` or `177,000` or `~177` -- replace with `322K` or `~320K`
- `18 crates` -- replace with `36 crates`
- `~85 routes` or `85 routes` -- replace with `200+ routes`
- `1,568 tests` or `1568` -- replace with `3,761 tests` or `~3,700 tests`

Be careful to only fix references to the overall codebase. If a doc says
"roko-core has 18 modules" that is a different number and should not be
changed.

### 3. Catch remaining 0-code present-tense claims

Search `docs/` for present-tense usage of the 0-code concepts listed above.
Focus on claims like:
- "Engrams carry demurrage balance"
- "Pulse messages flow through..."
- "The Bus trait provides..."
- "Datum abstracts over..."

If AUD02-AUD06 already handled the file, skip it. Only fix files NOT covered
by those batches. Add a brief qualifier like `(target-state)` or
`(planned; not yet implemented)` after the present-tense claim.

## Write scope

Primary (Signal sweep):
- `docs/12-interfaces/02-roko-new-scaffolders.md`
- `docs/07-conductor/01-watcher-ensemble.md`
- `docs/07-conductor/INDEX.md`
- `docs/11-safety/15-forensic-ai.md`
- `docs/01-orchestration/00-layer-overview.md`
- `docs/CLI-REFERENCE.md` (if it exists)
- Any other docs found with stale `Signal` type references

Secondary (number fixes):
- Any doc in `docs/` that cites 177K LOC, 18 crates, ~85 routes, or 1,568 tests

Tertiary (0-code qualifiers):
- Any doc in `docs/` NOT already covered by AUD02-AUD06 that uses 0-code
  concepts in present tense

## Rules

1. **Signal -> Engram is mechanical.** Do not rewrite surrounding prose. Just
   swap the type name and update variable names in code snippets.
2. **Number fixes are mechanical.** Replace old number with new number. Do not
   rewrite surrounding context.
3. **0-code qualifiers are minimal.** Add `(target-state)` or `(planned)` --
   do not add multi-line callouts. The callouts were handled by AUD02-AUD06.
4. **Do NOT touch files already fully handled by AUD02-AUD06.** If a file was
   in their write scope AND the specific issue was addressed there, skip it.
   If a file was in their scope but they did not address this specific issue,
   fix it.
5. **Preserve the glossary's retired-terms table.** Do not remove Signal from
   the retired list. That entry is correct and useful.
6. **Do not rewrite prose.** This is a factual correction batch, not a
   narrative rewrite.
7. **Be thorough.** Search the entire `docs/` tree. The audit found 8 files
   with Signal issues; there may be more.

## Done when

- Zero docs use `Signal` as a live Rust type name in code snippets (except
  the retired-terms table and Unix signal references)
- Zero docs cite 177K LOC, 18 crates, ~85 routes, or 1,568 tests
- Any 0-code concept used in present tense (outside AUD02-AUD06 scope) has
  a qualifier
- All edits are mechanical (type swap, number swap, brief qualifier) -- no
  prose rewrites
- Final message lists: (a) number of files with Signal->Engram fixes, (b) number
  of files with stale numbers fixed, (c) number of files with 0-code qualifiers
  added, (d) the full list of files edited
