# Placement Strategies — The Lost-in-the-Middle U-Shape

> How the Composer places content in the prompt to mitigate the "lost-in-the-middle" effect
> in LLM attention.

**Status**: Shipping
**Crate**: `roko-compose`
**Last reviewed**: 2026-04-19

---

## The Lost-in-the-Middle Effect

Research ([Liu et al., 2023](https://arxiv.org/abs/2307.03172)) showed that LLMs attend to
content at the **beginning** and **end** of a long context window more reliably than to
content in the **middle**. Content placed in the middle of a long prompt is more likely to
be ignored.

For a Roko agent with 16+ recalled memories, placing all memories in the middle of the
prompt risks losing the most important context.

---

## The U-Shape Strategy

Place the most important content at both ends:

```
[TOP]         Role + Safety                       ← high attention (start)
              Task description
              Top-k most relevant memories        ← important: early position
[MIDDLE]      Additional context                  ← lower attention zone
[BOTTOM]      Format instructions
              Top-j most relevant memories again  ← important: late position
              Metadata                            ← high attention (end)
```

Key insight: the most salient recalled memories appear twice — once early (Context layer)
and once late (Memory layer with UShape strategy). The LLM sees them at both ends of its
attention distribution.

---

## Configuration

```rust
// source: crates/roko-compose/src/lib.rs
SystemPromptBuilder {
    memory_selection: MemorySelectionStrategy::UShape { top_k: 5, bottom_k: 3 },
    layer_order: vec![
        PromptLayer::Role,
        PromptLayer::Safety,
        PromptLayer::Task,
        PromptLayer::Context,   // top_k memories here
        PromptLayer::Format,
        PromptLayer::Memory,    // bottom_k memories here
        PromptLayer::Metadata,
    ],
    ..Default::default()
}
```
<!-- source: crates/roko-compose/src/lib.rs -->

---

## When to Use Other Strategies

- **`TopK(k)`** — when the prompt is short enough that position effects are negligible
  (< 4,000 tokens).
- **`Budget(max_tokens)`** — when fitting as many memories as possible matters more than
  positional placement.
- **`UShape`** — default for long prompts (> 8,000 tokens) where the lost-in-middle effect
  is significant.

---

## See Also

- [Implementation](./03-implementation.md)
- [Semantics](./02-semantics.md)

## Open Questions

- Should the U-shape strategy be validated empirically against Roko agents, or is the
  Liu et al. finding sufficient justification?
