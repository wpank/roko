# Composer — 7-Layer SystemPromptBuilder

> The shipping `Composer` implementation. `SystemPromptBuilder` constructs the system
> prompt by stacking 7 named layers in a configurable order.

**Status**: Shipping
**Crate**: `roko-compose`
**Last reviewed**: 2026-04-19

---

## The 7 Layers

```rust
// source: crates/roko-compose/src/lib.rs

pub struct SystemPromptBuilder {
    pub context_window_limit: usize, // default: 128_000 tokens
    pub layer_order: Vec<PromptLayer>,
    pub memory_selection: MemorySelectionStrategy,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PromptLayer {
    Role,      // Agent persona and capabilities
    Safety,    // Hard constraints
    Task,      // Current action description
    Context,   // Top-k recalled engrams (most salient)
    Memory,    // Remaining recalled engrams
    Format,    // Output format instructions
    Metadata,  // Agent version, session, timestamp
}
```
<!-- source: crates/roko-compose/src/lib.rs -->

---

## Default Layer Order

The default order implements the U-shape placement strategy:

```
Role → Safety → Task → Context → Memory → Format → Metadata
```

Role and Safety are at the top (high attention). Task is upper-middle. Context (most salient
memories) comes before additional memory to exploit early-position attention. Format and
Metadata are at the bottom.

---

## Memory Selection

The `memory_selection` field controls which recalled `Engram`s are included:

| Strategy | Description |
|---|---|
| `TopK(k)` | Include the k highest-score engrams |
| `Budget(max_tokens)` | Include as many as fit in `max_tokens` |
| `UShape { top_k, bottom_k }` | Top-k for Context layer, bottom-k for Memory layer |

The `UShape` strategy explicitly implements the lost-in-the-middle mitigation: the most
important memories go at both the Context (early position) and Memory (late position) layers.

---

## Layer Rendering

Each layer is rendered by a layer-specific renderer:

```rust
// source: crates/roko-compose/src/lib.rs
fn render_role(config: &AgentConfig) -> String {
    format!("You are {}. {}",
        config.persona_name,
        config.capabilities_description)
}

fn render_context(engrams: &[Engram], max_tokens: usize) -> String {
    engrams.iter()
        .take_while(|e| /* token budget */)
        .map(|e| format!("- {}", e.body.summary()))
        .collect::<Vec<_>>()
        .join("\n")
}
```
<!-- source: crates/roko-compose/src/lib.rs -->

---

## See Also

- [Placement Strategies](./09-placement-strategies.md)
- [Trait Surface](./01-trait-surface.md)
