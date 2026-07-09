# Gate — Trait Surface

**Status**: Shipping
**Crate**: `roko-gate`
**Last reviewed**: 2026-04-19

---

## The Trait

```rust
// source: crates/roko-gate/src/lib.rs

/// Filters an [`Engram`] based on its [`Score`] and content.
///
/// Returns a [`Verdict`] — `Pass`, `Reject`, or `Abstain`.
/// Never returns an error for a filtering decision; errors are
/// for implementation failures (I/O, computation panics), not
/// for cases where the gate disagrees with the input.
pub trait Gate: Send + Sync {
    /// Evaluate whether this `Engram` should proceed.
    ///
    /// # Parameters
    /// - `engram`: the `Engram` being evaluated.
    /// - `score`: the accumulated `Score` from the Scorer chain.
    ///
    /// # Returns
    /// - `Ok(Verdict::Pass)`: this gate approves the Engram.
    /// - `Ok(Verdict::Reject(reason))`: this gate rejects the Engram.
    /// - `Ok(Verdict::Abstain)`: this gate has no opinion; skip it.
    /// - `Err(GateError)`: this gate crashed — the loop should log and treat
    ///   as `Abstain` by default (configurable).
    fn evaluate(
        &self,
        engram: &Engram,
        score: &Score,
    ) -> Result<Verdict, GateError>;
}
```
<!-- source: crates/roko-gate/src/lib.rs -->

---

## Supporting Types

```rust
// source: crates/roko-gate/src/lib.rs

#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Pass,
    Reject(String),   // rejection reason for logging
    Abstain,
}

impl Verdict {
    pub fn is_reject(&self) -> bool { matches!(self, Verdict::Reject(_)) }
    pub fn is_pass(&self) -> bool { matches!(self, Verdict::Pass) }
}

#[derive(Debug, thiserror::Error)]
pub enum GateError {
    #[error("gate computation failed: {0}")]
    Computation(String),
}
```
<!-- source: crates/roko-gate/src/lib.rs -->

---

## See Also

- [Semantics](./02-semantics.md)
- [Invariants](./05-invariants.md)
