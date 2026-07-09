# Composer Failure Modes

**Status**: Shipping
**Last reviewed**: 2026-04-19

---

<!-- ADDED -->

## F1 — Context Window Exceeded

The assembled prompt is too long for the model's context window.

**Recovery**: Reduce `k` in the memory selection strategy. Use `Budget(max_tokens)` to hard-cap memory content. The loop can retry with fewer memories.

## F2 — Missing Agent Config

`ctx.agent_config` lacks required fields (persona name, etc.).

**Recovery**: `ComposerError::Computation("missing persona_name")`. The runtime should validate config at startup.

## See Also

- [Invariants](./05-invariants.md)
- [Placement Strategies](./09-placement-strategies.md)
