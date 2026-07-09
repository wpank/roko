# Composer API Reference

**Status**: Shipping
**Crate**: `roko-compose`
**Last reviewed**: 2026-04-19

---

```rust
// source: crates/roko-compose/src/lib.rs
pub trait Composer: Send + Sync {
    fn compose(&self, ctx: &LoopContext, action: &Action) -> Result<PromptOutput, ComposerError>;
}
```
<!-- source: crates/roko-compose/src/lib.rs -->

## `PromptOutput` Fields

| Field | Type | Description |
|---|---|---|
| `system_prompt` | `String` | Assembled prompt |
| `estimated_tokens` | `usize` | Token estimate |
| `included_engrams` | `Vec<ContentHash>` | Engrams used |

## `ComposerError` Variants

| Variant | Meaning |
|---|---|
| `ContextWindowExceeded { tokens, limit }` | Prompt too long |
| `Computation(String)` | Build error |
