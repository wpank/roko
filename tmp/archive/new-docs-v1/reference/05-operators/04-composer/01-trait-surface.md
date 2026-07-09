# Composer — Trait Surface

**Status**: Shipping
**Crate**: `roko-compose`
**Last reviewed**: 2026-04-19

---

```rust
// source: crates/roko-compose/src/lib.rs

/// Builds the LLM system prompt from loop context and selected action.
pub trait Composer: Send + Sync {
    fn compose(
        &self,
        ctx: &LoopContext,
        action: &Action,
    ) -> Result<PromptOutput, ComposerError>;
}

#[derive(Debug, Clone)]
pub struct PromptOutput {
    /// The assembled system prompt string.
    pub system_prompt: String,
    /// Estimated token count (for context window management).
    pub estimated_tokens: usize,
    /// Which engrams from `ctx.recalled` were included.
    pub included_engrams: Vec<ContentHash>,
}

#[derive(Debug, thiserror::Error)]
pub enum ComposerError {
    #[error("prompt exceeds context window: {tokens} tokens > {limit}")]
    ContextWindowExceeded { tokens: usize, limit: usize },
    #[error("composition error: {0}")]
    Computation(String),
}
```
<!-- source: crates/roko-compose/src/lib.rs -->
