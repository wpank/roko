# Gate Trait -- The Verify Protocol

> Depth file for [07-GATES.md](../../07-GATES.md) section 1.
> Source: `crates/roko-core/src/traits.rs`, `crates/roko-gate/src/`

---

## 1. The Trait Signature

```rust
// crates/roko-core/src/traits.rs

pub trait Gate: Send + Sync {
    /// Verify the Signal and return a verdict.
    async fn verify(&self, signal: &Signal, ctx: &Context) -> Verdict;

    /// Human-readable name (appears in verdicts).
    fn name(&self) -> &str;
}
```

Two methods. That is the entire surface area. Every verification step in Roko
-- from a 5 ms regex check to a 15-minute integration test suite -- implements
this trait.

### 1.1 Why Two Methods, Not Three

There is no `fn cost(&self) -> Cost`, no `fn timeout(&self) -> Duration`, no
`fn supports(&self, signal: &Signal) -> bool`. Those concerns belong
elsewhere:

| Concern | Where it lives | Why not in the trait |
|---------|---------------|---------------------|
| Cost estimation | `Rung` ordering | Static property of the rung, not the gate instance |
| Timeout | Per-gate field (`timeout_ms`) | Configuration, not interface |
| Applicability | `RungCaps` + `PlanComplexity` | Selector decides; gate always runs if selected |
| Error handling | Inside `verify()` via `Verdict::fail()` | Gates are total functions (see section 3) |

Adding methods to the trait would force every implementation to carry concerns
that belong to the orchestrator. The minimal trait keeps gates composable.

---

## 2. Verdict, Not Result

### 2.1 The Design Decision

`verify()` returns `Verdict`, not `Result<Verdict>`. This is the single most
important design decision in the gate system and it diverges deliberately from
idiomatic Rust.

**Gate failure is a verdict, not an error.**

When `cargo check` reports a compilation error, that is not infrastructure
failure -- it is ground truth. The gate encodes the outcome into
`Verdict::fail()` with a reason string and optional error digest. The caller
never handles two failure paths; there is only one: the verdict.

### 2.2 Consequences

Every downstream consumer of gate output -- the pipeline, the ratchet, the
adaptive threshold system, the feedback filter, the forensic replay builder --
receives a `Verdict` and acts on it. None of them pattern-match on
`Result::Err`. This eliminates an entire class of error-handling code:

```rust
// What does NOT exist anywhere in the gate system:
match gate.verify(signal, ctx).await {
    Ok(verdict) => { ... }
    Err(e) => { ... }   // never written
}

// What every consumer actually writes:
let verdict = gate.verify(signal, ctx).await;
if verdict.passed { ... } else { ... }
```

### 2.3 How Infrastructure Failures Become Verdicts

| Failure mode | Verdict |
|-------------|---------|
| Cannot spawn subprocess | `Verdict::fail("spawn failed: permission denied")` |
| Subprocess times out | `Verdict::fail("timed out after 600000 ms")` |
| Malformed signal body | `Verdict::fail("signal body is not a GatePayload: ...")` |
| Test suite: 3 failures | `Verdict::fail("test foo::bar ... FAILED; ...")` |
| I/O error reading files | `Verdict::fail("I/O error: ...")` |
| OOM during compilation | `Verdict::fail("process killed (signal 9)")` |

All six are verdicts. The pipeline aggregates them uniformly.

---

## 3. The Verdict Type

```rust
pub struct Verdict {
    pub passed: bool,                    // Did the gate pass?
    pub gate: String,                    // Name of the gate
    pub reason: String,                  // Human-readable explanation
    pub detail: Option<String>,          // Full output for debugging
    pub error_digest: Option<String>,    // Machine-parseable error summary
    pub duration_ms: u64,                // Wall-clock time
    pub test_count: Option<TestCount>,   // Parsed test counts
}
```

### 3.1 Construction Patterns

Every gate implementation follows the same builder pattern:

```rust
// Passing verdict
Verdict::pass(&self.name)
    .with_detail(combined_output)
    .with_duration(elapsed_ms)

// Failing verdict with reason
Verdict::fail(&self.name, reason_string)
    .with_detail(combined_output)
    .with_duration(elapsed_ms)

// Failing verdict with machine-parseable digest
Verdict::fail(&self.name, "3 symbol expectations unmet")
    .with_error_digest(digest)
    .with_detail(digest.clone())
    .with_duration(elapsed_ms)

// Test gate attaches parsed counts
verdict.with_test_count(TestCount::new(passed, failed, ignored))
```

### 3.2 The `detail` Field

The `detail` field carries the full subprocess output (stdout + stderr
concatenated). It is the raw evidence that supports the verdict. Downstream
consumers use it:

- `feedback_for_agent()` parses it into structured `GateFeedback`
- `ArtifactStore` hashes it for content-addressed storage
- `ForensicReplayBuilder` stores it for causal chain reconstruction
- The TUI renders it in the gate output panel

### 3.3 The `error_digest` Field

Machine-parseable summary of the error. Used by:

- The ratchet to detect identical regressions across attempts
- The experiment system to cluster failures by type
- The replan engine to detect same-signature failure streaks

### 3.4 The `test_count` Field

```rust
pub struct TestCount {
    pub passed: u64,
    pub failed: u64,
    pub ignored: u64,
}
```

Only populated by gates that run test suites (`TestGate`,
`GeneratedTestGate`, `IntegrationGate`). Enables downstream policies that
distinguish "9/10 tests pass" from "0/10 tests pass" -- both are
`Verdict::fail()` but the former is much closer to success.

---

## 4. The Gate Contract

Every gate implementation must satisfy four invariants.

### 4.1 Total Function

`verify()` must always return a `Verdict`. It must not panic, must not hang
indefinitely. Every gate enforces a timeout via `tokio::time::timeout` and
converts timeout expiration into `Verdict::fail()`.

Timeout defaults by gate:

| Gate | Timeout | Rationale |
|------|---------|-----------|
| CompileGate | 600,000 ms (10 min) | Full workspace rebuild |
| TestGate | 900,000 ms (15 min) | Large test suites |
| ClippyGate | 300,000 ms (5 min) | Lint analysis |
| ShellGate | 300,000 ms (5 min) | General subprocess |
| SymbolGate | N/A (no subprocess) | Pure file I/O |

### 4.2 Deterministic on Identical Inputs

Given the same signal body and filesystem state, a gate produces the same
verdict. Gates do not use randomness. The sole exception is `LlmJudgeGate`,
which has its own reproducibility constraints (temperature, seed).

### 4.3 Side-Effect Free on Source

Gates read the filesystem and run subprocesses, but never modify the source
code being verified. Build artifacts go into `CARGO_TARGET_DIR` or the
`ArtifactStore`, not the source tree. A gate that rewrites source to make it
pass defeats its purpose.

### 4.4 Duration Tracking

Every verdict carries `duration_ms`. This feeds:

- Adaptive threshold retry budgets
- Per-gate timing telemetry
- Efficiency event logging
- Dashboard verification health display

---

## 5. Why `async`

Gates shell out to external tools: compilers, test runners, linters, static
analyzers. These are I/O-bound operations measured in seconds to minutes.
Making `verify()` async allows:

1. Running multiple independent gates concurrently (the `ParallelGate`
   wrapper)
2. Applying timeouts via `tokio::time::timeout` without blocking the executor
3. Composing with the rest of Roko's async runtime (agent dispatch, signal
   persistence, SSE delivery)

Every concrete gate implementation uses `#[async_trait]` from the
`async-trait` crate to satisfy the trait's async method.

---

## 6. Why `Send + Sync`

Gates are composed into pipelines (`GatePipeline`) that may be shared across
tasks in the plan executor. The `Send + Sync` bounds ensure gates can be
stored in `Vec<Box<dyn Gate>>` and passed between threads safely.

This is a load-bearing requirement: the graph engine's `GatePipelineCell`
(#250) holds gates across await points in a multi-threaded Tokio runtime.

---

## 7. The `name()` Method

```rust
fn name(&self) -> &str;
```

Every verdict carries the name of the gate that produced it. Three purposes:

1. **Traceability.** The agent (and operator) sees which gate failed:
   `compile:cargo` vs `test:cargo` vs `clippy:cargo` vs `diff` vs `symbol`.
   Different gates require different remediation.

2. **Ratcheting.** `GateRatchet` tracks the highest rung passed per plan.
   Gate names associate verdicts with rungs via `rung_for_gate_name()`.

3. **Adaptive thresholds.** `AdaptiveThresholds` tracks per-rung pass rates.
   Gate names allow the threshold system to correlate verdicts with rungs for
   EMA updates.

Naming convention: `category:tool` format.

| Examples |
|----------|
| `compile:cargo`, `compile:npm`, `compile:go` |
| `test:cargo`, `test:npm`, `test:go` |
| `clippy:cargo` |
| `shell:true`, `shell:custom_script` |
| `diff`, `symbol`, `format_check` |

---

## 8. Gates vs Scorers

Both Gates and Scorers evaluate signals, but they serve fundamentally
different roles:

| Dimension | Gate | Scorer |
|-----------|------|--------|
| Output | `Verdict` (pass/fail + metadata) | `Score` (numeric, 0.0-1.0) |
| Truth source | External tool (compiler, test runner) | Internal heuristic or model |
| Determinism | Deterministic (same code, same result) | May be probabilistic |
| Cost | High (subprocess spawn, seconds-minutes) | Low (computation, milliseconds) |
| Role in loop | Verification (ground truth) | Evaluation (estimation) |

A Scorer might estimate "this code looks 80% correct." A Gate runs the
compiler and says definitively "this code compiles" or "this code does not
compile." The Scorer's estimate guides routing; the Gate's verdict is truth.

---

## 9. Position in the Universal Loop

The universal loop: **query -> score -> route -> compose -> act -> verify ->
write -> react**. Gates occupy the **verify** step.

```
Agent produces output (act)
    |
Gate pipeline verifies output (verify)
    |
Verdict flows to:
    -> Substrate (write: persist verdict as signal)
    -> Scorer (react: update scoring model)
    -> Router (react: update bandit arms)
    -> Composer (react: adjust prompt sections)
    -> Ratchet (react: track highest rung passed)
    -> Adaptive thresholds (react: update per-rung EMA)
    -> Agent feedback (react: filter output for agent context)
    -> Telemetry (react: emit observable event)
```

The gate verdict drives eight different adaptation mechanisms. This is why
returning `Verdict` directly (not `Result<Verdict>`) matters -- every
downstream consumer expects a definitive answer.

---

## 10. Relationship to the GVU Framework

The Generation-Verification-Update (GVU) framework (Song et al., ICLR 2025)
proves: **self-improvement succeeds when the verifier is strong, not when the
generator is strong.** The Variance Inequality shows that an oracle verifier
(verification noise sigma_V approximately 0) enables improvement despite
arbitrarily high generation noise.

Compilers and test suites are oracle verifiers for their respective
properties -- zero false positive rate. This is why Roko invests in 19 gates
and 7 rungs rather than relying solely on better prompts: **the returns to
stronger verification compound, while the returns to stronger generation
plateau.**

---

## 11. Multi-Language Extensibility

The Gate trait is language-agnostic. The Signal body carries a `GatePayload`
with `BuildSystem` (Cargo, Npm, Go, Make). Only the concrete implementations
know about specific build systems.

Adding support for a new language requires:

1. Adding a `BuildSystem` variant (e.g., `BuildSystem::Gradle`)
2. Implementing `check_args()`, `test_args()`, `lint_args()` for the variant
3. Optionally: a new `parse_*_test_counts()` for the test runner's output

No changes to the Gate trait are needed.

---

## Verification

```bash
cargo test -p roko-gate
cargo check -p roko-gate
cargo clippy -p roko-gate --no-deps -- -D warnings
```
