# Design Principles

> The seven principles that govern every architectural decision in Roko. Each principle
> is stated as a directive, explained with rationale, and illustrated with concrete examples
> from the codebase. When two principles conflict, they are listed in priority order below.

**Status**: Written (principles apply to Shipping-tier code)
**Crate**: — (cross-cutting; no single crate)
**Depends on**: [`status/vision.md`](../status/vision.md)
**Last reviewed**: 2026-04-17

---

## TL;DR

Seven principles. In rough priority order:

1. The scaffold IS the product
2. Nothing is lost
3. Composability over completeness
4. Gate failure is a verdict, not an error
5. Memory decays; knowledge compounds
6. Speeds must not bleed
7. The system must be able to read its own requirements

They are not independent — each reinforces the others. Principle 1 motivates all the rest.

---

## Principle 1 — The Scaffold IS the Product

> Given the same LLM, agent performance varies dramatically based on the surrounding
> harness. The harness is the differentiated asset. Build the harness.

### Rationale

The empirical case is made in [`status/vision.md`](../status/vision.md): SWE-bench shows
a 2× performance spread from harness design alone; Meta-Harness shows +7.7 points on
classification and +4.7 on math from harness optimization at 4× fewer tokens; FrugalGPT
shows GPT-4 quality at 2% cost from cascade routing. The model is not the moat.

This principle has a strategic implication: Roko should never bet its value on proprietary
model access. Model access is a commodity; scaffold accumulation is not. A Roko deployment
that has run for a year has learned its domain in ways that cannot be replicated by
spinning up a fresh instance with the same LLM.

### Examples

- `roko-compose`'s `SystemPromptBuilder` implements Liu et al. U-shape placement — a
  scaffold-level optimization that improves recall without changing the model.
- `roko-gate`'s adaptive thresholds use EMA over observed pass rates — the gate pipeline
  calibrates to domain difficulty over time, compounding scaffold knowledge.
- `roko-learn`'s `CascadeRouter` bandit learns which models work best per task type —
  experience accumulated in the scaffold, not in the model.

---

## Principle 2 — Nothing Is Lost

> Every execution leaves a learning signal. No work is discarded without being recorded.
> Failure is as valuable as success.

### Rationale

In a system designed to improve from experience, discarding information is a design error.
A failed gate run contains exactly the information needed to calibrate gate thresholds. A
slow or expensive LLM call teaches the router which tasks to avoid routing to that backend.
A hallucinated code reference teaches the context assembler which retrieval strategies to
weight down.

Nothing is lost has two implementation manifestations:

1. **Everything produces an `Engram`** — every significant execution artifact, whether a
   task output, a gate verdict, a cost event, or a failure, is recorded as a scored,
   content-addressed `Engram`. The `Engram` persists even when the agent's plan fails.

2. **Failure signals update learning** — the `roko-learn` subsystem's episode logger records
   the full execution trace including failures. Playbook rules, gate calibration, and bandit
   updates all consume failure signals.

### Examples

- A gate pipeline that terminates at the Compile rung records the failure as an `Engram`
  tagged with `Kind::GateFailure`. The learning subsystem uses this to update the gate's
  adaptive threshold.
- `roko-orchestrator`'s hash-chained event log records every state transition. When a crash
  occurs, the full execution history is preserved for replay — nothing is lost on failure.
- `roko-learn`'s regression detector compares current performance against a baseline window.
  It can only do this because every task's outcome is recorded, not just the successes.

---

## Principle 3 — Composability over Completeness

> Ship six general traits, not sixty domain-specific APIs. Users compose the scaffold
> they need from general-purpose building blocks.

### Rationale

A complete framework tries to anticipate every use case. A composable framework provides
the smallest set of general primitives from which every use case can be assembled. Roko
chooses composability because:

- The set of agent domains is unbounded. A coding agent needs different gates than a chain
  agent; a research agent needs different context assembly than a document agent.
- Domain-specific code paths create maintenance burden that scales with the number of
  domains rather than with the number of primitives.
- Composability is a force multiplier: a user who masters the six traits can build any
  agent; a user who learns domain-specific APIs must relearn for each domain.

### Examples

- Six traits (`Substrate`, `Scorer`, `Gate`, `Router`, `Composer`, `Policy`) cover every
  agent capability Roko ships today and every capability described in the research docs.
  No new trait has been needed since the Synapse Architecture was defined.
- The `roko-gate` crate ships 11 gate implementations, but they are all implementations
  of the single `Gate` trait. A domain-specific gate is a new `struct` that implements
  one method — `evaluate(&self, engram: &Engram) -> GateVerdict`.
- The `SystemPromptBuilder` is a `Composer` implementation, not a special-purpose type.
  A domain-specific prompt template is a new `Composer` implementation, not a new API.

---

## Principle 4 — Gate Failure Is a Verdict, Not an Error

> A gate that rejects output is performing its function correctly. Rejection is not
> a failure of the system — it is the system working as designed.

### Rationale

Many systems treat verification failure as an exceptional condition: an error that
should not happen in a well-functioning system, to be caught and handled. This framing
produces fragile verification: gates are made lenient to avoid "errors", thresholds are
padded to reduce noise, and genuine quality problems slip through as the system optimizes
for a clean error log.

Roko inverts this: gate rejection is a first-class outcome with its own `GateVerdict`
variant. The pipeline does not "fail" when a gate rejects — it produces an authoritative
verdict about the quality of the output. The verdict is recorded, used for learning, and
returned to the caller as a clean result, not an exception.

### Examples

- `roko-gate`'s pipeline is defined over `GateVerdict` values, not `Result` types.
  A rejected output produces `GateVerdict::Rejected` with a causal chain — not a panic
  or an `Err`.
- Monotonic ratcheting: once a test run passes, the test gate threshold increases. The
  system commits to maintaining quality, not just achieving it once.
- Forensic causal replay: when a gate rejects, the system records enough information to
  replay the exact conditions that triggered the rejection. This enables debugging of
  gate behavior over time.
- The `roko-learn` subsystem receives `GateVerdict` as a training signal. A rejected
  output is as valuable a learning input as an accepted one.

---

## Principle 5 — Memory Decays; Knowledge Compounds

> Short-term data expires. Long-term knowledge grows. The system uses decay to manage
> cognitive load and knowledge accumulation to build durable advantage.

### Rationale

Biological memory is not a perfect record — it decays, is reconstructed, and prioritizes
recently reinforced patterns. This is not a limitation; it is an adaptation. A cognitive
system that never discards anything is overwhelmed by irrelevant historical data. Decay
creates attention: the records that are most recent, most reinforced, or most novel remain
available; the rest fade.

Knowledge (encoded in `Neuro`) is different from data (encoded in individual `Engram`s).
Knowledge is the extracted, validated pattern — the heuristic, the causal link, the warning.
Knowledge does not decay the way data does; it compounds as more evidence confirms it and
migrates through validation tiers (Transient → Working → Consolidated → Persistent).

### Examples

- Four decay variants in `Decay`: demurrage (balance-based holding fee), reinforcement
  (decay rate slows under repeated access), novelty weighting (new records score higher
  and decay faster), cold-tier freeze/thaw (dormant records frozen, revived on query).
- `Neuro` knowledge tiers: a new heuristic starts as `Transient`, advances to `Working`
  when supported by multiple episodes, to `Consolidated` when cross-validated, to
  `Persistent` when robust across domains. Only `Persistent` knowledge survives an agent
  restart.
- `Dreams` (scaffold): the offline consolidation loop replays high-utility episodes
  (NREM-style) and generates imaginative counterfactuals (REM-style) to build robust
  knowledge without additional real-world execution.

---

## Principle 6 — Speeds Must Not Bleed

> Gamma, Theta, and Delta cognitive speeds operate independently. Slow consolidation work
> must not block fast reactive work. Fast reactive noise must not pollute slow consolidated
> knowledge.

### Rationale

The three-speed design (see [`reference/07-speeds/`](07-speeds/)) maps to the different
temporal requirements of different cognitive tasks:

- **Gamma (~5–15s)**: A user or upstream system is waiting. Latency is the constraint.
  No blocking work, no database scans, no heavy computation.
- **Theta (~75s)**: Multi-step reasoning is in progress. Moderate latency is acceptable.
  Context assembly and plan evaluation can run.
- **Delta (hours)**: The system is idle or running background consolidation. Throughput
  is the constraint. Heavy computation, knowledge compression, playbook construction.

Allowing these speeds to mix produces one of two failures: either Delta-speed work blocks
Gamma-speed responses (user-facing latency spikes), or Gamma-speed noise contaminates
Delta-speed knowledge (the consolidation window fills with low-quality reactive outputs).

### Examples

- `Dreams` runs in a dedicated background process, not in the critical path of the
  cognitive loop. A Delta-speed consolidation pass cannot delay a Gamma-speed tool call.
- `Neuro` knowledge updates are buffered and applied in batches during Theta or Delta
  windows — not inline during Gamma-speed execution.
- `roko-learn`'s 10+ update operations apply asynchronously after the agent turn completes.
  The turn itself (Gamma-speed) is not blocked by learning subsystem writes.

---

## Principle 7 — The System Must Be Able to Read Its Own Requirements

> Self-hosting is not aspirational. It is a current operational capability and a
> continuous integration requirement.

### Rationale

A system that improves itself must be able to understand its own specification. If the
requirements are opaque to the system — too informal, too ambiguous, or in a format the
system cannot parse — then self-improvement is blocked at the input.

This principle has a practical test: `roko prd` must be able to read any PRD in the
`docs/` tree, extract actionable tasks, and produce a valid execution plan. When the
PRDs are rewritten, the test is whether Roko can still parse and act on them.

This principle also drives the documentation refactor that produced this file. The target
documentation structure — one concept per file, explicit status tags, relative links —
is optimized not just for human readability but for agent parsability. A documentation
tree where every concept lives on exactly one page with a consistent template is a
documentation tree that an agent can navigate reliably.

### Examples

- `roko prd` reads PRD files in the `docs/` tree and generates structured task plans.
  The active development workflow uses this command daily.
- The refactor from `docs/` to `new-docs/` is itself being executed by a Roko agent that
  reads the cluster plan, understands the target structure, and writes the output files.
- The `CONVENTIONS.md` writing rules include constraints that serve agent parsability:
  relative links (not absolute), consistent frontmatter, one concept per file. These
  constraints make the documentation tree machine-navigable.

---

## Principle Conflicts and Resolution

The seven principles occasionally conflict. The general resolution order is:

1. **Principle 1** (scaffold is the product) is never compromised — it is the strategic
   foundation.
2. **Principle 4** (gate failure is a verdict) is never compromised — compromising
   verification to avoid "errors" is the single most common failure mode in agent systems.
3. **Principle 6** (speeds must not bleed) governs latency-sensitive decisions — always
   check whether a proposed operation belongs in the right speed tier.
4. **Principle 3** (composability) and **Principle 5** (memory decays) are guidelines
   rather than hard constraints — reasonable exceptions exist.
5. **Principles 2 and 7** are aspirational: violations are technical debt, not correctness
   failures.

---

## See Also

- [`status/vision.md`](../status/vision.md) — empirical evidence for Principle 1
- [`reference/11-crate-map.md`](11-crate-map.md) — how these principles manifest in crate structure
- [`research/frontier-summary.md`](../research/frontier-summary.md) — how the principles map to research frontiers

## Open Questions

- Should Principle 4 (gate failure is a verdict) extend to all operator trait failures,
  not just `Gate`? The `Router`, `Composer`, and `Scorer` traits all produce outcomes
  that are either correct or informative — not truly exceptional.
- Is there a Principle 8 missing? The "two mediums, two fabrics" medium split feels
  principle-level, but it is currently described as architecture rather than principle.
