# COMPOSE — Stage 4 of the Cognitive Loop

> Assemble the context window for the selected execution target.

**Status**: Shipping
**Crate**: `roko-agent`
**Depends on**: [Composer operator](../05-operators/composer.md),
[RouteDecision](03-stage-route.md), [ScoredEngram](02-stage-score.md)
**Used by**: [ACT](05-stage-act.md), [loop\_tick()](09-loop-tick-code.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

COMPOSE takes the route decision and the ranked scored candidates and builds a
`ComposedContext` — the prompt, tool call spec, or sub-agent instruction that will
be passed to the selected target in the ACT stage. The Composer respects a token
budget, ensures coherence, and applies the right system prompt for the selected target.

---

## The Idea

Context assembly is where cognitive agents spend most of their "intelligence budget."
A well-composed context is the difference between a correct answer and a hallucinated
one. Yet context assembly is often an afterthought — a grab-bag of whatever fits in
the window.

Roko's COMPOSE stage treats context assembly as a first-class optimization problem:
given a token budget, a set of ranked candidates, and a target capability, produce
the context that maximizes expected output quality. The Composer operator is the
abstraction for this optimization.

The default `GreedyComposer` fills the window greedily from the top of the scored
list, stopping when the budget is exhausted. Custom composers may re-rank with
coherence penalties, use chain-of-thought scaffolding, or structure the context as
a dialogue history.

---

## Specification

```rust
// source: crates/roko-agent/src/loop/compose.rs
pub struct ComposedContext {
    pub target:       RouteTarget,
    pub system_prompt: Option<String>,
    pub messages:     Vec<ContextMessage>,
    pub token_count:  usize,
    pub budget_used:  f32,   // fraction of token budget consumed
}

pub enum ContextMessage {
    System(String),
    User(String),
    Assistant(String),
    ToolResult { tool: ToolId, content: String },
}

pub trait Composer: Send + Sync {
    fn compose(
        &self,
        route:      &RouteDecision,
        candidates: &[ScoredEngram],
        stimulus:   &Pulse,
        context:    &ComposerContext,
    ) -> Result<ComposedContext, ComposerError>;
}
```

`ComposerContext` carries the token budget, the system prompt library, and the current
speed tier (which controls how aggressively to fill the window).

---

## Semantics

1. Load the system prompt for the selected target (from the prompt library keyed by
   `RouteTarget`).
2. Always include the stimulus as the final user message.
3. Walk the scored candidates in order. For each:
   a. Estimate token cost of including this Engram.
   b. If adding it would exceed the budget, stop.
   c. Otherwise, format it as a `ContextMessage` and append.
4. Return `ComposedContext`.

Token budget by speed tier (defaults, all configurable):

| Tier | Max tokens in context |
|---|---|
| T0 (Gamma) | 4 096 |
| T1 (Theta) | 16 384 |
| T2 (Delta) | 128 000 |

---

## Failure Modes

| Failure | Cause | Recovery |
|---|---|---|
| `ComposerError::BudgetExhausted` | Even the stimulus alone exceeds budget | Truncate stimulus; log warning |
| `ComposerError::NoSystemPrompt` | Target has no registered system prompt | Use empty system prompt; log warning |
| `ComposerError::Timeout` | Composition took > stage budget | Return partial context (stimulus + top-1 candidate) |

---

## Performance

| Metric | Target | P99 budget |
|---|---|---|
| Wall time (T0, 16 candidates) | < 2 ms | < 5 ms |
| Wall time (T1, 64 candidates) | < 5 ms | < 12 ms |
| Token counting | O(N tokens) | — |

Token counting uses a cached tokenizer. The first call per model populates the cache;
subsequent calls are O(1) per token.

---

## Examples

### 1. Minimal Gamma context

A reactive tick with 8 candidates, 4 096-token budget. The stimulus is 120 tokens.
The top 3 candidates add 380 tokens total. All 3 are included; the remaining 5 exceed
the budget. Context has 500 tokens total — well under budget.

### 2. Rich Theta context

A reflective tick. Budget is 16 384 tokens. The Composer includes 12 candidates,
a structured chain-of-thought scaffold, and a 500-token system prompt. Total context:
11 200 tokens. The richer context allows the model to cross-reference multiple prior
observations before generating its response.

### 3. Single-message tool call

The route target is a `Tool` (e.g., `web_search`). The Composer formats the tool call
spec from the stimulus. No prior Engrams are included — tool calls are typically
stateless. The `ComposedContext.messages` has one entry.

---

## See also

- [Composer operator](../05-operators/composer.md) — how to implement custom context assembly
- [ROUTE](03-stage-route.md) — the route decision that drives composition
- [ACT](05-stage-act.md) — consumes the composed context
- [Dual-Process](10-dual-process.md) — token budget scaling by tier
- [Performance](14-performance.md) — composition contributes to total tick latency
