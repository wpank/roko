# Router — Trait Surface

**Status**: Shipping
**Crate**: `roko-agent`
**Last reviewed**: 2026-04-19

---

```rust
// source: crates/roko-agent/src/router.rs

/// Selects an [`Action`] for the current loop tick.
///
/// `route` returns `Ok(None)` when the router has no applicable strategy
/// for this input — this is a valid signal to try the next router in a
/// cascade, not an error.
pub trait Router: Send + Sync {
    fn route(
        &self,
        engram: &Engram,
        score: &Score,
    ) -> Result<Option<Action>, RouterError>;
}

#[derive(Debug, Clone)]
pub struct Action {
    pub kind: ActionKind,
    pub target: Option<String>,  // tool name, agent id, etc.
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActionKind {
    ExecuteTask,
    RetrieveContext,
    AskClarification,
    DeferToAgent(String),
    Respond,
    NoOp,
}

#[derive(Debug, thiserror::Error)]
pub enum RouterError {
    #[error("routing computation failed: {0}")]
    Computation(String),
}
```
<!-- source: crates/roko-agent/src/router.rs -->

---

## See Also

- [Semantics](./02-semantics.md)
- [Invariants](./05-invariants.md)
