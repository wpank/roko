# 33-01 -- Category-Theoretic Functor Model

> **Parent**: [33-CROSS-CUTS](../../33-CROSS-CUTS.md)
>
> The cross-cut architecture borrows three definitions from category theory: category,
> endofunctor, and natural transformation. This depth file explains the mathematical
> foundations, why they were chosen over alternatives, and how the production Rust code
> realizes each concept.

---

## 1. The Category Sig

A **category** consists of objects, morphisms between objects, an associative composition
operation, and identity morphisms (Mac Lane 1971, Ch. I, Def. 1).

In Roko, the category **Sig** is defined as:

| Component | Concrete realization |
|-----------|---------------------|
| Objects | Typed Signal bundles: `Vec<Signal>` conforming to a particular schema |
| Morphisms | Cells: functions `Vec<Signal> -> Vec<Signal>` |
| Composition | Graph sequencing: Cell A's output feeds Cell B's input (edge relation) |
| Identity | Pass-through Cell: `output = input` |

**Associativity** holds because Graph edge ordering is deterministic: `(A . B) . C`
and `A . (B . C)` produce the same topological execution order. **Identity** holds
because composing with a pass-through Cell leaves the other Cell's behavior unchanged.

Sig is a **concrete category** -- its objects have an underlying set (the set of all
possible `Vec<Signal>` values). This grounds the abstraction: we are not reasoning about
arbitrary categorical structures but about a specific category of Signal transformations.

---

## 2. Endofunctors

An **endofunctor** F: C -> C is a functor from a category to itself (Mac Lane 1971,
Ch. I, Sec. 3). It maps objects to objects and morphisms to morphisms, preserving
composition and identity.

### 2.1 Functor Laws

For F to be a valid endofunctor on Sig:

1. **Object mapping**: F takes each Signal bundle S to an enriched bundle F(S).
2. **Morphism mapping**: F takes each Cell c to an enriched Cell F(c), where
   F(c)(input) = post_enrich(c(pre_enrich(input))).
3. **Identity preservation**: F(id_S) = id_{F(S)}. Enriching the pass-through Cell
   must produce a Cell that is also a pass-through (modulo the enrichment metadata).
   In production, this is satisfied by the `should_short_circuit()` contract.
4. **Composition preservation**: F(g . f) = F(g) . F(f). Enriching a composition of
   Cells is equivalent to composing the enriched Cells.

### 2.2 Why Endofunctors and Not Monads

A monad M on Sig would provide `unit: S -> M(S)` and `join: M(M(S)) -> M(S)` with
coherence laws. This is strictly more structure than cross-cuts need. Cross-cuts do not
nest (you do not apply MemoryFunctor to an already-memory-enriched bundle expecting
further flattening). The enrichment is idempotent at each step, not recursive.

Endofunctors without the monad structure give us:
- Composability (F . G is also an endofunctor)
- Independence (each F_i can be enabled or disabled)
- Testability (test F in isolation by comparing F(cell)(input) vs cell(input))

Without the overhead of:
- A meaningful `unit` operation (Signals are already in the category)
- A meaningful `join` operation (nested enrichment is not a use case)

### 2.3 Production Realization

The `CrossCutFunctor` trait is the morphism-mapping half of the endofunctor. The
object-mapping half is implicit: F(S) = S with additional metadata Signals appended or
violating Signals removed. The `EnrichedCell` wrapper composes multiple functors into
a single enriched morphism.

The `should_short_circuit()` method is the identity-preservation law made explicit:
when a functor has nothing to contribute (empty store, neutral PAD, per-tick Dreams),
it degrades to the identity transformation.

---

## 3. Natural Transformations

A **natural transformation** eta: F => G between two functors F, G: C -> C assigns
to each object X in C a morphism eta_X: F(X) -> G(X) such that the naturality square
commutes (Mac Lane 1971, Ch. IV, Def. 1).

### 3.1 Naturality Condition

For each morphism f: X -> Y in Sig:

```
G(f) . eta_X = eta_Y . F(f)
```

In words: it does not matter whether you first transform under F and then apply
eta, or first apply eta and then transform under G. The result is the same.

### 3.2 The Six Transformations

The three behavioral functors (Memory, Daimon, Dreams) form a fully connected triangle
with six natural transformations. Each transformation maps domain-specific state from
one functor's output to another functor's input:

```
         eta_MN
Memory ---------> Daimon
  ^  \             / ^
  |   \           /  |
  |    eta_MD    /   |
  |     \       /    |
  |      v     v     |
  |       Dreams     |
  |      /     \     |
  |  eta_DM  eta_DN  |
  |    /         \   |
  +--+           +---+
     eta_NM      eta_ND
```

### 3.3 The Commuting Triangle

The critical coherence condition is that the composite path
Daimon -> Memory -> Dreams must equal the direct path Daimon -> Dreams:

```
eta_MD . eta_NM = eta_ND
```

More precisely, the outputs must agree on the three fields that determine dream
behavior: `episode_ids`, `priority`, and `trigger_delta`. This is verified by
assertion in both the unit test and the production `run_gate_failure_cascade`
function (via `debug_assert_eq!`).

### 3.4 Why Naturality Matters

Naturality ensures that the order in which transformations compose does not introduce
inconsistencies. If the triangle did not commute, the system could enter a state where:

- The direct path triggers a delta dream (high priority, Struggling state)
- The indirect path does not (different priority from stored entry)

This would mean the system's behavior depends on the implementation order of the
cascade, not on the underlying affect state. Commutativity eliminates this class
of bugs by construction.

---

## 4. Composition of Endofunctors

The four cross-cut functors compose as:

```
F_total = F_safety . F_daimon . F_memory . F_dreams
```

Since the composition of endofunctors is itself an endofunctor (Mac Lane 1971,
Ch. I, Prop. 2), `F_total` satisfies the functor laws by construction. The
`EnrichedCell` wrapper enforces this composition through its forward-pre/reverse-post
execution pattern.

The order is not arbitrary:
- Safety is outermost (hard constraint, cannot be bypassed)
- Daimon is next (behavioral gating sees all surviving Signals)
- Memory is next (knowledge retrieval feeds into Daimon context)
- Dreams is innermost (per-tick identity, no effect)

---

## References

- Mac Lane, S. (1971). *Categories for the Working Mathematician*. Springer.
  Chapters I (Categories, Functors), III (Natural Transformations), IV (Adjoints).
- Awodey, S. (2010). *Category Theory*. 2nd ed. Oxford University Press.
  Chapters 1-4 for accessible introduction.
- Barr, M. & Wells, C. (1990). *Category Theory for Computing Science*.
  Prentice Hall. Application-oriented treatment.
