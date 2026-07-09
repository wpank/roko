# A — Gate Foundation (Docs 00, 01)

Post-audit refresh for the gate-foundation section.

---

## Verdict

The verification foundation is **present and shipped**.

What the old parity note got wrong was not the existence of gaps, but the scale of them. `docs/04-verification/00` and `01` should no longer read like the project is still waiting for a gate system to appear.

---

## Keep

These are current-code truths and should stay in present tense:

- The `Gate` trait in `roko-core` returns `Verdict`, not `Result<Verdict>`.
- The gate foundation lives in `roko-core` + `roko-gate`, not as a target-state design.
- `roko-gate` has a real inventory of gate implementations.
- The runtime has an active rung-to-gate mapping through `rung_dispatch.rs`.

Key anchors:

- `crates/roko-core/src/traits.rs:102-108`
- `crates/roko-gate/src/lib.rs:17-56`
- `crates/roko-gate/src/rung_dispatch.rs:76-120`

---

## Rewrite

### 1. Treat 11 gates as a shipped floor, not a missing wishlist

The audit baseline for this section is simple:

- **11 gate implementations exist and work**
- the `Gate` trait is live
- the verification layer is already part of the running system

The crate inventory is broader than the audit floor once specialized modules are counted, but this parity pack should avoid turning that into another overscoped taxonomy. The important correction is that verification foundation is real.

### 2. Stop framing the section as “only compile/test/clippy”

That was directionally stale. The gate surface includes, at minimum:

- shell / compile / clippy / test
- symbol / diff
- generated-test / property-test / integration
- additional specialized gates such as fact-check, verify-chain, and judge-style gates

Not every gate is equally central on the default path, but the docs should not collapse the whole crate down to the smallest historical subset.

### 3. Separate “exists” from “default runtime path”

This section should say:

- the foundation exists
- several gates are on the current runtime path
- some specialized gates require richer inputs and therefore surface as conditional/stubbed runtime behavior

It should **not** imply that every gate is fully exercised on every task.

---

## Narrow

Keep these caveats explicit:

- `GatePipeline` and selector abstractions are real, but they belong more naturally in section `B`.
- Some specialized gates are conditionally wired or require additional runtime inputs.
- `CodeExecutionGate` should not be described as a fully live verification primitive unless the runtime path actually uses it.

---

## Replacement Summary

Use this posture in the refreshed docs:

- `Gate` is a current kernel trait.
- Verification foundation is already implemented.
- The main remaining work is not “build a gate system.”
- The real remaining work is keeping runtime documentation honest about which gates run by default, which are conditional, and which are future-facing.

---

## Carry-Forward

Do not open new batch work here for:

- new gate families
- large gate-taxonomy rewrites
- backend completion for speculative gate types

The foundation section only needs to be truthful and scoped.
