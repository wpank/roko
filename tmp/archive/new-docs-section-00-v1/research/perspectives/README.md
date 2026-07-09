# Research Perspectives

> Six extended essay collections, each viewing Roko through a distinct conceptual lens.
> This folder is for context and intellectual framing, not specification. Nothing here
> is required reading to implement or operate Roko — but it rewards readers who want to
> understand *why* the architecture looks the way it does.

**Status**: Active (Cluster G — all six perspectives complete)
**Last reviewed**: 2026-04-19

---

## What a Perspective Is

A perspective is not a specification, a tutorial, or a design document. It is an extended
essay that asks: *if we viewed Roko through the lens of X, what would we see?* The value of
a perspective is in what it *illuminates* — design choices that appear arbitrary in isolation
often make deep sense when seen through the right lens.

Each perspective folder is self-contained. You do not need to read the others first. Within
a folder, the standard structure is:

| File | Role |
|---|---|
| `00-overview.md` | Introduces the lens: what is this perspective about? Why apply it to Roko? |
| `01-<theory>.md` | The foundational theory: key figures, core concepts, intellectual history |
| `02-<mechanism>.md` | How the mechanism works in its home domain |
| `03-roko-application.md` | How Roko maps onto the theory: operator-by-operator or concept-by-concept |
| `04-implications.md` | Design constraints, measurement criteria, and protocol recommendations |
| `05-open-questions.md` | Unresolved frontier questions — candidates for research, not immediate tasks |

Some perspectives have an additional file when the theory requires more unpacking.

---

## The Six Perspectives

### 1. Attention as Currency

> *Roko's scoring and routing system is an attention market.*

| # | File | Topic |
|---|---|---|
| 0 | [00-overview.md](attention-as-currency/00-overview.md) | Attention as a scarce resource |
| 1 | [01-the-metaphor.md](attention-as-currency/01-the-metaphor.md) | Rival goods, budget constraints, attention poverty |
| 2 | [02-market-mechanics.md](attention-as-currency/02-market-mechanics.md) | Auction theory, VCG, sponsored search, Simon & Goldhaber |
| 3 | [03-roko-application.md](attention-as-currency/03-roko-application.md) | Scorer as bid generator, Gate as reserve price, Router as allocator |
| 4 | [04-implications.md](attention-as-currency/04-implications.md) | 6 design implications |
| 5 | [05-open-questions.md](attention-as-currency/05-open-questions.md) | Open frontier questions |

**Illuminates**: [Scorer](../../reference/05-operators/scorer.md),
[Router](../../reference/05-operators/router.md),
[Gate](../../reference/05-operators/gate.md),
[Policy](../../reference/05-operators/policy.md)

---

### 2. Cognitive Immune System

> *Roko's safety and gating layers are a two-tier immune response: fast innate and slow adaptive.*

| # | File | Topic |
|---|---|---|
| 0 | [00-overview.md](immune-system/00-overview.md) | Immunity as a defence analogy |
| 1 | [01-innate-vs-adaptive.md](immune-system/01-innate-vs-adaptive.md) | Gate as innate, Neuro as memory, clonal selection |
| 2 | [02-recognition-and-response.md](immune-system/02-recognition-and-response.md) | PAMPs, DAMPs, Matzinger danger theory, cytokine storms |
| 3 | [03-roko-application.md](immune-system/03-roko-application.md) | Full mapping: Gate, Scorer, Neuro, Dreams, Daimon, Provenance |
| 4 | [04-implications.md](immune-system/04-implications.md) | 5 design implications |
| 5 | [05-open-questions.md](immune-system/05-open-questions.md) | Open frontier questions |

**Illuminates**: [Gate](../../reference/05-operators/gate.md),
[Scorer](../../reference/05-operators/scorer.md),
[Neuro cross-cut](../../reference/09-cross-cuts/README.md),
[Dreams](../../reference/09-cross-cuts/README.md),
[Provenance](../../reference/10-types/provenance.md)

---

### 3. Temporal Topology of Knowledge

> *Knowledge isn't stored in flat lists — it has topological shape that changes over time.*

| # | File | Topic |
|---|---|---|
| 0 | [00-overview.md](temporal-topology/00-overview.md) | What topology studies |
| 1 | [01-knowledge-as-topology.md](temporal-topology/01-knowledge-as-topology.md) | Metric spaces, small-world, scale-free networks |
| 2 | [02-temporal-shape.md](temporal-topology/02-temporal-shape.md) | Ingestion, decay, consolidation, contradiction as operators |
| 3 | [03-decay-as-topological-operator.md](temporal-topology/03-decay-as-topological-operator.md) | Formal operator math, persistent homology |
| 4 | [04-roko-application.md](temporal-topology/04-roko-application.md) | Engram graph, HDC as coordinates, decay tier matrix |
| 5 | [05-implications.md](temporal-topology/05-implications.md) | 5 design implications with table |
| 6 | [06-open-questions.md](temporal-topology/06-open-questions.md) | Open frontier questions |

**Illuminates**: [Engram](../../reference/01-engram/README.md),
[Substrate](../../reference/03-substrate/README.md),
[Decay variants](../../reference/10-types/decay.md),
[Dreams](../../reference/09-cross-cuts/README.md),
[HDC fingerprint](../../reference/10-types/hdc-fingerprint.md),
[Three cognitive speeds](../../reference/07-speeds/README.md)

---

### 4. Emergent Goal Structures

> *Goals are not always programmed — they can emerge from the dynamics of the system itself.*

| # | File | Topic |
|---|---|---|
| 0 | [00-overview.md](emergent-goals/00-overview.md) | Designed vs emergent goals; Omohundro, Goodhart, mesa-optimization |
| 1 | [01-goal-as-attractor.md](emergent-goals/01-goal-as-attractor.md) | Attractors, basins, bifurcations, Lyapunov functions |
| 2 | [02-emergence-mechanisms.md](emergent-goals/02-emergence-mechanisms.md) | 5 mechanisms for goal emergence |
| 3 | [03-roko-application.md](emergent-goals/03-roko-application.md) | Daimon as goal attractor, Policy as goal anchor |
| 4 | [04-implications.md](emergent-goals/04-implications.md) | 5 design implications |
| 5 | [05-open-questions.md](emergent-goals/05-open-questions.md) | Open frontier questions |

**Illuminates**: [Daimon](../../reference/09-cross-cuts/README.md),
[Policy](../../reference/05-operators/policy.md),
[Composer](../../reference/05-operators/composer.md),
[Universal Cognitive Loop](../../reference/06-loop/README.md)

---

### 5. Cognitive Energy Model

> *Cognitive processing costs energy; the three-speed architecture is a metabolic system.*

| # | File | Topic |
|---|---|---|
| 0 | [00-overview.md](energy-model/00-overview.md) | ATP, mitochondria, metabolic states |
| 1 | [01-cognitive-energy.md](energy-model/01-cognitive-energy.md) | CEU definition, ATP cycle, mitochondria analogy, Kahneman |
| 2 | [02-allocation-dynamics.md](energy-model/02-allocation-dynamics.md) | Thermodynamic principles, activation energy, recovery |
| 3 | [03-roko-application.md](energy-model/03-roko-application.md) | T0/T1/T2 as energy tiers, Router as budget controller |
| 4 | [04-implications.md](energy-model/04-implications.md) | 6 design implications |
| 5 | [05-open-questions.md](energy-model/05-open-questions.md) | Open frontier questions |

**Illuminates**: [Three cognitive speeds](../../reference/07-speeds/README.md),
[Router](../../reference/05-operators/router.md),
[Daimon](../../reference/09-cross-cuts/README.md),
[Dreams](../../reference/09-cross-cuts/README.md),
[Scorer](../../reference/05-operators/scorer.md),
[Gate](../../reference/05-operators/gate.md)

---

### 6. Collective Intelligence

> *Roko is not a single agent — it is a cognitive collective, and its design should be evaluated as such.*

| # | File | Topic |
|---|---|---|
| 0 | [00-overview.md](collective-intelligence/00-overview.md) | C-factor, group as cognitive unit |
| 1 | [01-c-factor.md](collective-intelligence/01-c-factor.md) | Measurement methodology, temporal stability, human-AI hybrids |
| 2 | [02-from-individuals-to-collectives.md](collective-intelligence/02-from-individuals-to-collectives.md) | Bridging mechanisms: transactive memory, diversity, sensemaking |
| 3 | [03-roko-application.md](collective-intelligence/03-roko-application.md) | Router as directory, Scorer as ensemble, Composer as synthesiser |
| 4 | [04-implications.md](collective-intelligence/04-implications.md) | 6 design implications with measurement criteria |
| 5 | [05-open-questions.md](collective-intelligence/05-open-questions.md) | Open frontier questions |

**Illuminates**: [Router](../../reference/05-operators/router.md),
[Scorer](../../reference/05-operators/scorer.md),
[Composer](../../reference/05-operators/composer.md),
[Policy](../../reference/05-operators/policy.md),
[Neuro cross-cut](../../reference/09-cross-cuts/README.md),
[Three cognitive speeds](../../reference/07-speeds/README.md)

---

## Reading Paths

### "I want to understand why Roko is built this way"

Start with the perspective most relevant to the component you're examining:
- Scoring and ranking → **Attention as Currency** + **Collective Intelligence**
- Safety and filtering → **Cognitive Immune System**
- Memory and decay → **Temporal Topology**
- Goal-setting and Daimon → **Emergent Goals**
- Compute and speed tiers → **Energy Model**

### "I want to think about long-term risks"

Read the Open Questions files across perspectives in this order:
`emergent-goals/05` → `collective-intelligence/05` → `energy-model/05` → `temporal-topology/06`

### "I want design recommendations I can act on now"

All `04-implications.md` files are structured as concrete design constraints with measurement
criteria. They can be read in any order.

### "I want the theoretical background before the application"

For each perspective, read `00-overview.md` and `01-<theory>.md` before `03-roko-application.md`.
The foundations in [`../foundations/`](../foundations/) provide deeper background where
perspectives reference the same theories.

---

## See Also

- [`../foundations/README.md`](../foundations/README.md) — the theoretical foundations
- [`../README.md`](../README.md) — the full research tree index
- [`../../reference/README.md`](../../reference/README.md) — the specification tree (what the perspectives explain)

## Open Questions

- Should perspectives link to each other more aggressively? Currently each is self-contained;
  cross-perspective links would surface interactions (e.g., attention markets × energy budgets).
- Should each perspective have a "Related Perspectives" section in its README?
- As the system evolves, should `04-implications.md` files be promoted to `reference/` once their
  recommendations are accepted as architectural constraints?
