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
