# The 7-Rung Selector

> Depth file for [07-GATES.md](../../07-GATES.md) section 3.
> Source: `crates/roko-gate/src/rung_selector.rs`, `crates/roko-gate/src/rung_dispatch.rs`

---

## 1. Overview

Not every task needs every gate. A one-line rename does not need
property-based testing. A new subsystem does. The rung selector solves
this: given a plan's complexity and the gates available in this environment,
it produces a sorted list of rungs to execute.

Three inputs drive the selection:

1. **Plan complexity** -- how ambitious is this change?
2. **Rung capabilities** -- which gates are actually available?
3. **Prior failures** -- did previous attempts fail, warranting escalation?

---

## 2. The Rung Enum

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rung {
    Compile       = 0,   // Does it compile?
    Lint          = 1,   // Does it pass linting?
    Test          = 2,   // Do tests pass?
    Symbol        = 3,   // Are required symbols present?
    GeneratedTest = 4,   // Do auto-generated tests pass?
    PropertyTest  = 5,   // Do property-based tests pass?
    Integration   = 6,   // Do integration tests pass?
}
```

### 2.1 Rung-by-Rung Details

**Rung 0: Compile.** Cheapest gate. Runs `cargo check --workspace` or
equivalent. Takes seconds. Catches syntax errors, type mismatches, missing
imports. Every plan runs at least this rung.

**Rung 1: Lint.** Runs `cargo clippy -- -D warnings` or language equivalent.
Catches code quality issues, common mistakes, style violations. Still fast --
typically under a minute.

**Rung 2: Test.** First "medium cost" gate. Tests can take minutes for large
projects. Catches functional regressions. Zero false positive rate for
deterministic tests.

**Rung 3: Symbol.** Verifies required symbols exist with correct kind,
visibility, and module path. Zero cost -- no subprocess, pure file walking.

**Rung 4: GeneratedTest.** Runs auto-generated tests that specifically
exercise the agent's output. More targeted than existing tests.

**Rung 5: PropertyTest.** Property-based tests (proptest style). Asserts
invariants over randomized inputs, providing stronger guarantees than
example-based tests.

**Rung 6: Integration.** Most expensive gate. Full integration tests that
may involve external services, databases, or network connections.

---

## 3. Plan Complexity

The selector's primary input classifies change scope:

```rust
pub enum PlanComplexity {
    Trivial,    // Rename, typo fix, config change
    Simple,     // Single function change, small bug fix
    Standard,   // Multi-file feature, medium scope
    Complex,    // New subsystem, architectural change
}
```

### 3.1 Complexity-to-Rung Mapping

| Complexity | Baseline Rungs | Rationale |
|------------|---------------|-----------|
| `Trivial` | Compile only (0) | No functional changes |
| `Simple` | Compile + Lint (0-1) | Small changes; lint catches quality issues |
| `Standard` | Compile + Lint + Test + Symbol (0-3) | Feature work needs test coverage |
| `Complex` | All available (0-6) | Architectural changes need full verification |

---

## 4. Escalation on Failure

When a plan fails a gate, complexity escalates:

```rust
impl PlanComplexity {
    pub fn escalate(&self) -> Self {
        match self {
            Self::Trivial  => Self::Simple,
            Self::Simple   => Self::Standard,
            Self::Standard => Self::Complex,
            Self::Complex  => Self::Complex,  // Already maximal
        }
    }
}
```

Escalation adds rungs on retry. If the easy checks catch a problem, the
change is more complex than initially classified, and deeper verification
is warranted.

### Escalation in Practice

```
Attempt 1: Trivial complexity -> Compile only
  -> Compile fails
  -> Escalate to Simple (prior_failures = 1)

Attempt 2: Simple complexity -> Compile + Lint
  -> Both pass
  -> Record pass at Rung 1
  -> Ratchet prevents regression below Rung 1

Attempt 3 (if task retry): Simple complexity -> Compile + Lint
  -> Lint fails
  -> Escalate to Standard (prior_failures = 2)

Attempt 4: Standard complexity -> Compile + Lint + Test + Symbol
  -> All pass
  -> Record pass at Rung 3
  -> Task verified
```

---

## 5. Rung Capabilities

Not every environment has every gate available:

```rust
pub struct RungCaps {
    pub compile: bool,
    pub lint: bool,
    pub test: bool,
    pub symbol: bool,
    pub generated_test: bool,
    pub property_test: bool,
    pub integration: bool,
}
```

The selector intersects complexity-implied rungs with available capabilities.
If the complexity says "run through Rung 3" but `symbol` is `false`, the
symbol rung is skipped.

### Capability Detection

`RungCaps` is constructed by the orchestrator based on project detection:

- If `Cargo.toml` exists: `compile: true, lint: true, test: true`
- If `roko.toml` specifies `[gates.symbol]`: `symbol: true`
- If property test fixtures exist: `property_test: true`

---

## 6. The select_rungs() Function

```rust
pub fn select_rungs(
    complexity: PlanComplexity,
    caps: &RungCaps,
    prior_failures: u32,
) -> Vec<Rung>
```

### Algorithm

1. If `prior_failures > 0`, escalate complexity by that many levels
   (capped at Complex).
2. Map complexity to maximum rung: Trivial -> 0, Simple -> 1,
   Standard -> 3, Complex -> 6.
3. Collect all rungs <= maximum that are available in `caps`.
4. Sort ascending (cheapest first).
5. Return the rung list.

### Worked Example

```
Input:
  complexity = Simple
  caps = { compile: true, lint: true, test: true, symbol: false, ... }
  prior_failures = 1

Step 1: Escalate Simple by 1 -> Standard
Step 2: Standard -> max rung 3
Step 3: Available rungs <= 3 = [Compile(0), Lint(1), Test(2)]
         (Symbol(3) excluded because caps.symbol = false)
Step 4: Already sorted
Result: [Compile, Lint, Test]
```

---

## 7. Rung Cost Ordering

| Rung | Typical Cost | Notes |
|------|-------------|-------|
| 0 (Compile) | 1-10 seconds | Incremental builds faster |
| 1 (Lint) | 2-60 seconds | Clippy slow on large codebases |
| 2 (Test) | 5s - 15 minutes | Depends on test count |
| 3 (Symbol) | 10-100 ms | Pure file I/O, no subprocess |
| 4 (GeneratedTest) | 30s - 5 minutes | Generation + execution |
| 5 (PropertyTest) | 10s - 10 minutes | Randomized exploration |
| 6 (Integration) | 1 minute - 1 hour | Infrastructure dependent |

Rung numbers encode logical ordering (compile before test before
integration), not strict cost ordering. Symbol (Rung 3) is actually
cheaper than Compile (Rung 0) but logically sits after Test.

---

## 8. The Verification-First Architecture

**Cheap gates that run first prevent expensive retries.**

- A compile failure caught in 3 seconds saves a 15-minute test run.
- A lint failure caught in 10 seconds saves the same 15-minute test run.
- A symbol check caught in 50 ms saves a 3-second compile + 10-second lint
  + 15-minute test run.

The cost savings compound. For a plan with 50 tasks averaging 3 attempts
each, efficient rung ordering can save hours of verification time.

---

## 9. Rung Dispatch

The `rung_dispatch.rs` module maps rungs to concrete gate instances and
provides the `GatePipelineBuilder`:

```rust
pub fn run_rung(rung: Rung, ...) -> Verdict { ... }
pub fn run_canonical_rung(rung: Rung, ...) -> Verdict { ... }
```

The `GatePipelineBuilder` constructs pipelines from `RungExecutionConfig`
and `RungExecutionInputs`, handling timeout overrides, parallelism limits,
and environment variables per rung.

---

## 10. Relationship to Adaptive Thresholds

The adaptive threshold system tracks per-rung pass rates. If a rung
consistently passes (20+ consecutive passes), the threshold system
recommends skipping it (advisory only). This provides runtime-adaptive
refinement complementing the static complexity-based selection.

The two systems compose: the rung selector determines the *baseline* set,
and adaptive thresholds *refine* it based on historical performance.

---

## 11. Extension: Adding a New Rung

To add Rung 7 (e.g., "SecurityAudit"):

1. Add `SecurityAudit = 7` to the `Rung` enum.
2. Add `security_audit: bool` to `RungCaps`.
3. Update `select_rungs()` to include Rung 7 for `Complex` plans.
4. Implement a `SecurityAuditGate` in a new module.
5. Map `Rung::SecurityAudit` to the gate in the orchestrator.

---

## Verification

```bash
cargo test -p roko-gate -- rung
cargo test -p roko-gate -- select_rungs
```
