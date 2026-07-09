# Composer Examples

**Status**: Shipping
**Last reviewed**: 2026-04-19

---

## Example: Using SystemPromptBuilder

```rust
// source: crates/roko-compose/src/lib.rs
let composer = SystemPromptBuilder {
    context_window_limit: 128_000,
    memory_selection: MemorySelectionStrategy::UShape { top_k: 5, bottom_k: 3 },
    ..Default::default()
};

let output = composer.compose(&ctx, &action)?;
println!("prompt ({} tokens): {}", output.estimated_tokens, &output.system_prompt[..200]);
```
<!-- source: crates/roko-compose/src/lib.rs -->
