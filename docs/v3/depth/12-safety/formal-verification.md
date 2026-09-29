# Formal Verification Pipeline

> **v3 depth file** -- `/docs/v3/depth/12-safety/formal-verification.md`
> Canonical source: v1 `docs/v1/11-safety/13-formal-verification.md`
> Status: **Specified**. The five-stage pipeline pattern (fast filter, medium analysis,
> deep proof) is a design-target framework. The existing gate pipeline implements
> analogous staging: compile (fast), test (medium), clippy (medium), diff (fast),
> cost (fast), with oracle rungs for deeper verification.

---

## 1. The Pipeline Pattern

Formal verification is a spectrum of thoroughness vs. speed. A production pipeline uses
multiple stages in sequence, matching verification depth to available time and risk
magnitude:

| Stage | Speed | Depth | Example (code domain) | Example (chain domain) |
|---|---|---|---|---|
| 1. Fast filter | Milliseconds | Syntactic | Clippy, format check | ABI validation |
| 2. Static analysis | Seconds | Structural | Type checking, data flow | Slither, invariant analysis |
| 3. Property testing | Minutes | Behavioral | Fuzz testing, quickcheck | Echidna, stateful fuzzing |
| 4. Symbolic execution | Minutes-hours | Path coverage | KLEE, hevm | hevm symbolic |
| 5. Formal proof | Hours-days | Logical | Coq, Lean, Certora | Certora, Kontrol |

The pipeline runs stages in order and short-circuits: if Stage 1 fails, Stages 2-5 do
not run. This saves time on obviously broken code while reserving deep verification for
code that passes all cheaper checks.

---

## 2. Stage 1: Fast Filter

### Code domain

Clippy and `cargo fmt --check` run in milliseconds. They catch:

- Unused variables and imports.
- Common antipatterns (`.unwrap()` in production paths).
- Formatting violations.
- Dead code.

### Chain domain

ABI validation checks that contract interfaces match expected schemas. Bytecode
decompilation (Heimdall-rs) recovers function selectors and event signatures.

### General

Schema validation for tool call arguments. The `ToolDispatcher` already validates
JSON schemas before execution -- this is Stage 1 verification applied to agent actions.

---

## 3. Stage 2: Static Analysis

### Code domain

Type checking and data flow analysis. Rust's type system provides strong static
guarantees:

- Ownership and borrowing prevent data races.
- Lifetime checking prevents use-after-free.
- Trait bounds enforce interface contracts.

Beyond the type system, tools like `cargo-audit` check dependencies for known
vulnerabilities.

### Chain domain

Slither (Trail of Bits) performs static analysis on Solidity contracts:

- Reentrancy detection.
- Unchecked external calls.
- Integer overflow (pre-Solidity 0.8).
- Access control issues.

### General

The corrigibility pipeline (see `corrigibility-5-head.md`) is a form of static analysis:
it evaluates action descriptions against structural properties without executing the
action.

---

## 4. Stage 3: Property Testing

### Code domain

Cargo test, property-based testing (proptest, quickcheck), and integration tests. The
gate pipeline's compile and test rungs implement this stage for agent-produced code.

### Chain domain

Echidna (Trail of Bits) performs stateful property-based fuzzing of smart contracts:

- Generates random transaction sequences.
- Checks user-defined invariants after each sequence.
- Guided by coverage feedback.

### General

The gate pipeline's oracle rungs (4-6) provide property testing for agent behavior:
domain-specific checks that verify the agent's output against ground-truth properties.

---

## 5. Stage 4: Symbolic Execution

### Code domain

Symbolic execution (KLEE, MIRI) explores all feasible execution paths through a program,
checking for assertion violations, memory errors, and undefined behavior. Path explosion
limits practical applicability to bounded programs.

### Chain domain

hevm (dapptools) provides symbolic execution for EVM bytecode. It explores all possible
transaction paths and checks assertions expressed in Solidity.

### General

For agent verification, symbolic execution of the task DAG checks all possible execution
orderings of parallel tasks, verifying that no ordering violates safety properties.

---

## 6. Stage 5: Formal Proof

### Code domain

Formal proofs in Coq, Lean, or Isabelle/HOL verify mathematical properties of algorithms.
For a coding agent, this means proving that the generated code satisfies its specification.

### Chain domain

Certora Prover verifies smart contract properties using a custom specification language
(CVL). It proves that all possible states satisfy defined rules, providing the strongest
available guarantee.

### General

The temporal logic properties from `temporal-logic.md` are candidates for formal
verification: proving that the agent loop invariant holds for all possible event sequences.

---

## 7. Integration with the Gate Pipeline

The existing gate pipeline maps to the five-stage pattern:

| Gate rung | Pipeline stage | What it checks |
|---|---|---|
| Rung 1: Compile | Stage 1 (fast filter) | Syntactic correctness |
| Rung 2: Test | Stage 3 (property testing) | Behavioral correctness |
| Rung 3: Clippy | Stage 2 (static analysis) | Code quality |
| Rung 4: Diff | Stage 1 (fast filter) | Change scope |
| Rung 5: Cost | Stage 1 (fast filter) | Budget compliance |
| Rungs 4-6: Oracle | Stage 3/4 | Domain-specific verification |

The gate pipeline already implements the short-circuit pattern: if compile fails, test
does not run. The formal verification pipeline extends this pattern to deeper stages
that the current gates do not cover.

---

## 8. Structural Invariants

The following invariants are candidates for formal verification:

### 8.1 Corrigibility ordering

The five-head ordering is immutable. Formally: the set of head priorities is a fixed
total order `{1, 2, 3, 4, 5}` that cannot be modified at runtime.

### 8.2 Taint lattice

The join operator satisfies commutativity, associativity, idempotence, and monotonicity.
These properties are amenable to automated theorem proving.

### 8.3 Capability narrowing

Capability intersection is a pure narrowing operation: the effective capability set is
always a subset of each individual layer's capability set.

### 8.4 Budget non-negativity

The remaining budget in every dimension is non-negative. Once exhausted, no further
actions are permitted in that dimension.

---

## Academic References

| Paper | Contribution |
|---|---|
| King (1976), "Symbolic Execution and Program Testing" | Foundational symbolic execution |
| Clarke, Grumberg & Peled (1999), "Model Checking" | Model checking theory |
| de Moura & Bjorner (2008), "Z3: An Efficient SMT Solver" | SMT solving for verification |
| Trail of Bits, "Slither" | Smart contract static analysis |
| Trail of Bits, "Echidna" | Smart contract property-based fuzzing |

---

## Implementation References

| Component | Location |
|---|---|
| Gate pipeline | `crates/roko-gate/` |
| Gate dispatch | `crates/roko-cli/src/runner/gate_dispatch.rs` |
| Corrigibility invariants | `crates/roko-core/src/corrigibility.rs` |
| Taint lattice | `crates/roko-core/src/provenance.rs` |
| Capability intersection | `crates/roko-agent/src/safety/capabilities.rs` |
