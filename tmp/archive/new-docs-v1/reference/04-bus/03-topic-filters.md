# Topic Filters

> `TopicFilter` is the subscription matcher. A subscriber provides a `TopicFilter`; the
> `Bus` delivers all `Pulse`s whose `Topic` matches it.

**Status**: Specified
**Crate**: `roko-core` (planned)
**Depends on**: [Topics](./02-topics.md), [Trait Surface](./01-trait-surface.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Three filter variants: `Exact` (one topic), `Prefix` (subtree), `Glob` (MQTT-style pattern
with `+` for one segment and `#` for any suffix). All matching is done at the Bus layer —
subscribers never filter themselves.

---

## Filter Variants

### `TopicFilter::Exact(topic)`

Matches one topic and no other.

```rust
// source: crates/roko-core/src/bus.rs  [target-state]
let filter = TopicFilter::Exact(Topic::new("loop.step.score"));
// Matches: "loop.step.score"
// No match: "loop.step.recall", "loop.step.score.extended"
```
<!-- source: crates/roko-core/src/bus.rs -->

### `TopicFilter::Prefix(prefix)`

Matches any topic that starts with the given prefix (including the prefix itself).

```rust
// source: crates/roko-core/src/bus.rs  [target-state]
let filter = TopicFilter::Prefix("loop.step".into());
// Matches: "loop.step", "loop.step.score", "loop.step.recall", "loop.step.act"
// No match: "loop", "agent.affect"
```
<!-- source: crates/roko-core/src/bus.rs -->

### `TopicFilter::Glob(pattern)`

MQTT-style pattern matching:
- `+` matches exactly one segment (not `.`).
- `#` matches zero or more segments (must be the last segment).

```rust
// source: crates/roko-core/src/bus.rs  [target-state]
let filter = TopicFilter::Glob("agent.+.valence".into());
// Matches: "agent.affect.valence", "agent.neuro.valence"
// No match: "agent.affect.arousal", "agent.affect.valence.high"

let filter = TopicFilter::Glob("loop.#".into());
// Matches: "loop", "loop.step", "loop.step.score", "loop.step.score.extended"
```
<!-- source: crates/roko-core/src/bus.rs -->

---

## Matching Algorithm

```
Exact(t): topic == t
Prefix(p): topic.starts_with(p) && (topic.len() == p.len() || topic[p.len()] == '.')
Glob(g):  recursive segment-by-segment match
          - "+" matches exactly one segment (characters until next "." or end)
          - "#" at end matches any suffix (including empty)
          - "#" not at end is invalid (implementation error)
```

The matching is O(depth) for Exact and Prefix, O(depth × pattern_length) for Glob. For
typical depths ≤ 8 and pattern lengths ≤ 128 characters, matching is negligible.

---

## Performance Note

The Bus evaluates every `publish` call against all active subscriber filters to determine
which subscribers receive the event. With n subscribers and depth d topics:

- `Exact` matching: O(n) hash lookups.
- `Prefix` matching: O(n × d) string comparisons.
- `Glob` matching: O(n × depth × pattern) recursive.

In practice, most subscribers use `Exact` or short `Prefix` patterns, and n (active
subscribers per agent) is small (< 20). The matching cost is negligible relative to event
processing.

---

## See Also

- [Topics](./02-topics.md)
- [Publish / Subscribe](./04-publish-subscribe.md)

## Open Questions

- Should `TopicFilter::Glob` support `**` (recursive segment wildcard) as an alternative to
  `#`, for readability?
- Should `TopicFilter` be a sealed trait to prevent third-party filter implementations that
  might break performance assumptions?
