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
