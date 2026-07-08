# Research-to-Runtime Pipeline

> **TL;DR**: Roko should consume academic and industry research
> continuously, not as a one-time inspiration. This doc proposes a
> typed pipeline — `Paper → Claim → Heuristic → Trial → Calibration`
> — where every cited result becomes a testable hypothesis in the
> running system. Papers are Engrams; claims are candidate
> Heuristics; trials are episodes; calibrations are published.
> Over time, the system builds an empirical map of which academic
> findings hold up *in its actual deployment*. This is
> evidence-based engineering for agent runtimes.

> **For first-time readers**: Roko already draws on research — HDC from
> Kanerva, active inference from Friston, c-factor from Woolley, demurrage
> from Gesell. Today that influence is folklore: someone read a paper and
> wrote some code. This doc promotes *the paper itself* to a first-class
> Engram, *the claim* to a testable hypothesis with a falsifier, and
> *the system's own trials* to continuous re-replication. Read 14
> (heuristics) first; this is heuristic infrastructure specialized to
> academic provenance.

## 1. The state of research-in-code today

Roko already has research-inspired primitives scattered through the
codebase:

- HDC from Kanerva 2009.
- Active inference / FEP from Friston 2006.
- Predictive processing from Clark 2013.
- Stigmergy from Grassé 1959.
- c-factor from Woolley 2010.
- Demurrage from Gesell 1916.
- Bandits from Robbins/Auer.
- Playbook distillation echoes Sutton/Schmidhuber meta-learning.

But each is *imported as folklore*: an engineer reads a paper, writes
some Rust, the paper's specific claims and their calibration context
are lost. When the system behaves oddly, nobody can check whether
we're violating a precondition the paper stated.

The pipeline proposed here keeps papers and their claims *alive and
testable*.

## 2. Paper as Engram

```rust
pub struct Paper {
    pub id: Uuid,
    pub doi: Option<String>,
    pub arxiv: Option<String>,
    pub title: String,
    pub authors: Vec<String>,
    pub year: u16,
    pub venue: Option<String>,
    pub abstract_: String,
    pub full_text_hash: Option<EngramHash>,

    /// HDC fingerprint of the abstract + title.
    pub fingerprint: HdcVector,

    /// Claims we extracted or asserted from this paper.
    pub claims: Vec<ClaimId>,

    /// How we've decided to trust this paper.
    pub provenance: PaperProvenance,
}

pub struct PaperProvenance {
    pub source: Source,       // arxiv, nature, blog, etc
    pub citation_count: Option<u32>,
    pub venue_tier: Option<VenueTier>,
    pub replication_status: ReplicationStatus,
    pub our_notes: Option<String>,
}
```

Paper Engrams live in the same Substrate as everything else. They
get content-addressed hashes. Heuristics and episodes can cite them
in lineage.

## 3. Claim as testable hypothesis

A `Claim` is a sharper, structured version of a sentence from the
paper:

```rust
pub struct Claim {
    pub id: Uuid,
    pub paper: PaperId,
    pub quote: String,

    /// Structured restatement.
    pub hypothesis: Hypothesis,

    /// What would refute this claim in our context.
    pub falsifier: Predicate,

    /// Conditions under which the paper says the claim applies.
    pub context: Vec<Predicate>,

    /// Effect size reported in the paper.
    pub effect_size: Option<EffectSize>,

    /// Our empirical evaluation so far.
    pub calibration: Calibration,
}

pub enum Hypothesis {
    Causal { cause: Predicate, effect: Predicate, sign: Sign },
    Statistical { distribution: Expr, parameters: Vec<f64> },
    Algorithmic { invariant: String, guarantee: Expr },
    Architectural { structure: Predicate, property: Predicate },
}
```

Claims are *authored* — a human or agent reads a paper and writes
one down. Extraction can be semi-automated: the Composer can draft
claims from an abstract for review. The human-in-the-loop step is
the falsifier — stating *what would prove this wrong* is the work
Popper demanded and LLMs do poorly unsupervised.

## 4. Claim → Heuristic lifting

When a Claim reaches sufficient structure, it *becomes* a Heuristic:

```rust
impl From<Claim> for Heuristic {
    fn from(c: Claim) -> Heuristic {
        Heuristic {
            claim: c.quote,
            preconditions: c.context,
            prediction: c.hypothesis.into_predicate(),
            lineage: vec![], // citation captured separately
            calibration: c.calibration,
            ..default()
        }
    }
}
```

The same lifecycle from `14-worldview-validation.md` applies:
trials, confirmations, violations, refinement, retirement. The only
difference is lineage points back to a Paper Engram, and the
calibration diverges from the paper's reported effect over time —
this divergence *is the interesting signal*.

## 5. The replication ledger

For each paper-derived claim, Roko maintains a replication ledger:

```rust
pub struct ReplicationLedger {
    pub claim: ClaimId,
    pub paper_effect: f64,       // what the paper reported
    pub our_effect: f64,         // what we observe
    pub our_n: u32,              // trials in our context
    pub divergence_ci: (f64, f64), // confidence interval of the gap
    pub status: ReplicationStatus,
}

pub enum ReplicationStatus {
    Untested,
    Insufficient(u32),  // too few trials
    Replicates,         // effect within CI
    PartialReplicates,  // same sign, smaller effect
    FailsToReplicate,
    ContextDependent,   // replicates in some situations, not others
}
```

This makes "does this research hold up on our stack?" a structured
dashboard query. Nothing in the agent-framework space does this.

## 6. Cited research as first-class config

Instead of:

```rust
const CASCADE_EPSILON: f64 = 0.1; // "from auer et al"
```

We prefer:

```rust
CascadeRouter::new()
    .epsilon_from(claim!["auer2002", "epsilon_greedy", "default_0.1"])
    .with_fallback(0.1)
```

The `claim!` macro resolves to a Claim ID at build time; at runtime,
if the claim's calibration has drifted enough, a signal is emitted:
"cascade-router-01 is using a parameter whose source claim has
failed to replicate in 87 recent trials." Engineering decisions
become *traceable* and *self-auditing*.

## 7. Research sources and ingestion

Three ingestion lanes:

### 7.1 Manual

A human (or agent) reads a paper, creates the Paper Engram, drafts
Claims. Highest-quality ingestion; appropriate for foundational
work.

### 7.2 Agent-curated

An agent in "researcher" role crawls a source (arxiv daily digest,
Papers With Code trending), drafts Paper+Claim Engrams, publishes
them to a Bus topic `research.candidate`. Other agents review and
either promote to `research.approved` or reject.

### 7.3 Watchdog

A Watchdog subscribes to a claim's falsifier Predicate *across all
episodes*. When the falsifier matches an observed outcome, the
Watchdog publishes `claim.violated` and triggers recalibration.
Passive monitoring; zero operator overhead.

## 8. A curated starter kit

The proposal includes importing ~40 foundational claims at launch:

- Kanerva 2009 on HDC capacity and near-orthogonality.
- Friston 2006 on free-energy minimization.
- Woolley 2010 on c-factor predictors.
- Sutton 1988 on temporal-difference learning.
- Robbins 1952 and Auer 2002 on bandits.
- Hanson 1999 on prediction markets.
- Axelrod 1984 on cooperation / tit-for-tat.
- Janis 1972 on groupthink symptoms.
- Dehaene 2020 on consciousness and global workspace.
- Holland 1995 on complex adaptive systems.
- Kahneman 2011 on System 1/2 and bias catalog.
- Sapolsky 2004 on stress and decision-making (for pacing).
- Clark 2013 on predictive processing.
- Surowiecki 2004 on wisdom-of-crowds conditions.
- Weick 1995 on sensemaking.
- Ostrom 1990 on commons governance (for the heuristic commons).
- Mead 1934 on role-taking (for peer-prediction).
- Simon 1956 on bounded rationality.
- Gesell 1916 on demurrage.
- Hofstadter 1979 on strange loops (for self-modeling).

Each gets a Paper Engram and 1–3 Claims, with a falsifier stated
explicitly. This becomes the starter heuristic library every new
deployment inherits. Calibration against each deployment's reality
takes over from there.

## 9. Provenance as a first-class quality signal

When an agent uses a heuristic in a prompt, the prompt includes its
provenance:

```
[heuristic] When tests are flaky, add logging before touching logic.
  source:   rooted in Kernighan & Pike 1999 §5.2
  our n:    41 trials, 28 confirmations
  paper CI: not quantified
  our CI:   (0.54, 0.79)
```

This is *radical transparency* about what the agent believes and
why. Most LLM agent systems are opaque; Roko's are legible. The
legibility itself is a product feature — you can audit, review,
correct.

## 10. Refuting a paper

If Roko's calibration strongly diverges from a paper's reported
effect, that's publishable information. The ReplicationLedger can
export to a standard format (a markdown template with CIs, trial
counts, context specification). Someone running Roko can contribute
to meta-science just by running the system.

This is downstream but is a genuine possibility — *a coding
assistant that contributes to the replication crisis in a positive
direction*. None of the agent frameworks can make this claim.

## 11. Integration with the chain (Phase 2)

When a claim's replication is chain-witnessed across many
deployments, it becomes a *consensus claim*. Consensus claims carry
very high trust and anchor a shared scientific substrate across the
Roko ecosystem. This is `roko-chain` in the service of empirical
knowledge rather than financial transactions — the same primitive
applied with a very different flavor.

## 12. Minimal viable implementation

1. Paper + Claim Engram types. One day.
2. Starter kit of 20 canonical papers, manually authored. Three days
   of librarian work.
3. Research-role agent that reviews arxiv daily. One week.
4. ReplicationLedger + export. Three days.
5. Watchdog hooks for falsifier monitoring. One week.
6. Claim-resolved config parameters (`claim!` macro). Two days.
7. Prompt-provenance injection. One day.

This whole module is a couple of eng-weeks and establishes a
capability no other agent system has: *living research*.

## 13. What makes a good falsifier

The falsifier is the load-bearing part of a Claim. It separates
"inspirational reading" from "testable hypothesis." Good falsifiers
share three properties:

1. **Observable from runtime signals.** A falsifier that requires
   a lab experiment Roko can't run is useless. Rewrite as a
   condition on Engrams, Pulses, or metrics.
2. **Time-bounded.** A prediction that takes 10 years to fail isn't
   useful to a runtime that iterates daily. Frame falsifiers as
   "over the next N trials / N days, [observable] should hold."
3. **Discriminating.** A falsifier that passes even when the claim
   is wrong is noise. Write the falsifier so the claim has to
   actually work for it to pass.

Bad: *"Epsilon-greedy will converge."*
Good: *"Over the next 500 arm pulls, the cumulative regret should be
bounded by 2·ε·t·|A|·log(t); check at n=100, n=250, n=500. If any
checkpoint exceeds by > 3σ, flag."*

The good version is a literal statistical test the Watchdog can run.

## 14. Replication contract format

For ledger exports (to contribute to external meta-science), a
canonical format. Markdown + front-matter so humans and parsers
both handle it:

```markdown
---
claim_id: c.kanerva2009.orthogonality
paper_doi: 10.1007/s12559-009-9009-8
paper_effect: "Two random 10,000-bit vectors have cosine similarity ~0 ± 0.01"
our_effect: 0.0097 ± 0.004
our_n: 1_000_000
roko_version: "2.3.1"
context:
  vector_dim: 10240
  encoder: default_v1
  deployment_profile: coding
status: replicates
first_observed: 2026-03-01
last_observed: 2026-04-14
---

## Notes

We observe expected orthogonality within 95% CI of paper's claim
across a random sample of 1M vector pairs from our production
Substrate. No dependence on kind or body type detected. Deployment
profile does not alter the result.
```

This is a format other research groups can import, parse, and
cross-check against their own replications. It's the minimal
infrastructure for a decentralized replication network.

## 15. The ingestion conflict-of-interest problem

If Roko's own agents ingest and evaluate research, there's a
conflict: Roko might preferentially confirm research that justifies
Roko. Mitigations:

1. **Separate ingestion agents from calibration agents.** The
   researcher role ingests and proposes; a separate evaluator role
   maintains the ledger. The evaluator is forbidden from reading
   the ingested paper directly — only from observing runtime
   behavior.
2. **Adversarial prompts.** Ingest known-false papers (retracted,
   failed-to-replicate) into the starter kit. If Roko's evaluator
   confirms them anyway, the evaluator is broken.
3. **Human review of high-stakes claims.** Claims that would
   change system defaults (e.g. "reduce epsilon from 0.1 to 0.05")
   require human sign-off before taking effect, even if replication
   signals support.

The third is a permission-gradient item; see
`32-safety-sandbox-provenance.md` §4.

## 16. Research-driven roadmap refinement

A distinctive loop this doc enables: *the replication ledger drives
architectural decisions*. Two examples:

- If Kanerva's orthogonality claim fails to replicate in a specific
  Substrate backend, that's a signal HDC encoding is broken in that
  backend; file an issue against `roko-hdc`.
- If Woolley's c-factor predictors fail to replicate in Roko's
  multi-agent cohorts, it suggests either the agent analog doesn't
  map (revise the analogy) or the cohort is too small; adjust
  experimental design.

This closes the loop: research informs the system, the system tests
the research, results inform the next round of architectural work.
`35-consolidated-roadmap.md` has a placeholder for "replication-ledger-
driven" roadmap items that emerge organically over months.
