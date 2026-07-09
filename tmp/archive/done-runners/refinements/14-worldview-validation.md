# Worldview Validation from Lived Experience

> **TL;DR**: Agents don't just accumulate facts — they accumulate
> *heuristics*, *priors*, and *mental models*. These are the most
> valuable Engrams in the system and the most dangerous if stale.
> This doc proposes a structured type — `Heuristic` — with
> explicit pre/post conditions, a track record, and a calibration
> score. Every episode either reinforces, violates, or extends a
> heuristic. Over time, worldviews emerge from the co-citation
> network of heuristics that hold up under lived experience.
> Nothing in the literature does this for code agents.

> **For first-time readers**: Roko today stores distilled knowledge as
> playbooks — concrete sequences of actions that worked before.
> Heuristics are more abstract: rules of thumb with preconditions, a
> prediction, and a track record of confirmations vs violations.
> Worldviews cluster heuristics that co-occur in successful episodes.
> This doc is the most user-facing of the learning refinements — it
> produces externalized, inspectable "beliefs" the user can browse,
> challenge, and export. Read 11 (HDC) and 12 (demurrage) first.

## 1. What's missing in playbooks

Roko's current "learned knowledge" is expressed as *playbooks* in
`roko-learn`: concrete distilled sequences of actions that worked
before. Playbooks are useful but are:

- **Over-specific**: bound to particular tools and file layouts.
- **Brittle**: one changed path breaks the whole playbook.
- **Flat**: no notion of *why* they worked, only *that* they did.
- **Unattributable**: a playbook's success can't be traced back to
  a specific belief that was validated.

Humans don't operate with playbooks alone. They operate with
**heuristics**: compact, often-wrong-but-useful rules of thumb with
a sense of when to apply them. "If the test is flaky, add logging
before touching the logic." "If the build fails after a merge,
check lockfiles first." These are *meta* — they tell you what to
do, not what to do it with.

## 2. The `Heuristic` type

A first-class Engram variant:

```rust
pub struct Heuristic {
    pub id: Uuid,

    /// Natural language, canonical form.
    pub claim: String,

    /// Conditions under which the claim applies.
    /// Structured so they can be matched against a candidate
    /// situation automatically.
    pub preconditions: Vec<Predicate>,

    /// Expected outcome if the claim holds and preconditions match.
    pub prediction: Predicate,

    /// HDC fingerprint of the (preconditions + claim) tuple.
    /// Enables similarity search in heuristic space.
    pub fingerprint: HdcVector,

    /// Calibration record, incrementally maintained.
    pub calibration: Calibration,

    /// Parent heuristics this specializes or generalizes.
    pub lineage: Vec<HeuristicId>,

    /// The episodes where it applied (as lineage).
    pub receipts: Vec<EpisodeHash>,
}

pub struct Calibration {
    pub trials: u32,              // times preconditions matched
    pub confirmations: u32,       // times prediction held
    pub violations: u32,          // times prediction failed
    pub brier_score: f64,         // calibration quality
    pub last_trial_at: Timestamp,
    pub confidence_interval: (f64, f64), // Wilson CI
}
```

`Predicate` is a small, extensible enum that can be matched against
a situation Engram:

```rust
pub enum Predicate {
    LanguageIs(Language),
    FileMatches(Glob),
    ToolAvailable(ToolId),
    GateRecentlyFailed(GateId),
    AgentRoleIs(Role),
    Custom(Box<dyn PredicateFn>),
    And(Vec<Predicate>),
    Or(Vec<Predicate>),
    Not(Box<Predicate>),
    SimilarTo { fingerprint: HdcVector, threshold: f64 },
}
```

## 3. The lifecycle of a heuristic

### 3.1 Birth

Heuristics are born three ways:

1. **Distilled** from a cluster of episodes by the existing playbook
   distillation in `roko-learn`, but emitting a *precondition →
   prediction* rather than a sequence of steps.
2. **Stated** by an agent explicitly: "I think X, because Y." The
   agent writes a Heuristic Engram as a tool-call; other agents see
   it; it enters the calibration loop.
3. **Imported** from research (see `16-research-to-runtime.md`).

### 3.2 Test

Every episode is a potential test:

- Before executing, the Composer scans for heuristics whose
  preconditions match the current situation (HDC similarity +
  predicate evaluation).
- Matching heuristics are injected into the system prompt as
  *advisory claims*.
- The agent is free to use, ignore, or contradict them.
- After the episode, a Policy inspects outcomes and updates each
  heuristic's calibration.

```rust
pub trait Calibrator {
    fn score(&self, h: &Heuristic, episode: &Episode) -> Verdict;
}

pub enum Verdict {
    Confirmed,                // prediction held
    Violated,                 // prediction failed
    Irrelevant,               // preconditions didn't actually match
    Refined(Heuristic),       // new, narrower version learned
    Generalized(Heuristic),   // new, broader version learned
    Refuted,                  // so badly wrong we should retire it
}
```

### 3.3 Adjust

Calibration updates are a conjugate Beta update:

```text
confirmations ← confirmations + 1  if Confirmed
violations    ← violations + 1     if Violated
```

Confidence interval recomputed via Wilson score interval. The
heuristic's *effective weight* in prompt selection is a function of
confidence-lower-bound — optimistic-under-uncertainty. Young
heuristics with two confirmations are *exactly as useful* as
veterans with 200 until one fails.

### 3.4 Retire

Heuristics don't get deleted. Their balance (see `12-knowledge-demurrage.md`)
decays, they move to cold tier, but their hash remains valid so any
Engram that cited them still has a resolvable lineage. *History is
preserved, attention is not owed*.

### 3.5 Evolution

When a heuristic is violated, the Calibrator can spawn a *refined*
version that adds a new precondition ("except when X is true"). Over
time, a heuristic tree emerges: a rough root, specializations for
discovered exception domains, generalizations when a pattern shows
up across domains. This is Quinlan's ID3 in a different dress, but
on a live stream rather than a batch.

## 4. Worldviews as co-citation clusters

A **worldview** is not a single heuristic — it's a set of
mutually-citing heuristics that co-occur in successful episodes.

```rust
pub struct Worldview {
    pub id: Uuid,
    pub core_heuristics: Vec<HeuristicId>,
    pub coherence_score: f64,     // avg pairwise HDC similarity
    pub effectiveness_score: f64, // avg calibration of members
    pub domain_fingerprint: HdcVector, // where this worldview applies
}
```

Worldviews emerge from community detection on the heuristic
citation graph. No one declares a worldview; it's *observed* from
the statistics. Worldview X scores well on web-frontend tasks;
worldview Y scores well on distributed-systems tasks. When a new
task comes in, the Router picks the worldview whose
`domain_fingerprint` is closest and injects its heuristics.

This generalizes *personality* across agents without requiring
hand-engineered personas.

## 5. Multiple worldviews, deliberately

One of the key moves from `13-collective-intelligence-c-factor.md`:
**diversity of opinion is a load-bearing property**. Therefore: keep
more than one active worldview even when one is currently dominant.

- **Main worldview**: highest effectiveness on recent tasks.
- **Challenger worldview**: second-highest, deliberately invoked on
  some fraction of tasks (ε-greedy, Thompson-style).
- **Niche worldviews**: specialists preserved in cold tier, thawed
  when domain fingerprints match.

This is the structural answer to "how do we avoid monoculture" posed
in doc 13 — the answer is *keep your priors plural on purpose*.

## 6. Lived experience, not stated belief

The critical difference between this and "LLM-stated preferences":
**every heuristic's calibration comes from actual outcomes, not from
the agent's self-report**. An agent can confidently claim X; if X
fails five times in a row, the heuristic's calibration drops
regardless of how confident the claim was. The substrate is empirical
ground truth; the agent is a hypothesis generator.

This is the central philosophical commitment: **assert beliefs, test
them against Pulses, preserve outcomes, update calibrations.** The
Bus is the reality-check channel.

## 7. Perspective-taking: heuristics about heuristics

Agents should model *other agents' heuristics*. A
`peer_heuristic_model`:

```rust
pub struct PeerModel {
    pub agent: AgentId,
    pub believed_heuristics: Vec<(HeuristicId, f64)>, // with weight
    pub calibration: Calibration, // how well we predict *them*
}
```

This directly feeds `peer_prediction_accuracy` in the c-factor
computation (`13`). Being a good team member *means* modeling your
teammates' priors accurately. It's social perceptiveness made
algorithmic.

## 8. Dissonance detection

When the active Heuristic set contains two that would predict
different outcomes for the current situation, that's a *dissonance
event*. Dissonance is surfaced explicitly:

```rust
pub struct Dissonance {
    pub heuristics: [HeuristicId; 2],
    pub predictions: [Predicate; 2],
    pub situation: SituationHash,
}
```

Dissonance events are high-information-content. The Policy layer
can:

1. Spawn a decisive agent to act and collect ground truth.
2. Use the outcome to update both heuristics' calibrations.
3. Prefer dissonance-resolving tasks in scheduling (active learning).

This is Festinger's cognitive dissonance operationalized as a
learning signal. Most agent systems *hide* contradictions by
picking one; we *surface* them because they're where learning
happens fastest.

## 9. Externalization: the heuristic library

The heuristic store is queryable via CLI and HTTP:

```bash
roko heuristic list --confidence 0.7
roko heuristic show <id>
roko heuristic similar <situation>
roko heuristic stats         # calibration health
roko heuristic export        # JSONL for analysis
```

This is what makes Roko *inspectable*. A user can read the agent's
current beliefs, see which ones are battle-tested, see which ones
are recent hypotheses. You get a literal answer to "what does my
agent think it knows?"

No other framework offers this. It's net-new product surface area.

## 10. Sharing heuristics across deployments

Because heuristics are content-addressed Engrams with calibration
metadata, they're *exportable and importable* between Roko
instances:

```bash
roko heuristic export --confidence 0.8 > my-heuristics.jsonl
# on another machine:
roko heuristic import --trust 0.5 < my-heuristics.jsonl
```

Imported heuristics enter with their remote calibration and a
configured trust factor; they get revalidated in the new context
before they influence decisions. This is the primitive for a
**heuristic commons** — a git-like exchange of empirically-validated
agent knowledge.

Phase 2+ chain witnesses could be applied here: a heuristic with
1,000 confirmations across 50 deployments and a chain signature
from each is *very* trustworthy. A heuristic someone typed in last
week is not.

## 11. Why this is different from RAG

RAG retrieves facts. Heuristic-validation retrieves *operating
principles with a track record*. The retrieval isn't "find me
similar text" but "find me the priors that held up last time I was
in a situation like this." That's a categorically different kind of
memory than vector-store RAG, and it's enabled by demurrage +
lineage + HDC + active inference all being in the same system.

## 12. Minimal viable implementation

Fast path:

1. Add `Heuristic` Engram variant + `Predicate` enum. Two days.
2. Wire heuristic retrieval into the Composer (prompt injection). One day.
3. Wire Calibrator into the post-episode Policy pass. Two days.
4. Implement co-citation-based Worldview clustering. One week.
5. CLI surface (`roko heuristic *`). Three days.
6. Dissonance detection and active-learning scheduler. One to two weeks.

Biggest win per engineering-day is steps 1–3. They produce a
learnable library of priors without any further infrastructure.

## 13. Heuristic-level granularity: a worked example

Two heuristics about flaky tests:

```yaml
- id: h.flaky.42
  claim: "When a test is flaky in CI but passes locally, check for \
         timing or resource ordering assumptions."
  preconditions:
    - LanguageIs: Rust
    - GateRecentlyFailed: { gate: unit, intermittent: true }
  prediction:
    Predicate: TestTiminaRaceSuspected
  lineage: []
  calibration:
    trials: 41, confirmations: 28, violations: 13, brier_score: 0.21
  receipts: [ep_8712, ep_9311, ep_9822, ...]

- id: h.flaky.57  (refined child of h.flaky.42)
  claim: "When a Rust test is flaky in CI and uses tokio::test, \
         check the runtime flavor first."
  preconditions:
    - LanguageIs: Rust
    - GateRecentlyFailed: { gate: unit, intermittent: true }
    - Similar: { fingerprint: hdc(tokio_runtime_flavor), threshold: 0.7 }
  prediction:
    Predicate: TokioRuntimeFlavorMismatchSuspected
  lineage: [h.flaky.42]
  calibration:
    trials: 7, confirmations: 6, violations: 1
  receipts: [ep_9822, ep_9911, ...]
```

The parent is general; the child specializes. Both stay in the
library. Retrieval prefers the child (more specific precondition)
when it matches; falls back to the parent otherwise. This is the
ID3-on-a-stream pattern from §3.5 made concrete.

## 14. Meta-heuristics — heuristics about heuristics

The same framework applies one level up. A meta-heuristic:

```yaml
- id: mh.distill.01
  claim: "When refining a heuristic after a violation, the new \
         precondition should be observable *before* the action, \
         not inferred only from the outcome."
  preconditions:
    - Event: HeuristicRefinementProposed
  prediction:
    Predicate: RefinedHeuristicWillImproveCalibration
```

Meta-heuristics drive the Calibrator itself. A meta-heuristic with
low calibration says the refining strategy is broken; the system
adjusts how it produces child heuristics.

This is where Minsky's "society of mind" (§6.3 of doc 10) lands:
a layered architecture of predictors about predictors, each with
its own calibration. No magic; just recursive application of the
same primitive.

## 15. Permission gradient for user actions

Not all user-driven heuristic actions are equal. A safety gradient:

| Action | Auto? | Permission needed |
|---|---|---|
| Browse heuristics | Yes | None |
| Stake on a heuristic | Yes | None (it's play money) |
| Add a receipt manually | Yes | None |
| Edit a heuristic's *claim* | Ask | Per-session confirm |
| Edit a heuristic's *preconditions* | Ask | Per-session confirm |
| Retire a heuristic | Ask | Per-session confirm |
| Challenge (force recalibration on next 10 trials) | Yes | None |
| Import from commons (unsigned) | Ask | Explicit confirm |
| Import from commons (signed, >10 deployments) | Auto | None |
| Export to commons | Ask | Explicit confirm |

See `32-safety-sandbox-provenance.md` for the full permission model
the CLI/TUI/Web UI apply these to.

## 16. Interaction with domain profiles

Domain profiles from `25-domain-specific-agents.md` seed heuristic
libraries. A coding profile starts with ~30 canonical coding
heuristics; a research profile starts with the research claims from
`16-research-to-runtime.md` §8; a blockchain profile starts with
historical-exploit heuristics (reentrancy patterns, etc.).

When two profiles are installed together, their heuristics coexist.
Calibration is per-heuristic, not per-profile, so a cross-domain
heuristic that actually pays off in both domains gets reinforced
from both — exactly what a general principle should do. This is
the structural reason profiles *compose* cleanly (25 §10).
