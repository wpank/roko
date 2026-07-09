# Operators Overview — What a Synapse Trait Is

> Roko calls its operator traits "Synapse traits." A Synapse trait is the contract that an
> operator must fulfil to participate in the cognitive loop. This page explains what that
> means for a reader with no prior Roko context.

**Status**: Shipping
**Crate**: `roko-core`
**Depends on**: [Engram](../01-engram/README.md), [Substrate](../03-substrate/README.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

A "Synapse trait" is a Rust trait that defines one step of an agent's cognitive loop.
Implementations of the trait can be stacked, swapped, or composed without changing the loop
code. There are six Synapse traits: Substrate, Scorer, Gate, Router, Composer, and Policy.

---

## The Biological Analogy

The name "Synapse" is borrowed from neuroscience: a synapse is the junction between neurons
where a signal is transformed and passed on. In Roko, each operator trait is a junction in
the cognitive loop where data arrives in one form and leaves in another:

```
Engram (raw memory)
    │
    ▼ Scorer
Score (7-axis appraisal)
    │
    ▼ Gate
Verdict (pass / fail)
    │
    ▼ Router
Selected action
    │
    ▼ Composer
System prompt (for LLM)
    │
    ▼ Policy
Control signal (circuit break / escalate / approve)
```

Each step is one trait. Each trait can have multiple implementations.

---

## Two Mediums, Two Fabrics, Six Operators

Roko's architecture is organised along three axes:

| Axis | Elements |
|---|---|
| **Mediums** | `Engram` (durable), `Pulse` (ephemeral, target-state) |
| **Fabrics** | `Substrate` (storage), `Bus` (transport, target-state) |
| **Operators** | Scorer, Gate, Router, Composer, Policy (the five cognitive transforms) |

The fabrics are infrastructure. The operators are cognitive logic. Together, they implement
the cognitive loop. This folder covers operators; fabrics are in `../03-substrate/` and
`../04-bus/`.

---

## Properties Shared by All Operator Traits

1. **Object-safe** — all operators are held as `Box<dyn Operator>` in the runtime, allowing
   swap at construction time.
2. **Composable** — operators are designed to stack. Multiple `Scorer` implementations form
   a chain; multiple `Gate` implementations form a pipeline.
3. **Stateless by default** — operators receive inputs and return outputs without owning
   mutable state unless the trait explicitly declares it (e.g., `&mut self`).
4. **Single responsibility** — each trait does one thing and one thing only. A `Scorer`
   scores. It does not route, gate, or compose.
5. **Layer-located** — each trait belongs to exactly one of the five layers in Roko's
   dependency taxonomy. See [Trait × Layer Map](./02-trait-layer-map.md).

---

## Composition Rules

Operators compose in two ways:

1. **Stacking** (same trait, sequential) — multiple implementations of the same trait are
   applied in sequence, with the output of one feeding the next.
   Example: `Scorer` chain where a base `RecencyScorer` and a `ConfidenceScorer` are both
   applied, and their outputs are merged.

2. **Layering** (different traits, different steps) — the loop calls `Scorer`, then passes
   its output to `Gate`, then `Router`, etc.

Stacking is an opt-in pattern — the default is one implementation per trait per loop tick.
The `loop_tick` function signature makes the stacking explicit. See
[Trait Composition Model](./01-trait-composition-model.md).

---

## The Synapse Vocabulary

Throughout the operator docs, these terms appear consistently:

| Term | Meaning |
|---|---|
| Operator | Any implementation of a Synapse trait |
| Synapse trait | The Rust trait defining the operator's contract |
| Loop tick | One execution of the full cognitive loop (all operators, one data item) |
| Stacking | Calling multiple operators of the same trait sequentially |
| Verdict | A `Gate` output: `Pass`, `Fail`, or `Abstain` |
| Cascade | A `Router` fallback chain: Static → Confidence → UCB |

---

## See Also

- [Trait Composition Model](./01-trait-composition-model.md)
- [Trait × Layer Map](./02-trait-layer-map.md)
- [Scorer](./01-scorer/README.md)
- [Gate](./02-gate/README.md)
- [Router](./03-router/README.md)
- [Composer](./04-composer/README.md)
- [Policy](./05-policy/README.md)
- [Universal Cognitive Loop](../06-loop/README.md)
