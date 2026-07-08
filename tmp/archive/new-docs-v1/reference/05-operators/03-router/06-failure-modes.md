# Router Failure Modes

**Status**: Shipping
**Last reviewed**: 2026-04-19

---

<!-- ADDED -->

## F1 — No Strategy Matches (All Return None)

`CascadeRouter` returns `Ok(None)`. The loop uses a default action (`Action::NoOp` or configured fallback).

## F2 — UCB Arm Initialisation

If no arms have been initialised, `UCBRouter` returns `Ok(None)`. The cascade falls back to `NoOp`.

## F3 — Computation Error

Returns `Err(RouterError::Computation(...))`. The loop logs the error and uses the configured fallback action.

## See Also

- [Semantics](./02-semantics.md)
