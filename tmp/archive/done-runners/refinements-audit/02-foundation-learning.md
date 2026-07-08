# Foundation And Learning

## Foundation: what to keep

### Keep the diagnosis, narrow the doctrine

The foundation set correctly identifies the right redesign pressure:
- durable records are not the whole runtime story;
- transport deserves explicit architectural status;
- several downstream concerns become cleaner once the runtime has a first-class
  bus or pulse concept.

That should survive the audit.

What should not survive unchanged is the jump from:
"transport is under-modeled"
to
"therefore every operator, trait, and noun should be redefined around a total
dual-medium worldview."

### Strongest foundational moves

- Treat storage vs transport as a real architectural axis.
- Keep `Pulse` as the likely transport noun if a new noun is needed.
- Use `Bus` as the runtime seam that carries transport explicitly.
- Use StateHub/projection logic as the practical bridge from live events to
  stable UI and operator surfaces.

### Foundational moves that need narrowing

- `Datum` should not become universal just because it is elegant. Use it only
  where medium polymorphism proves its worth.
- Do not rewrite every operator API around dual-medium input in the first pass.
- Treat the seven-step loop as a helpful reference architecture, not a law that
  every crate must immediately mirror.

## Foundation: biggest risks

### 1. Over-generalized operator algebra

The proposed operator generalization is attractive on paper but too broad as a
first migration target. Different operators want different abstractions.

Safer sequence:
- add a transport contract;
- identify which operators genuinely need dual-medium handling;
- only then widen traits or introduce local polymorphic wrappers.

### 2. Kernel rhetoric can outrun kernel need

The redesign does not need a full metaphysical restatement of the kernel before
it has a small number of new runtime contracts that are obviously useful.

Prefer:
- a transport contract;
- a small set of event topics or envelopes;
- projection contracts;
- explicit replay and subscription semantics.

Be careful with:
- total renaming passes;
- universal operator algebra;
- new foundational nouns that do not buy a concrete simplification.

### 3. Glossary can harden hypotheses too early

The glossary is useful, but it currently hardens many proposed nouns as if they
were settled redesign-level concepts. It should distinguish:
- current canonical terms;
- target-state terms likely to become canonical;
- exploratory or historical terms.

## Learning: what to keep

### Evented calibration is the best core idea

The strongest learning idea is not grand cybernetics. It is the practical move
toward shared calibration loops:
- expectation;
- outcome;
- discrepancy;
- adjustment.

This should remain central.

### Heuristics as a middle layer are worth building

A typed, inspectable heuristic layer between raw episodes and distilled
playbooks is one of the best ideas across the entire set.

That means:
- typed heuristic objects;
- visible provenance;
- challenge and contradiction records;
- calibration history;
- promotion/demotion rules tied to runtime evidence.

### HDC has real value in a narrower role

HDC is useful as:
- cheap similarity search;
- clustering aid;
- retrieval acceleration;
- lightweight representation for durable knowledge indexing.

It should not become:
- universal semantic truth geometry;
- reliable consensus detector;
- the hidden explanation for all future memory or reasoning behavior.

## Learning: what needs narrowing or deferral

### 1. Active inference claims are too large

Better framing:
- "calibration-driven control";
- "evented prediction/outcome scaffolding";
- "routing and prompt feedback loops first."

### 2. Worldview and falsifier rhetoric exceeds current mechanism

The conceptual story is interesting, but it is better as a later layer on top
of heuristics, contradictions, and typed claims. As a redesign target, it is
too abstract too early.

### 3. Demurrage is ahead of the memory model

Demurrage should be treated as:
- a hypothesis for future memory shaping,
not
- the governing explanation of memory or forgetting.

### 4. c-factor is not yet a stable core metric

Until it has one clear interpretation and one trusted measurement path, it
should be described as a coordination-health experiment, not a mature
collective-intelligence scalar.

### 5. Research-to-runtime is still a narrative layer

The instinct is good: make external knowledge auditable and contestable. The
problem is scope. Build this in ascending order:
- typed claims;
- provenance and source quality;
- contradiction and replication;
- only later, richer research-economy semantics.

## Recommended rewrite principles for this area

1. Keep the transport diagnosis and the need for cleaner runtime seams.
2. Rewrite foundation docs as a tighter target-state architecture, not a full
   kernel ideology.
3. Treat `Bus` as the main kernel addition and `Datum` as optional.
4. Reframe learning around calibration and typed heuristics, not sweeping
   cybernetic claims.
5. Reduce the number of places where HDC, demurrage, c-factor, and claims are
   described as foundational laws.
6. Move the more speculative parts into clearly marked research or future-work
   sections.
