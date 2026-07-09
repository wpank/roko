# Refinement Audit Runner — Batch AUD04

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

--- BEGIN 03-moat-audit.md ---

# Audit: Refinements 17-21 (Moat & Modularity Arc)

**Auditor**: Claude (cross-referencing proposals against actual codebase)
**Date**: 2026-04-17
**Scope**: Refinements 17 (Plugin SPI), 18 (Competitive Moat), 19 (Net-New Innovations), 20 (Modularity/Composability), 21 (From-Scratch Redesigns)

---

## Refinement 17: Plugin & Extension Architecture

**Verdict: DEFER (Tiers 1-3 can SIMPLIFY into something small; Tiers 4-5 REJECT for now)**

### What it proposes

A five-tier plugin SPI ranging from "drop a TOML file" (Tier 1) to "WASM sandboxed extensions" (Tier 5), a `roko plugin` CLI with 8 subcommands, a plugin registry at `plugins.roko.dev`, manifest-driven discovery, and a new `roko-spi` crate for ABI stability. Also proposes a `roko-wasm-host` crate with a full WASM host surface (7 host imports).

### What the codebase actually has

- `roko-plugin` already exists as a crate (`crates/roko-plugin/`). It is a single `lib.rs` file (~200 lines) defining `EventSource`, `FeedbackCollector`, `FeedbackOutcome`, and `FeedbackSignal`. It depends on `roko-core`, `notify` (file watching), `cron` (scheduling), and `globset`. It is *not* the SPI described in doc 17 -- it is a narrow, concrete SDK for event sources and feedback loops.
- Tools live in `roko-std/src/tool/builtin/` -- 18 builtin tool handlers (bash, grep, read_file, write_file, etc). These are Rust implementations registered via `ToolRegistry`. Adding a tool means writing Rust and adding it to the registry.
- Role templates live in `roko-compose/src/templates/` -- 9 template modules (implementer, reviewer, strategist, etc) mixed with the builder/engine code.
- MCP servers exist as separate crates (`roko-mcp-code`, `roko-mcp-github`, `roko-mcp-slack`, `roko-mcp-scripts`, `roko-mcp-stdio`). These are already effectively "plugins" in the MCP protocol sense.
- The six kernel traits (`Substrate`, `Scorer`, `Gate`, `Router`, `Composer`, `Policy`) in `roko-core/src/traits.rs` are clean, well-documented, and *already* the extension surface. Anyone can implement them today by adding a crate to the workspace.

### Honest assessment

1. **How many plugin authors exist today?** Zero. The project has one developer (Will). There are no external contributors, no third-party deployments, no community.

2. **Is the Tier 3 declarative tool manifest genuinely useful?** Yes, but only to Will. Being able to drop a TOML file to add a tool instead of writing Rust would be a real ergonomic win for the single user. But you do not need a five-tier SPI for this. You need a `plugins/tools/*.toml` loader and ~200 lines of Rust in the tool dispatcher.

3. **Are Tiers 4-5 premature?** Massively. Tier 4 proposes a C-FFI ABI bridge (`roko-extension-abi`) for cdylib loading. Tier 5 proposes a full WASM runtime with 7 host imports, CPU budgeting, memory limits, and rate limiting. These are multi-month engineering efforts that solve the problem of untrusted third-party code execution. There are no third parties.

4. **The `roko-wasm-host` crate with host imports like `engram_get`, `bus_publish`, `substrate_query_similar`**: These reference types (`Pulse`, `Engram` graduation, HDC substrate queries) that do not exist in the codebase. There is no `Pulse` type anywhere. There is no `substrate_query_similar`. The WASM host surface is specified against a future codebase that has not been built.

5. **The plugin registry (`plugins.roko.dev`)**: This is aspirational infrastructure for a community that does not exist. The doc acknowledges this is Phase 2+ but still specifies it in detail.

### What to do instead

- **Extract the declarative tool loader (Tier 3 only)**: Add TOML-based tool manifests that the existing `ToolRegistry` can load. This is ~300 lines of code and gives the single user a real workflow improvement.
- **Separate templates from engine in `roko-compose`**: Move template files to `plugins/prompts/` or equivalent. This is a file-move, not a new crate.
- **Defer everything else** until there is at least one external user asking for it.

---

## Refinement 18: Competitive Moat

**Verdict: SKEPTICAL**

### What it proposes

Five structural moat components: (1) architectural coherence (Substrate + Bus + HDC + demurrage + c-factor as mutually reinforcing), (2) a heuristic commons with cross-deployment sharing, (3) a plugin ecosystem with network effects, (4) a replication ledger for scientific self-correction, (5) Rust-level correctness guarantees.

### What the codebase actually has

Let me check each proposed moat component against reality:

1. **"Substrate + Bus + HDC + demurrage + c-factor integrated"**:
   - Substrate: exists and works (`roko-core/src/traits.rs`, `roko-fs/`).
   - Bus: exists as `EventBus<RokoEvent>` in `roko-runtime/src/event_bus.rs` (~430 lines). Has 2 event types: `PlanRevision` and `PrdPublished`. This is a concrete, working event bus, not the abstract "Bus as a kernel trait" the refinements envision.
   - HDC: exists in `roko-primitives/src/hdc.rs` (10,240-bit vectors, XOR bind, bundle, Hamming similarity). Used by `roko-learn`, `roko-neuro`, `roko-dreams`, `roko-serve`. This is real and works.
   - Demurrage: does not exist. `grep -r demurrage crates/` returns zero results. The `Decay` enum in `roko-core` has `Exponential`, `Linear`, `Step`, `None` variants -- standard decay, not economic demurrage.
   - c-factor: partially exists. `roko-core/src/cfactor.rs` and `roko-learn/src/cfactor.rs` define `CFactor` and `CFactorPolicy`. The cascade router uses it for model routing. But this is a single numeric signal used in routing, not the continuously-computed Woolley collective-intelligence metric the doc describes.

2. **Heuristic commons**: Does not exist. There is one `HeuristicRule` struct in `roko-neuro/src/tier_progression.rs`. No cross-deployment sharing, no commons, no curation mechanism.

3. **Plugin ecosystem**: See doc 17 audit above. Does not exist and has no users.

4. **Replication ledger**: Does not exist. Zero matches for "replication ledger" in the codebase.

5. **Rust-level correctness**: Real, but this is a property of the language choice, not a defensible moat. Any Rust project gets this. The specific claims about "trait contracts actually hold" and "Bus backpressure is actually backpressure" are true of any well-written Rust code.

### Honest assessment

The moat doc describes a system that does not exist yet. Of the five components:
- 1 fully exists (HDC)
- 1 partially exists (c-factor)
- 1 exists but in a much simpler form than described (Bus)
- 2 do not exist at all (demurrage, replication ledger)
- 1 is a language property, not a product property (Rust correctness)
- 1 depends on an ecosystem that has zero participants (plugins)
- 1 depends on cross-deployment sharing that has zero deployments (heuristic commons)

The switching-cost table in section 11 is honest about the timeline (day-30 switching cost is "an afternoon") but projects forward to day-720 with accumulated assets that depend entirely on features being built. This is a fundraising narrative, not an engineering assessment.

### What this gets right

- Section 7 ("anti-moat failures to avoid") is genuinely useful guidance.
- Section 8 ("where the moat doesn't apply") is honest about IDE vendors and model providers being existential threats.
- Section 10 ("the non-moat that matters") correctly states that none of this matters if the product does not deliver value today.
- The framing that *composition* of features can be defensive even when individual features are not is correct in principle.

### What to do instead

- Stop writing moat docs and ship features. The moat is the working product, not the architecture diagram.
- If you want to make the moat argument honestly, list only what exists today: a working Rust agent orchestrator with multi-backend LLM dispatch, a 7-rung gate pipeline, HDC fingerprinting, episode logging, and a TUI. That is already more than most agent frameworks have.

---

## Refinement 19: Net-New Innovations Catalog

**Verdict: SIMPLIFY (honest about 3 of 10; the rest is aspiration)**

### What it proposes

A pitch-deck catalog of 10 primitives, 7 patterns, and 6 APIs that are claimed to be net-new innovations no other agent framework has.

### What the codebase actually has

Checking each claimed innovation:

| Claimed innovation | Exists in code? | Notes |
|---|---|---|
| 1.1 Pulse as first-class type | No | No `struct Pulse` anywhere. The Bus has `RokoEvent`, which is a concrete enum, not a typed ephemeral medium. |
| 1.2 HDC fingerprint on every Engram | Partial | HDC exists and is used, but Engrams do not have a fingerprint field. The `Engram` struct has no HDC vector. Fingerprinting happens in `roko-learn` and `roko-dreams` as a side-channel, not as a per-Engram property. |
| 1.3 Demurrage | No | Zero code. |
| 1.4 Heuristic with explicit falsifier | No | One `HeuristicRule` struct in `roko-neuro` with `condition` and `action` fields. No falsifier field, no calibration, no Bayesian updating. |
| 1.5 Replication ledger | No | Zero code. |
| 1.6 c-factor as runtime signal | Partial | `CFactor` struct exists, used in routing. Not continuously computed, not surfaced in dashboards. |
| 1.7 Worldview as emergent object | No | Zero matches for "worldview" or "Worldview" in crates/. |
| 1.8 Two-fabric operator generalization | No | All six traits operate on `Engram` only. No `Pulse` medium, no dual-fabric dispatch. |
| 1.9 Demurrage-taxed learned parameters | No | Zero code. |
| 1.10 Prediction markets on heuristics | No | Zero code. |
| 2.1 Predict-publish-correct loops | No | No prediction-correction wiring. |
| 2.2 Stigmergy via Engrams | Partial | Agents read/write Engrams, which is trivially stigmergic, but not as a designed coordination pattern. |
| 2.4 Dream cycles | Partial | `roko-dreams` exists (~6K lines) with `DreamReplayPolicy`, `DreamReplayMode`, cycle/hypnagogia/imagination modules. But it is Phase 2+ and not wired into the runtime. |
| 3.1 `roko heuristic` CLI | No | Not a CLI command. |
| 3.2 `roko dashboard` with c-factor tile | Partial | Dashboard exists. Whether it has a c-factor tile is unclear but c-factor is defined in the dashboard snapshot types. |
| 3.3 `roko plugin` CLI | No | Not a CLI command. |

### Honest assessment

Section 8 of the doc itself is admirably honest: "Rereading the list with an honest eye, three entries are genuinely primitive." Those three (falsifier heuristic, replication ledger, c-factor as runtime signal) do not exist in the codebase either. The doc is being honest about the *design* novelty while the *implementation* novelty is approximately zero.

This is a pitch deck for features that have not been built. As a roadmap of what to build, it has value. As a "what does Roko let you do that nothing else does" answer, the honest answer today is: multi-backend LLM orchestration with a compile/test/clippy gate pipeline, in Rust, with persistence and resume. That is real and useful. Everything in this catalog is aspiration.

### What to do instead

- Maintain this as a roadmap, not a pitch deck. Rename it "Innovation Roadmap" and add a status column showing what exists vs. what is planned.
- Ship one primitive from this list before writing more docs about it.

---

## Refinement 20: Modularity, Composability, and Cleaner Dependencies

**Verdict: SIMPLIFY (one extraction is justified; the rest is premature)**

### What it proposes

Three new kernel crates (`roko-bus`, `roko-hdc`, `roko-spi`), two crate splits (`roko-std` -> `roko-defaults` + `roko-tools`, `roko-compose` -> `roko-compose-core` + `roko-templates`), a strict dependency graph with layer rules, CI enforcement of the graph, and a multi-phase migration plan.

### What the codebase actually has

Current workspace: 29 crate directories under `crates/roko-*/`, plus 3 apps. The workspace `Cargo.toml` lists 28 members. This is already a lot of crates for a single-developer project.

Actual coupling analysis:

1. **`roko-agent` depends on `roko-learn`**: Only in `dev-dependencies` (tests). The doc claims `roko-agent` "reaches into `roko-learn` to persist efficiency events" -- this is wrong. `roko-agent/Cargo.toml` has `roko-learn` only under `[dev-dependencies]`. `grep 'use roko_learn' crates/roko-agent/src/` returns zero matches. The stated problem does not exist.

2. **`roko-cli` imports from almost everything**: True, and the doc acknowledges this is "warranted (it's the main binary)." This is not a problem to solve.

3. **`roko-primitives` (HDC) leaked into crates**: `roko-primitives` is a dependency of 7 crates. Of those, `roko-compose`, `roko-serve`, `roko-fs`, and `roko-neuro` depend on it behind feature flags (`hdc = ["dep:roko-primitives"]`). This is already well-managed. Only `roko-core`, `roko-learn`, and `roko-dreams` have unconditional dependencies.

4. **No `roko-bus` crate**: The event bus in `roko-runtime/src/event_bus.rs` is ~430 lines. It is used by 4 files: `roko-cli/src/prd.rs`, `roko-cli/src/orchestrate.rs`, `roko-serve/src/routes/prds.rs`, and `roko-core/src/state_hub.rs`. This is modest coupling. Extracting it into a separate crate does not solve a practical problem today.

5. **Role templates live next to template engines**: True. `roko-compose/src/templates/` has 9 role modules alongside the builder code. Separating data from engine is reasonable but does not require a new crate -- a directory restructure within `roko-compose` suffices.

### Honest assessment

The proposal would take the workspace from 29 to 34 crates (adding `roko-bus`, `roko-hdc`, `roko-spi`, `roko-defaults`, `roko-tools`, `roko-compose-core`, `roko-templates` while removing `roko-std` and `roko-compose`; net +5). For a single-developer project with no external consumers, this is pure overhead:

- More `Cargo.toml` files to maintain.
- More `pub use` shims during migration.
- More import paths to remember.
- More CI build graph complexity.
- Zero benefit to the end user, who runs `roko plan run`.

The dependency graph in section 3 is beautifully drawn but solves for a problem (multiple teams working on independent subsystems, substrate/bus swaps, plugin ABI stability) that does not exist and may never exist.

The "non-goals" section (9) says "every new crate and trait boundary must justify its existence with an actual use case within the next 6 months." By this standard, none of the proposed crates pass:
- `roko-bus`: no one is swapping the bus.
- `roko-hdc`: already well-managed behind feature flags.
- `roko-spi`: no plugin ecosystem exists.
- `roko-defaults` / `roko-tools`: no one needs a minimal runtime without builtins.
- `roko-compose-core` / `roko-templates`: no third-party template contributors.

### What to do instead

- **Do nothing** with the crate structure for now. The current coupling is modest and well-managed with feature flags.
- **If any extraction is justified**, it is moving HDC from `roko-primitives` to a standalone `roko-hdc` crate, since `roko-primitives` currently contains two unrelated concerns (HDC vectors and tier routing). But even this can wait.
- **Add the CI dep-check script** (section 11) -- this is cheap and prevents future coupling mistakes regardless of crate structure.

---

## Refinement 21: From-Scratch Redesigns

**Verdict: DEFER (all five rewrites are premature; incremental refactoring is sufficient)**

### What it proposes

Five from-scratch rewrite candidates:
1. `roko-core` kernel: Add `Pulse` as second medium, expand to 7 operators, semver-major bump. 2-3 weeks.
2. `roko-learn` reorganization: Split into 5 focused crates (`roko-episode`, `roko-playbook`, `roko-bandit`, `roko-experiment`, `roko-heuristic`). 2 weeks.
3. Substrate trait rewrite: Add `query(predicate)`, `scan(range)`, `freeze/thaw` for cold tier. 1 week.
4. Gate pipeline: Replace state machine with pure-function composition combinators. 1-2 weeks.
5. `roko-compose` engine: Replace fixed templates with query-driven compose (HDC retrieval). 2 weeks.

Total: ~10 weeks of focused rewrite work.

### What the codebase actually has

1. **`roko-core` kernel** is ~41K lines. The "1 noun + 6 verbs" framing is coherent and well-documented. The Engram type is clean. The six traits are clean. The doc says "the current framing actively misrepresents the system" because there is no Pulse -- but the system does not *have* Pulses. The framing is accurate for what exists. Adding Pulse means building a new concept, not correcting a misrepresentation.

2. **`roko-learn`** is ~36K lines across 42 files. It is large and has mixed concerns (episodes, bandits, cascade routing, experiments, HDC clustering, pattern discovery, efficiency tracking). But splitting it into 5 crates does not make the code better -- it makes it spread across 5 `Cargo.toml` files. The coupling between components (e.g., cascade router using c-factor, efficiency events feeding into episodes) is *feature-level*, not *accidental*. Breaking these apart means adding `pub` APIs and `Bus` subscriptions where direct function calls currently work.

3. **Substrate trait** has `put/get/query/prune/len/is_empty/name`. The proposed additions (`scan`, `freeze/thaw`, `subscribe-style notifications`) serve demurrage and cold-tier graduation, which do not exist. Adding API surface for unbuilt features violates YAGNI.

4. **Gate pipeline** in `roko-gate/` is ~11K lines across 24 files. It has 11 gate implementations, a 7-rung pipeline, and adaptive thresholds. The doc's own verdict: "Maybe. Gates are already working; this is cleaner but not unlocking a specific user-facing capability." Correct. The current gates work. Leave them alone.

5. **`roko-compose` engine** is ~25K lines. The 6-layer prompt builder with role templates works. The proposed "query-driven compose" (assemble prompts from HDC-retrieved Engrams instead of fixed templates) is a fascinating idea but depends on (a) HDC fingerprints being on every Engram (they are not), (b) the Substrate having a `query_similar` method (it does not), and (c) enough Engrams existing in the Substrate to make retrieval useful (unknown).

### Honest assessment

The five rewrites are "build the features from the refinement docs" disguised as "clean up existing code." Let me be specific:

- **Rewrite 2.1 (kernel)**: This is not a rewrite of existing code. It is adding a new concept (Pulse) that does not exist. Call it what it is: a new feature.
- **Rewrite 2.2 (learn)**: This is a crate split. The code does not get better; it gets reorganized. The doc says "no public API break if the CLI retains its current shape" -- correct, which means the user sees no benefit.
- **Rewrite 2.3 (substrate)**: This adds new methods. The existing methods do not change. This is an API extension, not a rewrite.
- **Rewrite 2.4 (gates)**: The doc says "maybe." Trust the maybe.
- **Rewrite 2.5 (compose)**: This is a new feature (query-driven compose). The doc says "short-term the existing engine is fine." Trust that.

The doc's own heuristic for when a rewrite is justified (section 1) requires "at least three of five" criteria. For most candidates, only one or two criteria are met. The doc then argues for the rewrites anyway, which undermines the heuristic.

Section 8 ("what we risk by not committing") argues that incremental patching produces a "Frankenstein." This is a valid concern *if* the features from docs 2-16 are all being built. But demurrage, replication ledger, prediction markets, worldviews, and dream cycles are Phase 2+ or unbuilt. The "Frankenstein" scenario is hypothetical because the features that would cause it are hypothetical.

### What to do instead

- **Do not rewrite anything.** Build the features you want (Pulse, demurrage, etc.) when you want them, as new code alongside existing code. The existing code works.
- **When adding Pulse**: add it as a new type in `roko-core`. Do not rewrite Engram. Let both exist. If they converge on a shared operator trait later, refactor then.
- **When adding `query_similar` to Substrate**: add it as a default method on the trait that returns `Ok(vec![])`. Implementations opt in. No rewrite needed.
- **Leave gates and compose alone** until a specific user-facing problem demands a change.

---

## Cross-Cutting Assessment: Is the "Moat" Framing Honest?

### The core claim

Docs 17-21 collectively argue that Roko's defensibility comes from the *composition* of Substrate + Bus + HDC + demurrage + c-factor + heuristics + plugins + replication ledger, and that this composition is expensive to replicate.

### The honest assessment

The composition *as specified* would indeed be hard to replicate. But the composition does not exist. Here is what actually exists vs. what the moat claim depends on:

| Component | Exists in code | Moat claim depends on |
|---|---|---|
| Substrate (Engram storage) | Yes | Substrate + HDC + demurrage + freeze/thaw integrated |
| Bus (event system) | Yes (2 event types) | Bus as kernel trait with topic-based subscribe, backpressure |
| HDC vectors | Yes | HDC fingerprint on every Engram, query_similar |
| Demurrage | No | Central to memory management and cold-tier |
| c-factor | Partial | Continuously computed, surfaced in dashboards |
| Heuristics with falsifiers | No | Calibrated, commons-shared, prediction-market staked |
| Replication ledger | No | Continuously replicated, publishable |
| Plugin ecosystem | No | 50+ plugins with network effects |
| Worldviews | No | Emergent from heuristic citation |
| Prediction markets | No | Belief price discovery among agents |

Of 10 components the moat depends on, 2 exist fully, 2 exist partially, and 6 do not exist at all.

### The danger

The danger is not that these docs are wrong -- they describe a genuinely interesting system. The danger is that writing moat docs *before building the moat* creates a false sense of progress. Every hour spent specifying the WASM host surface for Tier 5 plugins is an hour not spent making `roko plan run` work better for the single user who exists.

The refinement docs contain 35+ documents totaling tens of thousands of words. The codebase is 177K LOC. A substantial fraction of development effort appears to be going into architecture documents rather than shipping features. The "moat" these docs describe would take years to build. The immediate priority is clear from `CLAUDE.md`:

> After items 10-11, roko can fully self-host: read its own PRDs, generate plans, execute them, validate results, learn from failures, and iterate.

Items 10-11 are "automatic plan generation" and "feedback loop." Neither requires new crates, new kernel types, or new plugin architectures. They require wiring existing code: emit a `PrdPublished` event (already defined) and have a subscriber that calls `prd plan` (already a CLI command). This is a day of work, not a two-month rewrite.

### What the moat actually is today

Roko's real moat today is simpler and more honest:

1. **It works.** `roko plan run` executes agent tasks, runs gates, persists state, and can resume. Most agent frameworks demo but do not ship.
2. **It is in Rust.** Performance, safety, and single-binary deployment are real advantages.
3. **It has a complete gate pipeline.** 11 gates, 7 rungs, adaptive thresholds. This is more verification than any Python agent framework offers.
4. **It has multi-backend LLM support.** Claude, Codex, Cursor, OpenAI-compat, Ollama, Gemini, Perplexity. This flexibility is real.
5. **It self-hosts.** The PRD -> plan -> execute -> gate loop works end-to-end.

That is a real product with real capabilities. The moat docs describe a *future* product that would be even more defensible. The gap between the two is approximately 6-12 months of focused implementation work, not architecture documents.

### Recommendation

1. **Stop writing refinement docs.** 35 is enough. The design is over-specified relative to implementation.
2. **Ship items 10-11 from `CLAUDE.md`.** Automatic plan generation and feedback loop. One week of work. Closes the self-hosting loop completely.
3. **If you want one big-bet feature from these docs, pick HDC-per-Engram (innovation 1.2).** The HDC code exists and works. Adding a fingerprint field to `Engram` and populating it at `Substrate::put` time is tractable and would be genuinely novel.
4. **Defer everything else** (Pulse, demurrage, WASM plugins, replication ledger, kernel rewrite, crate splits) until the self-hosting loop has run for a month and produced real data about what needs improving.
5. **When you do build new features, build them incrementally.** Add methods to existing traits with default implementations. Add types alongside existing types. Do not rewrite working code to accommodate unbuilt features.

--- END 03-moat-audit.md ---

--- BEGIN 03-extensions-and-surfaces.md ---

# Extensions, Domain Profiles, And User Surfaces

## Extensions and modularity

### What is right

The strongest extensibility idea is simple:
- prefer low-power extension tiers first;
- make discovery local and boring;
- treat manifests and profile bundles as real user-facing leverage;
- clean up the crate graph where obvious seams are mixed today.

This part is good.

### What is overstated

The five-tier story becomes weaker as it moves up the power curve. The redesign
benefit is obvious for:
- prompt packs;
- profile bundles;
- manifest tools;
- MCP adapters.

It becomes much less obviously necessary for:
- stable native ABI commitments;
- WASM host abstractions;
- registry/network-effect strategy.

### Best near-term modularity sequence

1. Build local Tier 1/2/3 loading first.
2. Add a minimal real plugin command surface.
3. Ship one or two concrete external examples.
4. Clean internal crate seams after those extension points are real.
5. Only revisit native ABI and WASM host ideas if real usage demands them.

### What to demote

- registry/network-effect claims;
- stable ABI rhetoric before actual extension pressure exists;
- moat language built on extension surfaces rather than user value.

## Developer UX

### Strong core

The "time to first working agent" goal is good. The layered SDK story is also
good in principle:
- one-liner;
- builder;
- trait-level customization;
- runtime implementation boundary.

### Main problem

The four-layer Rust SDK story is good as a design target, but it risks becoming
too baroque if every layer gets too much bespoke machinery too early.

Better framing:
- desired SDK shape;
- minimum stable path first;
- sharper rules about what must be ergonomic vs what can stay advanced.

### What to keep

- typed errors at the public API surface;
- docs/examples discipline;
- example-driven onboarding;
- cargo-native ergonomics as an aspirational design constraint.

### What to narrow

- runtime-impl rhetoric before the underlying runtime interfaces are stable;
- macros and cargo subcommands unless they remove real friction;
- type and API layering that adds ceremony without reducing cognitive load.

## Domain profiles

### Strong part

Domain profiles are a sensible packaging abstraction:
- they bundle tools, roles, gates, starter heuristics, and defaults;
- they give a practical unit of adoption;
- they work well with low-power extension tiers.

### Risk

The risk is over-formalizing domain abstraction too early:
- `TypedContext` is promising, but could become a premature universal
  substrate;
- `Custody` is valuable, but should stay tightly tied to safety and audit
  semantics instead of becoming a generic catch-all object.

### Better framing

Treat domain profiles as:
- curated bundles first;
- shared typed context and custody expectations second;
- full typed domain kernels later.

## User UX and CLI parity

### What is genuinely useful

- one verb set across surfaces is a good target;
- better first-run onboarding is high leverage;
- diff-first review and visible approvals are good ideas;
- resumption, transcripts, and undo/replay should be first-class.

### What is misleading today

The risk is promising symmetry before the shared interaction model is clear.
The redesign should not aim for parity as aesthetic tidiness. It should aim for
parity where the user mental model is genuinely the same across surfaces.

### Better near-term UX sequence

1. Make one good interactive session surface real.
2. Expose stable session/resume/transcript mechanics.
3. Unify action names only after actual flows exist behind them.
4. Add slash commands and per-hunk review where the data model supports them.
5. Treat the browser as a read-first ops console before a full five-page rich
   app.

## StateHub, realtime, and web UI

### Strong part

StateHub is one of the most promising refinement directions because it gives
the redesign a clean place to put:
- projections;
- subscriptions;
- replayable state transitions;
- interface-friendly read models.

### Main risk

The redesign can overreach here by trying to standardize all transport,
projection, query, auth, replay, and UI semantics in one move. Better to keep
StateHub small and composable:
- projection contract;
- subscription contract;
- snapshot/replay contract;
- auth and tenancy layered around those contracts.

### Rich UX primitives

These are strongest when they are treated as rendering consequences of upstream
contracts.

Prioritize first:
- tool banners;
- gate badges;
- replay milestones;
- heuristic footnotes.

Be careful with:
- raw reasoning streams as a stable product primitive;
- confidence bars without calibrated semantics;
- "everything everywhere" UX promises before projection semantics are settled.

## Recommended rewrite principles for this area

1. Keep plugin and profile docs target-state, but narrower and more local-first.
2. Pull platform rhetoric back toward local-first, low-power extensibility.
3. Treat domain profiles as bundles before treating them as typed domain
   runtimes.
4. Rewrite user UX docs around the ideal user journey and only then decide
   which surface parity is genuinely needed.
5. Recast StateHub as a small projection kernel, not an all-at-once interface
   platform.
6. Make the first web story read-only and ops-facing before promising a richer
   full product surface.

--- END 03-extensions-and-surfaces.md ---

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

# Batch AUD04: Mark moat/plugin overscoping as aspirational (REF17-21)

**Audit refs**: 03-moat-audit.md (full file), 05-refinement-matrix.md (REF17-21 rows).
Applies the audit's "defer" and "skeptical" verdicts to `docs/20-technical-analysis/`
and `docs/18-tools/`.

Read these files first:

- `tmp/refinement-audit-runner/context-pack/00-AUDIT-RULES.md`
- `tmp/refinements-audit/03-moat-audit.md` (full file -- all 5 REFs audited)
- `tmp/refinements-audit/05-refinement-matrix.md` (REF17-21 rows)
- `tmp/refinements-audit/00-MASTER-SUMMARY.md` ("The 10 Things to Defer" section)
- `docs/18-tools/14-plugin-sdk.md`
- `docs/18-tools/16-plugin-loading.md`
- `docs/18-tools/INDEX.md`
- `docs/20-technical-analysis/00-vision-ta-generalized.md`
- `docs/20-technical-analysis/INDEX.md`
- `docs/00-architecture/17-design-principles-and-frontier-summary.md`
- `docs/00-architecture/30-cross-pollination-innovations.md`
- `docs/00-architecture/23-architectural-analysis-improvements.md`

## Task

The refinements-runner wrote a five-tier plugin SPI, WASM sandboxing, a plugin
registry, moat claims based on interaction density, and net-new innovation
catalogs into the tools and technical-analysis docs. The audit found: zero
plugin authors exist, the moat components are mostly aspirational (2 of 10
exist fully), and the innovation claims oversell speculative pieces. Mark
these as aspirational/deferred.

## Current state (evidence)

The audit found these specific issues:

1. **Plugin SPI (REF17)**: Tiers 1-3 (TOML manifests, prompt packs, declarative
   tools) are reasonable but unbuilt. Tiers 4-5 (C-FFI ABI bridge, WASM
   runtime with 7 host imports) are premature -- no third-party code execution
   need exists. The WASM host surface references types (`Pulse`, `Engram`
   graduation, `substrate_query_similar`) that do not exist in code.

2. **Plugin registry (`plugins.roko.dev`)**: Aspirational infrastructure for a
   community that does not exist. The doc acknowledges Phase 2+ but still
   specifies in detail.

3. **`roko-plugin` crate**: Already exists (~200 lines) as a narrow SDK for
   event sources and feedback loops. It is NOT the SPI described in the docs.

4. **Competitive moat (REF18)**: Of 10 claimed moat components, the audit
   found: HDC fully exists, c-factor partially exists, Bus exists in simpler
   form, demurrage does not exist (0 lines), replication ledger does not exist
   (0 lines), plugin ecosystem has zero participants, heuristic commons has
   zero deployments. The switching-cost table projects to day-720 based on
   features that are not built.

5. **Net-new innovations (REF19)**: The catalog format oversells speculative
   pieces. Audit verdict: **REWRITE** -- convert to research hypotheses.

6. **Modularity (REF20)**: The target dep graph adds `roko-bus`, `roko-hdc`,
   `roko-spi` -- none of which exist. The cleanup direction is right but the
   new crates are premature.

7. **From-scratch redesigns (REF21)**: Useful as a pressure test, dangerous
   as the default implementation mindset. Existing code works.

## Implementation

### 1. Mark plugin tiers 4-5 and registry as aspirational

In `docs/18-tools/14-plugin-sdk.md`:
- Add an implementation-status callout at the top:
  `> **Implementation status**: `roko-plugin` exists (~200 lines) as a narrow
  > SDK for event sources and feedback loops. Tiers 1-3 (prompt packs, profile
  > bundles, declarative tool manifests) are a reasonable near-term target.
  > Tiers 4-5 (C-FFI ABI, WASM sandboxed extensions) and the plugin registry
  > are **aspirational** -- zero plugin authors exist today.`
- Where WASM host imports are described, add a note that the referenced types
  (`Pulse`, `substrate_query_similar`) do not exist in code

In `docs/18-tools/16-plugin-loading.md`:
- Add a similar callout about the gap between current tool registration
  (Rust `ToolRegistry`) and the proposed manifest-driven discovery

### 2. Mark moat framing as aspirational

In `docs/20-technical-analysis/00-vision-ta-generalized.md`:
- If this doc makes moat claims based on the interaction density of 10
  primitives, add a callout:
  `> **Reality check**: Of the 10 primitives cited as moat components, 2 exist
  > fully (Engram, Substrate), 2 partially (HDC, c-factor), and 6 are
  > unimplemented (Pulse, Bus trait, Demurrage, Heuristic commons, Replication
  > ledger, Plugin SPI). The moat framing is aspirational.`

In `docs/00-architecture/30-cross-pollination-innovations.md`:
- If innovation claims cite unbuilt primitives, qualify them

In `docs/00-architecture/17-design-principles-and-frontier-summary.md`:
- If frontier claims cite unbuilt primitives, add appropriate qualifiers

### 3. Mark target crates as proposed in modularity docs

In `docs/00-architecture/23-architectural-analysis-improvements.md`:
- If it describes `roko-bus`, `roko-hdc`, `roko-spi` as existing, mark them as
  "proposed target crates"
- Note that the cleanup direction is correct but the new crates are not yet
  created

### 4. Qualify innovation catalog

In `docs/20-technical-analysis/INDEX.md` and relevant sub-docs:
- Where net-new innovation claims are made, add a qualifier distinguishing:
  - "Shipping" innovations (things that are actually built and novel)
  - "Research hypotheses" (interesting ideas not yet validated)
  - "Prior art integrations" (things that integrate existing research)

### 5. Acknowledge what actually exists as the real moat

Where moat language appears, add a grounding note:
`The actual competitive edge today is: a working Rust agent orchestrator with
multi-backend LLM dispatch, a 7-rung gate pipeline, HDC episode fingerprinting,
episode logging with feedback loops, and an interactive TUI. That is already
more than most agent frameworks have.`

## Write scope

- `docs/18-tools/14-plugin-sdk.md`
- `docs/18-tools/16-plugin-loading.md`
- `docs/18-tools/INDEX.md` (if it overstates plugin system status)
- `docs/20-technical-analysis/00-vision-ta-generalized.md`
- `docs/20-technical-analysis/INDEX.md`
- `docs/00-architecture/17-design-principles-and-frontier-summary.md`
- `docs/00-architecture/30-cross-pollination-innovations.md`
- `docs/00-architecture/23-architectural-analysis-improvements.md`

## Rules

1. **Mark, do not delete.** Aspirational designs are valuable as future specs.
   Add implementation-status callouts; do not remove design content.
2. **Be specific about what exists.** The `roko-plugin` crate is real but
   narrow. The 6 kernel traits are real extension surfaces. HDC is real. Name
   the real things.
3. **Do not claim nothing works.** The working product IS the moat. Qualify
   aspirational claims without denigrating what is built.
4. **Use "aspirational" not "wrong."** The moat/innovation framing is a vision
   doc, not a lie. Frame it as forward-looking, not as fiction.
5. **Do not touch learning docs** -- that is AUD03's scope.
6. **Do not touch safety docs** -- that is AUD06's scope.
7. **Do not touch the glossary** -- that is AUD06's scope.

## Done when

- Plugin SDK docs distinguish tiers 1-3 (reasonable near-term) from tiers 4-5
  (aspirational)
- WASM host surface is marked as referencing types that do not exist
- Moat claims are qualified with "X of 10 primitives currently exist"
- Innovation catalog distinguishes shipping vs. research hypotheses
- Target crates are marked as proposed, not existing
- The real working product is acknowledged as the actual competitive edge
- No design content was deleted
- Final message lists every doc edited and the key qualifier added
