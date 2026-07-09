# Router API Reference

**Status**: Shipping
**Crate**: `roko-agent`
**Last reviewed**: 2026-04-19

---

```rust
// source: crates/roko-agent/src/router.rs
pub trait Router: Send + Sync {
    fn route(&self, engram: &Engram, score: &Score) -> Result<Option<Action>, RouterError>;
}
```
<!-- source: crates/roko-agent/src/router.rs -->

## `ActionKind` Variants

| Variant | Meaning |
|---|---|
| `ExecuteTask` | Execute the primary task |
| `RetrieveContext` | Fetch more context before acting |
| `AskClarification` | Ask the user for more information |
| `DeferToAgent(id)` | Hand off to a named agent |
| `Respond` | Generate a response immediately |
| `NoOp` | Do nothing this tick |

## Implementations

| Type | Strategy |
|---|---|
| `StaticRouter` | Rule-based |
| `ConfidenceRouter` | Score-driven |
| `UCBRouter` | UCB1 bandit |
| `CascadeRouter` | Static → Confidence → UCB |
| `NoOpRouter` | Always `Ok(None)` |
