# Operators

> Roko provides six operator traits — `Substrate`, `Scorer`, `Gate`, `Router`, `Composer`,
> and `Policy` — that together implement the cognitive loop. Each operator is a trait; each
> trait is a typed, composable transform applied to agent data at a specific layer.

**Status**: Shipping (all six operators)
**Crate**: `roko-core` (traits), various (implementations)
**Depends on**: [Engram](../01-engram/README.md), [Substrate](../03-substrate/README.md)
**Last reviewed**: 2026-04-19

---

## What an Operator Is

An operator is a **Rust trait** that the cognitive loop calls at a specific step. It receives
data (an `Engram`, a `Score`, a context), does exactly one thing, and returns a typed result.
Operators are:

- **Composable** — operators stack (multiple `Scorer` implementations can be chained).
- **Typed** — inputs and outputs are concrete types, not `Any`.
- **Object-safe** — operators are held as `Box<dyn Operator>`, allowing runtime swap.
- **Stateless by contract** — an operator should not own mutable state unless it declares it.

---

## The Six Operators

| # | Operator | Role | Status | Crate |
|---|---|---|---|---|
| — | [`Substrate`](../03-substrate/README.md) | Durable storage fabric | Shipping | `roko-core` |
| 1 | [`Scorer`](./01-scorer/README.md) | Assign 7-axis `Score` to an `Engram` | Shipping | `roko-core` |
| 2 | [`Gate`](./02-gate/README.md) | Pass/fail verdict on an `Engram` or action | Shipping | `roko-gate` |
| 3 | [`Router`](./03-router/README.md) | Select which action to execute | Shipping | `roko-agent` |
| 4 | [`Composer`](./04-composer/README.md) | Build the LLM system prompt | Shipping | `roko-compose` |
| 5 | [`Policy`](./05-policy/README.md) | Reactive control: circuit breakers, safety, escalation | Shipping | `roko-core` |

`Substrate` is also an operator conceptually, but it is covered in its own folder because it
is a storage fabric, not a cognitive transform.

---

## Contents of This Folder

| File | Content |
|---|---|
| [README.md](./README.md) | This index |
| [00-overview.md](./00-overview.md) | What a Synapse operator is; the composition model |
| [01-trait-composition-model.md](./01-trait-composition-model.md) | How operators compose; the loop tick; stacking rules |
| [02-trait-layer-map.md](./02-trait-layer-map.md) | Which operators live at which of the five layers |
| [01-scorer/](./01-scorer/README.md) | Scorer operator — all pages |
| [02-gate/](./02-gate/README.md) | Gate operator — all pages |
| [03-router/](./03-router/README.md) | Router operator — all pages |
| [04-composer/](./04-composer/README.md) | Composer operator — all pages |
| [05-policy/](./05-policy/README.md) | Policy operator — all pages |

---

## Suggested Reading Order

**New to Roko**: 00-overview → 01-trait-composition-model → 02-trait-layer-map → then each
operator's `00-overview.md`.

**Implementer (building a new operator)**: 00-overview → 01-trait-composition-model →
`<operator>/01-trait-surface.md` → `<operator>/11-invariants.md`.

---

## See Also

- [Substrate](../03-substrate/README.md)
- [Universal Cognitive Loop](../06-loop/README.md) — where operators are called
- [Five-Layer Taxonomy](../08-layers/README.md) — layer boundaries
