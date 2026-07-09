# Scorer Rationale

**Status**: Shipping
**Crate**: `roko-core`
**Last reviewed**: 2026-04-19

---

## Why a Trait (Not a Function)

Scoring is not a fixed formula — different agents, different tasks, and different deployment
contexts call for different scoring strategies. A trait enables agents to swap scoring logic
without changing the loop. The `prior` accumulation pattern enables specialised scorers to
coexist without overwriting each other.

## Why 7 Axes (Not 1 or 3)

A single-number "relevance score" conflates four independent dimensions: how confident you
are (confidence), how new it is (novelty), how useful it is for the task (utility), and how
trustworthy the source is (reputation). Conflating them makes it impossible to optimise
for one dimension without distorting the others.

The three extended axes (`precision`, `salience`, `coherence`) were added for operators
that need finer-grained appraisal (Gate uses coherence; Daimon uses salience). They default
to neutral to avoid breaking existing scorers that don't set them.

## Why `prior: Score` (Not `&mut Score`)

`&mut Score` would allow any scorer to modify the prior in-place and return `()`. This
makes the composition model implicit — it is unclear from the signature that scorers build
on each other. Returning a new `Score` makes the accumulation explicit and checkable.

## Open Questions

- Should extended axes be a separate struct to signal that they are optional?
