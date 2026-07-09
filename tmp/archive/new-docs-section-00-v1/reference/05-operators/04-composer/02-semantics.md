# Composer Semantics

> What the `Composer` contract guarantees: what goes in, what comes out, and what the
> loop does with the output.

**Status**: Shipping
**Crate**: `roko-compose`
**Last reviewed**: 2026-04-19

---

## Input: `LoopContext`

`LoopContext` carries the state available to the Composer:

```rust
// source: crates/roko-runtime/src/loop.rs
pub struct LoopContext {
    pub current_engram: Engram,
    pub recalled: Vec<Engram>,         // from RECALL step
    pub score: Score,                  // from SCORE step
    pub agent_config: AgentConfig,     // role, persona, etc.
    pub session_id: Uuid,
    pub tick: u64,
}
```
<!-- source: crates/roko-runtime/src/loop.rs -->

## Output: `PromptOutput`

The Composer returns a fully assembled prompt string plus metadata. The loop takes
`system_prompt` and passes it directly to the LLM client.

## Contracts

1. `system_prompt` must be a valid UTF-8 string.
2. `estimated_tokens` must be ≥ actual token count (safe overestimate).
3. `included_engrams` must be a subset of `ctx.recalled`.
4. If the context window would be exceeded, return `Err(ComposerError::ContextWindowExceeded)`
   rather than silently truncating.

## What the Loop Does with `PromptOutput`

1. Passes `system_prompt` to the LLM client as the system turn.
2. Uses `estimated_tokens` to validate against the model's context window limit.
3. Records `included_engrams` in the loop audit log.

---

## See Also

- [Placement Strategies](./09-placement-strategies.md)
- [Implementation](./03-implementation.md)
