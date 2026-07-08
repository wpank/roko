# Topics

> A `Topic` is the routing address for a `Pulse`. Topics are hierarchical dot-separated
> strings. This page covers naming conventions, hierarchy semantics, and the standard
> topic vocabulary used by Roko operators.

**Status**: Specified
**Crate**: `roko-core` (planned)
**Depends on**: [Trait Surface](./01-trait-surface.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

A `Topic` is a string like `"agent.cognition.recall"`. The `.` separates segments. Publishers
write to an exact topic; subscribers can match by exact name, prefix, or glob. Roko defines
a standard topic vocabulary so that operators can interoperate without custom wiring.

---

## Topic Syntax

```
topic     = segment ("." segment)*
segment   = [a-z][a-z0-9_-]*
```

- All lowercase.
- Segments are alphanumeric with `_` and `-` allowed.
- No leading or trailing `.`.
- Maximum depth: 8 segments (to prevent pathological subscriber matching).
- Maximum length: 128 characters.

Valid: `"agent.cognition.recall"`, `"runtime.error"`, `"loop.step.act"`.
Invalid: `"Agent.Recall"` (uppercase), `"a..b"` (empty segment), `"runtime."` (trailing dot).

---

## Hierarchy Semantics

Topics are organised in a tree. A segment adds one level of specificity:

```
agent
  └── cognition
        ├── recall
        ├── score
        └── store
  └── affect
        ├── valence
        └── arousal
loop
  └── step
        ├── sense
        ├── recall
        ├── score
        ├── act
        ├── observe
        ├── store
        └── learn
```

The hierarchy is purely a naming convention — it is not enforced by the `Bus` implementation.
Subscribers use `TopicFilter::Prefix` to subscribe to an entire subtree.

---

## Standard Topic Vocabulary

Roko defines the following standard topics for operator interoperability:

| Topic | Publisher | Consumers | Description |
|---|---|---|---|
| `loop.step.sense` | Cognitive loop | Operators | Loop tick started — new input available |
| `loop.step.recall` | Cognitive loop | Neuro cross-cut | RECALL step — substrate queried |
| `loop.step.score` | Scorer | Router, Gate | Score computed |
| `loop.step.act` | Router | Composer | Action selected |
| `loop.step.observe` | Cognitive loop | Daimon | Outcome observed |
| `loop.step.store` | Cognitive loop | Substrate | STORE step — engram written |
| `loop.step.learn` | Dreams | — | Delta-speed consolidation tick |
| `agent.affect.valence` | Daimon | Scorer, Router | Affective valence update |
| `agent.affect.arousal` | Daimon | Scorer | Arousal signal |
| `agent.error` | Any operator | Policy | Error signal |
| `agent.safety` | Policy | All | Safety override signal |
| `prediction.error` | Policy | Calibrator | Prediction error signal |

---

## Topic Registration

In the target state, topics are implicitly created on first `publish`. There is no explicit
topic registration step. The `Bus` backend creates the ring buffer lazily.

---

## See Also

- [Topic Filters](./03-topic-filters.md) — how subscribers match topics
- [Publish / Subscribe](./04-publish-subscribe.md)
- [Trait Surface](./01-trait-surface.md)

## Open Questions

- Should topic naming be enforced at compile time (a `topic!` macro that validates the
  string at compile time)?
- Should the standard vocabulary be extended with `versioned` prefixes
  (`loop.v2.step.score`) for non-breaking evolution?
