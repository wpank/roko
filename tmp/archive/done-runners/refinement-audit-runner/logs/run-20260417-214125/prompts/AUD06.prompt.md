# Refinement Audit Runner — Batch AUD06

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

--- BEGIN 05-integrator-audit.md ---

# Integrator Arc Audit (Refinements 31-35)

Auditor: Claude Opus 4.6 | Date: 2026-04-17 | Scope: Docs 31-35 vs actual codebase

This audit covers the five "integrator" documents that attempt to stitch the
previous 30 refinement docs into a coherent whole. The central question: do
these integrators actually integrate, or do they add another layer of
abstraction on top of an already-abstract stack?

---

## REF31: Synergy & Integration Map

**Verdict: SKEPTICAL**

### What the doc claims

A 10x10 matrix of ten "load-bearing primitives" (Engram, Pulse, Bus,
Substrate, HDC, Demurrage, Heuristics, c-factor, Replication ledger, Plugin
SPI), with every cell describing what one primitive gives to another. Ten
worked synergy examples. The thesis: Roko's moat is the interaction density
of the full matrix, not any single feature.

### What actually exists in code

Of the ten primitives:

| Primitive | Exists in code? | Status |
|---|---|---|
| P1 Engram | YES | `roko-core/src/engram.rs` -- real, tested, used everywhere |
| P2 Pulse | NO | No `Pulse` struct exists anywhere in the codebase |
| P3 Bus trait | NO | `EventBus<E>` exists in `roko-runtime/src/event_bus.rs` as a concrete struct, not a trait |
| P4 Substrate trait | YES | `roko-core/src/traits.rs` -- real, working |
| P5 HDC fingerprint | PARTIAL | `HdcVector` exists in `roko-primitives/src/hdc.rs`; `text_fingerprint` used by episode logger; but `query_similar` does not exist on Substrate |
| P6 Demurrage | NO | Zero occurrences of "demurrage" or "ReinforceKind" in any crate |
| P7 Heuristics | MINIMAL | `HeuristicRule` in `roko-neuro/src/tier_progression.rs` only; no `Heuristic` engram kind, no Calibrator, no Wilson CI |
| P8 c-factor | PARTIAL | `CFactorPolicy` in `roko-core/src/cfactor.rs`, `CFactor` struct in `roko-learn/src/cfactor.rs` -- exists, wired to Policy trait |
| P9 Replication ledger | NO | No `Claim`, `Paper`, or replication ledger code exists |
| P10 Plugin SPI | NO | No `roko-spi` crate; no plugin manifest schema; no tier system |

**Score: 3 of 10 primitives exist in meaningful form.** Two more exist
partially. Five are entirely aspirational.

### Specific problems

1. **The synergy matrix is aspirational fiction.** A matrix documenting
   interactions between things that do not exist is not a synergy map -- it
   is a wish list. Synergy 3.1 ("Demurrage x HDC -> self-trimming semantic
   memory") cites three primitives that collectively have zero lines of
   production code implementing the described interaction.

2. **The moat claim is circular.** The doc argues the moat is the matrix.
   The matrix mostly describes things that do not exist. Therefore the moat
   does not exist. This is fine for vision documents, but dangerous when
   framed as "the competitive edge we already have."

3. **The honest non-synergies section (section 7) is the best part.** It is
   the only section that demonstrates intellectual honesty about what
   actually connects. More of this, less of the wish-list cells.

4. **Section 8 claims three "emergent properties"** (self-improvement,
   inspectability, substrate neutrality) that the composition has. But only
   one (inspectability, via lineage chains on Engrams) is even partially
   real today. The other two depend on primitives P5-P10 that mostly do
   not exist.

### Practical alternative

Strip the matrix to the 3-4 primitives that actually exist (Engram,
Substrate, EventBus, HdcVector partial). Document only the synergies that
are live today: lineage-based auditability, content-addressed storage +
gate verdicts, HDC fingerprinting in episode logger. Mark everything else
as "planned" with a dependency on actual implementation. This would be
approximately 2 pages instead of 10.

---

## REF32: Safety, Sandbox, and Provenance

**Verdict: SIMPLIFY -- has a real foundation, but proposes 10x more than exists**

### What the doc claims

A comprehensive safety spine: `authorize(principal, action, target, ctx)`
returning `AuthzDecision`, plugin sandboxes across 5 tiers, taint tracking
with an enum (`Taint`), cryptographic attestation, custody records, egress
control, secrets management, multi-tenancy isolation, conflict-of-interest
policies, threat model, audit CLI, and incident response procedures.

### What actually exists in code

The safety layer at `crates/roko-agent/src/safety/` is **real and well-built**:

- `SafetyLayer` struct with `check_pre_execution()` -- works, tested
- `BashPolicy` -- command allowlist/denylist for shell tools
- `GitPolicy` -- branch protection (blocks force push to main)
- `NetworkPolicy` -- URL allowlist for web tools
- `PathPolicy` -- worktree escape prevention
- `ScrubPolicy` -- API key scrubbing from outputs
- `RateLimiter` -- per-tool/per-role rate limits
- `AgentContract` -- declarative constraints with governance rules
- `AgentWarrant` -- OCaps-style capability checking
- `Capability` enum -- Tool, ReadPath, WritePath, Exec, Network

This is a solid, practical safety layer. Tests exist. It works.

### The gap between real code and the proposal

| Feature | In safety/mod.rs | In REF32 |
|---|---|---|
| Tool authorization | YES (check_pre_execution) | Proposed as `authorize()` with different signature |
| Role-based tools | YES (ToolWhitelist per role) | Proposed as Principal/Action/Target/Context model |
| Secret scrubbing | YES (ScrubPolicy) | Proposed as `Secret` wrapper type |
| Rate limiting | YES (RateLimiter) | Not mentioned |
| Contract system | YES (AgentContract, GovernanceRule, Invariant) | Not mentioned at all |
| Capability/warrant | YES (AgentWarrant, Capability enum) | Not mentioned |
| Taint tracking | BOOLEAN ONLY (`Provenance.tainted: bool`) | Proposed as rich `Taint` enum with propagation |
| Attestation | YES (`Attestation` struct in roko-core, Ed25519 sign/verify) | Proposed with `AttestationLevel` enum (not in code) |
| Custody records | NO | Proposed as `Custody` struct |
| Plugin sandboxes | NO (no plugin system) | Proposed as 5-tier sandbox |
| Egress control | PARTIAL (NetworkPolicy blocks URLs) | Proposed as `Egress` trait |
| TypedContext | NO | Proposed as safety context carrier |
| Multi-tenancy | NO | Proposed |
| Conflict-of-interest | NO | Proposed |
| Audit CLI | NO | Proposed 7 commands |

### Specific problems

1. **The doc does not acknowledge the contract system.** The existing
   `AgentContract` with `Invariant` and `GovernanceRule` is a real, working
   authorization framework. REF32 proposes replacing it with a different
   model (`authorize()` function) without acknowledging what exists. This
   violates rule 1 from CLAUDE.md: "NEVER reimplement what already exists."

2. **The doc proposes custody, taint enum, TypedContext, plugin sandboxes,
   egress trait, multi-tenancy, and audit CLI** -- none of which exist.
   That is approximately 8-10 weeks of engineering for one developer. The
   staging section claims "two months of focused safety work" but doesn't
   account for the fact that this is one developer + AI agents.

3. **The permission table (section 3)** is well-designed and would be
   useful, but it implies a role system with researcher/planner/implementer/
   reviewer/ops roles that is more granular than what the existing contract
   system provides.

4. **The attestation model already partially exists.** `roko-core/src/attestation.rs`
   has `Attestation`, `Ed25519Signature`, `PublicKey`, `ChainAttestation`,
   plus working `sign()` and `verify()` functions. REF32 proposes adding
   `AttestationLevel` (LocalAgent/OrgRole/ChainWitness), which is a
   reasonable extension but should be framed as extending the existing code,
   not as a new design.

5. **The threat model (section 13) is genuinely useful.** It is the right
   shape, names the right things, and would be worth writing to
   `docs/security/threat-model.md` as-is.

### Practical alternative

Start from the existing `SafetyLayer` and extend it:

- Phase 1: Add `AttestationLevel` to existing `Attestation` struct. Expand
  `Provenance.tainted` from bool to a `Taint` enum. Write the threat model
  doc. (1 week)
- Phase 2: Add `Custody` engram kind and logging for destructive actions.
  (1 week)
- Phase 3: Everything else is deferred until a plugin system exists.

---

## REF33: Observability & Telemetry

**Verdict: DEFER -- useful spec, almost no existing infrastructure for it**

### What the doc claims

Four telemetry surfaces (structured logs, Prometheus metrics, OpenTelemetry
traces, Bus events), 60+ named metrics across 6 categories, default Grafana
dashboards, alert rules, cost dashboards, replay-with-override CLI, and
self-observability.

### What actually exists in code

- `roko-runtime/src/metrics.rs` -- an append-only JSONL metric recorder
  using `serde::Serialize`. This is NOT Prometheus exposition format. It is
  a simple file-based metric writer with `MetricRecorder::record()`.
- `tracing` crate is used for structured logging throughout (standard Rust
  ecosystem).
- `roko-learn/src/episode_logger.rs` -- append-only JSONL episode log.
  This IS working observability, but it is file-based, not
  Prometheus/OpenTelemetry.
- `roko-learn/src/efficiency.rs` -- per-turn efficiency events. Working.
- `roko-core/src/state_hub.rs` -- `StateHub` exists as a broadcast channel
  for dashboard events. Working but simple.
- OpenTelemetry: ONE grep match in the entire codebase
  (`roko-orchestrator/src/worktree.rs`). No OTLP exporter, no span creation,
  no trace context propagation.
- Prometheus: NO `/metrics` endpoint exists. No Prometheus client library
  in dependencies.

### Specific problems

1. **No Prometheus infrastructure exists.** The doc assumes a `/metrics`
   endpoint. There is no Prometheus client crate in the workspace. The
   existing metrics are JSONL files, not exposition format.

2. **No OpenTelemetry infrastructure exists.** The doc proposes detailed
   span trees (`op.sense`, `op.assess`, `op.compose`, etc.) but there is
   no OpenTelemetry SDK integration. Adding it to an 18-crate workspace is
   not trivial.

3. **The 60+ proposed metrics reference primitives that do not exist.** The
   "Roko-specific metrics" sections reference c-factor (partially exists),
   demurrage (does not exist), HDC diversity (does not exist), replication
   ledger (does not exist), Bus ring occupancy (EventBus has no ring buffer
   concept). Of the 6 metric categories, only "Gate pipeline" (section 5.4)
   maps cleanly to existing code.

4. **The cost metrics (section 5.6) are buildable** -- the cascade router
   tracks model selection, and efficiency events track per-turn tokens/cost.
   This is the most realistic section.

5. **The staging section estimates "two months of focused observability work"**
   which is honest but does not account for the dependency on primitives
   (Bus trait, Pulse, demurrage, etc.) that also do not exist yet.

6. **What actually exists is better than nothing.** The JSONL episode log,
   efficiency events, and StateHub broadcast are a pragmatic observability
   surface. The doc should acknowledge this as the Phase 0 baseline and
   build from there, rather than presenting a greenfield Prometheus+OTLP
   architecture.

### Practical alternative

Phase 0 (now): Document the existing observability (JSONL episode log,
efficiency events, StateHub, `tracing`-based structured logs) as the real
baseline.

Phase 1 (when needed): Add `metrics` crate for Prometheus exposition to
`roko-serve`. Wire the 10-15 metrics that correspond to actual subsystems
(gate verdicts, model routing decisions, token costs, agent turn durations).

Phase 2 (much later): OpenTelemetry spans, Grafana dashboards, alert rules.
These are useful but not urgent for a single-developer project.

---

## REF34: Glossary

**Verdict: SIMPLIFY -- partially accurate, but defines many terms for things that do not exist**

### What the doc claims

A canonical A-Z glossary of every term introduced or reclaimed across the
33 earlier refinement docs.

### Cross-reference against actual code names

I checked every bolded term against the codebase:

| Term | In glossary | In code | Match? |
|---|---|---|---|
| Agent | Yes | `roko-agent/` | YES |
| Attestation | Yes | `roko-core/src/attestation.rs` | YES |
| Balance (demurrage) | Yes | NO | NO -- does not exist |
| Bus (trait) | Yes | NO (EventBus<E> struct) | MISMATCH -- glossary says "proposed to become a trait" (honest) |
| c-factor | Yes | `roko-core/src/cfactor.rs`, `roko-learn/src/cfactor.rs` | YES |
| CascadeRouter | Yes | `roko-learn/src/cascade_router.rs` | YES |
| Claim | Yes | NO | NO -- does not exist |
| Cohort | Yes | NO | NO -- no struct |
| Composer | Yes | `roko-core/src/traits.rs` | YES |
| ContentHash | Yes | `roko-core/src/hash.rs` | YES |
| Context | Yes | `roko-core/src/context.rs` | YES |
| Custody | Yes | NO | NO -- does not exist |
| Daimon | Yes | `roko-core/src/affect.rs` (PadVector, EmotionalTag) | PARTIAL |
| Datum | Yes | NO | NO -- does not exist |
| Decay | Yes | `roko-core/src/decay.rs` | YES |
| Demurrage | Yes | NO | NO -- does not exist |
| Engram | Yes | `roko-core/src/engram.rs` | YES |
| Episode | Yes | `roko-learn/src/episode_logger.rs` | YES |
| EventBus (retired) | Yes | `roko-runtime/src/event_bus.rs` | NOTE: still the live code, not "retired" |
| Falsifier | Yes | NO | NO -- does not exist |
| Fingerprint (HDC) | Yes | `roko-primitives/src/hdc.rs` | PARTIAL (HdcVector exists; not "on every Engram at put time") |
| Fleet | Yes | NO | NO -- no Fleet struct |
| Gate | Yes | `roko-core/src/traits.rs` | YES |
| GateVerdict | Yes | `roko-core/src/kind.rs` | YES |
| Golem (retired) | Yes | NO | CORRECT -- retired |
| Graduation | Yes | NO | NO -- no graduation code |
| Grimoire (retired) | Yes | NO | CORRECT -- renamed to Neuro |
| Heuristic | Yes | `roko-neuro/src/tier_progression.rs` (HeuristicRule) | PARTIAL |
| HdcVector | Yes | `roko-primitives/src/hdc.rs` | YES |
| Kind | Yes | `roko-core/src/kind.rs` | YES |
| Lineage | Yes | `Engram.lineage: Vec<ContentHash>` | YES |
| loop_tick | Yes | `roko-core/src/loop_tick.rs` | YES |
| MCP | Yes | `roko-mcp-code/`, etc. | YES |
| Neuro | Yes | `crates/roko-neuro/` | YES |
| Operator (6 traits) | Yes | `roko-core/src/traits.rs` (6 traits) | YES |
| Paper | Yes | NO | NO -- does not exist |
| Plan | Yes | `Kind::Plan` in kind.rs | YES |
| Playbook | Yes | `Kind::PlaybookRule`, `roko-learn/src/playbook.rs` | YES |
| Policy | Yes | `roko-core/src/traits.rs` | YES |
| Provenance | Yes | `roko-core/src/provenance.rs` | YES |
| Projection (StateHub) | Yes | NO (StateHub exists but no typed Projection) | PARTIAL |
| Pulse | Yes | NO | NO -- does not exist |
| query_similar | Yes | NO | NO -- not on Substrate trait |
| Router | Yes | `roko-core/src/traits.rs` | YES |
| Score | Yes | `roko-core/src/score.rs` | YES |
| Scorer | Yes | `roko-core/src/traits.rs` | YES |
| Signal (retired) | Yes | Still used in trait doc comments ("Store a signal") | NOTE: partially renamed |
| StateHub | Yes | `roko-core/src/state_hub.rs` | YES |
| Substrate | Yes | `roko-core/src/traits.rs` | YES |
| Taint | Yes | `Provenance.tainted: bool` | PARTIAL -- bool, not the proposed enum |
| TypedContext | Yes | NO | NO -- does not exist |
| Worldview | Yes | NO | NO -- does not exist |

### Summary count

- 24 terms match actual code
- 7 terms partially match (exist in simpler form)
- 15 terms describe things that do not exist

### Specific problems

1. **The glossary labels `EventBus<E>` as "historical, being retired for
   `Bus` trait with `Pulse` payload"** -- but `EventBus<E>` is the live
   production code and no Bus trait or Pulse type exists. Marking production
   code as "retired" in a glossary is misleading.

2. **The glossary calls `Signal` "historical, retired in 877:5 rename"** --
   but the Substrate trait doc comments in `traits.rs` still say "Store a
   signal" and "signal" appears throughout the trait docs. The rename is
   incomplete.

3. **15 glossary entries define terms for things that do not exist in code.**
   This is fine for a vision glossary but dangerous if treated as a
   canonical reference -- newcomers will search for these types and find
   nothing.

### Practical alternative

Split into two sections: "Terms with code" (the 24 matches + 7 partials)
and "Planned terms" (the 15 that do not exist). This prevents confusion
when someone searches the codebase for `Pulse` or `Custody` and finds
nothing.

---

## REF35: Consolidated Roadmap

**Verdict: REJECT as timeline, SHIP as priority ordering**

### What the doc claims

A six-to-twelve-month roadmap across four quarters, requiring 5-7
engineers. Q1: two-medium kernel. Q2: learning substrate. Q3: ecosystem and
UX. Q4: scale, safety, domains. Q5-Q6: phase 2 (chain, mesh, dreams).

### The single-developer reality

The doc says "minimum team to land Q1-Q4 in 12 months" is 5-7 engineers.
The actual team is one developer (Will) and AI agents. This is not a minor
discrepancy -- it is a 5-7x staffing mismatch.

### Specific problems

1. **Q1 proposes a kernel refactor (Pulse, Bus trait, Datum, operator
   generalization, seven-step loop)** -- this is rewriting `roko-core` and
   `roko-runtime`. For one developer, this is not a quarter; it is likely
   3-6 months of careful, test-covered migration with high risk of breaking
   the existing working system.

2. **Q2 proposes HDC on every Engram, demurrage, heuristics as a type,
   c-factor measurement, and research-to-runtime.** These are five
   substantial features, each requiring new types, new storage, new CLI
   commands, and integration tests. For one developer, this is another
   3-6 months.

3. **Q3 proposes Plugin SPI, StateHub rearchitecture, realtime wire
   protocol, developer UX, user UX, CLI parity, web UI, rich UX primitives,
   and deployment UX.** This is nine workstreams. The doc estimates this as
   one quarter with 2 UX engineers + 1 platform engineer. For one developer,
   this is easily a year.

4. **Q4 proposes six domain profiles, safety spine, replication ledger,
   multi-tenancy, c-factor actuation, scaling instrumentation, and
   commons.** Another year for one developer.

5. **Total realistic timeline for one developer**: Q1-Q4 as described would
   take 3-5 years, not 12 months. The doc is calibrated for a startup
   team, not for the actual situation.

6. **The dependency graph (section 2) is useful** regardless of timeline.
   The ordering is correct: 01-09 must precede 10-16 must precede 17-25
   must precede 26-30.

7. **The not-doing list (section 9) is excellent.** It is the most honest
   and useful section of the document.

8. **The twelve-year view (section 11) is aspirational vaporware.** "The
   substrate architecture is studied in graduate compilers courses" is a
   prediction, not a plan.

### Practical alternative

Keep the dependency ordering and not-doing list. Discard the quarterly
timeline. Replace with a priority queue sized for one developer:

1. Finish self-hosting loop (items 10-11 from CLAUDE.md: auto plan
   generation + feedback loop). These are the highest ROI items and do NOT
   require any kernel refactor.
2. Tighten existing safety layer (extend Attestation, expand taint,
   threat model doc).
3. Wire existing observability (document the JSONL baseline, add a few
   Prometheus counters to roko-serve).
4. Address the ux-followup P0 items.
5. Everything else from Q1-Q4 goes into a "when the system needs it" pile.

---

## Meta-verdict: Do these integrator docs actually integrate?

**No. They add a layer of abstraction over an already-abstract refinement
stack.**

Here is the structural problem: the 30 prior refinement docs propose
changes to the codebase. Some of those proposals are grounded in real code
(the safety layer, the episode logger, the cascade router). Many are
aspirational (Pulse, Bus trait, demurrage, heuristics-as-type, replication
ledger, plugin SPI).

The integrator arc (31-35) attempts to weave the aspirational proposals
together into a coherent story. But the weave is between proposed features,
not between existing code. This produces a coherent vision document that
has very little grip on the actual system.

### What each integrator doc actually provides

| Doc | Claimed function | Actual function |
|---|---|---|
| 31 Synergy map | Shows how 10 primitives reinforce each other | Aspirational interaction matrix; 7 of 10 primitives do not exist |
| 32 Safety spine | Consolidates safety across all layers | Good extension of real safety code; buries the existing contract system under a new model |
| 33 Observability | Single instrumentation spec | Greenfield Prometheus/OTLP architecture that ignores existing JSONL-based observability |
| 34 Glossary | Canonical vocabulary | 60% accurate to code; 40% defines nonexistent types |
| 35 Roadmap | Sequencing for 5-7 engineers over 12 months | Correct ordering; 5-7x overstaffed timeline for actual team size |

### The integration problem

True integration would mean: "Here is what exists. Here is how these
existing pieces connect. Here is the minimal set of new code needed to
strengthen those connections." Instead, the integrator arc says: "Here is
what we plan to build. Here is how all of those plans connect to each
other."

The difference matters because:

1. Plans connecting to plans produces a dependency graph of unbuilt
   features. This is useful for architecture docs but dangerous for
   execution -- it can make the next step look like it requires 10
   prerequisites.

2. The existing system already works end-to-end (plan-execute-gate-persist).
   The refinements and their integrators risk making the system feel
   incomplete when it is actually functional.

3. A single developer's scarce attention should go to the gaps between
   existing, working code -- not to elaborate architectures for features
   that may never be built.

### Recommendation

**Ship:** REF32 section 13 (threat model), REF34 (glossary, with the
"exists in code / planned" split), REF35 section 9 (not-doing list),
REF35 section 2 (dependency graph as a priority ordering tool).

**Simplify aggressively:** REF32 (start from existing SafetyLayer, not a
new model), REF33 (document existing JSONL observability as baseline).

**Defer:** REF31 (synergy matrix is not useful until the primitives exist),
REF35 quarterly timeline (not calibrated for one developer).

**The core advice:** stop integrating plans with plans. Start integrating
code with code. The system works today. Make it work better before making
it work differently.

--- END 05-integrator-audit.md ---

--- BEGIN 04-safety-observability-roadmap.md ---

# Safety, Observability, Glossary, Synergy, And Roadmap

## Safety and provenance

### What is strong

The safety refinements are directionally strong because they treat safety as a
system contract instead of a grab-bag:
- authorization;
- sandboxes;
- taint;
- provenance;
- attestation;
- custody;
- tenant and identity concerns.

This should remain.

### What needs narrowing

The main risk is policy overdesign:
- auth and tenancy can sprawl into a full platform program too early;
- plugin sandbox tiers can get more elaborate than the extension model needs;
- custody and typed-context can become heavy universal requirements instead of
  targeted control points.

### Best rewrite

Describe the safety spine as:
- a small set of non-negotiable contracts;
- explicit trust boundaries;
- progressive hardening layers added as extension and deployment complexity
  grows.

## Observability and telemetry

### What is strong

Treating observability as a first-class part of the product is right. The
strongest part of the observability work is the insistence that Roko-specific
signals matter, not just generic CPU/memory/process metrics.

### What is overstated

The telemetry story is strongest when it stays attached to operator action:
- what happened;
- why it happened;
- what it cost;
- what changed;
- what needs intervention.

### What to keep

- structured logs;
- explicit metrics surface;
- traces around operator boundaries;
- replay as part of observability;
- cost visibility as a first-class operator concern.

### What to narrow

- Roko-specific metrics that depend on speculative primitives;
- attempts to unify every signal surface into one meta-model immediately;
- telemetry language that outruns clear actionability.

## Glossary and naming

### What is useful

One canonical glossary is valuable. Retiring stale names is also valuable.

### Main issue

The glossary currently acts like an authority amplifier for target-state ideas.
That makes it more dangerous than helpful in places.

### Better structure

Split entries into three categories:
- current canonical term;
- proposed target-state term;
- historical or retired term.

Only current canonical terms should be written as repo-wide settled truth.

## Synergy framing

### What is worth keeping

The synergy docs are useful as internal integration maps. They help show where
proposals reinforce each other and where a feature is isolated or premature.

### What is not worth keeping as-is

The synergy and moat rhetoric often drifts into architecture theater:
- interaction density becomes implied strategic proof;
- integration webs are treated as evidence of defensibility;
- the matrix format creates a false sense of inevitability.

### Better framing

Keep synergy as:
- internal coherence tooling,
- dependency visualization,
- gap-finding aid.

Do not let it become:
- proof of moat,
- proof of maturity,
- substitute for implementation evidence.

## Roadmap and sequencing

### Strong part

The roadmap at least tries to impose dependency order and risk budgeting. That
is an improvement over a flat wishlist.

### Weak part

The roadmap is still too optimistic and too architecture-led. It tries to
advance too many deep programs at once:
- kernel transport refactor;
- learning/memory refactor;
- plugin SPI;
- StateHub projections;
- unified UX surfaces;
- domain packaging;
- safety spine;
- multi-tenant hardening.

That is too much for one redesign stream. The target-state needs fewer
concurrent bets and sharper dependency discipline.

### Better sequencing

1. Transport unification and event-surface cleanup.
2. Projection/read-model cleanup for current dashboard and operator surfaces.
3. Typed heuristics and contradiction tracking.
4. Local-first extension bundles and minimal plugin lifecycle.
5. Better session UX and a thinner first web surface.
6. Only then: stronger memory economics, domain packaging maturity, and broader
   distributed/runtime claims.

### What should move out of near-term quarter language

- full demurrage rollout as a governing memory model;
- c-factor actuation;
- five-tier plugin maturity;
- full browser UX parity;
- broad multi-tenant and OIDC assumptions;
- chain/mesh consequences downstream of core runtime seams that still need to
  be earned.

## Recommended rewrite principles for this area

1. Keep safety as a spine, but keep the contract set small and enforceable.
2. Keep observability explicit, but tie metrics to clear operator actions.
3. Reduce glossary authority over proposed terms.
4. Recast synergy as internal architecture tooling, not strategic proof.
5. Rewrite the roadmap around fewer concurrent bets and sharper dependency
   checkpoints.

--- END 04-safety-observability-roadmap.md ---

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

# Batch AUD06: Fix integrator docs (REF31-35) — synergy, glossary, safety, roadmap

**Audit refs**: 05-integrator-audit.md (full file), 05-refinement-matrix.md (REF31-35 rows),
07-doc-quality-audit.md (sections 2.3, 2.4, 5). Applies the audit's "integrate code, not
plans" verdict to safety, synergy, glossary, and roadmap docs.

Read these files first:

- `tmp/refinement-audit-runner/context-pack/00-AUDIT-RULES.md`
- `tmp/refinements-audit/05-integrator-audit.md` (full file -- REF31-35 verdicts)
- `tmp/refinements-audit/05-refinement-matrix.md` (REF31-35 rows)
- `tmp/refinements-audit/06-codebase-reality-check.md` (section 6: Safety Layer Reality)
- `tmp/refinements-audit/07-doc-quality-audit.md` (sections 2.3, 2.4, 5)
- `docs/00-architecture/34-synergy-integration-map.md`
- `docs/00-architecture/35-consolidated-roadmap.md`
- `docs/00-architecture/24-cross-section-integration-map.md`
- `docs/00-architecture/01-naming-and-glossary.md`
- `docs/11-safety/INDEX.md`
- `docs/11-safety/00-defense-in-depth.md`
- `docs/11-safety/03-taint-tracking.md`
- `docs/11-safety/01-capability-tokens.md`

## Task

The integrator docs (REF31-35) try to stitch the previous 30 refinements into
a coherent whole. The audit found: the synergy matrix describes interactions
between things that mostly do not exist (3 of 10 primitives are real), the
glossary marks EventBus as "retired" despite it being the only live transport
code, the safety chapter ignores the existing AgentContract/Warrant system,
and the roadmap assumes 5-7 engineers. Fix these.

## Current state (evidence)

1. **Synergy matrix (REF31)**: 10 "load-bearing primitives" listed. Audit
   found: Engram (YES), Pulse (NO), Bus trait (NO -- EventBus<E> struct exists),
   Substrate (YES), HDC (PARTIAL), Demurrage (NO), Heuristics (MINIMAL),
   c-factor (PARTIAL), Replication ledger (NO), Plugin SPI (NO). Score: 3 of 10
   exist meaningfully.

2. **Safety (REF32)**: The existing safety layer at `roko-agent/src/safety/`
   has SafetyLayer, BashPolicy, GitPolicy, NetworkPolicy, PathPolicy,
   ScrubPolicy, RateLimiter, AgentContract, AgentWarrant, Capability enum --
   4,465 lines across 10 files. REF32 proposes replacing it without
   acknowledging it. The existing contract system is not mentioned.

3. **Glossary (REF34)**: Marks EventBus as "retired" despite it being the only
   live transport code. Defines terms for things that do not exist (Datum,
   Custody, Claim, Paper, Fleet, Graduation). Audit verdict: **REWRITE** --
   split into exists/planned.

4. **Roadmap (REF35)**: Assumes 5-7 engineers and quarterly milestones. Reality:
   1 developer + AI agents. Audit verdict: **REWRITE** -- calibrate for actual
   team.

## Implementation

### 1. Strip synergy matrix of unbuilt features

In `docs/00-architecture/34-synergy-integration-map.md`:
- Add an implementation-status callout at the top listing which primitives
  actually exist:
  `> **Implementation status**: Of the 10 primitives in this matrix, 3 exist
  > fully (Engram, Substrate, EventBus), 2 partially (HDC fingerprint,
  > c-factor), and 5 are target-state only (Pulse, Bus trait, Demurrage,
  > Heuristic commons, Replication ledger, Plugin SPI). Synergy cells
  > involving unbuilt primitives are aspirational.`
- In the matrix itself, mark each cell with a status indicator:
  - Cells where both primitives exist: leave as-is
  - Cells where one or both primitives do not exist: add `[target-state]` tag
- Keep the "Non-Synergies Worth Naming" section intact -- the audit called it
  the best part

In `docs/00-architecture/24-cross-section-integration-map.md`:
- Apply the same treatment: mark integration points involving unbuilt
  primitives as target-state

### 2. Fix glossary: split into exists vs. planned

In `docs/00-architecture/01-naming-and-glossary.md`:
- **Do NOT delete entries.** Instead, add a status column or tag to the A-Z
  glossary:
  - `[shipping]` -- term corresponds to a working type/module in the codebase
  - `[built]` -- code exists but not fully wired
  - `[planned]` -- target-state design, no code
  - `[retired]` -- deliberately replaced
- Specific fixes:
  - `EventBus`: Remove "retired" tag. It is the ONLY live transport code.
    Mark as `[shipping]` with a note: "The current transport mechanism.
    Target-state: evolve into Bus trait."
  - `Datum`: Mark as `[planned]`
  - `Custody`: Mark as `[planned]`
  - `Claim`: Mark as `[planned]`
  - `Paper`: Mark as `[planned]`
  - `Fleet`: Mark as `[planned]`
  - `Graduation`: Mark as `[planned]`
  - `Demurrage`: Mark as `[planned]`
  - `Worldview`: Mark as `[planned]`
  - `Falsifier`: Mark as `[planned]`
  - `Pulse`: Mark as `[planned]` (0 lines of code)
  - `Bus` (trait): Mark as `[planned]` with note that `EventBus<E>` struct
    is the current implementation
  - Keep all terms that match real code as `[shipping]` or `[built]`

### 3. Acknowledge existing safety system

In `docs/11-safety/INDEX.md`:
- Ensure the overview acknowledges the EXISTING safety system:
  `The safety layer at roko-agent/src/safety/ is **Shipping**: SafetyLayer
  (5-policy chain), BashPolicy, GitPolicy, NetworkPolicy, PathPolicy,
  ScrubPolicy, RateLimiter, AgentContract (with Invariant/GovernanceRule),
  AgentWarrant (OCaps-style), and Capability enum (Tool, ReadPath, WritePath,
  Exec, Network). 4,465 lines across 10 files, plus role-specific YAML
  contracts.`

In `docs/11-safety/00-defense-in-depth.md`:
- If it proposes `authorize(principal, action, target, ctx)` without
  acknowledging the existing `check_pre_execution()` chain, add a note:
  `> **Note**: The current implementation uses `SafetyLayer::check_pre_execution()`
  > which chains 5 policy checks. The `authorize()` signature described here
  > is a target-state API that would replace or wrap the current chain.`

In `docs/11-safety/03-taint-tracking.md`:
- Note that taint currently exists as `Provenance.tainted: bool` and the
  rich `Taint` enum is target-state

In `docs/11-safety/01-capability-tokens.md`:
- Acknowledge that `AgentWarrant` and `Capability` enum already exist and work

### 4. Qualify roadmap for actual team size

In `docs/00-architecture/35-consolidated-roadmap.md`:
- Add a callout:
  `> **Team calibration note**: This roadmap was drafted assuming 5-7
  > engineers. The actual project has 1 developer + AI agents. Timeline
  > estimates should be multiplied accordingly, and the number of
  > simultaneous work streams should be reduced.`
- Where "quarterly" milestones are specified, add: "estimated for a
  full team; single-developer timelines will differ"

## Write scope

- `docs/00-architecture/34-synergy-integration-map.md`
- `docs/00-architecture/35-consolidated-roadmap.md`
- `docs/00-architecture/24-cross-section-integration-map.md`
- `docs/00-architecture/01-naming-and-glossary.md`
- `docs/11-safety/INDEX.md`
- `docs/11-safety/00-defense-in-depth.md`
- `docs/11-safety/03-taint-tracking.md`
- `docs/11-safety/01-capability-tokens.md`

## Rules

1. **Mark, do not delete.** The synergy matrix and roadmap are useful as
   planning tools. Add reality markers; do not gut them.
2. **Credit existing safety code.** The AgentContract/Warrant system is real
   and working. REF32 should build on it, not replace it.
3. **The glossary is the highest-impact fix in this batch.** Every other batch
   depends on the glossary being honest about what exists. Get this right.
4. **Do not touch learning docs** -- that is AUD03's scope.
5. **Do not touch architecture foundation docs** (02b, 07b, 08, 09) -- those
   are AUD02's scope.
6. **Do not touch interfaces/deployment docs** -- those are AUD05's scope.
7. **Do not fix Signal->Engram references** -- that is AUD07's scope.

## Done when

- Synergy matrix has status indicators on every cell involving unbuilt
  primitives
- Glossary entries for unbuilt concepts are marked `[planned]`
- EventBus is NOT marked as retired in the glossary
- Safety INDEX acknowledges the existing 4,465-line safety system
- Safety docs reference existing `check_pre_execution()`, `AgentContract`,
  `AgentWarrant`
- Roadmap has a team-calibration note
- No content was deleted
- Final message lists: (a) how many glossary terms changed status, (b) how many
  synergy cells were marked target-state, (c) the safety acknowledgments added
