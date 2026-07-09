# S-safety-1: Audit permissive() call sites; mark test-only

## Task
Audit every `permissive(...)` call in `crates/roko-agent/src/safety/` and (broader) `crates/`. Each non-test site: replace with `restricted` or document why. Mark all test-only sites `#[cfg(test)]`.

## Runner Context
Runner audit-2026-05-01, group S. No dependencies. Wave 1.

## Source plan
`tmp/subsystem-audits/implementation-plans/28-safety-agent-hardening.md` § S-1.

## Read first

```bash
rg 'permissive\(' crates/ -g '*.rs' -n
```

For each match, classify:

- **Test code**: should be inside `#[cfg(test)]` mod or function.
- **Test helper**: should be marked `#[cfg(test)]`.
- **Production code**: should be replaced with `restricted` or have a documented justification.

## Exact changes

For each non-test, non-helper hit:

```rust
// Before
let layer = SafetyLayer::with_defaults().with_contract(AgentContract::permissive("default"));

// After (case A: replace)
let layer = SafetyLayer::with_defaults();    // restricted by default after T1-15
```

```rust
// Case B: was a test helper not gated
fn permissive_layer() -> SafetyLayer { ... }

// After
#[cfg(test)]
fn permissive_layer() -> SafetyLayer { ... }
```

```rust
// Case C: legitimately needs permissive in production (rare; document)
let layer = SafetyLayer::with_defaults()
    .with_contract(AgentContract::permissive("internal-bench"));   // rationale required
//
// SAFETY: This benchmark harness intentionally uses permissive
// contracts to measure raw model output speed. Not used in user-facing
// code paths. See bench/perf-baseline.md.
```

## Write Scope
- `crates/roko-agent/src/safety/mod.rs`
- `crates/roko-agent/src/safety/contract.rs` (only if defines `permissive`)
- (Any other crate that calls `permissive`)

## Verify

```bash
rg 'permissive\(' crates/ -g '*.rs'
# Each remaining hit is inside #[cfg(test)] or has a SAFETY comment.

rg 'permissive\(' crates/ -g '*.rs' -B 2 \
  | rg -B 2 'fn permissive_layer|fn permissive\(' \
  | head -30
```

## Acceptance Criteria

- All non-test `permissive(...)` calls removed or documented.
- All test helpers gated `#[cfg(test)]`.

## Do NOT

- Do NOT remove `AgentContract::permissive` from the API. Tests need it.
- Do NOT bundle with other S-safety batches.
- Do NOT change `SafetyLayer::with_defaults()` behavior — T1-15 already made it `restricted`.
- Do NOT introduce a "permissive_in_dev" feature flag.
