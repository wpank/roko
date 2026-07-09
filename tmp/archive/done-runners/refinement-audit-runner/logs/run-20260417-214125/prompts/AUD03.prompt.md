# Refinement Audit Runner — Batch AUD03

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

--- BEGIN 02-learning-audit.md ---

# Audit: Refinements 10-16 (Learning & Intelligence Arc)

Auditor: Claude Opus 4.6, 2026-04-17
Scope: Seven refinement proposals cross-referenced against the actual codebase.

---

## Method

Each refinement doc was read in full and checked against:
- `crates/roko-learn/src/` (41 files, ~35K LOC) -- the working learning subsystem
- `crates/roko-neuro/src/` (7 files) -- knowledge store, distiller, tier progression
- `crates/roko-primitives/src/hdc.rs` -- existing 10,240-bit HDC implementation
- `crates/roko-core/src/` -- Engram, Decay, Score, prediction, cfactor types
- `.roko/learn/` -- runtime data directory (currently empty)

For each doc: a verdict, what is useful, what is overengineered, what the
codebase already has, and what to actually do.

---

## Refinement 10: Self-Learning Cybernetic Feedback Loops

**Verdict: SIMPLIFY**

### What it proposes
Every operator becomes a predictor. Active inference (Friston's FEP)
becomes a literal implementation: operators publish predictions as Pulses,
get corrected by outcomes, and update. A universal `CalibrationPolicy`
subscribes to prediction/outcome pairs across all six kernel operators.

### What the codebase already has
The doc claims learning is "three things stapled on the side." That
understates it. `roko-learn` has **41 source files** including:

| Existing module | What it does |
|---|---|
| `active_inference.rs` | BeliefState over 90 latent routing states, Bayesian observe() updates |
| `prediction.rs` | PredictionRecord with register/resolve, residual tracking |
| `cascade_router.rs` | Three-stage cascade (static/confidence/UCB1), already integrates active_inference |
| `prompt_experiment.rs` | Bandit-driven A/B for prompt sections |
| `drift.rs` | Jensen-Shannon divergence drift detection on agent behavior |
| `pattern_discovery.rs` | Trigram mining + HDC-based cross-episode clustering |
| `runtime_feedback.rs` | Central LearningRuntime that orchestrates all subsystems per turn |
| `efficiency.rs` | Per-turn efficiency events |
| `regression.rs` | Regression detection against trailing windows |

The doc says "Scorer weights are hardcoded." False -- `PredictiveScorer` in
`roko-core/src/prediction.rs` already reads `PredictionCalibrationSummary`
(accuracy, coverage, bias, trend, confidence) and uses it for scoring. The
doc says "Gate thresholds are never learned." Also false -- adaptive gate
thresholds exist and are mentioned in CLAUDE.md as wired.

### What is useful
- The per-operator predict-publish-correct pattern is a clean abstraction.
  But the codebase already implements the substance of it for routing.
- DSPy-style continuous prompt optimization is genuinely useful. The
  existing `ExperimentStore` is already doing this with bandits.
- The CalibrationPolicy sketch (section 10) is the cleanest artifact here.

### What is overengineered
- The Friston/FEP framing adds no engineering value. The codebase already
  does Bayesian updates on routing beliefs without calling it "active
  inference" -- except it already calls it that (`active_inference.rs`).
  The framing is already in the code.
- "Every operator is a predictor" is a nice slogan but the actual
  engineering cost of instrumenting Scorer, Composer, Policy, and Substrate
  with prediction/outcome pairs is significant and the training signal for
  most of them is weak. The Router already has the richest signal because
  it makes discrete, measurable choices. The Scorer's "predictions" would
  need a definition of ground truth that doesn't exist today.
- Stafford Beer's Viable System Model / three speeds (gamma/theta/delta)
  is academic framing for "some things are fast, some are slow." The
  codebase already has this implicitly without needing Bus topic namespaces
  organized by speed.

### Risk
Low if done incrementally. High if someone tries to implement the full
"every operator predicts" vision at once -- it would require defining
ground-truth outcome signals for operators that don't have clean ones.

### What to actually do
1. Keep using `PredictionRecord` and `active_inference::BeliefState` as-is.
2. Extend prompt experiments to test more dimensions (already planned).
3. Skip the Bus-mediated CalibrationPolicy until a real Bus exists.
4. Do NOT rewrite the existing direct-call learning into pub/sub until
   there is a demonstrated need for decoupling.

---

## Refinement 11: Hyperdimensional Substrate

**Verdict: SHIP IT (the field on Engram) / DEFER (everything else)**

### What it proposes
Add `fingerprint: Option<HdcVector>` to every Engram. Build HDC-based
similarity queries, consensus via bundle, stigmergic pheromones,
compositional memory, analogy-driven retrieval, anti-hallucination gates,
and agent identity fingerprints.

### What the codebase already has
Substantially more than the doc acknowledges:

| What exists | Where |
|---|---|
| `HdcVector` (10,240-bit, bind/bundle/permute/similarity) | `roko-primitives/src/hdc.rs` |
| `fingerprint()` and `text_fingerprint()` | `roko-primitives/src/hdc.rs` |
| `KnowledgeHdcEncoder` (structured encoding with role/cause/effect binding) | `roko-neuro/src/hdc.rs` |
| HDC fingerprints on episodes | `roko-learn/src/hdc_fingerprint.rs` |
| K-medoids clustering over HDC vectors | `roko-learn/src/hdc_clustering.rs` |
| HDC-based cross-episode pattern discovery | `roko-learn/src/pattern_discovery.rs` |
| Feature-gated HDC on `KnowledgeStore` | `roko-neuro/src/knowledge_store.rs` |
| Base64 encode/decode for fingerprints | `roko-learn/src/hdc_fingerprint.rs` |

The primitives are built, tested, and partially integrated. The `HdcVector`
type has bind, bundle, permute, similarity, from_seed, serialization. The
`KnowledgeHdcEncoder` already does structured bind-and-bundle encoding
(cause/effect/domain/conditions) -- this is more sophisticated than the
doc's "DefaultEncoder" sketch.

### What is useful
- Adding `fingerprint: Option<HdcVector>` to Engram is the right move. It
  is a small change with compounding benefits. The infrastructure to
  compute and store it already exists.
- `query_similar()` on Substrate is genuinely valuable for retrieval.
  Brute-force scan over 800K fingerprints at ~1ms is realistic.

### What is overengineered
- HDC-based consensus voting (section 5) solves a problem roko doesn't
  have yet. There are rarely multiple agents producing competing outputs
  for the same task in the current self-hosting workflow.
- "Stigmergic pheromones as HDC vectors" (section 5.2) is an analogy
  looking for an implementation. The existing `Decay::HalfLife` on Engrams
  already serves the pheromone use case without HDC vectors.
- "Agent identity fingerprints" (section 10) is interesting research but
  not useful for self-hosting. You don't need to detect "team formation"
  when you have one agent pool running one plan.
- The "anti-hallucination via HDC consistency" gate (section 8) is a nice
  idea but the practical value is low -- HDC similarity between an output
  and its claimed sources can be high even when the output hallucinates
  (same keywords, wrong claims). The existing gate pipeline (compile, test,
  clippy, diff) catches real failures; this would catch vibes.

### Risk
Low for the Engram field addition. The HDC primitives are well-tested.
The risk is scope creep: once the fingerprint exists, someone will want
all 15 features in the doc, most of which aren't load-bearing yet.

### What to actually do
1. Add `fingerprint: Option<HdcVector>` to `Engram`. Use existing
   `fingerprint()` from `roko-primitives` at Substrate `put()` time.
2. Add `query_similar()` to `Substrate` trait, implement brute-force.
3. Stop there. Consensus, stigmergy, analogy, anti-hallucination, and
   identity fingerprints are Phase 2+ at earliest.

---

## Refinement 12: Knowledge Demurrage

**Verdict: SIMPLIFY**

### What it proposes
Replace time-based decay with an economic model: Engrams carry a `balance`
that costs holding fee per unit time but is reinforced by usage (cited,
retrieved, gated, surprised, quoted). Novelty-weighted reinforcement via
HDC neighbor similarity. Cold-tier graduation when balance hits floor.

### What the codebase already has

| What exists | Where |
|---|---|
| `Decay` enum (None/HalfLife/Ttl/Ebbinghaus) | `roko-core/src/decay.rs` |
| GC with decay in `roko-fs` | `roko-fs/` |
| Confidence-based GC threshold | `roko-neuro/src/knowledge_store.rs` (DEFAULT_GC_MIN_CONFIDENCE) |
| Tier progression (raw -> insight -> heuristic -> playbook) | `roko-neuro/src/tier_progression.rs` |
| Anti-knowledge confidence floor | `roko-neuro/src/knowledge_store.rs` |

The existing `Decay` enum is purely time-based, which is the gap the doc
identifies. However, `KnowledgeStore` already has confirmation-boosting
(`CONFIRMATION_BOOST = 1.5`), which is a form of usage-based reinforcement.

### What is useful
- The core insight is right: use-it-or-lose-it is better than pure
  time-based decay for a learning system. Playbooks that stop working
  should fade; ones that keep working should stay warm.
- The `ReinforceKind` enum (Cited, Retrieved, Gated, Surprised,
  AgentQuoted) is a good taxonomy of why something stays relevant.
- The worked example in section 12 is the best part of the doc -- it
  shows concrete numbers for how a playbook's balance evolves.

### What is overengineered
- Novelty-weighted reinforcement via HDC neighbor similarity (section 3)
  adds a dependency on the HDC substrate that doesn't exist yet for
  Engrams. It can be bolted on later; making it a prerequisite blocks
  the simpler version.
- `LearnedParam<T>` with demurrage on *policy parameters* (section 5) is
  elegant but premature. Roko doesn't have stable enough parameter values
  to worry about their staleness. The parameters are still being tuned by
  hand.
- The `ColdSubstrate` trait with freeze/thaw (section 7) is infrastructure
  for a scale problem that doesn't exist yet. Current Substrate fits
  comfortably in memory.
- "Demurrage-rates that learn their own rates" (section 6, last paragraph)
  is a recursive rabbit hole.

### Risk
Medium. Adding `balance` to Engram is a schema change that touches
everything that serializes Engrams. The rate law has tuning parameters
(flat_tax, exp_decay) that will need empirical calibration -- if the
defaults are wrong, either everything dies too fast or nothing ever fades.
Both failure modes look the same at first: "the system doesn't learn."

### What to actually do
1. Add a `Decay::UseBased` variant that tracks last-used timestamp and
   access count. Much simpler than a full balance/demurrage system.
2. Have `KnowledgeStore` and `PlaybookStore` track usage counts and
   last-access times on their entries (some of this exists).
3. Use the existing `CONFIRMATION_BOOST` pattern more broadly.
4. Skip the full economic model, cold-tier graduation, and
   novelty-weighted reinforcement. Revisit when there is enough data
   to show that time-based decay is actually failing.

---

## Refinement 13: Collective Intelligence & c-factor

**Verdict: SHIP IT (metrics only) / SKEPTICAL (active optimization)**

### What it proposes
Operationalize Woolley's c-factor for multi-agent cohorts. Measure
turn-taking entropy, peer-prediction accuracy, citation reciprocity,
delivery rate, HDC diversity. Combine into a scalar. Use it to adjust
Router temperature, spawn devil's advocates, inject outsiders.

### What the codebase already has
This is the most impressive "already built" case in the entire audit:

| What exists | Where |
|---|---|
| `CFactor` struct with `overall`, `CFactorComponents`, per-agent contributions | `roko-learn/src/cfactor.rs` |
| `CFactorComponents` with 11 axes including `turn_taking_equality`, `social_sensitivity`, `task_diversity_coverage`, `convergence_velocity`, `knowledge_integration_rate` | `roko-learn/src/cfactor.rs` |
| `compute_cfactor()` from episodes with time window | `roko-learn/src/cfactor.rs` |
| `AgentCFactorContribution` with leave-one-out scoring | `roko-learn/src/cfactor.rs` |
| `AgentDispatchBias` (PreferStronger/PreferCheaper/Neutral) | `roko-learn/src/cfactor.rs` |
| `CollectivePathology` enum (Cascade, Groupthink, EchoChamber, Deadlock, Hallucination) | `roko-learn/src/cfactor.rs` |
| `CFactorRegression` alert | `roko-learn/src/cfactor.rs` |
| `CFactorPolicy` implementing `Policy` trait | `roko-core/src/cfactor.rs` |
| `CFactorSummary` with trend, regression_drop, per-component values | `roko-core/src/cfactor.rs` |
| C-Factor integration in `CascadeRouter` | `roko-learn/src/cascade_router.rs` |

The codebase already has a c-factor implementation that is more grounded
than the doc's proposal. The doc proposes five process variables; the
codebase has eleven. The doc proposes a linear regression to learn weights;
the codebase has a direct computation from episode statistics. The doc's
`WisdomGate` with `min_hdc_diversity` and `max_lineage_overlap` doesn't
exist, but `CollectivePathology::Groupthink` and `EchoChamber` already
detect similar problems.

### What is useful
- The doc's framing of c-factor as "a covariate, not an objective"
  (section 13) is the single most important sentence. The codebase should
  have this as a comment.
- The WisdomGate concept (section 4) is interesting as a structural check
  against echo chambers, but it needs the multi-agent consensus workflow
  to matter.
- Peer-prediction accuracy (section 3.2) is the most novel proposal here.
  Having agents predict what other agents would say is a genuinely useful
  capability test for collaborative tasks.

### What is overengineered
- The five-axis operationalization in section 2.2 duplicates what
  `CFactorComponents` already does, with different names and fewer axes.
- The `CohortWeightsLearner` (section 12) that fits regression weights
  via stochastic gradient descent is over-engineered for a system that
  currently runs plans sequentially, not in multi-agent cohorts.
- Devil's-advocate role (section 6.1), outsider injection (6.2), and
  minority report preservation (6.3) are interesting in theory but
  irrelevant to the current self-hosting workflow where one agent does
  one task at a time.
- Cross-cohort c (section 9) is Phase 2+ at best.

### Risk
Low for metrics. The metrics already exist and are computed. The risk is
in active optimization (steps 3-5 in their phasing): adjusting Router
temperature based on c-factor can create oscillations where the system
alternates between "too diverse" and "too convergent."

### What to actually do
1. The c-factor computation already works. Keep it.
2. Add the "c is a covariate not an objective" caveat as a doc comment on
   `compute_cfactor`.
3. Expose c-factor in the TUI F4 tab if not already done.
4. Skip active optimization levers until multi-agent plans are common.

---

## Refinement 14: Worldview Validation with Falsifiers

**Verdict: SIMPLIFY (the Heuristic type is valuable; Worldviews are premature)**

### What it proposes
A `Heuristic` type with preconditions, predictions, calibration
(trials/confirmations/violations/Brier score), and lineage. Heuristics
are tested against every episode. Worldviews emerge from co-citation
clusters of heuristics. Dissonance detection surfaces conflicting priors.
Meta-heuristics apply recursively.

### What the codebase already has

| What exists | Where |
|---|---|
| `HeuristicRule` with id, insight_id, when_clause, then_clause | `roko-neuro/src/tier_progression.rs` |
| `InsightRecord` with support_count, confidence, source_episodes | `roko-neuro/src/tier_progression.rs` |
| Tier progression: episodes -> insights -> heuristics -> playbook | `roko-neuro/src/tier_progression.rs` |
| `KnowledgeKind` enum with Insight, Heuristic, Warning, CausalLink, Strategy, Anti-knowledge | `roko-neuro/src/lib.rs` (likely) |
| `KnowledgeEntry` with confidence, tags, source, tier | `roko-neuro/` |
| `PatternMiner` for recurring action sequences | `roko-learn/src/pattern_discovery.rs` |
| `SkillLibrary` with usage_count, success_rate per skill | `roko-learn/src/skill_library.rs` |

The codebase already has `HeuristicRule` in tier_progression.rs. It has
`when_clause` and `then_clause`. It tracks support count and confidence.
What it lacks is the structured `Predicate` enum for machine-evaluable
preconditions and the calibration lifecycle (trials/violations/Brier).

### What is useful
- The `Heuristic` type with explicit `Predicate` preconditions and a
  `Calibration` struct (trials, confirmations, violations, Brier score,
  Wilson CI) is the best-designed artifact in all seven docs. It turns
  vague "learned knowledge" into something testable.
- The lifecycle (birth -> test -> adjust -> retire -> evolve) is sound.
- The CLI surface (`roko heuristic list/show/similar/stats/export`) is
  genuinely useful product surface. Answering "what does my agent think
  it knows?" is a real user need.
- Sharing heuristics across deployments (section 10) is a concrete value
  proposition.

### What is overengineered
- Worldviews as co-citation clusters (section 4) require enough heuristics
  to form clusters. With the current volume of episodes, you won't have
  enough data points for community detection to produce meaningful results.
- The `Predicate` enum (section 2) is ambitious: `SimilarTo` with HDC
  threshold, `Custom(Box<dyn PredicateFn>)`, boolean combinators. Start
  with the simpler cases (LanguageIs, FileMatches, GateRecentlyFailed).
- PeerModel (section 7) -- agents modeling other agents' heuristics --
  is a research project, not a Phase 1 feature.
- Meta-heuristics (section 14) -- heuristics about heuristics -- are
  recursive and fascinating but not actionable until the base layer works.
- Dissonance detection (section 8) is elegant but assumes dense enough
  heuristic coverage that two will conflict. Not realistic short-term.

### Risk
Medium. The main risk is over-engineering the `Predicate` type before
knowing which predicates actually matter. Start simple, extend when
real episodes reveal what preconditions are useful.

### What to actually do
1. Extend `HeuristicRule` to include a `Calibration` struct with
   trials/confirmations/violations.
2. Add calibration updates in the post-episode pass (already in
   `runtime_feedback.rs`).
3. Add `roko heuristic list/show/stats` CLI commands.
4. Skip Worldviews, PeerModel, meta-heuristics, and dissonance detection.
5. Skip the full `Predicate` enum; use string-based when/then clauses
   (which already exist in `HeuristicRule`) until there's evidence that
   machine-evaluated predicates are needed.

---

## Refinement 15: Exponential Scaling Patterns

**Verdict: SKEPTICAL**

### What it proposes
Seven "compounding loops" that produce superlinear returns: demurrage-
weighted retrieval, heuristic calibration, HDC codebook cleanup,
c-factor feedback, playbook distillation, cross-deployment heuristic
commons, and plugin ecosystem.

### What the codebase already has
Most of the loops it describes are aspirational combinations of things
from other docs. The only ones with existing code are:
- Playbook distillation (exists in `roko-neuro/src/tier_progression.rs`)
- c-factor feedback (exists in `roko-learn/src/cfactor.rs`)
- Pattern discovery (exists in `roko-learn/src/pattern_discovery.rs`)

### What is useful
- The single metric: "mean time to first successful PR on a new codebase"
  (section 9). This is the right north-star for the product.
- Anti-metrics (section 11): episode count in warm tier should stabilize,
  heuristic count with <3 confirmations should not grow unbounded, mean
  lineage depth should not grow without quality gains. These are good
  health checks.
- Kill switches (section 13): `roko attention reset`, `roko heuristic
  retire --confidence-below 0.3`, etc. Every system that learns should
  have emergency resets.

### What is overengineered
- The entire framing. "Superlinear returns" and "exponential scaling" are
  claims, not designs. Each of the seven loops depends on multiple other
  refinements being fully implemented (demurrage, heuristics, HDC, c-factor,
  dreams, plugins, commons). The compounding claim is unfalsifiable until
  all prerequisites exist.
- Prediction markets on heuristics (section 5.1) -- agents staking balance
  on heuristic outcomes. This is a game theory paper masquerading as a
  feature. The coordination overhead exceeds any plausible information
  gain from internal prediction markets in a coding assistant.
- Self-modeling (section 5.3) -- a meta-agent reading its own metrics and
  proposing policy changes. This is the "Roko observing Roko" recursive
  loop that sounds powerful but in practice requires an extremely well-
  calibrated meta-agent to not make things worse.
- The "compositional tool curricula" via HDC arithmetic (section 5.2) is
  Plotkin's analogy mapped onto tool sequences. It assumes the HDC
  codebook has enough entries that `bundle(similar_plan_1, similar_plan_2)`
  produces something useful. In practice, early codebooks are too sparse
  for this to work.

### Risk
Low directly (this is a strategy doc, not a implementation spec). But
the real risk is that the "seven compounding loops" framing creates a
sense that all seven are prerequisites, which paralyzes incremental
progress. The system should ship one loop at a time and measure.

### What to actually do
1. Define the north-star metric (time to first PR on new codebase).
2. Instrument it.
3. Implement the loops one at a time, measuring the metric after each.
4. Cut any loop that doesn't move the metric.
5. Skip prediction markets, self-modeling meta-agents, and compositional
   tool curricula.

---

## Refinement 16: Research Papers as Engrams, Replication Ledger

**Verdict: DEFER / SKEPTICAL**

### What it proposes
A `Paper` Engram type with DOI, authors, claims. Each claim has a
structured `Hypothesis`, a `falsifier` Predicate, calibration, and a
`ReplicationLedger` tracking divergence between paper-reported effects
and Roko-observed effects. A `claim!` macro links config parameters to
their academic source. Agent-curated research ingestion. A starter kit
of ~40 foundational claims.

### What the codebase already has
Nothing specifically for paper tracking. Research artifacts go in
`.roko/research/` as unstructured output from the `roko research` command.
The heuristic infrastructure from `roko-neuro` could store paper-derived
knowledge as `KnowledgeEntry` values with `source` attribution, but there
is no structured `Paper` or `Claim` type.

### What is useful
- The idea of tracing config parameters to their academic source is
  genuinely useful for maintainability. Knowing that
  `CASCADE_EPSILON = 0.1` comes from Auer 2002 helps the next person who
  touches it.
- The falsifier concept from section 13 is well-written: "Over the next
  500 arm pulls, cumulative regret should be bounded by..." This is how
  you turn a paper citation into a testable claim.

### What is overengineered
- The full Paper/Claim/Hypothesis/Falsifier/ReplicationLedger type system
  is a research library management system embedded inside a coding
  assistant. The target user (a developer using roko for self-hosting)
  does not need a replication ledger. They need their agent to get smarter
  over time. Whether that improvement traces back to Kanerva 2009 or to
  trial-and-error is irrelevant to the user.
- The ingestion pipeline (manual, agent-curated, watchdog) is three
  separate systems for getting papers into the substrate. This is
  proportional effort for an academic meta-science project, not for a
  coding assistant.
- The "claim! macro" that resolves paper references at build time and
  emits runtime warnings when calibration drifts (section 6) is clever
  but couples the build to a runtime knowledge store. If the claim store
  is empty or corrupted, does the build fail?
- The "contributing to meta-science" angle (section 10) is aspirational
  but off-target. Roko's users want to ship code, not publish replication
  reports.
- The starter kit of 40 papers (section 8) would need someone to manually
  write structured claims with falsifiers for each one. That's a week of
  a domain expert's time for marginal runtime value.

### Risk
Low (unlikely to be implemented soon). The real risk is opportunity cost:
every hour spent on the replication ledger is an hour not spent on making
the plan-execute-gate-persist loop better.

### What to actually do
1. Add source attribution comments in the code where academic papers
   inform parameter choices. This costs nothing and helps maintainers.
2. Skip the Paper/Claim/ReplicationLedger type system entirely.
3. If heuristics (doc 14) get implemented, allow an optional `source`
   field that can reference a paper. That gives 80% of the traceability
   value at 5% of the cost.
4. Revisit when/if roko becomes a product used by multiple organizations
   who want to share validated knowledge.

---

## Collective Assessment

### The pattern across all seven docs

These documents share a consistent rhetorical structure:
1. Identify a real gap in the codebase (usually valid).
2. Propose an academically-inspired solution with deep references
   (Friston, Kanerva, Woolley, Gesell, Surowiecki, Beer, etc.).
3. Show how the solution composes with every other refinement.
4. Claim competitive moat / publishable novelty.
5. Estimate implementation at "a few weeks."

The problem is that step 2 consistently over-specifies. Each doc designs
a complete system at the granularity of Rust struct definitions and Bus
topic names, before the prerequisite infrastructure (the Bus itself, the
two-fabric Substrate) exists.

### What the codebase actually tells us

The codebase is **further along than the docs suggest**, but in simpler
forms:

| Refinement claim | Codebase reality |
|---|---|
| "Learning is three things stapled on" | 41 files, 35K LOC in roko-learn |
| "HDC is scaffolded" | Fully implemented: bind/bundle/permute/similarity, encoder, clustering, episode fingerprints, knowledge encoder |
| "c-factor doesn't exist" | 11-component CFactor with leave-one-out, pathology detection, dispatch bias, Policy |
| "No prediction tracking" | PredictionRecord with register/resolve/residuals, PredictiveScorer, active inference belief states |
| "Heuristics missing" | HeuristicRule exists in tier_progression, InsightRecord with confidence and support |

The docs were likely written before or in parallel with the implementation,
which is why they undercount what exists.

### Do these seven make the system better or just more complicated?

**Three are clearly useful (with simplification):**

1. **Doc 11 (HDC field on Engram)** -- the single highest-value change.
   Adding `fingerprint: Option<HdcVector>` and `query_similar()` unlocks
   real capabilities with infrastructure that already exists. SHIP IT.

2. **Doc 14 (Heuristic calibration)** -- upgrading `HeuristicRule` with
   trials/confirmations/violations is the right move. It turns pattern
   discovery into validated knowledge. SIMPLIFY and ship.

3. **Doc 13 (c-factor)** -- already built. The doc's main contribution
   is the "covariate not objective" framing, which should be a code
   comment. Keep the existing implementation, add the caveat.

**Two are directionally right but premature:**

4. **Doc 10 (self-learning loops)** -- the universal predict/correct loop
   is a good target architecture but the current direct-call wiring works
   fine. SIMPLIFY: instrument one more operator (Composer) and measure
   before going universal.

5. **Doc 12 (demurrage)** -- use-it-or-lose-it is the right decay model
   but the full economic framing (balance, tax rate, cold tier, novelty-
   weighted reinforcement) is over-designed for current scale. SIMPLIFY:
   add last_used timestamp and access_count to knowledge entries.

**Two should be set aside:**

6. **Doc 15 (exponential scaling)** -- strategy document, not engineering
   spec. The north-star metric and anti-metrics are useful; the seven
   loops are aspirational. SKEPTICAL: extract the metrics, skip the
   framework.

7. **Doc 16 (research-to-runtime)** -- a meta-science project that
   doesn't serve the self-hosting workflow. DEFER: add source comments
   in code, skip the type system.

### The dependency trap

The biggest risk is that each doc claims to "compose" with every other
doc. This creates an implicit dependency graph where nothing can ship
independently:
- Demurrage needs HDC for novelty-weighted reinforcement
- c-factor needs HDC for diversity measurement
- Heuristics need the Bus for calibration updates
- Scaling loops need all of the above
- Research-to-runtime needs heuristics

The correct approach is to break these dependencies and ship each piece
with simpler alternatives to its cross-doc dependencies. Add the HDC
fingerprint without demurrage. Add heuristic calibration without the Bus.
Add use-based decay without HDC novelty weighting. Each piece delivers
value independently; the composition is a bonus that arrives later.

### Recommended implementation order

1. `fingerprint: Option<HdcVector>` on Engram + `query_similar()` (doc 11)
2. `Calibration` struct on `HeuristicRule` + post-episode updates (doc 14)
3. `last_used_at` + `access_count` on knowledge/playbook entries (doc 12)
4. North-star metric instrumentation (doc 15, section 9 only)
5. "c-factor is a covariate" caveat on `compute_cfactor` (doc 13)
6. Source attribution comments for academic parameters (doc 16, minimal)

Total estimated effort for all six: 2-3 weeks. This captures 80% of the
value from all seven docs at 20% of the proposed cost.

--- END 02-learning-audit.md ---

--- BEGIN 02-foundation-learning.md ---

# Foundation And Learning

## Foundation: what to keep

### Keep the diagnosis, narrow the doctrine

The foundation set correctly identifies the right redesign pressure:
- durable records are not the whole runtime story;
- transport deserves explicit architectural status;
- several downstream concerns become cleaner once the runtime has a first-class
  bus or pulse concept.

That should survive the audit.

What should not survive unchanged is the jump from:
"transport is under-modeled"
to
"therefore every operator, trait, and noun should be redefined around a total
dual-medium worldview."

### Strongest foundational moves

- Treat storage vs transport as a real architectural axis.
- Keep `Pulse` as the likely transport noun if a new noun is needed.
- Use `Bus` as the runtime seam that carries transport explicitly.
- Use StateHub/projection logic as the practical bridge from live events to
  stable UI and operator surfaces.

### Foundational moves that need narrowing

- `Datum` should not become universal just because it is elegant. Use it only
  where medium polymorphism proves its worth.
- Do not rewrite every operator API around dual-medium input in the first pass.
- Treat the seven-step loop as a helpful reference architecture, not a law that
  every crate must immediately mirror.

## Foundation: biggest risks

### 1. Over-generalized operator algebra

The proposed operator generalization is attractive on paper but too broad as a
first migration target. Different operators want different abstractions.

Safer sequence:
- add a transport contract;
- identify which operators genuinely need dual-medium handling;
- only then widen traits or introduce local polymorphic wrappers.

### 2. Kernel rhetoric can outrun kernel need

The redesign does not need a full metaphysical restatement of the kernel before
it has a small number of new runtime contracts that are obviously useful.

Prefer:
- a transport contract;
- a small set of event topics or envelopes;
- projection contracts;
- explicit replay and subscription semantics.

Be careful with:
- total renaming passes;
- universal operator algebra;
- new foundational nouns that do not buy a concrete simplification.

### 3. Glossary can harden hypotheses too early

The glossary is useful, but it currently hardens many proposed nouns as if they
were settled redesign-level concepts. It should distinguish:
- current canonical terms;
- target-state terms likely to become canonical;
- exploratory or historical terms.

## Learning: what to keep

### Evented calibration is the best core idea

The strongest learning idea is not grand cybernetics. It is the practical move
toward shared calibration loops:
- expectation;
- outcome;
- discrepancy;
- adjustment.

This should remain central.

### Heuristics as a middle layer are worth building

A typed, inspectable heuristic layer between raw episodes and distilled
playbooks is one of the best ideas across the entire set.

That means:
- typed heuristic objects;
- visible provenance;
- challenge and contradiction records;
- calibration history;
- promotion/demotion rules tied to runtime evidence.

### HDC has real value in a narrower role

HDC is useful as:
- cheap similarity search;
- clustering aid;
- retrieval acceleration;
- lightweight representation for durable knowledge indexing.

It should not become:
- universal semantic truth geometry;
- reliable consensus detector;
- the hidden explanation for all future memory or reasoning behavior.

## Learning: what needs narrowing or deferral

### 1. Active inference claims are too large

Better framing:
- "calibration-driven control";
- "evented prediction/outcome scaffolding";
- "routing and prompt feedback loops first."

### 2. Worldview and falsifier rhetoric exceeds current mechanism

The conceptual story is interesting, but it is better as a later layer on top
of heuristics, contradictions, and typed claims. As a redesign target, it is
too abstract too early.

### 3. Demurrage is ahead of the memory model

Demurrage should be treated as:
- a hypothesis for future memory shaping,
not
- the governing explanation of memory or forgetting.

### 4. c-factor is not yet a stable core metric

Until it has one clear interpretation and one trusted measurement path, it
should be described as a coordination-health experiment, not a mature
collective-intelligence scalar.

### 5. Research-to-runtime is still a narrative layer

The instinct is good: make external knowledge auditable and contestable. The
problem is scope. Build this in ascending order:
- typed claims;
- provenance and source quality;
- contradiction and replication;
- only later, richer research-economy semantics.

## Recommended rewrite principles for this area

1. Keep the transport diagnosis and the need for cleaner runtime seams.
2. Rewrite foundation docs as a tighter target-state architecture, not a full
   kernel ideology.
3. Treat `Bus` as the main kernel addition and `Datum` as optional.
4. Reframe learning around calibration and typed heuristics, not sweeping
   cybernetic claims.
5. Reduce the number of places where HDC, demurrage, c-factor, and claims are
   described as foundational laws.
6. Move the more speculative parts into clearly marked research or future-work
   sections.

--- END 02-foundation-learning.md ---

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

# Batch AUD03: Simplify learning docs (REF10-16) and mark deferred concepts

**Audit refs**: 02-learning-audit.md (full file), 02-foundation-learning.md (learning section),
05-refinement-matrix.md (REF10-16 rows). Applies the audit's "simplify" and "defer" verdicts
to `docs/05-learning/` and `docs/06-neuro/`.

Read these files first:

- `tmp/refinement-audit-runner/context-pack/00-AUDIT-RULES.md`
- `tmp/refinements-audit/02-learning-audit.md` (full file -- verdict per REF10-16)
- `tmp/refinements-audit/02-foundation-learning.md` (learning section)
- `tmp/refinements-audit/05-refinement-matrix.md` (REF10-16 rows)
- `tmp/refinements-audit/06-codebase-reality-check.md` (section 5: Learning Subsystem Reality)
- `docs/05-learning/INDEX.md`
- `docs/05-learning/18-self-learning-cybernetic-loops.md`
- `docs/05-learning/19-heuristics-worldviews-and-falsifiers.md`
- `docs/05-learning/20-research-to-runtime.md`
- `docs/06-neuro/INDEX.md`
- `docs/06-neuro/12-4-tier-distillation-pipeline.md`
- `docs/00-architecture/04-decay-variants.md`
- `docs/00-architecture/14-c-factor-collective-intelligence.md`
- `docs/00-architecture/25-attention-as-currency.md`

## Task

The refinements-runner wrote demurrage, worldview algebra, replication ledger,
c-factor control doctrine, and universal active-inference framing into the
learning and neuro docs as if they were current or near-term architecture. The
audit found that roko-learn already has 42 modules and 35,847 LOC -- far more
than the refinements acknowledge -- and that the proposed additions are mostly
premature. Mark deferred concepts as deferred. Acknowledge what already exists.

## Current state (evidence)

The audit found these specific issues:

1. **Demurrage** (REF12): Zero lines of demurrage code exist. `Decay` enum has
   `Exponential`, `Linear`, `Step`, `None` -- standard decay, not economic
   demurrage. The docs describe demurrage as the governing memory model.
   Audit verdict: **DEFER**.

2. **Worldview/falsifier/dissonance** (REF14): Only `HeuristicRule` in
   `roko-neuro/src/tier_progression.rs` exists. No `Worldview` struct, no
   `Falsifier`, no dissonance tracking. Audit verdict: **NARROW** -- keep typed
   heuristics and contradiction tracking, defer the full worldview stack.

3. **Replication ledger** (REF16): Zero lines of code. No `Claim`, `Paper`, or
   replication ledger exists anywhere. Audit verdict: **NARROW** -- the
   provenance idea is good, the full paper economy is premature.

4. **c-factor control doctrine** (REF13): `CFactorPolicy` exists and is wired,
   but it is a single numeric signal for routing, not the continuously-computed
   Woolley collective-intelligence metric the docs describe. Audit verdict:
   **DEFER** as a canonical treatment.

5. **Universal active inference** (REF10): The docs frame every operator as a
   predictor. Reality: `active_inference.rs` is ~200 lines implementing a
   working Bayesian tier selector. The existing code IS active inference but is
   narrow and focused, not the universal doctrine the docs present.

6. **roko-learn undercount**: The docs say learning is "three things stapled on
   the side." Reality: 42 modules, 35,847 LOC, including cascade router, skill
   library, pattern discovery, drift detection, bandits, and more. The docs
   should acknowledge this.

## Implementation

### 1. Mark demurrage as deferred in learning/architecture docs

In `docs/00-architecture/04-decay-variants.md`:
- Add an implementation-status callout at the top:
  `> **Implementation status**: The Decay enum (Exponential, Linear, Step, None)
  > is **Shipping**. The demurrage extension (balance, reinforcement, cold-tier
  > freeze/thaw) described in this doc is **deferred** -- 0 lines of demurrage
  > code exist in the codebase.`

In `docs/00-architecture/25-attention-as-currency.md`:
- Add an implementation-status callout:
  `> **Implementation status**: Target-state concept. No demurrage, balance, or
  > attention-currency code exists. This doc describes a deferred research
  > direction.`

In `docs/05-learning/INDEX.md`:
- Where demurrage is mentioned in the overview, qualify it as deferred
- Where the "four durable learning surfaces" are listed, note which are
  shipping vs. planned

### 2. Narrow worldview/falsifier to "typed heuristics + contradiction tracking"

In `docs/05-learning/19-heuristics-worldviews-and-falsifiers.md`:
- Add an implementation-status callout:
  `> **Implementation status**: `HeuristicRule` exists in roko-neuro. The full
  > worldview/falsifier/dissonance stack described here is **target-state**.
  > Near-term: typed heuristic specs and contradiction tracking. Deferred:
  > worldview clustering, dissonance algebra, and belief export/import.`
- Do NOT delete the design content

In `docs/06-neuro/12-4-tier-distillation-pipeline.md`:
- If it describes worldview objects as current, mark them as target-state

### 3. Mark replication ledger as deferred

In `docs/05-learning/20-research-to-runtime.md`:
- Add an implementation-status callout:
  `> **Implementation status**: Target-state concept. No Claim, Paper, or
  > replication ledger code exists. The provenance-backed heuristic idea is
  > valuable; the full paper economy (claims, replication trials, ledger)
  > is deferred.`

### 4. Narrow c-factor to observability-first

In `docs/00-architecture/14-c-factor-collective-intelligence.md`:
- Add an implementation-status callout:
  `> **Implementation status**: `CFactorPolicy` exists in roko-core and is
  > wired to the cascade router as a routing signal. The broader c-factor
  > doctrine (continuous Woolley measurement, Bus/Substrate statistics,
  > conditional Policy intervention) described here is **target-state**.
  > Current recommendation: treat c-factor as an observability metric first,
  > a control input second.`

### 5. Qualify universal active-inference framing

In `docs/05-learning/18-self-learning-cybernetic-loops.md`:
- Add an implementation-status callout:
  `> **Implementation status**: Active inference EXISTS in roko-learn
  > (`active_inference.rs`, ~200 lines) as a working Bayesian tier selector.
  > Prediction tracking EXISTS (`prediction.rs`). The per-operator
  > predict-publish-correct doctrine described here is **target-state** --
  > currently only the Router has rich prediction/outcome signals.`

### 6. Acknowledge the existing learning subsystem

In `docs/05-learning/INDEX.md`:
- In the overview section, add or update a paragraph acknowledging:
  `roko-learn currently has 42 modules and ~36K LOC, making it the most
  substantial subsystem in the codebase. Key shipping modules include:
  cascade_router (3-stage model routing), runtime_feedback, skill_library,
  episode_logger, bandits, prediction tracking, active inference, drift
  detection, pattern discovery, and provider health circuit breaker.`
- This counters the refinements' implication that learning is nascent

## Write scope

- `docs/05-learning/INDEX.md`
- `docs/05-learning/18-self-learning-cybernetic-loops.md`
- `docs/05-learning/19-heuristics-worldviews-and-falsifiers.md`
- `docs/05-learning/20-research-to-runtime.md`
- `docs/06-neuro/INDEX.md` (if it cites deferred concepts as current)
- `docs/06-neuro/12-4-tier-distillation-pipeline.md`
- `docs/00-architecture/04-decay-variants.md`
- `docs/00-architecture/14-c-factor-collective-intelligence.md`
- `docs/00-architecture/25-attention-as-currency.md`

## Rules

1. **Mark, do not delete.** Deferred concepts are valuable future specs. Add
   implementation-status callouts; do not remove design content.
2. **Acknowledge existing code.** The audit found roko-learn is far more
   substantial than the docs imply. Credit what exists.
3. **Use three tiers**: "Shipping" for wired modules, "Target-state" for
   designed-but-not-built, "Deferred" for concepts the audit recommends
   postponing.
4. **Do not touch architecture foundation docs** (02b, 07b, 08, 09) -- those
   are AUD02's scope.
5. **Do not touch the glossary** -- that is AUD06's scope.
6. **Do not edit docs outside `05-learning/`, `06-neuro/`, or the three
   architecture files listed** in the write scope.

## Done when

- Demurrage, worldview algebra, replication ledger are explicitly marked as
  deferred in every doc that describes them
- c-factor is qualified as "observability-first, control-second"
- Active inference is qualified as "exists for routing, target-state for
  universal operator coverage"
- `docs/05-learning/INDEX.md` acknowledges the 42-module, 36K LOC reality
- No design content was deleted
- Every edited file has a visible implementation-status callout
- Final message lists every concept marked as deferred and the file it appears in
