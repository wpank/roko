# 00-ARCH -- Autocatalytic Improvement and Cybernetics

> **Parent**: [00-ARCHITECTURE](../../00-ARCHITECTURE.md)
>
> Roko is designed so that performance, capability, and quality improve superlinearly
> with accumulated usage, deployment count, and connected data. The seven compounding
> loops below make each unit of use cheaper, faster, and better than the last.
>
> Theoretical foundations: Kauffman (1993), Wiener (1948), Ashby (1956), Beer (1972).

---

## 1. Why Compounding Is the Point

Most agent systems drift toward diminishing returns:

- More agents add coordination overhead.
- More memory adds retrieval noise.
- More tools add context pressure.
- More deployments create support burden without reuse.

Roko is built to invert those curves. The point is not that every part improves in
isolation; it is that the parts are coupled so the improvement feedback keeps feeding
back into the next turn, the next session, and the next deployment.

This is autocatalytic in the sense of Kauffman (1993, "The Origins of Order", Oxford
University Press): the outputs of one part become the inputs that improve another part,
which in turn improves the first part. The system does not need a central planner to
make that happen. It needs:

- A durable store that preserves useful structure (Substrate)
- An ephemeral bus that moves coordination feedback quickly (Bus)
- Calibration loops that can learn from outcomes
- Compression loops that turn episodes into reusable shape
- A measurement story that shows whether the positive feedback is real

That is the load-bearing distinction between a feature set and a compounding system.

---

## 2. The Seven Compounding Loops

Each loop is a distinct source of positive feedback using different combinations of
the two fabrics (Substrate and Bus), the seven-step loop, and the learning primitives.

### 2.1 Loop Summary

| Loop | Core mechanism | Why it compounds | Main hooks |
|---|---|---|---|
| Demurrage-weighted retrieval | Idle memory is taxed; useful memory is reinforced | More usage tunes holding cost, makes effective memory denser | SENSE, ASSESS, PERSIST, REACT |
| Heuristic calibration | Heuristics are tested against falsifiers and outcomes | Better calibration improves decisions, producing better evidence | VERIFY, REACT |
| HDC codebook cleanup | HDC fingerprints snap noisy inputs to stable codes | More episodes improve cleanup quality up to codebook capacity | COMPOSE, PERSIST |
| C-factor feedback | Cohort quality measured from Bus statistics | Better teams produce better outputs, better calibration and routing | ASSESS, VERIFY, REACT |
| Playbook distillation | Episodes compress into reusable playbooks | Compression becomes cheaper and more transferable with corpus growth | PERSIST, REACT, Delta |
| Cross-deployment heuristic commons | Heuristics shared across deployments | Each deployment contributes once but benefits many times | BROADCAST, REACT |
| Plugin ecosystem | Plugins create a two-sided capability market | More plugins attract users; more users justify more plugins | ACT, BROADCAST, REACT |

### 2.2 Demurrage-Weighted Retrieval

Demurrage gives memory a cost for sitting idle. The result is a self-trimming
substrate: useful Signals retain balance, weak ones fade toward cold tier, and the
retrieval surface stays indexed toward what has actually been used.

The compounding effect: more usage produces more reinforcement evidence, which improves
the demurrage curve, which makes the next retrieval pass more selective, which improves
the quality of the next episode.

```
               +---> Better retrieval ---+
               |                         |
    Usage ---> More reinforcement        |
               |                         v
               +--- Tighter demurrage ---+
```

KPI: balance histogram and median tokens per task.

### 2.3 Heuristic Calibration

Heuristics only compound if they can be falsified. A heuristic that never sees a
counterexample does not get better; it just gets older. The Bus turns prediction and
outcome into a continuous calibration stream:

```
1. A heuristic predicts.
2. A Pulse or gate verdict contradicts or confirms it.
3. The calibration policy updates confidence.
4. The next decision is better.
5. The better decision produces better evidence.
```

This loop depends on `Heuristic`, `Falsifier`, `Pulse`, and `Bus` together. It is one
of the main reasons the architecture can improve without needing a manual policy
rewrite after every new domain. E25 (Advanced Learning Loops, 10/10) wires significance
testing and early stopping into this calibration path.

### 2.4 HDC Codebook Cleanup

HDC fingerprints turn similarity into a cheap cleanup operation. Every new episode,
gate result, and heuristic adds to the codebook, which makes future retrieval more
likely to land on a stable semantic neighborhood.

This loop compounds because the codebook is not just bigger; it is better organized by
use. The more real interactions the system sees, the more likely a future query will
collapse to the right Signal cluster on the first pass.

KPI: percentage of Composer prompts hitting HDC-clean cache on the first attempt.

### 2.5 C-Factor Feedback

C-factor measures how well a cohort cooperates, predicts, and routes under real load.
High c-factor teams produce higher-quality output, which produces better learning
evidence for routing, demurrage tuning, and heuristic calibration.

```
c-factor rises -> output quality rises -> learning quality rises -> c-factor rises
```

The important constraint: c-factor is a covariate, not the objective. See
[c-factor-collective-intelligence.md](c-factor-collective-intelligence.md) for the
full formula and diagnostic signals.

### 2.6 Playbook Distillation

Episodes compress into playbooks, and playbooks can compress into meta-playbooks. Once
the corpus is large enough, the cost per distilled unit drops while the transfer value
per unit rises. The system learns the reusable shape of the work.

KPI: dream-cycle retroactive improvements per week.

### 2.7 Cross-Deployment Heuristic Commons

Once heuristics can be imported across deployments, each deployment contributes to a
shared commons. The economics are simple: the marginal cost of sharing is low, but the
marginal value to other deployments is high.

KPI: first-task-after-install to success minutes.

### 2.8 Plugin Ecosystem

Each new plugin increases the value of Roko to users who need that capability, and each
new user increases the value of building a plugin. E32 (Tool/Plugin Ecosystem, 8/8)
provides the signed dependency graphs, bounded WASM hooks, strict admission, and
verified registry install/publish that make this ecosystem viable.

KPI: unique plugin count and unique plugin users.

---

## 3. How the Loops Fit the Seven-Step Loop

The seven compounding loops ride on the seven-step universal cognitive loop, with
`PERSIST` and `BROADCAST` treated as co-equal branches inside the same phase:

```
1. SENSE      - Substrate.query | Bus.subscribe | external I/O
2. ASSESS     - Scorer + Router choose what to do next
3. COMPOSE    - Composer assembles a prompt Signal under budget
4. ACT        - LLM | tool | chain execution emits Pulses and final Signals
5. VERIFY     - Gate pipeline and stream-gates emit verdicts
6. PERSIST    - Substrate.put for Signals
   BROADCAST  - Bus.publish for Pulses, in parallel
7. REACT      - Policy updates, new Pulses, new Signals, new calibration
```

Each step improves the next one:

- SENSE gets better when Substrate queries hit HDC-clean memory and Bus subscriptions
  expose more relevant Pulses
- ASSESS gets better when c-factor and demurrage calibrations sharpen the scoring
  surface
- COMPOSE gets better when playbooks and HDC fingerprints compress the prompt space
- ACT gets better when the plugin ecosystem and domain profiles make the action space
  richer
- VERIFY gets better when heuristics and falsifiers are tested continuously rather
  than only after failures
- PERSIST gets better when demurrage trims dead weight and preserves useful lineage
- BROADCAST gets better when shared heuristics and Pulse streams make the commons
  richer
- REACT gets better when the system learns which loops are actually paying back

---

## 4. Cybernetic Foundations

### 4.1 Wiener and Cybernetics

Wiener, N. (1948). "Cybernetics: or Control and Communication in the Animal and the
Machine." MIT Press.

Wiener defined cybernetics as the study of control and communication in animals and
machines. The central idea is that feedback is the mechanism by which systems regulate
themselves. Roko's seven compounding loops are cybernetic feedback loops: each one
measures an output, compares it with a desired state, and adjusts behavior to reduce
the error.

The Bus is the feedback nervous system. Prediction Pulses, outcome Pulses,
prediction-error Pulses, and calibration Pulses form the operational bridge between
cybernetics and the seven compounding loops. The system learns from mismatch, not just
from success.

```
              ┌────────────────────────────────┐
              │         Desired State          │
              │    (successful task outcome)    │
              └──────────────┬─────────────────┘
                             │ compare
              ┌──────────────▼─────────────────┐
              │       Prediction Error         │
              │   (gate verdict, outcome gap)   │
              └──────────────┬─────────────────┘
                             │ adjust
              ┌──────────────▼─────────────────┐
              │      Behavioral Adjustment     │
              │ (routing, calibration, PAD)     │
              └──────────────┬─────────────────┘
                             │ act
              ┌──────────────▼─────────────────┐
              │        System Output           │
              │     (agent action, signal)      │
              └──────────────┬─────────────────┘
                             │ measure
                             └────────→ back to compare
```

### 4.2 Ashby and Requisite Variety

Ashby, W. R. (1956). "An Introduction to Cybernetics." Chapman & Hall.

Ashby's Law of Requisite Variety: **only variety can absorb variety**. A regulator must
have at least as many states as the system it regulates. Roko's answer is not to
invent a new abstraction for every situation. It is to keep the kernel vocabulary
stable (Signal, Pulse, Cell, Graph, Protocol) while allowing implementations, topics,
heuristics, and plugins to expand.

```
Variety of problems -----> Must be matched by -----> Variety of responses
                                                       |
   Stable kernel vocabulary    +    Extensible implementations
```

The five primitives (Signal, Pulse, Cell, Graph, Protocol) provide the fixed
structural variety. The 12 traits provide the fixed behavioral variety. Everything
else -- tool definitions, provider adapters, gate implementations, role templates,
plugin surfaces -- is extensible variety that grows with the problem domain.

### 4.3 Conant-Ashby and Self-Models

Conant, R. C. & Ashby, W. R. (1970). "Every Good Regulator of a System Must Be a
Model of That System." International Journal of Systems Science 1(2):89-97.

The Good Regulator Theorem justifies self-modeling. That is why the compounding story
includes c-factor, heuristic calibration, and retrospection rather than just raw
throughput. The system needs a model of its own learning dynamics to regulate them well.

- The Daimon PAD vector is the self-model for individual agents
- C-factor is the self-model for cohorts
- E23 (Agent Cognitive Autonomy, 10/10) adds CorticalState energy fields and EFE
  routing for full self-regulatory capacity

### 4.4 Beer and Recursive Viability

Beer, S. (1972). "Brain of the Firm." Allen Lane / Penguin Press.

Beer's Viable System Model (VSM) identifies five necessary functions for any viable
system:

| VSM System | Roko analog | Where |
|---|---|---|
| S1: Operations | Agents executing tasks | roko-agent, provider dispatch |
| S2: Coordination | Bus-mediated coordination, c-factor measurement | roko-runtime Bus, E28 groups |
| S3: Operational control | Gate pipeline, budget enforcement | roko-gate, roko-graph cost state |
| S4: Environment scanning | Research agent, external feeds | roko-cli research, E27 feeds |
| S5: Policy | Cross-cut arbitration, safety constraints | E34 safety, E44 functors |

The recursive structure matters: the same five systems appear at every level of
organization, from individual agent (PAD as S5) to cohort (c-factor as S2) to
deployment (commons as S4).

```
┌─────────────────────────────────────────┐
│  S5: Policy + Safety constraints        │
│  ┌───────────────────────────────────┐  │
│  │  S4: Research + External feeds    │  │
│  │  ┌─────────────────────────────┐  │  │
│  │  │  S3: Gates + Budget control │  │  │
│  │  │  ┌───────────────────────┐  │  │  │
│  │  │  │  S2: Bus coordination │  │  │  │
│  │  │  │  ┌─────────────────┐  │  │  │  │
│  │  │  │  │  S1: Agent ops  │  │  │  │  │
│  │  │  │  └─────────────────┘  │  │  │  │
│  │  │  └───────────────────────┘  │  │  │
│  │  └─────────────────────────────┘  │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

### 4.5 Kauffman and Autocatalytic Sets

Kauffman, S. A. (1993). "The Origins of Order: Self-Organization and Selection in
Evolution." Oxford University Press.

An autocatalytic set is a collection of molecules where each molecule's formation is
catalyzed by at least one other molecule in the set. Once the set reaches a critical
diversity, it becomes self-sustaining.

The analogy to Roko: each compounding loop catalyzes at least one other loop.
Demurrage sharpens retrieval, which improves heuristic calibration, which sharpens
verification, which improves playbook distillation, which sharpens compression, which
improves retrieval again. The system reaches a critical diversity of feedback pathways
that makes improvement self-sustaining.

The autocatalytic condition is formally: for every loop L_i, there exists at least one
other loop L_j such that improvements in L_j increase the rate of improvement in L_i.
The seven loops satisfy this because they share the same primitives (Signal, Pulse,
Bus, Substrate) and the same measurement infrastructure (E33 telemetry, 39/39 ingress).

---

## 5. Measuring Compounding

### 5.1 KPI Panel

| KPI | Loop it measures | Expected curve |
|---|---|---|
| Mean time to first successful PR on a new codebase | All seven | Steep initial drop, then continued decline |
| Median tokens per task, by difficulty bucket | Demurrage, HDC, playbooks | Monotonic decrease |
| % of Composer prompts hitting HDC-clean cache | HDC cleanup | Asymptote toward 1 |
| Mean calibration CI width per heuristic | Heuristic calibration | Decrease with trials |
| % of heuristics sourced from commons | Cross-deployment commons | Increase, then stabilize |
| c-factor on randomly sampled cohorts | c-factor feedback | Stable or rising |
| Dream-cycle retroactive improvements per week | Playbook distillation | Growth with corpus size |
| Unique plugin count | Plugin ecosystem | Linear in time, value superlinear |
| First-task-after-install to success minutes | Cross-deployment commons | Decreases as commons grows |

### 5.2 Anti-Metrics

Three numbers should stay flat or shrink as usage grows:

- Warm-tier episode count should stabilize, not grow without bound, if demurrage is
  working
- Heuristic count with fewer than three confirmations should not grow indefinitely, or
  the calibrator is not probing enough
- Mean lineage depth per response should not drift upward unless the extra lineage is
  actually improving answer quality

If any of those blow out, the system is accumulating complexity without compounding
value.

### 5.3 Failure Modes

| Failure mode | What it looks like | Countermeasure |
|---|---|---|
| Echo chamber | Same beliefs keep winning unchallenged | Outsider injection, explicit falsifiers, challenger worldviews |
| Reward hacking | c-factor rises on easy work only | c-factor as covariate, sample by difficulty |
| Premature convergence | Heuristic stops being tested | Importance sampling, deliberate boundary tests |
| Substrate bloat | Warm storage grows without restraint | Demurrage tuning, cold tier promotion, balance histograms |

---

## 6. Theoretical Limits

### 6.1 Bottlenecks

- LLM throughput, not abstract compute, is the practical ceiling for many turns
- Context windows cap how much can be composed at once
- Cost is the first-order constraint on how often the system can use deep reasoning

### 6.2 Undecidability

Not every property can be decided automatically (Rice 1953, Trans. AMS 74(3)). Some
verification still requires escalation to deeper reasoning or human review. That is
why the system measures failure modes instead of pretending they can all be eliminated.

### 6.3 No Free Lunch

Wolpert & Macready (1997, IEEE TEVC 1(1)): no single optimization strategy dominates
all problems. The compounding architecture is therefore plural: multiple loops,
multiple feedback channels, multiple surfaces, one shared vocabulary.

---

## 7. Connection to SOFAI and Modern Dual-Process Systems

| SOFAI / dual-process idea | Roko mapping | Why it matters for compounding |
|---|---|---|
| Fast reasoning | Gamma (~5s) | Enables quick feedback and cheap corrections |
| Slow reasoning | Theta (~75s) | Enables deliberate calibration and verification |
| Offline consolidation | Delta (~hours) | Turns episodes into reusable playbooks and commons |

The Delta layer is what turns a good turn into a reusable future advantage. That is
why the superlinear claim depends on persistence across sessions, not just
within-session benchmark scores.

---

## Academic Foundations

| Citation | Contribution |
|---|---|
| Kauffman, S. A. 1993, "The Origins of Order", OUP | Autocatalytic sets and self-sustaining reaction networks |
| Wiener, N. 1948, "Cybernetics", MIT Press | Feedback, control, and communication in animals and machines |
| Ashby, W. R. 1956, "An Introduction to Cybernetics", Chapman & Hall | Law of Requisite Variety: only variety absorbs variety |
| Conant, R. C. & Ashby, W. R. 1970, IJSS 1(2):89-97 | Good Regulator Theorem: every good regulator must model its system |
| Beer, S. 1972, "Brain of the Firm", Allen Lane | Viable System Model (VSM) and recursive viability |
| Friston, K. 2010, Nature Reviews Neuroscience 11:127-138 | Free Energy Principle and active inference |
| Grasse, P. P. 1959, Insectes Sociaux 6(1) | Stigmergy through environmental modification |
| Dorigo, M. et al. 2000, Artificial Life 5(3) | Ant colony optimization as stigmergic coordination |
| Wolpert, D. & Macready, W. 1997, IEEE TEVC 1(1):67-82 | No Free Lunch Theorem |
| Rice, H. G. 1953, Trans. AMS 74(3):358-366 | Limits of general semantic decidability |
| McClelland, J. et al. 1995, Psychological Review 102(3) | Complementary Learning Systems theory |

---

## Cross-References

- [00-ARCHITECTURE](../../00-ARCHITECTURE.md) -- Parent chapter
- [c-factor-collective-intelligence.md](c-factor-collective-intelligence.md) -- C-factor metric details
- [cognitive-cross-cuts.md](cognitive-cross-cuts.md) -- Neuro, Daimon, Dreams injection points
- [design-principles-frontier-summary.md](design-principles-frontier-summary.md) -- Design principles
- [31-04 Autocatalytic Compounding](../31-self-hosting/04-autocatalytic-compounding.md) -- Self-hosting loop details
