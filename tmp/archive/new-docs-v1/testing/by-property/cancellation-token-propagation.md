# Cancellation Token Propagation

> Cancelling a parent token cancels all child tokens. Cancelling a child does not cancel the parent.

**Crate**: `roko-runtime`
**Test type**: Integration test (via orchestrator)
**Enforcement**: `CancellationToken::cancel`, `CancellationToken::child`
**Last reviewed**: 2026-04-19

---

## Statement

For all parent tokens P and child tokens C = P.child():
1. `P.cancel()` → `C.is_cancelled() == true`
2. `C.cancel()` → `P.is_cancelled() == false`

---

## Test

```rust
#[test]
fn cancellation_propagates_to_children() {
    let parent = CancellationToken::new();
    let child = parent.child();

    parent.cancel();

    assert!(child.is_cancelled(), "Child must be cancelled when parent is cancelled");
    assert!(parent.is_cancelled(), "Parent must be cancelled");
}

#[test]
fn cancelling_child_does_not_cancel_parent() {
    let parent = CancellationToken::new();
    let child = parent.child();

    child.cancel();

    assert!(child.is_cancelled(), "Child must be cancelled");
    assert!(!parent.is_cancelled(), "Parent must not be cancelled by child cancellation");
}
```

---

## See also

- [../by-subsystem/subsystem-runtime.md](../by-subsystem/subsystem-runtime.md)
